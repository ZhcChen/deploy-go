use std::{fs, path::Path, sync::Arc, time::Duration};

use deploy_go_agent::{
    connection::{MessageHandler, envelope},
    executor::Executor,
    task_handler::TaskHandler,
};
use deploy_go_agent_protocol::{
    DeploymentExecuteTask, Message, TaskAckDisposition, TaskDispatch, TaskPayload,
    TaskTerminalStatus,
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
