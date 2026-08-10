use std::{fs, path::Path, process::Command as StdCommand, sync::Arc, time::Duration};

use deploy_go_agent::{
    connection::{MessageHandler, envelope},
    executor::Executor,
    task_handler::TaskHandler,
};
use deploy_go_agent_protocol::{
    DeploymentPrepareTask, DeploymentReleaseTask, Environment, MakeTarget, Message,
    SecretLeasePurpose, SecretLeaseResponse, SourcePolicy, TaskAckDisposition, TaskDispatch,
    TaskPayload, TaskTerminalStatus,
};
use tokio::sync::mpsc;

const PREPARE_SCRIPT: &str = r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$DEPLOY_OUTPUT_DIR/demo"
printf 'hello\n' > "$DEPLOY_OUTPUT_DIR/demo/app.txt"
if command -v sha256sum >/dev/null 2>&1; then
  sha=$(sha256sum "$DEPLOY_OUTPUT_DIR/demo/app.txt" | awk '{print $1}')
else
  sha=$(shasum -a 256 "$DEPLOY_OUTPUT_DIR/demo/app.txt" | awk '{print $1}')
fi
size=$(wc -c < "$DEPLOY_OUTPUT_DIR/demo/app.txt" | tr -d ' ')
cat > "$DEPLOY_OUTPUT_DIR/deploy-go-artifact.json" <<EOF
{"schema_version":1,"release_version":"$DEPLOY_RELEASE_VERSION","commit_sha":"$DEPLOY_COMMIT_SHA","artifacts":[{"module":"demo","path":"demo/app.txt","sha256":"$sha","size":$size}]}
EOF
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.started"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"demo","module_name":"Demo 服务"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"demo","step_id":"demo.package","step":"打包发布物"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"demo","step_id":"demo.package","step":"打包发布物"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"demo","module_name":"Demo 服务"}'
"#;

const PREPARE_UNFINISHED_SCRIPT: &str = r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$DEPLOY_OUTPUT_DIR/demo"
printf 'hello\n' > "$DEPLOY_OUTPUT_DIR/demo/app.txt"
if command -v sha256sum >/dev/null 2>&1; then
  sha=$(sha256sum "$DEPLOY_OUTPUT_DIR/demo/app.txt" | awk '{print $1}')
else
  sha=$(shasum -a 256 "$DEPLOY_OUTPUT_DIR/demo/app.txt" | awk '{print $1}')
fi
size=$(wc -c < "$DEPLOY_OUTPUT_DIR/demo/app.txt" | tr -d ' ')
cat > "$DEPLOY_OUTPUT_DIR/deploy-go-artifact.json" <<EOF
{"schema_version":1,"release_version":"$DEPLOY_RELEASE_VERSION","commit_sha":"$DEPLOY_COMMIT_SHA","artifacts":[{"module":"demo","path":"demo/app.txt","sha256":"$sha","size":$size}]}
EOF
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.started"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"demo","module_name":"Demo 服务"}'
"#;

fn release_script(release_root: &Path) -> String {
    let template = r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "__RELEASE_ROOT__"
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.started"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"demo","module_name":"Demo 服务"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"demo","step_id":"demo.activate","step":"切换发布版本"}'
cp "$DEPLOY_ARTIFACT_DIR/demo/app.txt" "__RELEASE_ROOT__/app.txt"
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"demo","step_id":"demo.activate","step":"切换发布版本"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.verification.started","module":"demo","step_id":"demo.verify","step":"验证发布版本"}'
test "$(cat "__RELEASE_ROOT__/app.txt")" = 'hello'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.verification.succeeded","module":"demo","step_id":"demo.verify","step":"验证发布版本"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"demo","module_name":"Demo 服务"}'
"#;
    template.replace("__RELEASE_ROOT__", &release_root.display().to_string())
}

fn makefile() -> &'static str {
    ".PHONY: deploy-go-prepare deploy-go-release\ndeploy-go-prepare:\n\tbash scripts/prepare.sh\ndeploy-go-release:\n\tbash scripts/release.sh\n"
}

