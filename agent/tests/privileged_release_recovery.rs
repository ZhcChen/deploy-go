use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use deploy_go_agent::{
    connection::{MessageHandler, envelope},
    executor::Executor,
    executor_client::ExecutorClient,
    journal::{JournalState, JournalStore, TransferPhase},
    task_handler::TaskHandler,
};
use deploy_go_agent_executor::protocol::{
    ErrorResponse, MAX_FRAME_BYTES, PROTOCOL_VERSION, ReleaseExitedResponse, ReleaseJobState,
    ReleaseOutputResponse, ReleaseStatusResponse, Request, Response, read_request, write_message,
};
use deploy_go_agent_protocol::{
    ArtifactDownloadRequest, DeploymentReleaseTask, EnvSyncAction, Environment, MakeTarget,
    Message, RequiredEnvVersion, TaskAckDisposition, TaskCancel, TaskDispatch, TaskPayload,
    TaskTerminalStatus,
};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::mpsc,
};

const RESUME_OFFSET: u64 = 4;

fn privileged_task(task_id: &str) -> DeploymentReleaseTask {
    DeploymentReleaseTask {
        deployment_id: "deployment".into(),
        target_code: "test".into(),
        work_root: "/srv/tasks".into(),
        checkout_dir: format!("/srv/tasks/{task_id}/checkout"),
        artifact_dir: format!("/srv/tasks/{task_id}/artifact"),
        environment: Environment::Test,
        release_version: "release-1".into(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        modules: vec!["api".into()],
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
            lease_id: "lease".into(),
            archive_digest: "a".repeat(64),
            manifest_digest: "b".repeat(64),
        }),
        repository_url: Some("https://git.example.test/app.git".into()),
        git_credential_lease_id: None,
        application_slug: None,
        required_env: Vec::new(),
        checkout_mode: deploy_go_agent_protocol::ReleaseCheckoutMode::Git,
        source_materialization: None,
        secret_environment: None,
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

async fn receive_until_result(receiver: &mut mpsc::Receiver<Message>) -> Vec<Message> {
    let mut messages = Vec::new();
    loop {
        let message = tokio::time::timeout(Duration::from_secs(10), receiver.recv())
            .await
            .expect("特权发布恢复结果超时")
            .expect("任务发送通道提前关闭");
        let terminal = matches!(message, Message::TaskResult(_));
        messages.push(message);
        if terminal {
            return messages;
        }
    }
}

async fn write_response(stream: &mut UnixStream, response: Response) {
    write_message(stream, &response, MAX_FRAME_BYTES)
        .await
        .unwrap();
}

async fn serve_resume_executor(listener: UnixListener, unexpected_starts: Arc<AtomicUsize>) {
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream, MAX_FRAME_BYTES)
            .await
            .unwrap()
            .unwrap();
        match request {
            Request::ReleaseOutput(request) => {
                write_response(
                    &mut stream,
                    Response::ReleaseOutput(ReleaseOutputResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        frames: vec![deploy_go_agent_executor::protocol::ReleaseOutputFrame {
                            sequence: request.after_sequence + 1,
                            stream: deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout,
                            data: concat!(
                                "restart-resumed\n",
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
                    Response::ReleaseExited(ReleaseExitedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        state: ReleaseJobState::Succeeded,
                        exit_code: Some(0),
                        reason: "process_exited".into(),
                        last_sequence: RESUME_OFFSET + 1,
                    }),
                )
                .await;
            }
            Request::ReleaseCancel(request) => {
                write_response(
                    &mut stream,
                    Response::Error(ErrorResponse {
                        version: PROTOCOL_VERSION,
                        code: "unexpected_cancel".into(),
                    }),
                )
                .await;
                let _ = request;
            }
            _ => {
                unexpected_starts.fetch_add(1, Ordering::SeqCst);
                write_response(
                    &mut stream,
                    Response::Error(ErrorResponse {
                        version: PROTOCOL_VERSION,
                        code: "unexpected_request".into(),
                    }),
                )
                .await;
            }
        }
    }
}

/// 先回报一次 Running 再回报终态，用于在 monitor 已经接管原 job 之后注入本地持久化故障。
async fn serve_running_then_exit_executor(
    listener: UnixListener,
    output_calls: Arc<AtomicUsize>,
    first_status_served: Arc<AtomicBool>,
    unexpected_starts: Arc<AtomicUsize>,
) {
    let mut status_calls = 0_u32;
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream, MAX_FRAME_BYTES)
            .await
            .unwrap()
            .unwrap();
        match request {
            Request::ReleaseOutput(request) => {
                output_calls.fetch_add(1, Ordering::SeqCst);
                write_response(
                    &mut stream,
                    Response::ReleaseOutput(ReleaseOutputResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        frames: vec![deploy_go_agent_executor::protocol::ReleaseOutputFrame {
                            sequence: request.after_sequence + 1,
                            stream: deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout,
                            data: concat!(
                                "journal-retry\n",
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
                status_calls += 1;
                if status_calls == 1 {
                    first_status_served.store(true, Ordering::SeqCst);
                    write_response(
                        &mut stream,
                        Response::ReleaseStatus(ReleaseStatusResponse {
                            version: PROTOCOL_VERSION,
                            job_id: request.job_id,
                            state: ReleaseJobState::Running,
                            last_sequence: RESUME_OFFSET + 1,
                        }),
                    )
                    .await;
                } else {
                    write_response(
                        &mut stream,
                        Response::ReleaseExited(ReleaseExitedResponse {
                            version: PROTOCOL_VERSION,
                            job_id: request.job_id,
                            state: ReleaseJobState::Succeeded,
                            exit_code: Some(0),
                            reason: "process_exited".into(),
                            last_sequence: RESUME_OFFSET + 1,
                        }),
                    )
                    .await;
                }
            }
            _ => {
                unexpected_starts.fetch_add(1, Ordering::SeqCst);
                return;
            }
        }
    }
}

async fn serve_descriptorless_executor(
    listener: UnixListener,
    unexpected_requests: Arc<AtomicUsize>,
) {
    let (mut output, _) = listener.accept().await.unwrap();
    let request = read_request(&mut output, MAX_FRAME_BYTES)
        .await
        .unwrap()
        .unwrap();
    let Request::ReleaseOutput(request) = request else {
        unexpected_requests.fetch_add(1, Ordering::SeqCst);
        panic!("descriptor recovery must only read the original job output");
    };
    write_response(
        &mut output,
        Response::ReleaseOutput(ReleaseOutputResponse {
            version: PROTOCOL_VERSION,
            job_id: request.job_id,
            frames: Vec::new(),
            truncated: false,
        }),
    )
    .await;

    let (mut status, _) = listener.accept().await.unwrap();
    let request = read_request(&mut status, MAX_FRAME_BYTES)
        .await
        .unwrap()
        .unwrap();
    let Request::ReleaseStatus(request) = request else {
        unexpected_requests.fetch_add(1, Ordering::SeqCst);
        panic!("descriptor recovery must only observe the original job status");
    };
    write_response(
        &mut status,
        Response::ReleaseExited(ReleaseExitedResponse {
            version: PROTOCOL_VERSION,
            job_id: request.job_id,
            state: ReleaseJobState::Succeeded,
            exit_code: Some(0),
            reason: "process_exited".into(),
            last_sequence: 0,
        }),
    )
    .await;
}

/// 取消路径会先发 ReleaseCancel，因此这里只允许 cancel/output/status 三种只读或取消交互，
/// 终态保持节点侧的 succeeded，用于验证缺失 descriptor 时收敛为 interrupted。
async fn serve_cancel_without_descriptor_executor(
    listener: UnixListener,
    unexpected_starts: Arc<AtomicUsize>,
) {
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream, MAX_FRAME_BYTES)
            .await
            .unwrap()
            .unwrap();
        let response = match request {
            Request::ReleaseCancel(_) => Response::Error(ErrorResponse {
                version: PROTOCOL_VERSION,
                code: "cancel_requested".into(),
            }),
            Request::ReleaseOutput(request) => Response::ReleaseOutput(ReleaseOutputResponse {
                version: PROTOCOL_VERSION,
                job_id: request.job_id,
                frames: Vec::new(),
                truncated: false,
            }),
            Request::ReleaseStatus(request) => Response::ReleaseExited(ReleaseExitedResponse {
                version: PROTOCOL_VERSION,
                job_id: request.job_id,
                state: ReleaseJobState::Succeeded,
                exit_code: Some(0),
                reason: "process_exited".into(),
                last_sequence: 0,
            }),
            _ => {
                unexpected_starts.fetch_add(1, Ordering::SeqCst);
                return;
            }
        };
        write_response(&mut stream, response).await;
    }
}

async fn serve_cancel_executor(
    listener: UnixListener,
    cancel_calls: Arc<AtomicUsize>,
    cancel_seen: Arc<AtomicBool>,
    unexpected_starts: Arc<AtomicUsize>,
) {
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream, MAX_FRAME_BYTES)
            .await
            .unwrap()
            .unwrap();
        match request {
            Request::ReleaseOutput(request) => {
                write_response(
                    &mut stream,
                    Response::ReleaseOutput(ReleaseOutputResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        frames: Vec::new(),
                        truncated: false,
                    }),
                )
                .await;
            }
            Request::ReleaseStatus(request) => {
                if cancel_seen.load(Ordering::SeqCst) {
                    write_response(
                        &mut stream,
                        Response::ReleaseExited(ReleaseExitedResponse {
                            version: PROTOCOL_VERSION,
                            job_id: request.job_id,
                            state: ReleaseJobState::Canceled,
                            exit_code: None,
                            reason: "canceled".into(),
                            last_sequence: 0,
                        }),
                    )
                    .await;
                } else {
                    write_response(
                        &mut stream,
                        Response::ReleaseStatus(ReleaseStatusResponse {
                            version: PROTOCOL_VERSION,
                            job_id: request.job_id,
                            state: ReleaseJobState::Running,
                            last_sequence: 0,
                        }),
                    )
                    .await;
                }
            }
            Request::ReleaseCancel(request) => {
                cancel_calls.fetch_add(1, Ordering::SeqCst);
                cancel_seen.store(true, Ordering::SeqCst);
                write_response(
                    &mut stream,
                    Response::Error(ErrorResponse {
                        version: PROTOCOL_VERSION,
                        code: "cancel_requested".into(),
                    }),
                )
                .await;
                let _ = request;
            }
            _ => {
                unexpected_starts.fetch_add(1, Ordering::SeqCst);
                write_response(
                    &mut stream,
                    Response::Error(ErrorResponse {
                        version: PROTOCOL_VERSION,
                        code: "unexpected_request".into(),
                    }),
                )
                .await;
            }
        }
    }
}

