use deploy_go_agent_executor::pty::{MAX_INPUT_BYTES, PtySession};
use std::{path::Path, time::Duration};

fn session() -> PtySession {
    PtySession::spawn(Path::new("/bin/sh"), 24, 80, 16, Duration::from_millis(100)).unwrap()
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