fn git(repo: &Path, args: &[&str]) -> String {
    let output = StdCommand::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git 命令执行失败");
    assert!(
        output.status.success(),
        "git {:?} 失败: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn init_app_repo(root: &Path, release_root: &Path, prepare_script: &str) -> String {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("Makefile"), makefile()).unwrap();
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(root.join("scripts/prepare.sh"), prepare_script).unwrap();
    fs::write(
        root.join("scripts/release.sh"),
        release_script(release_root),
    )
    .unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    git(root, &["config", "commit.gpgsign", "false"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "initial"]);
    git(root, &["rev-parse", "HEAD"])
}

fn handler(directory: &Path) -> Arc<TaskHandler> {
    Arc::new(TaskHandler::new(
        Executor::new(directory.join("tasks"))
            .unwrap()
            .with_runner_binary(Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).to_owned()),
    ))
}

#[allow(clippy::too_many_arguments)]
fn prepare_dispatch(
    task_id: &str,
    idem: &str,
    digest: &str,
    deployment_id: &str,
    repo_url: &str,
    checkout_dir: &Path,
    work_root: &Path,
    output_dir: &Path,
    sha: &str,
    lease_id: Option<&str>,
) -> TaskDispatch {
    TaskDispatch {
        task_id: task_id.to_owned(),
        idempotency_key: idem.to_owned(),
        deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
        payload_digest: digest.to_owned(),
        task: TaskPayload::DeploymentPrepare(DeploymentPrepareTask {
            deployment_id: deployment_id.to_owned(),
            source_policy: SourcePolicy::Branch,
            repository_url: repo_url.to_owned(),
            commit_sha: sha.to_owned(),
            checkout_dir: checkout_dir.display().to_string(),
            work_root: work_root.display().to_string(),
            output_dir: output_dir.display().to_string(),
            environment: Environment::Test,
            release_version: "0.1.0".to_owned(),
            modules: vec!["demo".to_owned()],
            make_target: MakeTarget::DeployGoPrepare,
            git_credential_lease_id: lease_id.map(str::to_owned),
            timeout_seconds: 30,
            artifact_upload: None,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn release_dispatch(
    task_id: &str,
    idem: &str,
    digest: &str,
    deployment_id: &str,
    checkout_dir: &Path,
    work_root: &Path,
    artifact_dir: &Path,
    sha: &str,
) -> TaskDispatch {
    TaskDispatch {
        task_id: task_id.to_owned(),
        idempotency_key: idem.to_owned(),
        deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
        payload_digest: digest.to_owned(),
        task: TaskPayload::DeploymentRelease(DeploymentReleaseTask {
            deployment_id: deployment_id.to_owned(),
            target_code: "test".to_owned(),
            work_root: work_root.display().to_string(),
            checkout_dir: checkout_dir.display().to_string(),
            artifact_dir: artifact_dir.display().to_string(),
            environment: Environment::Test,
            release_version: "0.1.0".to_owned(),
            commit_sha: sha.to_owned(),
            modules: vec!["demo".to_owned()],
            make_target: MakeTarget::DeployGoRelease,
            timeout_seconds: 30,
            cancel_file: "unused".to_owned(),
            privileged: false,
            privileged_context: None,
            artifact_download: None,
            repository_url: None,
            git_credential_lease_id: None,
            application_slug: None,
            required_env: Vec::new(),
        }),
    }
}

async fn receive_until_result(receiver: &mut mpsc::Receiver<Message>) -> Vec<Message> {
    let mut messages = Vec::new();
    loop {
        let message = tokio::time::timeout(Duration::from_secs(20), receiver.recv())
            .await
            .expect("任务结果超时")
            .expect("任务发送通道提前关闭");
        if let Message::TaskAck(ack) = &message {
            assert_eq!(
                ack.disposition,
                TaskAckDisposition::Accepted,
                "任务被拒绝: {ack:?}"
            );
        }
        let terminal = matches!(message, Message::TaskResult(_));
        messages.push(message);
        if terminal {
            return messages;
        }
    }
}

fn progress_events(messages: &[Message]) -> Vec<&deploy_go_agent_protocol::DeployEvent> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::TaskProgress(progress) => Some(&progress.event),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn prepare_then_release_two_stage_loop_streams_progress() {
    let directory = tempfile::tempdir().unwrap();
    let work_root = directory.path().join("work");
    fs::create_dir(&work_root).unwrap();
    let repo = directory.path().join("repo");
    let release_root = work_root.join("runtime");
    let sha = init_app_repo(&repo, &release_root, PREPARE_SCRIPT);
    let checkout_dir = work_root.join("checkout");
    let output_dir = work_root.join("staging");
    let handler = handler(directory.path());

    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(prepare_dispatch(
                "task_prepare",
                "idem_prepare_0123456789abcdef",
                "sha256:1111111111111111",
                "dep_01",
                repo.to_str().unwrap(),
                &checkout_dir,
                &work_root,
                &output_dir,
                &sha,
                None,
            ))),
            sender.clone(),
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    let Message::TaskAck(ack) = &messages[0] else {
        panic!("第一条任务消息必须是 ACK");
    };
    assert_eq!(ack.disposition, TaskAckDisposition::Accepted);
    let events = progress_events(&messages);
    let names = events
        .iter()
        .map(|event| event.event.clone())
        .collect::<Vec<_>>();
    assert!(names.contains(&deploy_go_agent_protocol::DeployEventName::PreflightSucceeded));
    assert!(names.contains(&deploy_go_agent_protocol::DeployEventName::ModuleSucceeded));
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Succeeded);
    assert!(output_dir.join("deploy-go-artifact.json").exists());

    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(release_dispatch(
                "task_release",
                "idem_release_0123456789abcdef",
                "sha256:2222222222222222",
                "dep_01",
                &checkout_dir,
                &work_root,
                &output_dir,
                &sha,
            ))),
            sender,
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    let events = progress_events(&messages);
    let names = events
        .iter()
        .map(|event| event.event.clone())
        .collect::<Vec<_>>();
    assert!(names.contains(&deploy_go_agent_protocol::DeployEventName::VerificationSucceeded));
    assert!(names.contains(&deploy_go_agent_protocol::DeployEventName::DeployFinished));
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Succeeded);
    assert_eq!(
        fs::read_to_string(release_root.join("app.txt")).unwrap(),
        "hello\n"
    );
}