async fn serve_transient_executor(
    listener: UnixListener,
    unexpected_starts: Arc<AtomicUsize>,
    output_failures: Arc<AtomicUsize>,
    status_failures: Arc<AtomicUsize>,
) {
    let mut output_attempts = 0_u32;
    let mut status_attempts = 0_u32;
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream, MAX_FRAME_BYTES)
            .await
            .unwrap()
            .unwrap();
        match request {
            Request::ReleaseOutput(request) => {
                output_attempts += 1;
                if output_attempts == 1 {
                    output_failures.fetch_add(1, Ordering::SeqCst);
                    drop(stream);
                    continue;
                }
                let frames = if request.after_sequence == 0 {
                    vec![deploy_go_agent_executor::protocol::ReleaseOutputFrame {
                        sequence: 1,
                        stream:
                            deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout,
                        data: concat!(
                            "transient-retry-resumed\n",
                            "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                            "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n"
                        )
                        .as_bytes()
                        .to_vec(),
                    }]
                } else {
                    Vec::new()
                };
                write_response(
                    &mut stream,
                    Response::ReleaseOutput(ReleaseOutputResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        frames,
                        truncated: false,
                    }),
                )
                .await;
            }
            Request::ReleaseStatus(request) => {
                status_attempts += 1;
                if status_attempts == 1 {
                    status_failures.fetch_add(1, Ordering::SeqCst);
                    drop(stream);
                    continue;
                }
                write_response(
                    &mut stream,
                    Response::ReleaseExited(ReleaseExitedResponse {
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
                unexpected_starts.fetch_add(1, Ordering::SeqCst);
                write_response(
                    &mut stream,
                    Response::Error(ErrorResponse {
                        version: PROTOCOL_VERSION,
                        code: "unexpected_request".into(),
                    }),
                )
                .await;
            }
        }
    }
}

