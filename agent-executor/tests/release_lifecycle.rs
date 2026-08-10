#![cfg(unix)]

use deploy_go_agent_executor::{
    protocol::ReleaseJobState,
    release::{FIXED_MAKE_PATH, SealedRelease},
    release_job::{ReleaseJobError, ReleaseJobManager},
};
use deploy_go_release_authorization::{AUDIENCE, Claims, FileDigest, SCHEMA_VERSION};
use std::{fs, path::PathBuf, time::Duration};

fn sealed(
    root: &std::path::Path,
    job_id: &str,
    recipe: &str,
    deadline_after: i64,
) -> SealedRelease {
    assert!(std::path::Path::new(FIXED_MAKE_PATH).is_file());
    let job_dir = root.join(job_id);
    let checkout = job_dir.join("bundle/checkout");
    let artifact = job_dir.join("bundle/artifacts");
    let env = job_dir.join("bundle/env");
    fs::create_dir_all(&checkout).unwrap();
    fs::create_dir_all(&artifact).unwrap();
    fs::create_dir_all(&env).unwrap();
    fs::write(
        checkout.join("Makefile"),
        format!("deploy-go-release:\n\t{recipe}\n"),
    )
    .unwrap();
    SealedRelease {
        job_dir,
        checkout_dir: checkout,
        artifact_dir: artifact,
        env_dir: env,
        claims: claims(deadline_after),
    }
}

fn claims(deadline_after: i64) -> Claims {
    let now = chrono::Utc::now().timestamp();
    Claims {
        schema_version: SCHEMA_VERSION,
        audience: AUDIENCE.into(),
        authorization_id: "release_auth_LIFECYCLE".into(),
        nonce: "release_nonce_LIFECYCLE".into(),
        deployment_id: "deployment_LIFECYCLE".into(),
        target_run_id: "run_LIFECYCLE".into(),
        target_id: "target_LIFECYCLE".into(),
        node_id: "node_LIFECYCLE".into(),
        agent_id: "agent_LIFECYCLE".into(),
        snapshot_hash: format!("sha256:{}", "a".repeat(64)),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        checkout_tree_digest: format!("sha256:{}", "b".repeat(64)),
        artifact_manifest_digest: format!("sha256:{}", "c".repeat(64)),
        artifacts: vec![FileDigest {
            relative_path: "api/app.tar.gz".into(),
            digest: format!("sha256:{}", "d".repeat(64)),
        }],
        env_files: Vec::new(),
        environment: "test".into(),
        release_version: "20260810000000".into(),
        modules: vec!["api".into()],
        task_payload_digest: format!("sha256:{}", "e".repeat(64)),
        cancel_file: "/run/deploy-go/release/cancel".into(),
        issued_at: now,
        expires_at: now + deadline_after,
        deadline_at: now + deadline_after,
    }
}

fn wait_terminal(
    manager: &ReleaseJobManager,
    job_id: &str,
    digest: &str,
) -> deploy_go_agent_executor::release_job::ReleaseJobSnapshot {
    for _ in 0..300 {
        let state = manager.status(job_id, digest).unwrap();
        if matches!(
            state.state,
            ReleaseJobState::Succeeded
                | ReleaseJobState::Failed
                | ReleaseJobState::Canceled
                | ReleaseJobState::TimedOut
        ) {
            return state;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("release job did not reach terminal state");
}

#[test]
fn reports_success_nonzero_and_ordered_output() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("jobs");
    fs::create_dir(&root).unwrap();
    let manager = ReleaseJobManager::new(root.clone());
    let success = sealed(
        &root,
        "release_SUCCESS",
        "@printf 'stdout-line\\n'; printf 'stderr-line\\n' >&2",
        10,
    );
    let digest = success.claims.task_payload_digest.clone();
    manager.start(success, "test").unwrap();
    let state = wait_terminal(&manager, "release_SUCCESS", &digest);
    assert_eq!(state.state, ReleaseJobState::Succeeded);
    assert_eq!(state.exit_code, Some(0));
    let output = manager.output("release_SUCCESS", &digest, 0, 32).unwrap();
    assert!(
        output
            .frames
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );
    let bytes = output
        .frames
        .into_iter()
        .flat_map(|frame| frame.data)
        .collect::<Vec<_>>();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("stdout-line"));
    assert!(text.contains("stderr-line"));

    let failed = sealed(&root, "release_FAILED", "@exit 7", 10);
    let failed_digest = failed.claims.task_payload_digest.clone();
    manager.start(failed, "test").unwrap();
    let state = wait_terminal(&manager, "release_FAILED", &failed_digest);
    assert_eq!(state.state, ReleaseJobState::Failed);
    assert_eq!(state.exit_code, Some(2));
}

#[test]
fn cancel_timeout_and_output_limit_terminate_process_group() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("jobs");
    fs::create_dir(&root).unwrap();
    let manager = ReleaseJobManager::new(root.clone()).with_limits(1024, Duration::from_millis(50));

    let canceled = sealed(&root, "release_CANCELED", "@sleep 30", 20);
    let cancel_digest = canceled.claims.task_payload_digest.clone();
    manager.start(canceled, "test").unwrap();
    manager.cancel("release_CANCELED", &cancel_digest).unwrap();
    assert_eq!(
        wait_terminal(&manager, "release_CANCELED", &cancel_digest).state,
        ReleaseJobState::Canceled
    );

    let timed_out = sealed(&root, "release_TIMEOUT", "@sleep 30", 1);
    let timeout_digest = timed_out.claims.task_payload_digest.clone();
    manager.start(timed_out, "test").unwrap();
    assert_eq!(
        wait_terminal(&manager, "release_TIMEOUT", &timeout_digest).state,
        ReleaseJobState::TimedOut
    );

    let overflow = sealed(
        &root,
        "release_OVERFLOW",
        "@yes output | head -c 100000",
        10,
    );
    let overflow_digest = overflow.claims.task_payload_digest.clone();
    manager.start(overflow, "test").unwrap();
    let state = wait_terminal(&manager, "release_OVERFLOW", &overflow_digest);
    assert_eq!(state.state, ReleaseJobState::Failed);
    assert!(state.output_truncated);
    assert_eq!(state.reason.as_deref(), Some("output_limit_exceeded"));
}

#[test]
fn socket_independent_job_is_queryable_and_restart_blocks_while_child_lives() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("jobs");
    fs::create_dir(&root).unwrap();
    let manager = ReleaseJobManager::new(root.clone()).with_limits(4096, Duration::from_millis(50));
    let release = sealed(&root, "release_RECONNECT", "@sleep 30", 20);
    let digest = release.claims.task_payload_digest.clone();
    manager.start(release, "test").unwrap();

    let restarted = ReleaseJobManager::new(PathBuf::from(&root));
    assert_eq!(
        restarted.reconcile_after_restart(),
        Err(ReleaseJobError::RecoveryBlocked)
    );
    assert_eq!(
        restarted
            .status("release_RECONNECT", &digest)
            .unwrap()
            .state,
        ReleaseJobState::Running
    );
    manager.cancel("release_RECONNECT", &digest).unwrap();
    assert_eq!(
        wait_terminal(&manager, "release_RECONNECT", &digest).state,
        ReleaseJobState::Canceled
    );
    restarted.reconcile_after_restart().unwrap();
}
