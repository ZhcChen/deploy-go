use std::{fs, path::Path, sync::Arc, time::Duration};

use deploy_go_agent::{
    connection::{MessageHandler, envelope},
    executor::Executor,
    journal::{JournalStore, TransferPhase},
    task_handler::TaskHandler,
};
use deploy_go_agent_protocol::{
    ArtifactDownloadRequest, DeploymentExecuteTask, DeploymentReleaseTask, Environment, MakeTarget,
    Message, SystemInspectTask, TaskAckDisposition, TaskDispatch, TaskPayload, TaskTerminalStatus,
};
#[cfg(target_os = "linux")]
use deploy_go_agent_protocol::{TaskCancel, TaskLifecycleState};
use tokio::sync::mpsc;

fn make_script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn cancel_is_serialized_with_output_and_returns_one_terminal_result() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    let script = root.join("deploy.sh");
    make_script(&script, "while :; do printf 'working\\n'; sleep 0.05; done");
    let handler = Arc::new(TaskHandler::new(
        Executor::new(directory.path().join("tasks"))
            .unwrap()
            .with_cancel_grace(Duration::from_millis(100))
            .with_runner_binary(Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).to_owned()),
    ));
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch(&root, &script))),
            sender.clone(),
        )
        .await
        .unwrap();

    let mut messages = Vec::new();
    loop {
        let message = receiver.recv().await.unwrap();
        let running = matches!(
            message,
            Message::TaskState(ref state) if state.state == TaskLifecycleState::Running
        );
        messages.push(message);
        if running {
            break;
        }
    }
    handler
        .handle(
            envelope(Message::TaskCancel(TaskCancel {
                task_id: "task_01".to_owned(),
                reason: "test_cancel".to_owned(),
            })),
            sender,
        )
        .await
        .unwrap();
    messages.extend(receive_until_result(&mut receiver).await);

    let sequences = messages
        .iter()
        .filter_map(|message| match message {
            Message::TaskState(state) => Some(state.sequence),
            Message::TaskOutput(output) => Some(output.sequence),
            Message::TaskResult(result) => Some(result.sequence),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sequences, (1..=sequences.len() as u64).collect::<Vec<_>>());
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
}

fn dispatch(root: &Path, script: &Path) -> TaskDispatch {
    TaskDispatch {
        task_id: "task_01".to_owned(),
        idempotency_key: "idem_0123456789abcdef".to_owned(),
        deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
        payload_digest: "sha256:0123456789abcdef".to_owned(),
        task: TaskPayload::DeploymentExecute(DeploymentExecuteTask {
            deployment_id: "dep_01".to_owned(),
            work_root: root.display().to_string(),
            script_path: script.display().to_string(),
            argument_tokens: Vec::new(),
            environment_file_references: Vec::new(),
            timeout_seconds: 10,
            wrapper_version: "1".to_owned(),
        }),
    }
}

async fn receive_until_result(receiver: &mut mpsc::Receiver<Message>) -> Vec<Message> {
    let mut messages = Vec::new();
    loop {
        let message = tokio::time::timeout(Duration::from_secs(10), receiver.recv())
            .await
            .expect("任务结果超时")
            .expect("任务发送通道提前关闭");
        let terminal = matches!(message, Message::TaskResult(_));
        messages.push(message);
        if terminal {
            return messages;
        }
    }
}