#[tokio::test]
async fn monitor_rejects_wrong_version_and_job_responses_before_accepting_terminal() {
    let directory = tempfile::tempdir().unwrap();
    let executor = Executor::new(directory.path().join("tasks")).unwrap();
    let task_id = "task_monitor_response_binding";
    let digest = "sha256:monitor_binding";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(&executor, task_id, "idem_monitor_binding", digest, &task, 0)
        .await;
    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let status_requests = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&status_requests);
    let server = tokio::spawn(async move {
        let mut outputs = 0;
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            match read_request(&mut stream, MAX_FRAME_BYTES)
                .await
                .unwrap()
                .unwrap()
            {
                Request::ReleaseOutput(request) => {
                    outputs += 1;
                    assert_eq!(request.after_sequence, 0, "不得持久化其他协议或 job 的输出");
                    write_response(
                        &mut stream,
                        Response::ReleaseOutput(ReleaseOutputResponse {
                            version: if outputs == 1 {
                                PROTOCOL_VERSION + 1
                            } else {
                                PROTOCOL_VERSION
                            },
                            job_id: if outputs == 2 {
                                "release_other".into()
                            } else {
                                request.job_id
                            },
                            frames: if outputs <= 2 {
                                vec![deploy_go_agent_executor::protocol::ReleaseOutputFrame {
                            sequence: 1,
                            stream: deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout,
                            data: b"must-not-be-persisted\n".to_vec(),
                        }]
                            } else {
                                vec![]
                            },
                            truncated: false,
                        }),
                    )
                    .await;
                }
                Request::ReleaseStatus(request) => {
                    let attempt = observed.fetch_add(1, Ordering::SeqCst) + 1;
                    write_response(
                        &mut stream,
                        Response::ReleaseExited(ReleaseExitedResponse {
                            version: if attempt == 1 {
                                PROTOCOL_VERSION + 1
                            } else {
                                PROTOCOL_VERSION
                            },
                            job_id: if attempt == 2 {
                                "release_other".into()
                            } else {
                                request.job_id
                            },
                            state: if attempt < 3 {
                                ReleaseJobState::Succeeded
                            } else {
                                ReleaseJobState::Failed
                            },
                            exit_code: Some(if attempt < 3 { 0 } else { 1 }),
                            reason: "process_exited".into(),
                            last_sequence: 0,
                        }),
                    )
                    .await;
                }
                request => panic!("恢复期间不应重发启动: {request:?}"),
            }
        }
    });
    let handler = TaskHandler::new(executor.clone())
        .with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch(task_id, digest, task))),
            sender,
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    server.abort();
    assert!(messages.iter().any(|message| matches!(message, Message::TaskResult(result) if result.status == TaskTerminalStatus::Failed)));
    assert_eq!(status_requests.load(Ordering::SeqCst), 3);
    assert_eq!(executor.load(task_id).unwrap().external_output_sequence, 0);
}

async fn persist_privileged_restart_state(
    executor: &Executor,
    task_id: &str,
    idempotency_key: &str,
    payload_digest: &str,
    task: &DeploymentReleaseTask,
    output_sequence: u64,
) {
    let mut journal = executor
        .create_transfer_task(
            task_id,
            idempotency_key,
            payload_digest,
            TransferPhase::PrivilegedRelease,
        )
        .await
        .unwrap();
    for sequence in 1..=output_sequence {
        executor
            .persist_external_output(
                task_id,
                sequence,
                deploy_go_agent_protocol::OutputStream::Stdout,
                format!("prior-frame-{sequence}\n").as_bytes(),
            )
            .unwrap();
    }
    journal.external_output_sequence = output_sequence;
    executor.store_journal(&journal).unwrap();
    std::fs::write(
        executor
            .task_dir(task_id)
            .join("privileged-release-task.json"),
        serde_json::to_vec(task).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn restart_resumes_persisted_privileged_release_without_second_start() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_restart_resume";
    let payload_digest = "sha256:restart_resume_payload";
    let idempotency_key = "idem_restart_resume_01";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(
        &executor,
        task_id,
        idempotency_key,
        payload_digest,
        &task,
        RESUME_OFFSET,
    )
    .await;

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let unexpected_starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_resume_executor(
        listener,
        Arc::clone(&unexpected_starts),
    ));
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch(
                task_id,
                payload_digest,
                task.clone(),
            ))),
            sender.clone(),
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    server.abort();

    assert_eq!(unexpected_starts.load(Ordering::SeqCst), 0);
    let Message::TaskAck(ack) = &messages[0] else {
        panic!("恢复任务必须先返回 ACK");
    };
    assert_eq!(ack.disposition, TaskAckDisposition::Duplicate);
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(
        result.status,
        TaskTerminalStatus::Succeeded,
        "result {result:?}"
    );
    assert_eq!(result.exit_code, Some(0));
    let journal = JournalStore::new(tasks).load(task_id).unwrap();
    assert_eq!(journal.state, JournalState::Succeeded);
    assert_eq!(journal.external_output_sequence, RESUME_OFFSET + 1);
}

