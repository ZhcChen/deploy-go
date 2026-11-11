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
        "capacity" => {
            let expected = std::env::var("DEPLOY_GO_RUNNER_TEST_EXPECTED_CAPACITY").unwrap();
            let actual = runtime.block_on(client.probe_capacity());
            assert_eq!(
                actual.map_or_else(|| "none".to_owned(), |value| value.to_string()),
                expected
            );
        }
        "launch-rejected" => assert!(
            runtime
                .block_on(client.launch(&std::env::var("DEPLOY_GO_RUNNER_TEST_TASK").unwrap()))
                .is_err()
        ),
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
    set_owner_mode(&task_root, AGENT_UID, RUNNER_GID, 0o3710);
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
             ! ls '{}' >/dev/null 2>&1\n\
             test ! -r '{}/task_after_cancel/runner-spec.json'\n\
             printf '%s:%s:%s\\n' \"$(id -u)\" \"$(id -g)\" \"$(id -G)\"",
            credential.display(),
            executor_socket.display(),
            task_root.display(),
            task_root.display()
        ),
    );
    create_task(
        &task_root,
        "task_cancel",
        "trap 'exit 0' TERM; while :; do sleep 1; done",
    );
    create_task(&task_root, "task_after_cancel", "exit 0");
    create_task(&task_root, "task_revoke_failure", "sleep 1");
    create_task(&task_root, "task_blocked_after_revoke", "exit 0");

    let mut broker = spawn_broker(&socket, &task_root);
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
    wait_for_mode(
        &task_root.join("task_complete"),
        0o3700,
        Duration::from_secs(5),
    );
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
    assert_owner_mode(
        &task_root.join("task_cancel"),
        AGENT_UID,
        RUNNER_GID,
        0o3770,
    );
    broker.kill().unwrap();
    broker.wait().unwrap();
    broker = spawn_broker(&socket, &task_root);
    std::thread::sleep(Duration::from_millis(100));
    run_helper(
        "launch-rejected",
        Some("task_after_cancel"),
        AGENT_UID,
        AGENT_GID,
        &[RUNNER_GID],
        &socket,
        &credential,
        &executor_socket,
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
    wait_for_mode(
        &task_root.join("task_cancel"),
        0o3700,
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
    std::thread::sleep(Duration::from_millis(100));
    run_helper(
        "launch",
        Some("task_after_cancel"),
        AGENT_UID,
        AGENT_GID,
        &[RUNNER_GID],
        &socket,
        &credential,
        &executor_socket,
    );
    wait_for(
        &task_root.join("task_after_cancel/completion.json"),
        Duration::from_secs(5),
    );
    wait_for_mode(
        &task_root.join("task_after_cancel"),
        0o3700,
        Duration::from_secs(5),
    );
    broker.kill().unwrap();
    broker.wait().unwrap();
    set_owner_mode(
        &task_root.join("task_cancel"),
        AGENT_UID,
        RUNNER_GID,
        0o3770,
    );
    broker = spawn_broker(&socket, &task_root);
    std::thread::sleep(Duration::from_millis(100));
    wait_for_mode(
        &task_root.join("task_cancel"),
        0o3700,
        Duration::from_secs(5),
    );
    run_helper(
        "launch",
        Some("task_revoke_failure"),
        AGENT_UID,
        AGENT_GID,
        &[RUNNER_GID],
        &socket,
        &credential,
        &executor_socket,
    );
    wait_for(
        &task_root.join("task_revoke_failure/process.json"),
        Duration::from_secs(5),
    );
    set_owner_mode(
        &task_root.join("task_revoke_failure"),
        AGENT_UID,
        RUNNER_GID,
        0o3777,
    );
    wait_for(
        &task_root.join("task_revoke_failure/completion.json"),
        Duration::from_secs(5),
    );
    run_helper(
        "launch-rejected",
        Some("task_blocked_after_revoke"),
        AGENT_UID,
        AGENT_GID,
        &[RUNNER_GID],
        &socket,
        &credential,
        &executor_socket,
    );

    broker.kill().unwrap();
    broker.wait().unwrap();
}

/// 同节点并发部署：默认（未开启）仍是进程级全局串行；本机开启后可以同时运行多个 runner，
/// 每个任务目录的权限独立切换与回收，超过上限被拒绝。必须用真实 root、真实 uid/gid 的容器验证。
#[test]
fn runner_broker_runs_concurrent_tasks_only_when_enabled() {
    if std::env::var_os("DEPLOY_GO_RUNNER_IDENTITY_TEST").is_none() {
        eprintln!("跳过：设置 DEPLOY_GO_RUNNER_IDENTITY_TEST=1 后执行隔离 Linux 身份测试");
        return;
    }
    assert_eq!(unsafe { libc::geteuid() }, 0, "必须在隔离 root 容器运行");
    let groups = [AGENT_GID];
    let launch = |socket: &Path, credential: &Path, executor: &Path, action: &str, task: &str| {
        run_helper(
            action,
            Some(task),
            AGENT_UID,
            AGENT_GID,
            &groups,
            socket,
            credential,
            executor,
        );
    };
    let capacity = |socket: &Path, credential: &Path, executor: &Path, expected: &str| {
        let mut command = helper_command("capacity", None, socket, credential, executor);
        command.env("DEPLOY_GO_RUNNER_TEST_EXPECTED_CAPACITY", expected);
        set_command_identity(&mut command, AGENT_UID, AGENT_GID, &groups);
        assert!(
            command.status().unwrap().success(),
            "capacity 应为 {expected}"
        );
    };
    let hold = "trap 'exit 0' TERM; i=0; while [ $i -lt 20 ]; do sleep 1; i=$((i+1)); done";

    for (label, env, limit) in [
        ("串行（默认）", Vec::<(&str, &str)>::new(), 1_usize),
        (
            "并发（本机开启）",
            vec![("DEPLOY_GO_RUNNER_CONCURRENT", "true")],
            16,
        ),
    ] {
        let fixture = tempfile::tempdir().unwrap();
        let task_root = fixture.path().join("tasks");
        let runtime_root = fixture.path().join("run/runner");
        let executor_root = fixture.path().join("run/executor");
        let socket = runtime_root.join("runner.sock");
        let executor_socket = executor_root.join("executor.sock");
        let credential = fixture.path().join("credentials.json");
        std::fs::create_dir_all(&task_root).unwrap();
        set_owner_mode(&task_root, AGENT_UID, RUNNER_GID, 0o3710);
        std::fs::create_dir_all(&runtime_root).unwrap();
        std::fs::create_dir_all(&executor_root).unwrap();
        set_owner_mode(&executor_root, 0, AGENT_GID, 0o750);
        let _executor_listener = std::os::unix::net::UnixListener::bind(&executor_socket).unwrap();
        set_owner_mode(&executor_socket, 0, AGENT_GID, 0o660);
        std::fs::write(&credential, b"secret").unwrap();
        set_owner_mode(&credential, AGENT_UID, AGENT_GID, 0o600);
        // 并发开启时需要的任务数：上限 + 1（用于验证被拒绝）；这里取 4 个任务做代表性验证。
        let tasks = ["task_c1", "task_c2", "task_c3"];
        for task in tasks {
            create_task(&task_root, task, hold);
        }
        create_task(&task_root, "task_c4", "exit 0");

        let mut broker = spawn_broker_with_env(&socket, &task_root, &env);
        wait_for(&socket, Duration::from_secs(5));
        capacity(&socket, &credential, &executor_socket, &limit.to_string());

        // 第一个任务总能启动；任务目录权限切换为活动模式。
        launch(&socket, &credential, &executor_socket, "launch", "task_c1");
        wait_for_mode(&task_root.join("task_c1"), 0o3770, Duration::from_secs(5));
        if limit == 1 {
            // 默认串行：第二个任务被拒绝，且它的目录保持非活动权限（不对 runner 开放）。
            launch(
                &socket,
                &credential,
                &executor_socket,
                "launch-rejected",
                "task_c2",
            );
            assert_eq!(
                std::fs::metadata(task_root.join("task_c2"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                0o3700,
                "{label}：被拒绝的任务目录不得对 runner 开放"
            );
        } else {
            // 并发：其余任务同时启动成功，三个任务目录同时处于活动权限。
            for task in &tasks[1..] {
                launch(&socket, &credential, &executor_socket, "launch", task);
            }
            for task in tasks {
                wait_for_mode(&task_root.join(task), 0o3770, Duration::from_secs(5));
            }
            // 取消其中一个，只回收它自己的权限，其余任务不受影响。
            launch(&socket, &credential, &executor_socket, "cancel", "task_c2");
            wait_for_mode(&task_root.join("task_c2"), 0o3700, Duration::from_secs(10));
            for task in ["task_c1", "task_c3"] {
                assert_eq!(
                    std::fs::metadata(task_root.join(task))
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o7777,
                    0o3770,
                    "{label}：取消一个任务不得影响其他任务 {task}"
                );
            }
            // 释放出名额后，新的任务可以继续启动。
            launch(&socket, &credential, &executor_socket, "launch", "task_c4");
        }
        let _ = broker.kill();
        let _ = broker.wait();
    }
}

#[test]
fn runner_broker_rejects_unrecognized_concurrency_setting() {
    if std::env::var_os("DEPLOY_GO_RUNNER_IDENTITY_TEST").is_none() {
        eprintln!("跳过：设置 DEPLOY_GO_RUNNER_IDENTITY_TEST=1 后执行隔离 Linux 身份测试");
        return;
    }
    let fixture = tempfile::tempdir().unwrap();
    let task_root = fixture.path().join("tasks");
    let runtime_root = fixture.path().join("run/runner");
    std::fs::create_dir_all(&task_root).unwrap();
    set_owner_mode(&task_root, AGENT_UID, RUNNER_GID, 0o3710);
    std::fs::create_dir_all(&runtime_root).unwrap();
    // 配置错误必须使 broker 拒绝启动（fail-closed），而不是悄悄选一个并发数。
    let mut broker = spawn_broker_with_env(
        &runtime_root.join("runner.sock"),
        &task_root,
        &[("DEPLOY_GO_RUNNER_CONCURRENT", "yes")],
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = broker.try_wait().unwrap() {
            break status;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "broker 应在配置错误时退出"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(!status.success());
}

fn spawn_broker(socket: &Path, task_root: &Path) -> std::process::Child {
    spawn_broker_with_env(socket, task_root, &[])
}

fn spawn_broker_with_env(
    socket: &Path,
    task_root: &Path,
    extra_env: &[(&str, &str)],
) -> std::process::Child {
    let mut broker = Command::new(env!("CARGO_BIN_EXE_deploy-go-agent"));
    for (key, value) in extra_env {
        broker.env(key, value);
    }
    broker
        .arg("runner-service")
        .env("DEPLOY_GO_RUNNER_SOCKET", socket)
        .env("DEPLOY_GO_RUNNER_TASK_ROOT", task_root)
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
    broker.spawn().unwrap()
}

fn create_task(task_root: &Path, task_id: &str, script: &str) {
    let task_dir = task_root.join(task_id);
    std::fs::create_dir(&task_dir).unwrap();
    set_owner_mode(&task_dir, AGENT_UID, RUNNER_GID, 0o3700);
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

fn wait_for_mode(path: &Path, mode: u32, timeout: Duration) {
    use std::os::unix::fs::MetadataExt;

    let deadline = std::time::Instant::now() + timeout;
    loop {
        if std::fs::metadata(path).unwrap().mode() & 0o7777 == mode {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "等待权限恢复超时：{}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// 用真实的不同 UID 验证 checkout 属主检测，并锁住 Git 真实的 `dubious ownership` 文案：
/// 若 Git 升级后改了文案，Agent 对该错误的分类会悄悄失效，这里要能第一时间发现。
#[test]
fn checkout_owned_by_another_user_is_untrusted_ownership() {
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
    // 让 runner 用户也能进入 fixture，才能验证它自己运行 git 时看到的真实报错。
    std::fs::set_permissions(fixture.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    let git = |dir: &Path, args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };

    let origin = fixture.path().join("origin");
    std::fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--initial-branch=main"]);
    git(&origin, &["config", "user.email", "t@example.test"]);
    git(&origin, &["config", "user.name", "t"]);
    std::fs::write(origin.join("README.md"), "x\n").unwrap();
    git(&origin, &["add", "-A"]);
    git(&origin, &["commit", "-q", "-m", "init"]);
    let sha = git(&origin, &["rev-parse", "HEAD"]);

    // 以 root 物化，再把整个 checkout 的属主改成另一个用户：模拟“被别的身份创建/残留的目录”。
    let checkout = fixture.path().join("checkout");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime
        .block_on(deploy_go_agent::git::checkout_commit(
            origin.to_str().unwrap(),
            &sha,
            &checkout,
            None,
            60,
        ))
        .unwrap();
    let status = Command::new("chown")
        .args(["-R", &format!("{OTHER_UID}:{OTHER_GID}")])
        .arg(&checkout)
        .status()
        .unwrap();
    assert!(status.success());

    // 1) Agent 在运行 Git 之前就给出独立错误码和只含数字元数据的诊断，而不是泛化的 git_command_failed。
    let error = runtime
        .block_on(deploy_go_agent::git::checkout_commit(
            origin.to_str().unwrap(),
            &sha,
            &checkout,
            None,
            60,
        ))
        .unwrap_err();
    assert_eq!(
        error.error_code(),
        "git_checkout_ownership_untrusted",
        "{error}"
    );
    let diagnostics = deploy_go_agent::git::checkout_diagnostics(&checkout);
    assert!(diagnostics.contains("euid=0"), "{diagnostics}");
    assert!(
        diagnostics.contains(&format!("uid={OTHER_UID} gid={OTHER_GID}")),
        "{diagnostics}"
    );

    // 2) 锁住 Git 的真实文案：以 runner 用户去访问 root 属主的目录，Git 报 dubious ownership。
    let root_owned = fixture.path().join("root-owned");
    runtime
        .block_on(deploy_go_agent::git::checkout_commit(
            origin.to_str().unwrap(),
            &sha,
            &root_owned,
            None,
            60,
        ))
        .unwrap();
    let output = Command::new("git")
        .arg("-C")
        .arg(&root_owned)
        .arg("status")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .uid(RUNNER_UID)
        .gid(RUNNER_GID)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_ascii_lowercase().contains("dubious ownership"),
        "Git 的 dubious ownership 文案变了，Agent 的错误分类需要同步：{stderr}"
    );
}