#[tokio::test]
async fn dispatch_streams_ordered_events_and_duplicate_does_not_execute_again() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    let marker = root.join("executions");
    let script = root.join("deploy.sh");
    make_script(
        &script,
        &format!(
            "printf x >> '{}'; printf 'deployed\\n'; printf 'warning\\n' >&2",
            marker.display()
        ),
    );
    let handler = Arc::new(TaskHandler::new(
        Executor::new(directory.path().join("tasks"))
            .unwrap()
            .with_runner_binary(Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).to_owned()),
    ));
    let task = dispatch(&root, &script);
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(task.clone())),
            sender.clone(),
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;

    let Message::TaskAck(ack) = &messages[0] else {
        panic!("第一条任务消息必须是 ACK");
    };
    assert_eq!(ack.disposition, TaskAckDisposition::Accepted);
    let sequences = messages
        .iter()
        .filter_map(|message| match message {
            Message::TaskState(state) => Some(state.sequence),
            Message::TaskOutput(output) => Some(output.sequence),
            Message::TaskResult(result) => Some(result.sequence),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sequences, (1..=sequences.len() as u64).collect::<Vec<_>>());
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Succeeded);
    let result_sequence = result.sequence;
    assert_eq!(fs::read_to_string(&marker).unwrap(), "x");

    let (duplicate_sender, mut duplicate_receiver) = mpsc::channel(64);
    handler
        .handle(envelope(Message::TaskDispatch(task)), duplicate_sender)
        .await
        .unwrap();
    let duplicate = receive_until_result(&mut duplicate_receiver).await;
    let Message::TaskAck(ack) = &duplicate[0] else {
        panic!("重复任务必须先返回 ACK");
    };
    assert_eq!(ack.disposition, TaskAckDisposition::Duplicate);
    let Message::TaskResult(result) = duplicate.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.sequence, result_sequence);
    assert_eq!(fs::read_to_string(marker).unwrap(), "x");
}

#[tokio::test]
async fn system_inspect_returns_structured_capabilities() {
    let directory = tempfile::tempdir().unwrap();
    let work_root = directory.path().join("work");
    let secrets_root = directory.path().join("secrets");
    fs::create_dir(&work_root).unwrap();
    fs::create_dir(&secrets_root).unwrap();
    let handler = TaskHandler::new(Executor::new(directory.path().join("tasks")).unwrap());
    let dispatch = TaskDispatch {
        task_id: "task_inspect".to_owned(),
        idempotency_key: "inspect_0123456789abcdef".to_owned(),
        deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
        payload_digest: "sha256:abcdef0123456789".to_owned(),
        task: TaskPayload::SystemInspect(SystemInspectTask {
            work_root: work_root.display().to_string(),
            secrets_root: secrets_root.display().to_string(),
        }),
    };
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(envelope(Message::TaskDispatch(dispatch)), sender)
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Succeeded);
    let data = result.data.as_ref().unwrap();
    assert!(data["os_name"].is_string());
    assert!(data["architecture"].is_string());
    assert!(data["disk_available_bytes"].as_u64().is_some());
    assert_eq!(data["work_root_accessible"], true);
    assert_eq!(data["secrets_root_accessible"], true);
}

#[tokio::test]
async fn invalid_task_identity_is_rejected_before_creating_task_files() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let work_root = directory.path().join("work");
    let secrets_root = directory.path().join("secrets");
    fs::create_dir(&work_root).unwrap();
    fs::create_dir(&secrets_root).unwrap();
    let handler = TaskHandler::new(Executor::new(tasks.clone()).unwrap());

    for task_id in ["../outside", "/tmp/absolute-task"] {
        let dispatch = TaskDispatch {
            task_id: task_id.to_owned(),
            idempotency_key: "inspect_0123456789abcdef".to_owned(),
            deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
            payload_digest: "sha256:abcdef0123456789".to_owned(),
            task: TaskPayload::SystemInspect(SystemInspectTask {
                work_root: work_root.display().to_string(),
                secrets_root: secrets_root.display().to_string(),
            }),
        };
        let (sender, mut receiver) = mpsc::channel(4);
        handler
            .handle(envelope(Message::TaskDispatch(dispatch)), sender)
            .await
            .unwrap();
        let Message::TaskAck(ack) = receiver.recv().await.unwrap() else {
            panic!("非法任务标识必须返回 ACK")
        };
        assert_eq!(ack.disposition, TaskAckDisposition::Rejected);
        assert_eq!(ack.error_code.as_deref(), Some("invalid_task_identity"));
    }
    assert!(!tasks.exists() || fs::read_dir(tasks).unwrap().next().is_none());
    assert!(!directory.path().join("outside").exists());
}