#[tokio::test]
async fn missing_release_descriptor_observes_original_job_then_reports_interrupted() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_missing_descriptor";
    let payload_digest = "sha256:missing_descriptor_payload";
    executor
        .create_task(task_id, "idem_missing_descriptor_01", payload_digest)
        .await
        .unwrap();
    executor
        .begin_privileged_release(task_id, "deployment", "run")
        .unwrap();
    let descriptor = executor
        .task_dir(task_id)
        .join("privileged-release-task.json");
    assert!(!descriptor.exists());

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let unexpected_requests = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_descriptorless_executor(
        listener,
        Arc::clone(&unexpected_requests),
    ));
    let handler = TaskHandler::new(executor.clone())
        .with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::ReconcileRequest(
                deploy_go_agent_protocol::ReconcileRequest {
                    task_ids: vec![task_id.into()],
                },
            )),
            sender,
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    server.await.unwrap();

    assert_eq!(unexpected_requests.load(Ordering::SeqCst), 0);
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Interrupted);
    assert_eq!(
        result.error_code.as_deref(),
        Some("release_task_descriptor_unavailable")
    );
    let journal = JournalStore::new(tasks).load(task_id).unwrap();
    assert_eq!(journal.state, JournalState::Interrupted);
    assert_eq!(journal.external_output_sequence, 0);
    assert_eq!(journal.deployment_id.as_deref(), Some("deployment"));
    assert_eq!(journal.target_run_id.as_deref(), Some("run"));
}

/// FR-006：descriptor 存在但内容损坏时与缺失同样处理——只观测原 job，终态为 interrupted。
/// 第二个用例覆盖 journal 与 descriptor 的关联冲突分支。
#[tokio::test]
async fn corrupt_and_conflicting_release_descriptor_only_observe_original_job() {
    let mut conflicting = privileged_task("task_bad_descriptor");
    conflicting.deployment_id = "deployment_other".into();
    conflicting.privileged_context = Some(deploy_go_agent_protocol::PrivilegedReleaseContext {
        target_run_id: "run_other".into(),
        target_id: "target".into(),
        node_id: "node".into(),
        agent_id: "agent".into(),
        snapshot_hash: "snapshot".into(),
    });
    for (case, descriptor_bytes) in [
        ("corrupt", b"not-json".to_vec()),
        ("conflicting", serde_json::to_vec(&conflicting).unwrap()),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let tasks = directory.path().join("tasks");
        let executor = Executor::new(tasks.clone()).unwrap();
        let task_id = "task_bad_descriptor";
        let payload_digest = "sha256:bad_descriptor_payload";
        executor
            .create_task(task_id, "idem_bad_descriptor_01", payload_digest)
            .await
            .unwrap();
        executor
            .begin_privileged_release(task_id, "deployment", "run")
            .unwrap();
        std::fs::write(
            executor
                .task_dir(task_id)
                .join("privileged-release-task.json"),
            &descriptor_bytes,
        )
        .unwrap();

        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let unexpected_starts = Arc::new(AtomicUsize::new(0));
        let server = tokio::spawn(serve_descriptorless_executor(
            listener,
            Arc::clone(&unexpected_starts),
        ));
        let handler = TaskHandler::new(executor.clone())
            .with_privileged_release_executor(ExecutorClient::new(socket));
        let (sender, mut receiver) = mpsc::channel(64);
        handler
            .handle(
                envelope(Message::ReconcileRequest(
                    deploy_go_agent_protocol::ReconcileRequest {
                        task_ids: vec![task_id.into()],
                    },
                )),
                sender,
            )
            .await
            .unwrap();
        let messages = receive_until_result(&mut receiver).await;
        server.await.unwrap();

        assert_eq!(
            unexpected_starts.load(Ordering::SeqCst),
            0,
            "{case} descriptor must not restart release"
        );
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(message, Message::TaskResult(_)))
                .count(),
            1,
            "{case} descriptor must produce exactly one result"
        );
        let Message::TaskResult(result) = messages.last().unwrap() else {
            unreachable!();
        };
        assert_eq!(result.status, TaskTerminalStatus::Interrupted, "{case}");
        assert_eq!(
            result.error_code.as_deref(),
            Some("release_task_descriptor_unavailable"),
            "{case}"
        );
        // 关联冲突时 journal 中已有的可信关联值不得被 descriptor 覆盖。
        let journal = JournalStore::new(tasks).load(task_id).unwrap();
        assert_eq!(journal.state, JournalState::Interrupted, "{case}");
        assert_eq!(
            journal.deployment_id.as_deref(),
            Some("deployment"),
            "{case}"
        );
        assert_eq!(journal.target_run_id.as_deref(), Some("run"), "{case}");
    }
}

