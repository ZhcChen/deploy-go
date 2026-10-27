//! 旧单目标 Git 任务经真实授权、HTTP 上传/下载和 Agent 准入到模拟 executor。
use super::*;
use crate::{agents::auth::token_hash, artifacts::ArtifactStore, config::ArtifactConfig, db};
use deploy_go_agent::{
    artifact_transfer::{ArtifactTransferClient, PreparedArchive},
    connection::{MessageHandler, envelope},
    executor::Executor,
    executor_client::ExecutorClient,
    task_handler::TaskHandler,
};
use deploy_go_agent_executor::protocol::{
    MAX_FRAME_BYTES, PROTOCOL_VERSION, ReleaseExitedResponse, ReleaseJobState,
    ReleaseOutputResponse, ReleaseStartedResponse, Request, Response, read_request, write_message,
};
use deploy_go_agent_protocol::ReleaseAuthorizationResponse;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use std::{path::Path, process::Command, sync::Arc};
use tokio::{net::UnixListener, sync::mpsc};

struct Access;
#[async_trait::async_trait]
impl deploy_go_agent::token_refresh::AccessProvider for Access {
    async fn prepare(
        &self,
    ) -> Result<
        deploy_go_agent::token_refresh::PreparedAccess,
        deploy_go_agent::token_refresh::TokenRefreshError,
    > {
        Ok(deploy_go_agent::token_refresh::PreparedAccess {
            access_token: "handoff-fixture-token".into(),
            access_expires_at: "2099-01-01T00:00:00Z".into(),
            rotation_id: None,
        })
    }
    async fn commit(
        &self,
        _: &str,
    ) -> Result<(), deploy_go_agent::token_refresh::TokenRefreshError> {
        Ok(())
    }
}

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[tokio::test]
async fn single_git_handoff_uploads_downloads_and_enters_privileged_release_once() {
    for separate_target in [false, true] {
        run_handoff(separate_target).await;
    }
}

