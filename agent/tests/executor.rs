use std::{fs, path::Path};

use deploy_go_agent::{
    executor::{ExecuteError, Executor},
    journal::{JournalState, TransferPhase},
};
use deploy_go_agent_protocol::{
    ArtifactDownloadRequest, DeploymentExecuteTask, DeploymentReleaseTask, Environment,
    EnvironmentFileReference, MakeTarget,
};

fn make_script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

fn task(root: &Path, script: &Path) -> DeploymentExecuteTask {
    DeploymentExecuteTask {
        deployment_id: "dep_01".to_owned(),
        work_root: root.display().to_string(),
        script_path: script.display().to_string(),
        argument_tokens: vec!["--environment".to_owned(), "test".to_owned()],
        environment_file_references: Vec::new(),
        timeout_seconds: 10,
        wrapper_version: "1".to_owned(),
    }
}

fn cross_node_release(root: &Path) -> DeploymentReleaseTask {
    DeploymentReleaseTask {
        deployment_id: "dep_release".to_owned(),
        target_code: "production".to_owned(),
        work_root: root.display().to_string(),
        checkout_dir: root.join("checkout").display().to_string(),
        artifact_dir: root.join("artifact").display().to_string(),
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
        image_spec: None,
    }
}

#[tokio::test]
async fn executor_output_frame_replay_rebuilds_logs_without_duplication() {
    let directory = tempfile::tempdir().unwrap();
    let executor = Executor::new(directory.path().join("tasks")).unwrap();
    executor
        .create_transfer_task(
            "task_frames",
            "idem_frames_0123456789",
            "sha256:abcdef0123456789",
            deploy_go_agent::journal::TransferPhase::PrivilegedRelease,
        )
        .await
        .unwrap();
    executor
        .persist_external_output(
            "task_frames",
            1,
            deploy_go_agent_protocol::OutputStream::Stdout,
            b"one\n",
        )
        .unwrap();
    executor
        .persist_external_output(
            "task_frames",
            1,
            deploy_go_agent_protocol::OutputStream::Stdout,
            b"one\n",
        )
        .unwrap();
    executor
        .persist_external_output(
            "task_frames",
            2,
            deploy_go_agent_protocol::OutputStream::Stderr,
            b"two\n",
        )
        .unwrap();
    let task = executor.task_dir("task_frames");
    assert_eq!(fs::read(task.join("stdout.log")).unwrap(), b"one\n");
    assert_eq!(fs::read(task.join("stderr.log")).unwrap(), b"two\n");
    assert!(
        executor
            .persist_external_output(
                "task_frames",
                1,
                deploy_go_agent_protocol::OutputStream::Stdout,
                b"tampered\n",
            )
            .is_err()
    );
}

#[test]
fn cross_node_release_rejects_overlapping_and_symlinked_payload_paths() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    let executor = Executor::new(directory.path().join("tasks")).unwrap();

    let mut release = cross_node_release(&root);
    release.artifact_dir = release.checkout_dir.clone();
    assert!(matches!(
        executor.validate_cross_node_release_payload(&release),
        Err(ExecuteError::PathOutsideWorkRoot)
    ));
    release.artifact_dir = release.work_root.clone();
    assert!(matches!(
        executor.validate_cross_node_release_payload(&release),
        Err(ExecuteError::PathOutsideWorkRoot)
    ));
    release.artifact_dir = "/".to_owned();
    assert!(matches!(
        executor.validate_cross_node_release_payload(&release),
        Err(ExecuteError::PathOutsideWorkRoot)
    ));

    let external = directory.path().join("external");
    fs::create_dir(&external).unwrap();
    fs::write(external.join("sentinel"), b"unchanged").unwrap();
    let linked = directory.path().join("linked");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&external, &linked).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&external, &linked).unwrap();
    let release = cross_node_release(&linked);
    assert!(matches!(
        executor.validate_cross_node_release_payload(&release),
        Err(ExecuteError::PathOutsideWorkRoot)
    ));
    assert_eq!(fs::read(external.join("sentinel")).unwrap(), b"unchanged");
    assert!(!directory.path().join("tasks/task_release").exists());
}