#[tokio::test]
async fn cancel_without_descriptor_observes_original_job_then_reports_interrupted() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_cancel_without_descriptor";
    let payload_digest = "sha256:cancel_without_descriptor_payload";
    executor
        .create_task(task_id, "idem_cancel_no_descriptor_01", payload_digest)
        .await
        .unwrap();
    executor
        .begin_privileged_release(task_id, "deployment", "run")
        .unwrap();

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let unexpected_starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_cancel_without_descriptor_executor(
        listener,
        Arc::clone(&unexpected_starts),
    ));
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskCancel(TaskCancel {
                task_id: task_id.into(),
                reason: "cancel_without_descriptor".into(),
            })),
            sender,
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    server.abort();

    assert_eq!(
        unexpected_starts.load(Ordering::SeqCst),
        0,
        "cancel must not restart release"
    );
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Interrupted);
    assert_eq!(
        result.error_code.as_deref(),
        Some("release_task_descriptor_unavailable")
    );
    assert_eq!(
        JournalStore::new(tasks).load(task_id).unwrap().state,
        JournalState::Interrupted
    );
}

/// FR-003：journal 暂时不可读时 monitor 不能静默结束，必须保留原恢复阶段退避重试，
/// 恢复后从同一 job 与 durable 游标收敛出唯一终态。用目录占位让 journal 读取返回 IO 错误，
/// 区别于"核心 journal 损坏"的 fail-closed Unknown 分支。
#[tokio::test]
async fn temporarily_unreadable_journal_retries_and_converges_from_durable_cursor() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_journal_temporarily_unreadable";
    let payload_digest = "sha256:journal_retry_payload";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(
        &executor,
        task_id,
        "idem_journal_retry_01",
        payload_digest,
        &task,
        RESUME_OFFSET,
    )
    .await;

    let journal_path = executor.task_dir(task_id).join("journal.json");
    let saved_journal = std::fs::read(&journal_path).unwrap();
    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let output_calls = Arc::new(AtomicUsize::new(0));
    let first_status_served = Arc::new(AtomicBool::new(false));
    let unexpected_starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_running_then_exit_executor(
        listener,
        Arc::clone(&output_calls),
        Arc::clone(&first_status_served),
        Arc::clone(&unexpected_starts),
    ));
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::ReconcileRequest(
                deploy_go_agent_protocol::ReconcileRequest {
                    task_ids: vec![task_id.into()],
                },
            )),
            sender,
        )
        .await
        .unwrap();

    // 等 monitor 接管原 job 并观测到一次状态后再注入本地持久化故障。
    tokio::time::timeout(Duration::from_secs(5), async {
        while !first_status_served.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("monitor 未在预期时间内接管原 job");
    std::fs::remove_file(&journal_path).unwrap();
    std::fs::create_dir(&journal_path).unwrap();
    // monitor 在 Running 后固定退避 100ms 再读 journal；此处跨过该间隔，
    // 输出请求计数不增加即证明它停在 journal 读取失败上并进入退避重试。
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(
        output_calls.load(Ordering::SeqCst),
        1,
        "journal 不可读时 monitor 不得继续拉取输出"
    );
    std::fs::remove_dir(&journal_path).unwrap();
    std::fs::write(&journal_path, &saved_journal).unwrap();

    let messages = receive_until_result(&mut receiver).await;
    server.abort();

    assert_eq!(unexpected_starts.load(Ordering::SeqCst), 0);
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Succeeded, "{result:?}");
    let journal = JournalStore::new(tasks).load(task_id).unwrap();
    assert_eq!(journal.state, JournalState::Succeeded);
    assert_eq!(journal.external_output_sequence, RESUME_OFFSET + 1);
}

#[tokio::test]
async fn corrupt_core_journal_reconciles_unknown_without_executor_request() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_corrupt_journal";
    executor
        .create_transfer_task(
            task_id,
            "idem_corrupt_journal_01",
            "sha256:corrupt_journal_payload",
            TransferPhase::PrivilegedRelease,
        )
        .await
        .unwrap();
    std::fs::write(executor.task_dir(task_id).join("journal.json"), b"not-json").unwrap();

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(8);
    handler
        .handle(
            envelope(Message::ReconcileRequest(
                deploy_go_agent_protocol::ReconcileRequest {
                    task_ids: vec![task_id.into()],
                },
            )),
            sender,
        )
        .await
        .unwrap();
    let Some(Message::ReconcileReport(report)) =
        tokio::time::timeout(Duration::from_secs(2), receiver.recv())
            .await
            .unwrap()
    else {
        panic!("expected reconcile report");
    };
    assert_eq!(report.tasks.len(), 1);
    assert_eq!(
        report.tasks[0].state,
        deploy_go_agent_protocol::ReconciledTaskState::Unknown
    );
    assert!(report.tasks[0].payload_digest.is_empty());
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn transient_executor_output_and_status_failures_retry_until_terminal() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_transient_retry";
    let payload_digest = "sha256:transient_retry_payload";
    let idempotency_key = "idem_transient_retry_01";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(
        &executor,
        task_id,
        idempotency_key,
        payload_digest,
        &task,
        0,
    )
    .await;

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let unexpected_starts = Arc::new(AtomicUsize::new(0));
    let output_failures = Arc::new(AtomicUsize::new(0));
    let status_failures = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_transient_executor(
        listener,
        Arc::clone(&unexpected_starts),
        Arc::clone(&output_failures),
        Arc::clone(&status_failures),
    ));
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
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
    let messages = receive_until_result(&mut receiver).await;
    server.abort();

    assert_eq!(output_failures.load(Ordering::SeqCst), 1);
    assert_eq!(status_failures.load(Ordering::SeqCst), 1);
    assert_eq!(unexpected_starts.load(Ordering::SeqCst), 0);
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Succeeded);
    assert_eq!(result.exit_code, Some(0));
    let journal = JournalStore::new(tasks).load(task_id).unwrap();
    assert_eq!(journal.state, JournalState::Succeeded);
    assert_eq!(journal.external_output_sequence, 1);
}