#[tokio::test]
async fn unclosed_module_turns_exit_zero_prepare_into_protocol_conflict() {
    let directory = tempfile::tempdir().unwrap();
    let work_root = directory.path().join("work");
    fs::create_dir(&work_root).unwrap();
    let repo = directory.path().join("repo");
    let release_root = work_root.join("runtime");
    let sha = init_app_repo(&repo, &release_root, PREPARE_UNFINISHED_SCRIPT);
    let handler = handler(directory.path());
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(prepare_dispatch(
                "task_bad",
                "idem_bad_0123456789abcdef",
                "sha256:3333333333333333",
                "dep_02",
                repo.to_str().unwrap(),
                &work_root.join("checkout"),
                &work_root,
                &work_root.join("staging"),
                &sha,
                None,
            ))),
            sender,
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Failed);
    assert_eq!(
        result.error_code.as_deref(),
        Some("deploy_event_protocol_conflict")
    );
    assert_eq!(result.exit_code, Some(1));
}

#[tokio::test]
async fn secret_lease_roundtrip_writes_then_cleans_git_key() {
    let directory = tempfile::tempdir().unwrap();
    let work_root = directory.path().join("work");
    fs::create_dir(&work_root).unwrap();
    let repo = directory.path().join("repo");
    let release_root = work_root.join("runtime");
    let sha = init_app_repo(&repo, &release_root, PREPARE_SCRIPT);
    let handler = handler(directory.path());
    let (sender, mut receiver) = mpsc::channel(64);
    let dispatch = prepare_dispatch(
        "task_lease",
        "idem_lease_0123456789abcdef",
        "sha256:4444444444444444",
        "dep_03",
        repo.to_str().unwrap(),
        &work_root.join("checkout"),
        &work_root,
        &work_root.join("staging"),
        &sha,
        Some("lease_01"),
    );
    handler
        .handle(
            envelope(Message::TaskDispatch(dispatch.clone())),
            sender.clone(),
        )
        .await
        .unwrap();
    let mut saw_request = false;
    let result = loop {
        let message = tokio::time::timeout(Duration::from_secs(20), receiver.recv())
            .await
            .expect("secret lease 或结果超时")
            .unwrap();
        match message {
            Message::SecretLeaseRequest(request) => {
                saw_request = true;
                assert_eq!(request.lease_id, "lease_01");
                assert_eq!(request.task_id, "task_lease");
                assert_eq!(request.payload_digest, dispatch.payload_digest);
                assert_eq!(request.purpose, SecretLeasePurpose::GitCredential);
                handler
                    .handle(
                        envelope(Message::SecretLeaseResponse(SecretLeaseResponse {
                            lease_id: request.lease_id,
                            private_key: "-----BEGIN OPENSSH PRIVATE KEY-----\nfake\n-----END OPENSSH PRIVATE KEY-----\n"
                                .to_owned(),
                            expires_at: (chrono::Utc::now() + chrono::Duration::minutes(1))
                                .to_rfc3339(),
                            error_code: None,
                        })),
                        sender.clone(),
                    )
                    .await
                    .unwrap();
            }
            Message::TaskAck(ack) => {
                assert_eq!(
                    ack.disposition,
                    TaskAckDisposition::Accepted,
                    "任务被拒绝: {ack:?}"
                );
            }
            Message::TaskResult(result) => break result,
            _ => {}
        }
    };
    assert!(saw_request);
    assert_eq!(result.status, TaskTerminalStatus::Succeeded);
    assert!(!directory.path().join("tasks/task_lease/git-key").exists());
}

