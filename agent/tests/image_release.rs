use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, Response, StatusCode, header},
    routing::get,
};
use deploy_go_agent::{
    artifact_transfer::ArtifactTransferClient,
    connection::{MessageHandler, envelope},
    executor::Executor,
    executor_client::ExecutorClient,
    task_handler::TaskHandler,
    token_refresh::{AccessProvider, PreparedAccess, TokenRefreshError},
};
use deploy_go_agent_executor::protocol::{
    ErrorResponse, MAX_FRAME_BYTES, PROTOCOL_VERSION, ReleaseExitedResponse, ReleaseJobState,
    ReleaseOutputResponse, ReleaseStartedResponse, Request, Response as ExecutorResponse,
    read_request, write_message,
};
use deploy_go_agent_protocol::{
    ArtifactDownloadRequest, DeploymentReleaseTask, Environment, MakeTarget, Message,
    ReleaseAuthorizationResponse, ReleaseCheckoutMode, RequiredEnvVersion, TaskAckDisposition,
    TaskDispatch, TaskPayload, TaskTerminalStatus,
};
use deploy_go_container_template::{
    ImageDeploySpec, ImageTemplate, build_platform_artifact, checkout_digest,
};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::{Mutex, mpsc},
};

fn test_tempdir() -> tempfile::TempDir {
    let root = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("tmp")
        .join("image-release-tests");
    fs::create_dir_all(&root).unwrap();
    tempfile::Builder::new()
        .prefix("case-")
        .tempdir_in(root)
        .unwrap()
}

struct StaticAccess;

#[async_trait::async_trait]
impl AccessProvider for StaticAccess {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        Ok(PreparedAccess {
            access_token: "image-access-token-never-persisted".to_owned(),
            access_expires_at: "2099-01-01T00:00:00Z".to_owned(),
            rotation_id: None,
        })
    }

    async fn commit(&self, _rotation_id: &str) -> Result<(), TokenRefreshError> {
        Ok(())
    }
}

#[derive(Clone)]
struct ArtifactFixture {
    archive: Arc<Vec<u8>>,
}

async fn start_artifact_server(archive: Vec<u8>) -> (url::Url, tokio::task::JoinHandle<()>) {
    let fixture = ArtifactFixture {
        archive: Arc::new(archive),
    };
    let app = Router::new()
        .route(
            "/api/v1/agent/artifact-leases/{id}/download",
            get(download_range),
        )
        .with_state(fixture);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{address}/").parse().unwrap(), server)
}

async fn download_range(
    State(state): State<ArtifactFixture>,
    _: AxumPath<String>,
    headers: HeaderMap,
) -> Response<Body> {
    let range = headers.get(header::RANGE).unwrap().to_str().unwrap();
    let start = range
        .strip_prefix("bytes=")
        .unwrap()
        .strip_suffix('-')
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let total = state.archive.len();
    let (body, end) = if start == 0 {
        let split = total / 2;
        (Body::from(state.archive[..split].to_vec()), split - 1)
    } else {
        (Body::from(state.archive[start..].to_vec()), total - 1)
    };
    let mut response = Response::new(body);
    *response.status_mut() = StatusCode::PARTIAL_CONTENT;
    response.headers_mut().insert(
        header::CONTENT_RANGE,
        format!("bytes {start}-{end}/{total}").parse().unwrap(),
    );
    response
}

async fn write_response(stream: &mut UnixStream, response: ExecutorResponse) {
    write_message(stream, &response, MAX_FRAME_BYTES)
        .await
        .unwrap();
}