#[tokio::test]
async fn duplicate_cancel_and_disconnect_resume_produce_one_terminal_state() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_cancel_resume";
    let payload_digest = "sha256:cancel_resume_payload";
    let idempotency_key = "idem_cancel_resume_01";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(
        &executor,
        task_id,
        idempotency_key,
        payload_digest,
        &task,
        0,
    )
    .await;

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let cancel_calls = Arc::new(AtomicUsize::new(0));
    let cancel_seen = Arc::new(AtomicBool::new(false));
    let unexpected_starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_cancel_executor(
        listener,
        Arc::clone(&cancel_calls),
        Arc::clone(&cancel_seen),
        Arc::clone(&unexpected_starts),
    ));
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch(
                task_id,
                payload_digest,
                task,
            ))),
            sender.clone(),
        )
        .await
        .unwrap();
    let first_ack = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .expect("恢复任务 ACK 超时")
        .expect("任务发送通道提前关闭");
    assert!(matches!(
        first_ack,
        Message::TaskAck(ack) if ack.disposition == TaskAckDisposition::Duplicate
    ));

    handler
        .handle(
            envelope(Message::TaskCancel(TaskCancel {
                task_id: task_id.into(),
                reason: "test_cancel".into(),
            })),
            sender.clone(),
        )
        .await
        .unwrap();
    let canceled = receive_until_result(&mut receiver).await;
    assert_eq!(
        canceled
            .iter()
            .filter(|m| matches!(m, Message::TaskResult(_)))
            .count(),
        1
    );
    let Message::TaskResult(first_result) = canceled.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(first_result.status, TaskTerminalStatus::Canceled);
    let calls_at_terminal = cancel_calls.load(Ordering::SeqCst);
    assert!(calls_at_terminal >= 1);

    handler
        .handle(
            envelope(Message::TaskCancel(TaskCancel {
                task_id: task_id.into(),
                reason: "duplicate_cancel".into(),
            })),
            sender.clone(),
        )
        .await
        .unwrap();
    let duplicate = receive_until_result(&mut receiver).await;
    server.abort();

    let Message::TaskResult(second_result) = duplicate.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(second_result.status, TaskTerminalStatus::Canceled);
    assert_eq!(second_result.sequence, first_result.sequence);
    assert_eq!(cancel_calls.load(Ordering::SeqCst), calls_at_terminal);
    assert_eq!(
        duplicate
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    assert_eq!(
        JournalStore::new(tasks).load(task_id).unwrap().state,
        JournalState::Canceled
    );
}

#[tokio::test]
async fn cancel_without_active_monitor_resumes_and_produces_single_terminal_state() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_cancel_after_exit";
    let payload_digest = "sha256:cancel_after_exit_payload";
    let idempotency_key = "idem_cancel_after_exit_01";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(
        &executor,
        task_id,
        idempotency_key,
        payload_digest,
        &task,
        0,
    )
    .await;

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let cancel_calls = Arc::new(AtomicUsize::new(0));
    let cancel_seen = Arc::new(AtomicBool::new(false));
    let unexpected_starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_cancel_executor(
        listener,
        Arc::clone(&cancel_calls),
        Arc::clone(&cancel_seen),
        Arc::clone(&unexpected_starts),
    ));
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskCancel(TaskCancel {
                task_id: task_id.into(),
                reason: "cancel_after_monitor_exit".into(),
            })),
            sender,
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    server.abort();

    assert_eq!(unexpected_starts.load(Ordering::SeqCst), 0);
    assert!(cancel_calls.load(Ordering::SeqCst) >= 1);
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Canceled);
    assert_eq!(
        JournalStore::new(tasks).load(task_id).unwrap().state,
        JournalState::Canceled
    );
}

#[tokio::test]
async fn env_and_artifact_gate_failures_never_connect_executor() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let handler = TaskHandler::new(Executor::new(tasks).unwrap())
        .with_privileged_release_executor(ExecutorClient::new(socket));

    let mut env_task = privileged_task("task_env_gate");
    env_task.application_slug = Some("app".into());
    env_task.required_env = vec![RequiredEnvVersion {
        file_name: "api.env".into(),
        env_version: 1,
        digest: "d".repeat(64),
        action: EnvSyncAction::Write,
    }];
    let (sender, mut receiver) = mpsc::channel(4);
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch(
                "task_env_gate",
                "sha256:env_gate_payload",
                env_task,
            ))),
            sender,
        )
        .await
        .unwrap();
    let Message::TaskAck(ack) = receiver.recv().await.unwrap() else {
        panic!("Env gate 失败必须返回 ACK");
    };
    assert_eq!(ack.disposition, TaskAckDisposition::Rejected);
    assert_eq!(ack.error_code.as_deref(), Some("env_gate_failed"));
    assert!(
        tokio::time::timeout(Duration::from_millis(150), listener.accept())
            .await
            .is_err(),
        "Env gate 失败时不得连接 executor"
    );

    let mut artifact_task = privileged_task("task_artifact_gate");
    artifact_task.artifact_download = None;
    let (sender, mut receiver) = mpsc::channel(4);
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch(
                "task_artifact_gate",
                "sha256:artifact_gate_payload",
                artifact_task,
            ))),
            sender,
        )
        .await
        .unwrap();
    let Message::TaskAck(ack) = receiver.recv().await.unwrap() else {
        panic!("artifact gate 失败必须返回 ACK");
    };
    assert_eq!(ack.disposition, TaskAckDisposition::Rejected);
    assert_eq!(
        ack.error_code.as_deref(),
        Some("privileged_release_artifact_required")
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(150), listener.accept())
            .await
            .is_err(),
        "artifact gate 失败时不得连接 executor"
    );
}