#[tokio::test]
async fn tampered_artifact_blocks_release_before_make_target() {
    let directory = tempfile::tempdir().unwrap();
    let work_root = directory.path().join("work");
    fs::create_dir(&work_root).unwrap();
    let repo = directory.path().join("repo");
    let release_root = work_root.join("runtime");
    let sha = init_app_repo(&repo, &release_root, PREPARE_SCRIPT);
    let checkout_dir = work_root.join("checkout");
    let output_dir = work_root.join("staging");
    let handler = handler(directory.path());

    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(prepare_dispatch(
                "task_prepare",
                "idem_prepare_0123456789abcdef",
                "sha256:1111111111111111",
                "dep_01",
                repo.to_str().unwrap(),
                &checkout_dir,
                &work_root,
                &output_dir,
                &sha,
                None,
            ))),
            sender,
        )
        .await
        .unwrap();
    receive_until_result(&mut receiver).await;
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(output_dir.join("demo/app.txt"))
        .unwrap();
    use std::io::Write;
    writeln!(file, "tampered").unwrap();

    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            envelope(Message::TaskDispatch(release_dispatch(
                "task_release",
                "idem_release_0123456789abcdef",
                "sha256:2222222222222222",
                "dep_01",
                &checkout_dir,
                &work_root,
                &output_dir,
                &sha,
            ))),
            sender,
        )
        .await
        .unwrap();
    let messages = receive_until_result(&mut receiver).await;
    let Message::TaskResult(result) = messages.last().unwrap() else {
        unreachable!();
    };
    assert_eq!(result.status, TaskTerminalStatus::Failed);
    assert_eq!(
        result.error_code.as_deref(),
        Some("artifact_verification_failed")
    );
    assert!(!release_root.exists());
}
