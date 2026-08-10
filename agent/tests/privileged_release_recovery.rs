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

async fn serve_cancel_executor(
    listener: UnixListener,
    cancel_calls: Arc<AtomicUsize>,
    cancel_seen: Arc<AtomicBool>,
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
    let server = tokio::spawn(serve_cancel_executor(
        listener,
        Arc::clone(&cancel_calls),
        Arc::clone(&cancel_seen),
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
    assert_eq!(cancel_calls.load(Ordering::SeqCst), 1);

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
    assert_eq!(cancel_calls.load(Ordering::SeqCst), 1);
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