async fn serve_executor(listener: UnixListener, starts: Arc<AtomicUsize>) {
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream, MAX_FRAME_BYTES)
            .await
            .unwrap()
            .unwrap();
        match request {
            Request::ReleaseStart(request) => {
                starts.fetch_add(1, Ordering::SeqCst);
                write_response(
                    &mut stream,
                    ExecutorResponse::ReleaseStarted(ReleaseStartedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        state: ReleaseJobState::Running,
                    }),
                )
                .await;
            }
            Request::ReleaseOutput(request) => {
                write_response(
                    &mut stream,
                    ExecutorResponse::ReleaseOutput(ReleaseOutputResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        frames: vec![deploy_go_agent_executor::protocol::ReleaseOutputFrame {
                            sequence: request.after_sequence + 1,
                            stream: deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout,
                            data: concat!(
                                "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                                "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n"
                            )
                            .as_bytes()
                            .to_vec(),
                        }],
                        truncated: false,
                    }),
                )
                .await;
            }
            Request::ReleaseStatus(request) => {
                write_response(
                    &mut stream,
                    ExecutorResponse::ReleaseExited(ReleaseExitedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        state: ReleaseJobState::Succeeded,
                        exit_code: Some(0),
                        reason: "process_exited".into(),
                        last_sequence: 1,
                    }),
                )
                .await;
            }
            _ => {
                write_response(
                    &mut stream,
                    ExecutorResponse::Error(ErrorResponse {
                        version: PROTOCOL_VERSION,
                        code: "unexpected_request".into(),
                    }),
                )
                .await;
            }
        }
    }
}