/// 脚本化 executor：job 已终态（失败），输出帧序号 1..=`last_sequence`，每页最多 `page_size` 帧；
/// 第一次返回 Exited 之前调用 `before_exit`，用于在恰当的时间点注入本地故障。
async fn serve_scripted_exit(
    listener: UnixListener,
    last_sequence: u64,
    page_size: u64,
    outputs: Arc<AtomicUsize>,
    starts: Arc<AtomicUsize>,
    before_exit: impl Fn() + Send + 'static,
) {
    let mut exit_announced = false;
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream, MAX_FRAME_BYTES)
            .await
            .unwrap()
            .unwrap();
        match request {
            Request::ReleaseOutput(request) => {
                outputs.fetch_add(1, Ordering::SeqCst);
                let first = request.after_sequence + 1;
                let last = (request.after_sequence + page_size).min(last_sequence);
                write_response(
                    &mut stream,
                    Response::ReleaseOutput(ReleaseOutputResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        frames: (first..=last)
                            .map(|sequence| {
                                deploy_go_agent_executor::protocol::ReleaseOutputFrame {
                                sequence,
                                stream:
                                    deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout,
                                data: format!("scripted-{sequence}\n").into_bytes(),
                            }
                            })
                            .collect(),
                        truncated: false,
                    }),
                )
                .await;
            }
            Request::ReleaseStatus(request) => {
                if !exit_announced {
                    exit_announced = true;
                    before_exit();
                }
                write_response(
                    &mut stream,
                    Response::ReleaseExited(ReleaseExitedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: request.job_id,
                        state: ReleaseJobState::Failed,
                        exit_code: Some(1),
                        reason: "nonzero_exit".into(),
                        last_sequence,
                    }),
                )
                .await;
            }
            Request::ReleaseStart(_) => {
                starts.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }
    }
}

fn reconcile_request(task_id: &str) -> Message {
    Message::ReconcileRequest(deploy_go_agent_protocol::ReconcileRequest {
        task_ids: vec![task_id.into()],
    })
}

/// 取走通道里当前已有的全部消息（不等待）。
fn drain_available(receiver: &mut mpsc::Receiver<Message>) -> Vec<Message> {
    std::iter::from_fn(|| receiver.try_recv().ok()).collect()
}

fn set_mode(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// 作用域结束（含断言 panic）时把目录权限恢复为可写，避免遗留只读目录让临时目录无法清理。
struct RestoreMode(std::path::PathBuf);

impl Drop for RestoreMode {
    fn drop(&mut self) {
        if self.0.exists() {
            set_mode(&self.0, 0o700);
        }
    }
}

/// 目录只读权限对 root 不生效；探测一下，不生效时跳过依赖它的故障注入用例，而不是误报。
fn directory_permissions_are_enforced(scratch: &std::path::Path) -> bool {
    let probe_dir = scratch.join("permission-probe");
    std::fs::create_dir_all(&probe_dir).unwrap();
    set_mode(&probe_dir, 0o500);
    let enforced = std::fs::File::create(probe_dir.join("probe")).is_err();
    set_mode(&probe_dir, 0o700);
    enforced
}

/// 终态 job 的多页输出在 job 已退出后一次读完：每页都有新帧，不应逐页指数退避。
/// 8 页若按旧逻辑累计退避（250ms…5s）需要十几秒；这里要求整体在 3 秒内收敛。
#[tokio::test]
async fn multi_page_terminal_output_is_drained_without_backoff_between_pages() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    let task_id = "task_multi_page_terminal";
    let payload_digest = "sha256:multi_page_terminal_payload";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(
        &executor,
        task_id,
        "idem_multi_page_terminal",
        payload_digest,
        &task,
        RESUME_OFFSET,
    )
    .await;
    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let outputs = Arc::new(AtomicUsize::new(0));
    let starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_scripted_exit(
        listener,
        RESUME_OFFSET + 8,
        1,
        Arc::clone(&outputs),
        Arc::clone(&starts),
        || {},
    ));
    let handler =
        TaskHandler::new(executor).with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(256);
    let started = std::time::Instant::now();
    handler
        .handle(envelope(reconcile_request(task_id)), sender)
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    let elapsed = started.elapsed();
    server.abort();

    assert!(
        elapsed < Duration::from_secs(3),
        "终态多页输出不应逐页退避，实际耗时 {elapsed:?}"
    );
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let journal = JournalStore::new(tasks).load(task_id).unwrap();
    assert_eq!(journal.state, JournalState::Failed);
    assert_eq!(journal.external_output_sequence, RESUME_OFFSET + 8);
}

