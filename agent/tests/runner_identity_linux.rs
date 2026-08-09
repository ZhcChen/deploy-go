#![cfg(target_os = "linux")]

use std::{
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use deploy_go_agent::{
    executor::Executor,
    journal::{JournalState, RecoveryState},
    runner_service::RunnerServiceClient,
};

const AGENT_UID: u32 = 21001;
const AGENT_GID: u32 = 21001;
const RUNNER_UID: u32 = 21002;
const RUNNER_GID: u32 = 21002;
const OTHER_UID: u32 = 21003;
const OTHER_GID: u32 = 21003;

#[test]
fn runner_client_helper() {
    let Ok(action) = std::env::var("DEPLOY_GO_RUNNER_TEST_ACTION") else {
        return;
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let client = RunnerServiceClient::new(PathBuf::from(
        std::env::var_os("DEPLOY_GO_RUNNER_TEST_SOCKET").unwrap(),
    ));
    match action.as_str() {
        "probe" => assert!(runtime.block_on(client.probe())),
        "probe-rejected" => assert!(!runtime.block_on(client.probe())),
        "launch" => runtime
            .block_on(client.launch(&std::env::var("DEPLOY_GO_RUNNER_TEST_TASK").unwrap()))
            .unwrap(),
        "cancel" => runtime
            .block_on(client.cancel(
                &std::env::var("DEPLOY_GO_RUNNER_TEST_TASK").unwrap(),
                Duration::from_millis(200),
            ))
            .unwrap(),
        "runner-boundaries" => {
            assert!(
                std::fs::read(std::env::var_os("DEPLOY_GO_RUNNER_TEST_CREDENTIAL").unwrap())
                    .is_err()
            );
            assert!(
                std::os::unix::net::UnixStream::connect(
                    std::env::var_os("DEPLOY_GO_RUNNER_TEST_EXECUTOR_SOCKET").unwrap()
                )
                .is_err()
            );
        }
        "recover" => {
            let executor = Executor::new(PathBuf::from(
                std::env::var_os("DEPLOY_GO_RUNNER_TEST_TASK_ROOT").unwrap(),
            ))
            .unwrap();
            let RecoveryState::Terminal(journal) = executor
                .recover(&std::env::var("DEPLOY_GO_RUNNER_TEST_TASK").unwrap())
                .unwrap()
            else {
                panic!("task did not recover to a terminal state");
            };
            let expected = std::env::var("DEPLOY_GO_RUNNER_TEST_EXPECTED_STATE").unwrap();
            assert_eq!(
                journal.state,
                if expected == "succeeded" {
                    JournalState::Succeeded
                } else {
                    JournalState::Canceled
                }
            );
        }
        _ => panic!("unknown helper action"),
    }
}

#[test]
fn runner_broker_enforces_linux_identity_boundaries() {
    if std::env::var_os("DEPLOY_GO_RUNNER_IDENTITY_TEST").is_none() {
        eprintln!("跳过：设置 DEPLOY_GO_RUNNER_IDENTITY_TEST=1 后执行隔离 Linux 身份测试");
        return;
    }
    assert_eq!(
        unsafe { libc::geteuid() },
        0,
        "身份测试必须在隔离 root 容器运行"
    );

    let fixture = tempfile::tempdir().unwrap();
    let task_root = fixture.path().join("tasks");
    let runtime_root = fixture.path().join("run/runner");
    let executor_root = fixture.path().join("run/executor");
    let socket = runtime_root.join("runner.sock");
    let executor_socket = executor_root.join("executor.sock");
    let credential = fixture.path().join("credentials.json");
    std::fs::create_dir_all(&task_root).unwrap();
    set_owner_mode(&task_root, AGENT_UID, RUNNER_GID, 0o3770);
    std::fs::create_dir_all(&runtime_root).unwrap();
    std::fs::create_dir_all(&executor_root).unwrap();
    set_owner_mode(&executor_root, 0, AGENT_GID, 0o750);
    let _executor_listener = std::os::unix::net::UnixListener::bind(&executor_socket).unwrap();
    set_owner_mode(&executor_socket, 0, AGENT_GID, 0o660);
    std::fs::write(&credential, b"secret").unwrap();
    set_owner_mode(&credential, AGENT_UID, AGENT_GID, 0o600);

    create_task(
        &task_root,
        "task_complete",
        &format!(
            "test -z \"${{DEPLOY_GO_RUNNER_TEST_SECRET:-}}\"\n\
             test ! -r '{}'\n\
             test ! -e '{}'\n\
             printf '%s:%s:%s\\n' \"$(id -u)\" \"$(id -g)\" \"$(id -G)\"",
            credential.display(),
            executor_socket.display()
        ),
    );
    create_task(
        &task_root,
        "task_cancel",
        "trap 'exit 0' TERM; while :; do sleep 1; done",
    );

    let agent_binary = env!("CARGO_BIN_EXE_deploy-go-agent");
    let mut broker = Command::new(agent_binary);
    broker
        .arg("runner-service")
        .env("DEPLOY_GO_RUNNER_SOCKET", &socket)
        .env("DEPLOY_GO_RUNNER_TASK_ROOT", &task_root)
        .env("DEPLOY_GO_RUNNER_ALLOWED_UID", AGENT_UID.to_string())
        .env("DEPLOY_GO_RUNNER_ALLOWED_GID", AGENT_GID.to_string())
        .env("DEPLOY_GO_RUNNER_UID", RUNNER_UID.to_string())
        .env("DEPLOY_GO_RUNNER_GID", RUNNER_GID.to_string())
        .env("DEPLOY_GO_RUNNER_TEST_SECRET", "must-not-reach-runner")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    unsafe {
        broker.pre_exec(|| {
            let groups = [AGENT_GID];
            if libc::setgroups(groups.len(), groups.as_ptr()) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut broker = broker.spawn().unwrap();
    wait_for(&socket, Duration::from_secs(5));

    run_helper(
        "probe",
        None,
        AGENT_UID,
        AGENT_GID,
        &[RUNNER_GID],
        &socket,
        &credential,
        &executor_socket,
    );
    run_helper(
        "probe-rejected",
        None,
        RUNNER_UID,
        RUNNER_GID,
        &[AGENT_GID],
        &socket,
        &credential,
        &executor_socket,
    );
    run_helper(
        "probe-rejected",
        None,
        OTHER_UID,
        AGENT_GID,
        &[],
        &socket,
        &credential,
        &executor_socket,
    );
    run_helper(
        "probe-rejected",
        None,
        AGENT_UID,
        OTHER_GID,
        &[AGENT_GID],
        &socket,
        &credential,
        &executor_socket,
    );
    run_helper(
        "runner-boundaries",
        None,
        RUNNER_UID,
        RUNNER_GID,
        &[],
        &socket,
        &credential,
        &executor_socket,
    );

    run_helper(
        "launch",
        Some("task_complete"),
        AGENT_UID,
        AGENT_GID,
        &[RUNNER_GID],
        &socket,
        &credential,
        &executor_socket,
    );
    let completion = task_root.join("task_complete/completion.json");
    wait_for(&completion, Duration::from_secs(5));
    assert_owner_mode(
        &task_root.join("task_complete/runner-launch.lock"),
        0,
        RUNNER_GID,
        0o640,
    );
    assert_owner_mode(
        &task_root.join("task_complete/process.json"),
        RUNNER_UID,
        RUNNER_GID,
        0o640,
    );
    assert_owner_mode(
        &task_root.join("task_complete/stdout.log"),
        RUNNER_UID,
        RUNNER_GID,
        0o640,
    );
    assert_eq!(
        std::fs::read_to_string(task_root.join("task_complete/stdout.log"))
            .unwrap()
            .trim(),
        format!("{RUNNER_UID}:{RUNNER_GID}:{RUNNER_GID}")
    );
    run_recovery_helper(
        "task_complete",
        "succeeded",
        &task_root,
        &socket,
        &credential,
        &executor_socket,
    );

    run_helper(
        "launch",
        Some("task_cancel"),
        AGENT_UID,
        AGENT_GID,
        &[RUNNER_GID],
        &socket,
        &credential,
        &executor_socket,
    );
    wait_for(
        &task_root.join("task_cancel/process.json"),
        Duration::from_secs(5),
    );
    std::fs::write(task_root.join("task_cancel/cancel"), b"").unwrap();
    run_helper(
        "cancel",
        Some("task_cancel"),
        AGENT_UID,
        AGENT_GID,
        &[RUNNER_GID],
        &socket,
        &credential,
        &executor_socket,
    );
    wait_for(
        &task_root.join("task_cancel/completion.json"),
        Duration::from_secs(5),
    );
    let completion: serde_json::Value = serde_json::from_slice(
        &std::fs::read(task_root.join("task_cancel/completion.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(completion["error_code"], "task_canceled");
    let identity: serde_json::Value =
        serde_json::from_slice(&std::fs::read(task_root.join("task_cancel/process.json")).unwrap())
            .unwrap();
    assert!(
        deploy_go_agent::journal::process_start_time(identity["pid"].as_u64().unwrap() as u32)
            .is_err()
    );
    run_recovery_helper(
        "task_cancel",
        "canceled",
        &task_root,
        &socket,
        &credential,
        &executor_socket,
    );

    broker.kill().unwrap();
    broker.wait().unwrap();
}

fn create_task(task_root: &Path, task_id: &str, script: &str) {
    let task_dir = task_root.join(task_id);
    std::fs::create_dir(&task_dir).unwrap();
    set_owner_mode(&task_dir, AGENT_UID, RUNNER_GID, 0o3770);
    let script_path = task_dir.join("deploy.sh");
    std::fs::write(&script_path, format!("#!/bin/sh\nset -eu\n{script}\n")).unwrap();
    set_owner_mode(&script_path, AGENT_UID, RUNNER_GID, 0o750);
    let spec = serde_json::json!({
        "deployment_id": format!("deployment_{task_id}"),
        "script_path": script_path,
        "argument_tokens": [],
        "environment_file_references": [],
        "timeout_seconds": 30,
        "log_budget_bytes": 65536
    });
    let spec_path = task_dir.join("runner-spec.json");
    std::fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).unwrap();
    set_owner_mode(&spec_path, AGENT_UID, RUNNER_GID, 0o640);
    let journal = serde_json::json!({
        "task_id": task_id,
        "idempotency_key": format!("idem_{task_id}_0123456789"),
        "payload_digest": "sha256:0123456789abcdef",
        "state": "running",
        "pid": null,
        "process_start_time": null,
        "stdout_offset": 0,
        "stderr_offset": 0,
        "events_offset": 0,
        "last_sequence": 0,
        "result_sequence": null,
        "git_lease_id": null,
        "exit_code": null,
        "error_code": null,
        "result_data": null,
        "transfer_phase": null
    });
    let journal_path = task_dir.join("journal.json");
    std::fs::write(&journal_path, serde_json::to_vec(&journal).unwrap()).unwrap();
    set_owner_mode(&journal_path, AGENT_UID, RUNNER_GID, 0o640);
}

fn run_recovery_helper(
    task_id: &str,
    expected_state: &str,
    task_root: &Path,
    socket: &Path,
    credential: &Path,
    executor_socket: &Path,
) {
    let mut command = helper_command(
        "recover",
        Some(task_id),
        socket,
        credential,
        executor_socket,
    );
    command
        .env("DEPLOY_GO_RUNNER_TEST_TASK_ROOT", task_root)
        .env("DEPLOY_GO_RUNNER_TEST_EXPECTED_STATE", expected_state);
    set_command_identity(&mut command, AGENT_UID, AGENT_GID, &[RUNNER_GID]);
    assert!(
        command.status().unwrap().success(),
        "recovery helper failed"
    );
}

#[allow(clippy::too_many_arguments)]
fn run_helper(
    action: &str,
    task_id: Option<&str>,
    uid: u32,
    gid: u32,
    groups: &[u32],
    socket: &Path,
    credential: &Path,
    executor_socket: &Path,
) {
    let mut command = helper_command(action, task_id, socket, credential, executor_socket);
    set_command_identity(&mut command, uid, gid, groups);
    assert!(
        command.status().unwrap().success(),
        "helper failed: {action}"
    );
}

fn helper_command(
    action: &str,
    task_id: Option<&str>,
    socket: &Path,
    credential: &Path,
    executor_socket: &Path,
) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("runner_client_helper")
        .arg("--nocapture")
        .env("DEPLOY_GO_RUNNER_TEST_ACTION", action)
        .env("DEPLOY_GO_RUNNER_TEST_SOCKET", socket)
        .env("DEPLOY_GO_RUNNER_TEST_CREDENTIAL", credential)
        .env("DEPLOY_GO_RUNNER_TEST_EXECUTOR_SOCKET", executor_socket)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(task_id) = task_id {
        command.env("DEPLOY_GO_RUNNER_TEST_TASK", task_id);
    }
    command
}

fn set_command_identity(command: &mut Command, uid: u32, gid: u32, groups: &[u32]) {
    let groups = groups.to_vec();
    unsafe {
        command.pre_exec(move || {
            if libc::setgroups(groups.len(), groups.as_ptr()) != 0
                || libc::setgid(gid) != 0
                || libc::setuid(uid) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

fn set_owner_mode(path: &Path, uid: u32, gid: u32, mode: u32) {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::chown(path.as_ptr(), uid, gid) }, 0);
    std::fs::set_permissions(
        path.to_str().unwrap(),
        std::fs::Permissions::from_mode(mode),
    )
    .unwrap();
}

fn assert_owner_mode(path: &Path, uid: u32, gid: u32, mode: u32) {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path).unwrap();
    assert_eq!(metadata.uid(), uid, "owner mismatch: {}", path.display());
    assert_eq!(metadata.gid(), gid, "group mismatch: {}", path.display());
    assert_eq!(
        metadata.mode() & 0o7777,
        mode,
        "mode mismatch: {}",
        path.display()
    );
}

fn wait_for(path: &Path, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    while !path.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "等待文件超时：{}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