#[tokio::test]
async fn cross_node_release_ack_failure_keeps_release_download_phase() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let handler = TaskHandler::new(Executor::new(tasks.clone()).unwrap());
    let dispatch = TaskDispatch {
        task_id: "task_release_ack_failure".to_owned(),
        idempotency_key: "idem_release_ack_failure_01".to_owned(),
        deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
        payload_digest: "sha256:abcdef0123456789".to_owned(),
        task: TaskPayload::DeploymentRelease(DeploymentReleaseTask {
            deployment_id: "dep_release".to_owned(),
            target_code: "production".to_owned(),
            work_root: "/untrusted/work".to_owned(),
            checkout_dir: "/untrusted/work/checkout".to_owned(),
            artifact_dir: "/untrusted/work/artifact".to_owned(),
            environment: Environment::Production,
            release_version: "release-1".to_owned(),
            commit_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            modules: vec!["api".to_owned()],
            make_target: MakeTarget::DeployGoRelease,
            timeout_seconds: 60,
            cancel_file: String::new(),
            privileged: false,
            privileged_context: None,
            artifact_download: Some(ArtifactDownloadRequest {
                target_run_id: "run_01".to_owned(),
                lease_id: "lease_01".to_owned(),
                archive_digest: "a".repeat(64),
                manifest_digest: "b".repeat(64),
            }),
            repository_url: Some("https://git.example.test/app.git".to_owned()),
            git_credential_lease_id: None,
            application_slug: None,
            required_env: Vec::new(),
        }),
    };
    let (sender, receiver) = mpsc::channel(1);
    drop(receiver);
    handler
        .handle(envelope(Message::TaskDispatch(dispatch)), sender)
        .await
        .unwrap();
    let store = JournalStore::new(tasks);
    let journal = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Ok(journal) = store.load("task_release_ack_failure") {
                break journal;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(journal.transfer_phase, Some(TransferPhase::ReleaseDownload));
}

#[tokio::test]
async fn privileged_release_never_falls_back_to_legacy_runner_without_artifact() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let handler = TaskHandler::new(Executor::new(tasks.clone()).unwrap());
    let dispatch = TaskDispatch {
        task_id: "task_privileged_no_artifact".to_owned(),
        idempotency_key: "idem_privileged_no_artifact".to_owned(),
        deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
        payload_digest: "sha256:abcdef0123456789".to_owned(),
        task: TaskPayload::DeploymentRelease(DeploymentReleaseTask {
            deployment_id: "deployment".to_owned(),
            target_code: "test".to_owned(),
            work_root: "/srv/work".to_owned(),
            checkout_dir: "/srv/work/checkout".to_owned(),
            artifact_dir: "/srv/work/artifact".to_owned(),
            environment: Environment::Test,
            release_version: "release-1".to_owned(),
            commit_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            modules: vec!["api".to_owned()],
            make_target: MakeTarget::DeployGoRelease,
            timeout_seconds: 60,
            cancel_file: String::new(),
            privileged: true,
            privileged_context: Some(deploy_go_agent_protocol::PrivilegedReleaseContext {
                target_run_id: "run".to_owned(),
                target_id: "target".to_owned(),
                node_id: "node".to_owned(),
                agent_id: "agent".to_owned(),
                snapshot_hash: "a".repeat(64),
            }),
            artifact_download: None,
            repository_url: None,
            git_credential_lease_id: None,
            application_slug: None,
            required_env: Vec::new(),
        }),
    };
    let (sender, mut receiver) = mpsc::channel(4);
    handler
        .handle(envelope(Message::TaskDispatch(dispatch)), sender)
        .await
        .unwrap();
    let Message::TaskAck(ack) = receiver.recv().await.unwrap() else {
        panic!("特权任务缺少 artifact 时必须拒绝")
    };
    assert_eq!(ack.disposition, TaskAckDisposition::Rejected);
    assert_eq!(
        ack.error_code.as_deref(),
        Some("privileged_release_artifact_required")
    );
    assert!(!tasks.exists());
}
