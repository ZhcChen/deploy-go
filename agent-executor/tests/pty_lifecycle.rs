use deploy_go_agent_executor::pty::{MAX_INPUT_BYTES, PtySession};
use std::{path::Path, time::Duration};

fn session() -> PtySession {
    PtySession::spawn(
        Path::new("/bin/sh"),
        Path::new("/tmp"),
        24,
        80,
        16,
        Duration::from_millis(100),
        #[cfg(target_os = "linux")]
        None,
        None,
    )
    .unwrap()
}

#[test]
fn starts_as_a_root_login_environment_in_the_configured_home() {
    let session = session();
    session.input(b"stty -echo\n").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    while session.recv_output_timeout(Duration::ZERO).is_some() {}
    session
        .input(b"printf 'home=%s user=%s logname=%s shell=%s cwd=%s path=%s root-env-end\\n' \"$HOME\" \"$USER\" \"$LOGNAME\" \"$SHELL\" \"$PWD\" \"$PATH\"\n")
        .unwrap();
    let output = read_until(&session, "root-env-end");
    assert!(output.contains("home=/tmp"), "{output:?}");
    assert!(output.contains("user=root"), "{output:?}");
    assert!(output.contains("logname=root"), "{output:?}");
    assert!(output.contains("shell=/bin/sh"), "{output:?}");
    assert!(output.contains("cwd=/tmp"), "{output:?}");
    assert!(
        output.contains("path=") && output.contains("/usr"),
        "{output:?}"
    );
}

fn read_until(session: &PtySession, needle: &str) -> String {
    let mut output = Vec::new();
    for _ in 0..20 {
        if let Some(chunk) = session.recv_output_timeout(Duration::from_millis(100)) {
            output.extend(chunk);
            let text = String::from_utf8_lossy(&output);
            if text.contains(needle) {
                return text.into_owned();
            }
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

#[test]
fn supports_input_output_resize_interrupt_and_idempotent_close() {
    let mut session = session();
    session.input(b"printf 'executor-ready\\n'\n").unwrap();
    assert!(read_until(&session, "executor-ready").contains("executor-ready"));
    session.resize(40, 120).unwrap();
    session.input(b"sleep 10\n").unwrap();
    session.input(&[3]).unwrap();
    session.input(b"printf 'interrupted\\n'\n").unwrap();
    assert!(read_until(&session, "interrupted").contains("interrupted"));
    session.close().unwrap();
    session.close().unwrap();
}

#[test]
fn rejects_oversized_input_and_invalid_resize() {
    let session = session();
    assert!(session.input(&vec![0; MAX_INPUT_BYTES + 1]).is_err());
    assert!(session.resize(0, 80).is_err());
}

#[test]
fn dropping_session_terminates_child() {
    let mut session = session();
    session.input(b"sleep 30\n").unwrap();
    session.close().unwrap();
    drop(session);
}

#[cfg(target_os = "linux")]
#[test]
fn close_kills_descendants_that_ignore_term() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("descendant.pid");
    let mut session = session();
    session
        .input(
            format!(
                "trap '' TERM; sleep 30 & echo $! > {}; wait\n",
                pid_file.display()
            )
            .as_bytes(),
        )
        .unwrap();
    for _ in 0..50 {
        if pid_file.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let pid: i32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    session.close().unwrap();

    for _ in 0..50 {
        if unsafe { libc::kill(pid, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("PTY descendant {pid} survived session close");
}

#[test]
fn reports_normal_child_exit_without_waiting_for_idle_timeout() {
    let mut session = session();
    session.input(b"exit 7\n").unwrap();
    for _ in 0..50 {
        if let Some(code) = session.try_wait().unwrap() {
            assert_eq!(code, 7);
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("PTY child did not report its exit");
}
