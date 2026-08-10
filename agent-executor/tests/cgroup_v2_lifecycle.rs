#![cfg(target_os = "linux")]

use deploy_go_agent_executor::{
    cgroup::{ReleaseCgroup, SessionCgroup},
    pty::PtySession,
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[test]
fn cgroup_kills_detached_double_fork_and_releases_next_session() {
    if std::env::var_os("DEPLOY_GO_RUN_CGROUP_V2_TEST").is_none() {
        eprintln!("跳过真实 cgroup v2 测试；设置 DEPLOY_GO_RUN_CGROUP_V2_TEST=1 启用");
        return;
    }

    let fixture = tempfile::tempdir().unwrap();
    let pid_file = fixture.path().join("detached.pid");
    let launcher = Path::new(env!("CARGO_BIN_EXE_deploy-go-agent-executor")).to_path_buf();
    let session_id = format!("term_CGROUP_{}", std::process::id());
    let cgroup = SessionCgroup::create_with_launcher(&session_id, launcher.clone()).unwrap();
    let mut session = PtySession::spawn(
        Path::new("/bin/sh"),
        Path::new("/tmp"),
        24,
        80,
        16,
        Duration::from_millis(500),
        Some(cgroup),
        None,
    )
    .unwrap();
    session
        .input(
            format!(
                "setsid sh -c '(trap \"\" TERM; echo $$ > {}; while :; do sleep 1; done) &'\n",
                pid_file.display()
            )
            .as_bytes(),
        )
        .unwrap();
    wait_for_file(&pid_file);
    let pid: i32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    session.close().unwrap();
    assert_process_gone(pid);

    let next_id = format!("term_CGROUP_NEXT_{}", std::process::id());
    let next_cgroup = SessionCgroup::create_with_launcher(&next_id, launcher).unwrap();
    let mut next = PtySession::spawn(
        Path::new("/bin/sh"),
        Path::new("/tmp"),
        24,
        80,
        16,
        Duration::from_millis(500),
        Some(next_cgroup),
        None,
    )
    .unwrap();
    next.close().unwrap();
}

#[test]
fn escaped_root_process_cannot_block_reader_cleanup() {
    if std::env::var_os("DEPLOY_GO_RUN_CGROUP_V2_TEST").is_none() {
        return;
    }

    let fixture = tempfile::tempdir().unwrap();
    let pid_file = fixture.path().join("escaped.pid");
    let ready_file = fixture.path().join("escaped.ready");
    let parent_procs = current_cgroup_dir().join("cgroup.procs");
    let launcher = Path::new(env!("CARGO_BIN_EXE_deploy-go-agent-executor")).to_path_buf();
    let session_id = format!("term_CGROUP_READER_{}", std::process::id());
    let cgroup = SessionCgroup::create_with_launcher(&session_id, launcher).unwrap();
    let mut session = PtySession::spawn(
        Path::new("/bin/sh"),
        Path::new("/tmp"),
        24,
        80,
        16,
        Duration::from_millis(250),
        Some(cgroup),
        None,
    )
    .unwrap();
    session
        .input(
            format!(
                "setsid sh -c 'echo $$ > {}; echo $$ > {} || exit 91; touch {}; exec sleep 30' &\n",
                pid_file.display(),
                parent_procs.display(),
                ready_file.display()
            )
            .as_bytes(),
        )
        .unwrap();
    wait_for_file(&ready_file);
    let pid: i32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    let started = Instant::now();
    session.close().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(process_is_running(pid));

    unsafe { libc::kill(pid, libc::SIGKILL) };
    assert_process_gone(pid);
}

#[test]
fn release_cgroup_launcher_uses_fixed_make_and_kills_detached_descendants() {
    if std::env::var_os("DEPLOY_GO_RUN_CGROUP_V2_TEST").is_none() {
        return;
    }
    let fixture = tempfile::tempdir().unwrap();
    let pid_file = fixture.path().join("release-detached.pid");
    std::fs::write(
        fixture.path().join("Makefile"),
        format!(
            "deploy-go-release:\n\t@setsid sh -c 'trap \"\" TERM; echo $$$$ > {}; while :; do sleep 1; done' &\n",
            pid_file.display()
        ),
    )
    .unwrap();
    let launcher = Path::new(env!("CARGO_BIN_EXE_deploy-go-agent-executor")).to_path_buf();
    let job_id = format!("release_CGROUP_{}", std::process::id());
    let cgroup = ReleaseCgroup::create_with_launcher(&job_id, launcher).unwrap();
    let (program, arguments) = cgroup.launcher_command();
    let status = std::process::Command::new(program)
        .args(arguments)
        .current_dir(fixture.path())
        .status()
        .unwrap();
    assert!(status.success());
    wait_for_file(&pid_file);
    let pid: i32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(process_is_running(pid));

    cgroup.kill_all().unwrap();
    cgroup
        .wait_empty_and_remove(Duration::from_secs(2))
        .unwrap();
    assert_process_gone(pid);
}

fn wait_for_file(path: &Path) {
    for _ in 0..100 {
        if path.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("detached process did not write pid file");
}

fn assert_process_gone(pid: i32) {
    for _ in 0..100 {
        let state = std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|value| value.rsplit_once(") ").map(|(_, fields)| fields.to_owned()))
            .and_then(|fields| fields.chars().next());
        if state.is_none() || state == Some('Z') {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("detached process {pid} survived cgroup cleanup");
}

fn process_is_running(pid: i32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|value| value.rsplit_once(") ").map(|(_, fields)| fields.to_owned()))
        .and_then(|fields| fields.chars().next())
        .is_some_and(|state| state != 'Z')
}

fn current_cgroup_dir() -> std::path::PathBuf {
    let cgroup = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap()
        .trim_start_matches('/');
    Path::new("/sys/fs/cgroup").join(relative)
}