fn image_task(task_id: &str, required_env: Vec<RequiredEnvVersion>) -> DeploymentReleaseTask {
    DeploymentReleaseTask {
        deployment_id: "deployment_image".into(),
        target_code: "prod".into(),
        work_root: format!("/srv/tasks/{task_id}"),
        checkout_dir: format!("/srv/tasks/{task_id}/checkout"),
        artifact_dir: format!("/srv/tasks/{task_id}/artifact"),
        environment: Environment::Production,
        release_version: "release-image-1".into(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        modules: vec!["redis".into()],
        make_target: MakeTarget::DeployGoRelease,
        timeout_seconds: 60,
        cancel_file: String::new(),
        privileged: true,
        privileged_context: Some(deploy_go_agent_protocol::PrivilegedReleaseContext {
            target_run_id: "run".into(),
            target_id: "target".into(),
            node_id: "node".into(),
            agent_id: "agent".into(),
            snapshot_hash: "a".repeat(64),
        }),
        artifact_download: Some(ArtifactDownloadRequest {
            target_run_id: "run".into(),
            lease_id: "lease_image".into(),
            archive_digest: "a".repeat(64),
            manifest_digest: "b".repeat(64),
        }),
        repository_url: None,
        git_credential_lease_id: None,
        application_slug: (!required_env.is_empty()).then(|| "image-app".into()),
        required_env,
        checkout_mode: ReleaseCheckoutMode::Artifact,
    }
}

fn dispatch(task_id: &str, payload_digest: &str, task: DeploymentReleaseTask) -> TaskDispatch {
    TaskDispatch {
        task_id: task_id.into(),
        idempotency_key: format!("idem_{task_id}_0123456789"),
        deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
        payload_digest: payload_digest.into(),
        task: TaskPayload::DeploymentRelease(task),
    }
}

#[tokio::test]
async fn platform_artifact_release_generates_checkout_and_starts_executor_once() {
    let directory = test_tempdir();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_image_release";
    let payload_digest = "sha256:image_release_payload";
    let spec = ImageDeploySpec {
        template: ImageTemplate::Redis,
        image: "redis:7-alpine".into(),
        host_port: 6379,
        env_files: vec!["compose.env".into(), "redis.env".into()],
    };
    let commit_sha = "0123456789abcdef0123456789abcdef01234567";
    let platform =
        build_platform_artifact(&spec, "release-image-1", commit_sha, directory.path()).unwrap();
    let archive = fs::read(&platform.archive_path).unwrap();
    let (base, artifact_server) = start_artifact_server(archive).await;
    let client = ArtifactTransferClient::new(base, Arc::new(StaticAccess), true);
    let mut task = image_task(task_id, Vec::new());
    task.artifact_download = Some(ArtifactDownloadRequest {
        target_run_id: "run".into(),
        lease_id: "lease_image".into(),
        archive_digest: platform.archive_digest.clone(),
        manifest_digest: platform.manifest_digest.clone(),
    });

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_executor(listener, Arc::clone(&starts)));
    let handler = TaskHandler::new(executor)
        .with_artifact_transfer(client)
        .with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
    let reply_handler = handler.clone();
    let reply_sender = sender.clone();
    let messages = Arc::new(Mutex::new(Vec::new()));
    let reply_messages = Arc::clone(&messages);
    let auth_digests = Arc::new(Mutex::new(Vec::new()));
    let reply_digests = Arc::clone(&auth_digests);
    let reply_done = Arc::new(AtomicBool::new(false));
    let done = Arc::clone(&reply_done);
    tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if let Message::ReleaseAuthorizationRequest(request) = &message {
                reply_digests
                    .lock()
                    .await
                    .push(request.checkout_tree_digest.clone());
                let response = ReleaseAuthorizationResponse {
                    task_id: request.task_id.clone(),
                    authorization_id: request.authorization_id.clone(),
                    authorization: Some("signed-authorization".into()),
                    error_code: None,
                };
                let _ = reply_handler
                    .handle(
                        envelope(Message::ReleaseAuthorizationResponse(response)),
                        reply_sender.clone(),
                    )
                    .await;
            }
            let terminal = matches!(message, Message::TaskResult(_));
            reply_messages.lock().await.push(message);
            if terminal {
                done.store(true, Ordering::SeqCst);
                return;
            }
        }
    });
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch(
                task_id,
                payload_digest,
                task,
            ))),
            sender,
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while !reply_done.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("回复任务未在超时内结束");
    artifact_server.abort();
    server.abort();

    let messages = messages.lock().await.clone();
    assert!(
        messages.iter().any(|message| matches!(
            message,
            Message::TaskAck(ack) if ack.disposition == TaskAckDisposition::Accepted
        )),
        "缺少 Accepted ACK: {messages:?}"
    );
    assert!(
        messages.iter().any(|message| matches!(
            message,
            Message::TaskResult(result) if result.status == TaskTerminalStatus::Succeeded
        )),
        "缺少成功 TaskResult: {messages:?}"
    );
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert_eq!(auth_digests.lock().await.len(), 1);
    assert_eq!(
        auth_digests.lock().await[0],
        checkout_digest(&spec).unwrap()
    );
    let checkout = tasks.join(task_id).join("checkout");
    assert!(checkout.join("Makefile").is_file());
    assert!(checkout.join("scripts").join("release.sh").is_file());
    assert!(!checkout.join("compose.yaml").exists());
    assert!(!checkout.join(".git").exists());
    let makefile = fs::read_to_string(checkout.join("Makefile")).unwrap();
    assert!(makefile.contains("deploy-go-release"));
}

#[tokio::test]
async fn image_release_env_gate_failure_never_calls_executor() {
    let directory = test_tempdir();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_image_env_gate";
    let payload_digest = "sha256:image_env_gate_payload";
    let task = image_task(
        task_id,
        vec![RequiredEnvVersion {
            file_name: "redis.env".into(),
            env_version: 1,
            digest: "selected-digest".into(),
            action: deploy_go_agent_protocol::EnvSyncAction::Write,
        }],
    );
    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_executor(listener, Arc::clone(&starts)));
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(16);
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch(
                task_id,
                payload_digest,
                task,
            ))),
            sender,
        )
        .await
        .unwrap();
    let Message::TaskAck(ack) = receiver.recv().await.expect("Env gate 失败必须返回 ACK")
    else {
        panic!("Env gate 失败必须返回 ACK");
    };
    server.abort();

    assert_eq!(ack.disposition, TaskAckDisposition::Rejected);
    assert_eq!(ack.error_code.as_deref(), Some("env_gate_failed"));
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    assert!(!tasks.join(task_id).exists());
}