#[tokio::test]
async fn transfer_admission_is_durable_and_cancelable_without_a_runner() {
    let directory = tempfile::tempdir().unwrap();
    let executor = Executor::new(directory.path().join("tasks")).unwrap();
    let admitted = executor
        .create_transfer_task(
            "task_release",
            "idem_release_0123456789",
            "sha256:0123456789abcdef",
            TransferPhase::ReleaseDownload,
        )
        .await
        .unwrap();
    assert_eq!(admitted.state, JournalState::Running);
    assert_eq!(
        admitted.transfer_phase,
        Some(TransferPhase::ReleaseDownload)
    );

    let canceled = executor.cancel("task_release").await.unwrap();
    assert_eq!(canceled.state, JournalState::Canceled);
    assert_eq!(canceled.transfer_phase, None);
    assert!(executor.task_dir("task_release").join("cancel").is_file());

    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    assert!(matches!(
        executor
            .start_admitted_cross_node_release(
                "task_release",
                "sha256:0123456789abcdef",
                &cross_node_release(&root),
                None,
                None,
            )
            .await,
        Err(ExecuteError::Duplicate)
    ));

    executor
        .create_transfer_task(
            "task_cancel_marker",
            "idem_cancel_marker_012345",
            "sha256:abcdef0123456789",
            TransferPhase::ReleaseDownload,
        )
        .await
        .unwrap();
    executor.request_cancel("task_cancel_marker").await.unwrap();
    assert!(matches!(
        executor
            .start_admitted_cross_node_release(
                "task_cancel_marker",
                "sha256:abcdef0123456789",
                &cross_node_release(&root),
                None,
                None,
            )
            .await,
        Err(ExecuteError::InvalidState)
    ));
    assert!(
        !executor
            .task_dir("task_cancel_marker")
            .join("runner-spec.json")
            .exists()
    );
}

#[tokio::test]
async fn executes_once_and_persists_bounded_output_and_exact_result() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    let secrets = directory.path().join("secrets");
    fs::create_dir(&secrets).unwrap();
    let secret_file = secrets.join("config-value");
    let secret_value = "canary-sensitive-value-019fbddb";
    fs::write(&secret_file, secret_value).unwrap();
    let script = root.join("deploy.sh");
    make_script(
        &script,
        "test \"$(cat \"$CONFIG_FILE\")\" = 'canary-sensitive-value-019fbddb'; test \"$DEPLOY_ID\" = dep_01; printf 'hello stdout\\n'; printf 'hello stderr\\n' >&2; exit 7",
    );
    let executor = Executor::new(directory.path().join("tasks"))
        .unwrap()
        .with_runner_binary(Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).to_owned());

    let mut deployment = task(&root, &script);
    deployment
        .environment_file_references
        .push(EnvironmentFileReference {
            environment_key: "CONFIG_FILE".to_owned(),
            file_path: secret_file.display().to_string(),
        });
    let running = executor
        .execute(
            "task_01",
            "idem_0123456789abcdef",
            "sha256:0123456789abcdef",
            &deployment,
        )
        .await
        .unwrap();
    assert_eq!(running.state, JournalState::Running);
    assert!(matches!(
        executor
            .execute(
                "task_01",
                "idem_0123456789abcdef",
                "sha256:0123456789abcdef",
                &deployment,
            )
            .await,
        Err(ExecuteError::Duplicate)
    ));
    let finished = executor.finish("task_01").await.unwrap();
    assert_eq!(finished.state, JournalState::Failed);
    assert_eq!(finished.exit_code, Some(7));
    let task_dir = directory.path().join("tasks/task_01");
    assert_eq!(
        fs::read_to_string(task_dir.join("stdout.log")).unwrap(),
        "hello stdout\n"
    );
    assert_eq!(
        fs::read_to_string(task_dir.join("stderr.log")).unwrap(),
        "hello stderr\n"
    );
    let journal = fs::read_to_string(task_dir.join("journal.json")).unwrap();
    assert!(!journal.contains("--environment"));
    for entry in fs::read_dir(&task_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            assert!(
                !fs::read(entry.path())
                    .unwrap()
                    .windows(secret_value.len())
                    .any(|bytes| bytes == secret_value.as_bytes())
            );
        }
    }
}

#[tokio::test]
async fn output_budget_is_shared_between_streams() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    let script = root.join("deploy.sh");
    make_script(
        &script,
        "printf '12345678901234567890'; printf 'abcdefghijklmnopqrst' >&2",
    );
    let executor = Executor::new(directory.path().join("tasks"))
        .unwrap()
        .with_runner_binary(Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).to_owned())
        .with_log_budget(24);
    executor
        .execute(
            "task_budget",
            "idem_budget_0123456789",
            "sha256:0123456789abcdef",
            &task(&root, &script),
        )
        .await
        .unwrap();
    assert_eq!(
        executor.finish("task_budget").await.unwrap().state,
        JournalState::Succeeded
    );
    let task_dir = directory.path().join("tasks/task_budget");
    let bytes = fs::metadata(task_dir.join("stdout.log")).unwrap().len()
        + fs::metadata(task_dir.join("stderr.log")).unwrap().len();
    assert_eq!(bytes, 24);
}