/// FR-003/SC-001：outbox 写入暂时失败（目录只读）时，monitor 不得丢弃输出或静默结束，
/// 而是保留 PrivilegedRelease 恢复阶段退避重试；故障清除后补齐输出并只产生一个逻辑终态。
#[tokio::test]
async fn output_outbox_failure_retries_without_losing_output_or_restarting_release() {
    let directory = tempfile::tempdir().unwrap();
    if !directory_permissions_are_enforced(directory.path()) {
        eprintln!("跳过：当前身份（root）不受目录只读权限约束，无法注入 outbox 写入故障");
        return;
    }
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    executor.enable_reliable_logs(true);
    let task_id = "task_output_outbox_failure";
    let payload_digest = "sha256:output_outbox_failure_payload";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(
        &executor,
        task_id,
        "idem_output_outbox_failure",
        payload_digest,
        &task,
        RESUME_OFFSET,
    )
    .await;
    let outbox = executor.task_dir(task_id).join("log-outbox-v1");
    let _restore_outbox = RestoreMode(outbox.clone());
    std::fs::create_dir_all(&outbox).unwrap();
    set_mode(&outbox, 0o500);

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let outputs = Arc::new(AtomicUsize::new(0));
    let starts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(serve_scripted_exit(
        listener,
        RESUME_OFFSET + 2,
        2,
        Arc::clone(&outputs),
        Arc::clone(&starts),
        || {},
    ));
    let handler = TaskHandler::new(executor.clone())
        .with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(256);
    handler
        .handle(envelope(reconcile_request(task_id)), sender)
        .await
        .unwrap();

    tokio::time::timeout(Duration::from_secs(5), async {
        while outputs.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("monitor 未接管原 job");
    // 跨过两个以上退避周期（250ms + 500ms）：交付持续失败期间不得产生终态或丢失输出。
    tokio::time::sleep(Duration::from_millis(900)).await;
    let during_fault = drain_available(&mut receiver);
    assert!(
        during_fault
            .iter()
            .all(|message| !matches!(message, Message::TaskResult(_) | Message::TaskOutput(_))),
        "outbox 不可写时不得交付输出或终态: {during_fault:?}"
    );
    let journal = JournalStore::new(tasks.clone()).load(task_id).unwrap();
    assert_eq!(journal.state, JournalState::Running);
    assert_eq!(
        journal.transfer_phase,
        Some(TransferPhase::PrivilegedRelease)
    );

    set_mode(&outbox, 0o700);
    let messages = receive_until_result(&mut receiver).await;
    server.abort();

    assert_eq!(starts.load(Ordering::SeqCst), 0, "恢复不得再次启动 release");
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let mut sequences: Vec<u64> = messages
        .iter()
        .filter_map(|message| match message {
            Message::TaskOutput(output) => Some(output.sequence),
            Message::TaskProgress(progress) => Some(progress.sequence),
            Message::TaskResult(result) => Some(result.sequence),
            _ => None,
        })
        .collect();
    assert!(!sequences.is_empty());
    let expected: Vec<u64> = (sequences[0]..sequences[0] + sequences.len() as u64).collect();
    sequences.sort_unstable();
    assert_eq!(sequences, expected, "故障恢复后序号必须连续且不重复");
    let journal = JournalStore::new(tasks).load(task_id).unwrap();
    assert_eq!(journal.state, JournalState::Failed);
    assert_eq!(journal.external_output_sequence, RESUME_OFFSET + 2);
}

/// FR-003/SC-001：终态已 durable 保存、但 TaskResult 入 outbox 暂时失败时，monitor 保留终态
/// 并持续重投同一个逻辑结果，不重新执行 release，故障清除后只交付一个 TaskResult。
#[tokio::test]
async fn result_outbox_failure_retries_the_durable_terminal_without_a_second_result() {
    let directory = tempfile::tempdir().unwrap();
    if !directory_permissions_are_enforced(directory.path()) {
        eprintln!("跳过：当前身份（root）不受目录只读权限约束，无法注入 outbox 写入故障");
        return;
    }
    let tasks = directory.path().join("tasks");
    let executor = Executor::new(tasks.clone()).unwrap();
    executor.enable_reliable_logs(true);
    let task_id = "task_result_outbox_failure";
    let payload_digest = "sha256:result_outbox_failure_payload";
    let task = privileged_task(task_id);
    persist_privileged_restart_state(
        &executor,
        task_id,
        "idem_result_outbox_failure",
        payload_digest,
        &task,
        RESUME_OFFSET,
    )
    .await;
    let outbox = executor.task_dir(task_id).join("log-outbox-v1");
    let _restore_outbox = RestoreMode(outbox.clone());

    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let outputs = Arc::new(AtomicUsize::new(0));
    let starts = Arc::new(AtomicUsize::new(0));
    let fault_installed = Arc::new(AtomicBool::new(false));
    let hook_outbox = outbox.clone();
    let hook_flag = Arc::clone(&fault_installed);
    // 历史输出已在 job 退出前交付；在 executor 宣告退出的瞬间把 outbox 置为只读，
    // 使故障只打在 TaskResult 交付上。
    let server = tokio::spawn(serve_scripted_exit(
        listener,
        RESUME_OFFSET,
        4,
        Arc::clone(&outputs),
        Arc::clone(&starts),
        move || {
            set_mode(&hook_outbox, 0o500);
            hook_flag.store(true, Ordering::SeqCst);
        },
    ));
    let handler = TaskHandler::new(executor.clone())
        .with_privileged_release_executor(ExecutorClient::new(socket));
    let (sender, mut receiver) = mpsc::channel(256);
    handler
        .handle(envelope(reconcile_request(task_id)), sender)
        .await
        .unwrap();

    tokio::time::timeout(Duration::from_secs(5), async {
        while !fault_installed.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("monitor 未到达终态交接");
    tokio::time::sleep(Duration::from_millis(900)).await;
    let during_fault = drain_available(&mut receiver);
    assert!(
        during_fault
            .iter()
            .all(|message| !matches!(message, Message::TaskResult(_))),
        "outbox 不可写时不得交付终态: {during_fault:?}"
    );
    let journal = JournalStore::new(tasks.clone()).load(task_id).unwrap();
    assert_eq!(
        journal.state,
        JournalState::Failed,
        "终态必须先 durable 保存"
    );
    assert_eq!(journal.result_sequence, None, "结果尚未进入交付路径");

    set_mode(&outbox, 0o700);
    let messages = receive_until_result(&mut receiver).await;
    server.abort();

    assert_eq!(starts.load(Ordering::SeqCst), 0, "恢复不得再次启动 release");
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::TaskResult(_)))
            .count(),
        1
    );
    let journal = JournalStore::new(tasks).load(task_id).unwrap();
    assert_eq!(journal.state, JournalState::Failed);
    assert!(journal.result_sequence.is_some());
}
