use deploy_go_agent::journal::{JournalState, JournalStore, RecoveryState, TransferPhase};

#[cfg(target_os = "linux")]
use deploy_go_agent::journal::process_start_time;

#[test]
fn journal_contains_identity_and_offsets_but_not_task_payload() {
    let directory = tempfile::tempdir().unwrap();
    let store = JournalStore::new(directory.path().join("tasks"));
    let task = store
        .create(
            "task_01",
            "idem_0123456789abcdef",
            "sha256:0123456789abcdef",
        )
        .unwrap();
    assert_eq!(task.state, JournalState::Accepted);

    let serialized =
        std::fs::read_to_string(store.task_dir("task_01").join("journal.json")).unwrap();
    assert!(!serialized.contains("script_path"));
    assert!(!serialized.contains("environment"));
    assert!(!serialized.contains("secret"));
}

#[test]
#[cfg(target_os = "linux")]
fn running_process_is_recovered_only_when_pid_identity_matches() {
    let directory = tempfile::tempdir().unwrap();
    let store = JournalStore::new(directory.path().join("tasks"));
    let mut task = store
        .create(
            "task_01",
            "idem_0123456789abcdef",
            "sha256:0123456789abcdef",
        )
        .unwrap();
    task.state = JournalState::Running;
    task.pid = Some(std::process::id());
    task.process_start_time = Some(process_start_time(std::process::id()).unwrap());
    store.store(&task).unwrap();
    assert!(matches!(
        store.recover("task_01").unwrap(),
        RecoveryState::Running(_)
    ));

    task.process_start_time = Some(task.process_start_time.unwrap() + 1);
    store.store(&task).unwrap();
    let RecoveryState::Interrupted(interrupted) = store.recover("task_01").unwrap() else {
        panic!("mismatched process identity must be interrupted");
    };
    assert_eq!(
        interrupted.error_code.as_deref(),
        Some("process_identity_lost")
    );
    assert_eq!(store.load("task_01").unwrap(), interrupted);
}

#[test]
fn invalid_task_identity_cannot_escape_the_journal_root() {
    let directory = tempfile::tempdir().unwrap();
    let store = JournalStore::new(directory.path().join("tasks"));
    assert!(
        store
            .create(
                "../escape",
                "idem_0123456789abcdef",
                "sha256:0123456789abcdef",
            )
            .is_err()
    );
    assert!(
        store
            .create("task_01", "idem_0123456789abcdef", "bad digest value")
            .is_err()
    );
}

#[test]
fn completion_marker_wins_over_a_disappeared_process() {
    let directory = tempfile::tempdir().unwrap();
    let store = JournalStore::new(directory.path().join("tasks"));
    let mut task = store
        .create(
            "task_01",
            "idem_0123456789abcdef",
            "sha256:0123456789abcdef",
        )
        .unwrap();
    task.state = JournalState::Running;
    task.pid = Some(u32::MAX);
    task.process_start_time = Some(1);
    store.store(&task).unwrap();
    std::fs::write(
        store.task_dir("task_01").join("completion.json"),
        r#"{"exit_code":0,"error_code":null}"#,
    )
    .unwrap();

    let deploy_go_agent::journal::RecoveryState::Terminal(recovered) =
        store.recover("task_01").unwrap()
    else {
        panic!("completion marker must produce a terminal state");
    };
    assert_eq!(recovered.state, JournalState::Succeeded);
    assert_eq!(recovered.exit_code, Some(0));
}

#[test]
fn accepted_task_without_a_started_runner_recovers_as_interrupted() {
    let directory = tempfile::tempdir().unwrap();
    let store = JournalStore::new(directory.path().join("tasks"));
    store
        .create(
            "task_01",
            "idem_0123456789abcdef",
            "sha256:0123456789abcdef",
        )
        .unwrap();
    let deploy_go_agent::journal::RecoveryState::Interrupted(recovered) =
        store.recover("task_01").unwrap()
    else {
        panic!("task without a runner must be interrupted");
    };
    assert_eq!(recovered.error_code.as_deref(), Some("runner_not_started"));
}

#[test]
fn active_transfer_phase_recovers_without_a_runner_process() {
    let directory = tempfile::tempdir().unwrap();
    let store = JournalStore::new(directory.path().join("tasks"));
    for (index, phase) in [
        TransferPhase::PrepareUpload,
        TransferPhase::ReleaseDownload,
        TransferPhase::ReleaseExtract,
    ]
    .into_iter()
    .enumerate()
    {
        let task_id = format!("task_transfer_{index}");
        let mut task = store
            .create(
                &task_id,
                &format!("idem_transfer_{index}_0123456789"),
                "sha256:0123456789abcdef",
            )
            .unwrap();
        task.state = JournalState::Running;
        task.transfer_phase = Some(phase.clone());
        store.store(&task).unwrap();
        let RecoveryState::Running(recovered) = store.recover(&task_id).unwrap() else {
            panic!("活动传输阶段必须保持可恢复状态")
        };
        assert_eq!(recovered.transfer_phase, Some(phase));
        assert_eq!(recovered.error_code, None);
    }
}