#[tokio::test]
async fn oversized_log_line_is_truncated_while_the_pipe_is_fully_consumed() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    let script = root.join("deploy.sh");
    make_script(
        &script,
        "head -c 70000 /dev/zero | tr '\\0' x; printf '\\nafter\\n'",
    );
    let executor = Executor::new(directory.path().join("tasks"))
        .unwrap()
        .with_runner_binary(Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).to_owned());
    executor
        .execute(
            "task_line_limit",
            "idem_line_limit_012345",
            "sha256:0123456789abcdef",
            &task(&root, &script),
        )
        .await
        .unwrap();
    assert_eq!(
        executor.finish("task_line_limit").await.unwrap().state,
        JournalState::Succeeded
    );
    let output = fs::read(directory.path().join("tasks/task_line_limit/stdout.log")).unwrap();
    assert!(
        output
            .windows(b"[deploy-go:line_truncated]".len())
            .any(|window| window == b"[deploy-go:line_truncated]")
    );
    assert!(output.ends_with(b"after\n"));
    assert!(output.len() < 66 * 1024);
}

#[tokio::test]
async fn rejects_path_escape_unreadable_reference_and_payload_conflict() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    let outside = directory.path().join("outside.sh");
    make_script(&outside, "exit 0");
    let executor = Executor::new(directory.path().join("tasks"))
        .unwrap()
        .with_runner_binary(Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).to_owned());
    assert!(matches!(
        executor
            .execute(
                "task_escape",
                "idem_escape_0123456789",
                "sha256:0123456789abcdef",
                &task(&root, &outside)
            )
            .await,
        Err(ExecuteError::PathOutsideWorkRoot)
    ));

    let script = root.join("deploy.sh");
    make_script(&script, "sleep 1");
    let mut invalid_reference = task(&root, &script);
    invalid_reference
        .environment_file_references
        .push(EnvironmentFileReference {
            environment_key: "SECRET_FILE".to_owned(),
            file_path: root.join("missing").display().to_string(),
        });
    assert!(matches!(
        executor
            .execute(
                "task_missing",
                "idem_missing_012345678",
                "sha256:0123456789abcdef",
                &invalid_reference
            )
            .await,
        Err(ExecuteError::InaccessiblePath)
    ));

    executor
        .execute(
            "task_conflict",
            "idem_conflict_01234567",
            "sha256:0123456789abcdef",
            &task(&root, &script),
        )
        .await
        .unwrap();
    assert!(matches!(
        executor
            .execute(
                "task_conflict",
                "idem_conflict_01234567",
                "sha256:fedcba9876543210",
                &task(&root, &script)
            )
            .await,
        Err(ExecuteError::PayloadConflict)
    ));

    assert!(matches!(
        executor
            .execute(
                "task_other",
                "idem_conflict_01234567",
                "sha256:fedcba9876543210",
                &task(&root, &script),
            )
            .await,
        Err(ExecuteError::PayloadConflict)
    ));
    assert!(matches!(
        executor
            .execute(
                "task_duplicate_key",
                "idem_conflict_01234567",
                "sha256:0123456789abcdef",
                &task(&root, &script),
            )
            .await,
        Err(ExecuteError::Duplicate)
    ));
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn cancellation_escalates_for_a_process_group_that_ignores_term() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("work");
    fs::create_dir(&root).unwrap();
    let script = root.join("deploy.sh");
    make_script(&script, "trap '' TERM; while :; do sleep 1; done");
    let executor = Executor::new(directory.path().join("tasks"))
        .unwrap()
        .with_runner_binary(Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).to_owned())
        .with_cancel_grace(std::time::Duration::from_millis(50));
    executor
        .execute(
            "task_cancel",
            "idem_cancel_0123456789",
            "sha256:0123456789abcdef",
            &task(&root, &script),
        )
        .await
        .unwrap();
    let canceled = executor.cancel("task_cancel").await.unwrap();
    assert_eq!(canceled.state, JournalState::Canceled);
}