async fn run_handoff(separate_target: bool) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let repo = root.join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    // deploy-go 自身合成脚本；模拟 executor 不执行任何业务命令。
    std::fs::write(repo.join("Makefile"), "deploy-go-release:\n\t@true\n").unwrap();
    git(&repo, &["add", "Makefile"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-qm",
            "fixture",
        ],
    );
    let sha = git(&repo, &["rev-parse", "HEAD"]);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('user','user','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')")
        .execute(&pool)
        .await
        .unwrap();
    for name in ["build", "target"] {
        sqlx::query(
            "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?,?,?,'online')",
        )
        .bind(name)
        .bind(name)
        .bind(root.to_str().unwrap())
        .bind(root.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,protocol_version,capabilities_json,connection_generation) VALUES(?,?,18,'[\"pty_terminal\",\"privileged_release\"]',1)")
            .bind(name).bind(name).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO agent_credential_families(id,agent_id) VALUES(?,?)")
            .bind(name)
            .bind(name)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agent_access_sessions(id,family_id,agent_id,token_hash,token_key_version,expires_at) VALUES(?,?,?,?,1,'2099-01-01T00:00:00Z')")
            .bind(name).bind(name).bind(name).bind(token_hash("access", if name == "build" { "handoff-fixture-token" } else { "handoff-target-token" })).execute(&pool).await.unwrap();
    }
    // 两个身份使用不同 token_hash，下载客户端按目标身份取 token。
    sqlx::query("UPDATE agent_access_sessions SET token_hash=? WHERE agent_id='target'")
        .bind(token_hash("access", "handoff-target-token"))
        .execute(&pool)
        .await
        .unwrap();
    let node = if separate_target { "target" } else { "build" };
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,execution_mode,environment,script_path,timeout_seconds,status) VALUES('dest','app',?,'two_stage','test','/unused',60,'active')")
        .bind(node).execute(&pool).await.unwrap();
    let snapshot = json!({"execution_mode":"two_stage","target_id":"dest",
        "target":{"application_id":"app","node_id":node,"environment":"test","timeout_seconds":60},
        "source":{"repository_url":repo.to_str().unwrap(),"resolved_commit_sha":sha,"build_agent_id":"build"},
        "two_stage":{"release_version":"fixture-1","modules":["api"]}});
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment','app','dest','user','queued','queued','fixture-idem','hash',?,?)")
        .bind("a".repeat(64)).bind(snapshot.to_string()).execute(&pool).await.unwrap();
    let store = ArtifactStore::initialize(ArtifactConfig {
        root: temp.path().join("store"),
        max_file_bytes: 1024 * 1024,
        max_total_bytes: 1024 * 1024,
        max_files: 16,
        max_chunk_bytes: 1024 * 1024,
        upload_ttl_seconds: 600,
        retention_ttl_seconds: 3600,
    })
    .unwrap();
    let signer = deploy_go_release_authorization::ReleaseSigner::from_seed([10; 32]);
    let state = AppState::new(pool.clone())
        .with_artifact_store(store)
        .with_cross_node_artifacts_enabled(true)
        .with_release_signer(signer);
    ensure_deployment_task(&state, "deployment")
        .await
        .unwrap()
        .unwrap();
    let (prepare_id, prepare_json): (String, String) =
        sqlx::query_as("SELECT id,payload_json FROM agent_tasks WHERE stage='prepare'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let TaskPayload::DeploymentPrepare(prepare) = serde_json::from_str(&prepare_json).unwrap()
    else {
        panic!("prepare")
    };
    sqlx::query("UPDATE agent_tasks SET status='running' WHERE id=?")
        .bind(&prepare_id)
        .execute(&pool)
        .await
        .unwrap();
    let content = b"deploy-go synthetic artifact\n";
    let manifest = json!({"schema_version":1,"release_version":"fixture-1","commit_sha":sha,
        "artifacts":[{"module":"api","path":"api/app.bin","sha256":format!("{:x}",Sha256::digest(content)),"size":content.len()}]}).to_string();
    let mut bytes = Vec::new();
    {
        let mut tar = tar::Builder::new(&mut bytes);
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "api/app.bin", content.as_slice())
            .unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_size(manifest.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "deploy-go-artifact.json", manifest.as_bytes())
            .unwrap();
        tar.finish().unwrap();
    }
    let path = temp.path().join("source.tar");
    std::fs::write(&path, &bytes).unwrap();
    let notice = ArtifactPrepared {
        task_id: prepare_id.clone(),
        authorization_id: prepare.artifact_upload.unwrap().authorization_id,
        deployment_id: "deployment".into(),
        manifest_digest: format!("{:x}", Sha256::digest(manifest.as_bytes())),
        manifest_json: manifest,
        total_size: content.len() as u64,
        file_count: 1,
        archive_size: bytes.len() as u64,
        archive_digest: format!("{:x}", Sha256::digest(&bytes)),
    };
    let lease = authorize_artifact_upload(&state, "build", &notice)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base: url::Url = format!("http://{}/", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let http_app = crate::app(state.clone());
    let server = tokio::spawn(async move { axum::serve(listener, http_app).await.unwrap() });
    // AbortOnDrop 确保测试断言失败时也关闭服务，不遗留后台任务。
    let _server_guard = AbortOnDrop(server.abort_handle());
    let client = ArtifactTransferClient::with_client(
        base.clone(),
        Arc::new(Access),
        true,
        reqwest::Client::builder().no_proxy().build().unwrap(),
    );
    client
        .upload(&lease, &PreparedArchive { path, notice })
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='succeeded' WHERE id=?")
        .bind(&prepare_id)
        .execute(&pool)
        .await
        .unwrap();
    let release_id = ensure_deployment_task(&state, "deployment")
        .await
        .unwrap()
        .unwrap();
    ensure_deployment_task(&state, "deployment").await.unwrap();
    let (payload,digest,deadline,idem):(String,String,String,String)=sqlx::query_as("SELECT payload_json,payload_digest,deadline_at,idempotency_key FROM agent_tasks WHERE id=?").bind(&release_id).fetch_one(&pool).await.unwrap();
    let task: TaskPayload = serde_json::from_str(&payload).unwrap();
    let TaskPayload::DeploymentRelease(release) = &task else {
        panic!("release")
    };
    let download = release.artifact_download.as_ref().unwrap();
    assert_eq!(release.privileged_context.as_ref().unwrap().agent_id, node);
    let archived = temp.path().join("download.tar");
    // 对不同节点客户端使用目标会话，同节点复用构建会话。
    let access = Arc::new(TargetAccess(separate_target));
    let transfer = ArtifactTransferClient::with_client(
        base,
        access,
        true,
        reqwest::Client::builder().no_proxy().build().unwrap(),
    );
    transfer
        .download(&download.lease_id, &archived, &download.archive_digest)
        .await
        .unwrap();
    assert_eq!(std::fs::read(archived).unwrap(), bytes);
    let socket = temp.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let starts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = starts.clone();
    let executor = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream, MAX_FRAME_BYTES)
                .await
                .unwrap()
                .unwrap();
            let response = match request {
                Request::ReleaseStart(request) => {
                    seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    assert!(!request.authorization.is_empty());
                    Response::ReleaseStarted(ReleaseStartedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        state: ReleaseJobState::Running,
                    })
                }
                Request::ReleaseOutput(request) => Response::ReleaseOutput(ReleaseOutputResponse {
                    version: PROTOCOL_VERSION,
                    job_id: request.job_id,
                    frames: if request.after_sequence == 0 {
                        vec![deploy_go_agent_executor::protocol::ReleaseOutputFrame {
                        sequence: 1, stream: deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout,
                        data: concat!(
                            "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                            "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n",
                            "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.module.started\",\"module\":\"api\",\"module_name\":\"API\"}\n",
                            "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.module.succeeded\",\"module\":\"api\",\"module_name\":\"API\"}\n"
                        ).as_bytes().to_vec(),
                    }]
                    } else {
                        vec![]
                    },
                    truncated: false,
                }),
                Request::ReleaseStatus(request) => Response::ReleaseExited(ReleaseExitedResponse {
                    version: PROTOCOL_VERSION,
                    job_id: request.job_id,
                    state: ReleaseJobState::Succeeded,
                    last_sequence: 1,
                    exit_code: Some(0),
                    reason: "process_exited".into(),
                }),
                _ => panic!("unexpected executor request"),
            };
            write_message(&mut stream, &response, MAX_FRAME_BYTES)
                .await
                .unwrap();
        }
    });
    let _executor_guard = AbortOnDrop(executor.abort_handle());
    let handler = Arc::new(
        TaskHandler::new(Executor::new(root.join("tasks")).unwrap())
            .with_artifact_transfer(transfer)
            .with_privileged_release_executor(ExecutorClient::new(socket)),
    );
    sqlx::query("UPDATE agent_tasks SET status='delivered' WHERE id=?")
        .bind(&release_id)
        .execute(&pool)
        .await
        .unwrap();
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(TaskDispatch {
                task_id: release_id.clone(),
                idempotency_key: idem,
                deadline_at: deadline,
                payload_digest: digest,
                task,
            })),
            sender.clone(),
        )
        .await
        .unwrap();
    let mut authorizations = 0;
    loop {
        let message = tokio::time::timeout(std::time::Duration::from_secs(20), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        match message {
            Message::ReleaseAuthorizationRequest(request) => {
                authorizations += 1;
                let token = authorize_privileged_release(&state, node, 1, &request)
                    .await
                    .unwrap();
                handler
                    .handle(
                        envelope(Message::ReleaseAuthorizationResponse(
                            ReleaseAuthorizationResponse {
                                task_id: request.task_id,
                                authorization_id: request.authorization_id,
                                authorization: Some(token),
                                error_code: None,
                            },
                        )),
                        sender.clone(),
                    )
                    .await
                    .unwrap();
            }
            Message::TaskAck(ack) => assert_eq!(
                ack.disposition,
                TaskAckDisposition::Accepted,
                "{:?}",
                ack.error_code
            ),
            Message::TaskResult(result) => {
                assert_eq!(
                    result.status,
                    deploy_go_agent_protocol::TaskTerminalStatus::Succeeded,
                    "{:?}",
                    result.error_code
                );
                break;
            }
            _ => {}
        }
    }
    assert_eq!(authorizations, 1);
    assert_eq!(starts.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks WHERE stage='release'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

struct AbortOnDrop(tokio::task::AbortHandle);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
struct TargetAccess(bool);
#[async_trait::async_trait]
impl deploy_go_agent::token_refresh::AccessProvider for TargetAccess {
    async fn prepare(
        &self,
    ) -> Result<
        deploy_go_agent::token_refresh::PreparedAccess,
        deploy_go_agent::token_refresh::TokenRefreshError,
    > {
        let mut access = Access.prepare().await?;
        if self.0 {
            access.access_token = "handoff-target-token".into();
        }
        Ok(access)
    }
    async fn commit(
        &self,
        _: &str,
    ) -> Result<(), deploy_go_agent::token_refresh::TokenRefreshError> {
        Ok(())
    }
}

#[tokio::test]
async fn disabling_transfer_only_fails_unscheduled_target_and_preserves_running_release() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('user','user','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node','Node','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id) VALUES('agent','node')")
        .execute(&pool)
        .await
        .unwrap();
    for id in ["running", "waiting"] {
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,target_code,environment,execution_mode,script_path,timeout_seconds,status) VALUES(?,'app','node',?,?,'two_stage','/unused',60,'active')").bind(id).bind(id).bind(if id == "running" {"test"} else {"staging"}).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment','running','user','running','targets_running','idem','hash','snapshot','{\"targets\":[]}')").execute(&pool).await.unwrap();
    for (id, status) in [("running", "running"), ("waiting", "pending")] {
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,status) VALUES(?,'deployment',?,'node','agent',?)").bind(id).bind(id).bind(status).execute(&pool).await.unwrap();
    }
    let state = AppState::new(pool.clone());
    let snapshot = json!({"execution_mode":"two_stage","_target_run_id":"waiting"});
    assert!(
        create_stage_task(&state, "deployment", "release", &snapshot.to_string())
            .await
            .unwrap()
            .is_none()
    );
    let running: String =
        sqlx::query_scalar("SELECT status FROM deployment_target_runs WHERE id='running'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(running, "running");
    let waiting: (String, String) =
        sqlx::query_as("SELECT status,error_code FROM deployment_target_runs WHERE id='waiting'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        waiting,
        ("failed".into(), "cross_node_artifacts_disabled".into())
    );
    let deployment: String =
        sqlx::query_scalar("SELECT status FROM deployments WHERE id='deployment'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(deployment, "running");
}
