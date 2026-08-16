use deploy_go_agent_protocol::{
    ArtifactUploadRequest, DeploymentPrepareTask, Environment, MakeTarget, Message, OutputStream,
    ReconcileReport, ReconciledTask, ReconciledTaskState, SourcePolicy, TaskAck,
    TaskAckDisposition, TaskLifecycleState, TaskOutput, TaskPayload, TaskResult, TaskState,
    TaskTerminalStatus,
};
use deploy_go_api::{
    AppState,
    agents::dispatcher::{
        dispatch_next_deployment, enqueue_deployment, ensure_deployment_task, handle_agent_message,
        request_deployment_cancel, requeue_expired_deliveries, try_dispatch,
    },
    db,
};
use deploy_go_api::{artifacts::ArtifactStore, config::ArtifactConfig};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePoolOptions;

async fn fixture(with_roots: bool) -> (AppState, sqlx::SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_agent','Agent App','agent-app','active')").execute(&pool).await.unwrap();
    if with_roots {
        sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_agent','Agent Node','/srv/apps','/srv/secrets','online')").execute(&pool).await.unwrap();
    } else {
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node_agent','Agent Node','online')")
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_runtime','node_agent','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.1.0',11,'[\"pty_terminal\",\"privileged_release\"]')").execute(&pool).await.unwrap();
    let schema = json!({"type":"object","properties":{"release-version":{"type":"string"}},"required":["release-version"],"additionalProperties":false});
    sqlx::query("UPDATE applications SET parameter_schema=? WHERE id='app_agent'")
        .bind(schema.to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_agent','app_agent','node_agent','test','/srv/apps/deploy.sh',60,'active')").execute(&pool).await.unwrap();
    let snapshot = json!({"target":{"application_id":"app_agent","node_id":"node_agent","environment":"test","script_path":"/srv/apps/deploy.sh","parameter_schema":schema,"timeout_seconds":60,"verification_config":{},"secret_file_references":[{"environment_key":"TOKEN_FILE","file_path":"/srv/secrets/token"}],"version":1},"parameters":{"release-version":"1.0.0"}});
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment_agent','target_agent','admin','queued','queued','request-agent-0001','hash','snapshot',?)").bind(snapshot.to_string()).execute(&pool).await.unwrap();
    (AppState::new(pool.clone()), pool)
}

#[tokio::test]
async fn cross_node_prepare_fans_out_independent_releases_and_retry_skips_success() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_multi','Multi','multi','active')").execute(&pool).await.unwrap();
    for (node, agent, root) in [
        ("node_build", "agent_build", "/srv/build"),
        ("node_b", "agent_b", "/srv/b"),
        ("node_c", "agent_c", "/srv/c"),
    ] {
        sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?,?,'/srv/secrets','online')")
            .bind(node).bind(node).bind(root).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES(?,?, '2026-08-07T00:00:00Z','2026-08-07T00:00:00Z','0.1.0',11,'[\"pty_terminal\",\"privileged_release\"]')")
            .bind(agent).bind(node).execute(&pool).await.unwrap();
    }
    for (target, node) in [("target_b", "node_b"), ("target_c", "node_c")] {
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES(?, 'app_multi',?,'prod','two_stage','/unused',60,'active')")
            .bind(target).bind(node).execute(&pool).await.unwrap();
    }
    let target = |target_id: &str, node_id: &str, agent_id: &str| {
        json!({
            "target_id": target_id,
            "node_id": node_id,
            "agent_id": agent_id,
            "target": {"node_id":node_id,"environment":"prod","timeout_seconds":60}
        })
    };
    let snapshot = json!({
        "application_id":"app_multi",
        "execution_mode":"two_stage",
        "source": {
            "repository_url":"https://git.example.test/app.git",
            "resolved_commit_sha":"0123456789abcdef0123456789abcdef01234567",
            "build_agent_id":"agent_build",
            "git_credential_id":null
        },
        "two_stage":{"release_version":"release-1","modules":["api"]},
        "targets":[target("target_b","node_b","agent_b"),target("target_c","node_c","agent_c")],
        "multi_target_dispatch_version":3
    });
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('dep_multi','app_multi','target_b','admin','queued','targets_pending','idem-multi','hash','snapshot',?)")
        .bind(snapshot.to_string()).execute(&pool).await.unwrap();
    for (run, target_id, node_id, agent_id) in [
        ("run_b", "target_b", "node_b", "agent_b"),
        ("run_c", "target_c", "node_c", "agent_c"),
    ] {
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,target_snapshot_json,status,env_gate_status) VALUES(?,'dep_multi',?,?,?,'{}','pending','not_required')")
            .bind(run).bind(target_id).bind(node_id).bind(agent_id).execute(&pool).await.unwrap();
    }
    let temp = tempfile::tempdir().unwrap();
    let store = ArtifactStore::initialize(ArtifactConfig {
        root: temp.path().join("artifacts"),
        max_file_bytes: 1024,
        max_total_bytes: 4096,
        max_files: 8,
        max_chunk_bytes: 1024,
        upload_ttl_seconds: 600,
        retention_ttl_seconds: 3600,
    })
    .unwrap();
    let state = AppState::new(pool.clone())
        .with_artifact_store(store)
        .with_cross_node_artifacts_enabled(true);

    let mut blocked_snapshot = snapshot.clone();
    blocked_snapshot["source"]["build_agent_id"] = json!("agent_missing");
    for index in 0..16 {
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json,queued_at) VALUES(?, 'app_multi','target_b','admin','queued','targets_pending',?,?,?,?, '2026-08-06T00:00:00Z')")
            .bind(format!("dep_blocked_{index:02}"))
            .bind(format!("idem-blocked-{index:02}"))
            .bind(format!("hash-blocked-{index:02}"))
            .bind(format!("snapshot-blocked-{index:02}"))
            .bind(blocked_snapshot.to_string())
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE nodes SET status='offline' WHERE id='node_b'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        dispatch_next_deployment(&state).await.unwrap().as_deref(),
        Some("dep_multi")
    );
    let prepare_id: String = sqlx::query_scalar(
        "SELECT id FROM agent_tasks WHERE deployment_id='dep_multi' AND stage='prepare'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let prepare_payload: String =
        sqlx::query_scalar("SELECT payload_json FROM agent_tasks WHERE id=?")
            .bind(&prepare_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let TaskPayload::DeploymentPrepare(prepare) = serde_json::from_str(&prepare_payload).unwrap()
    else {
        panic!("expected prepare")
    };
    assert!(prepare.artifact_upload.is_some());
    assert_eq!(prepare.repository_url, "https://git.example.test/app.git");
    sqlx::query("UPDATE agent_tasks SET status='succeeded' WHERE id=?")
        .bind(&prepare_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE nodes SET status='online' WHERE id='node_b'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE nodes SET status='offline' WHERE id='node_c'")
        .execute(&pool)
        .await
        .unwrap();
    let archive_digest = "c".repeat(64);
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_json,manifest_digest,total_size,file_count,storage_key,status,upload_offset,upload_size,archive_digest,expires_at,verified_at) VALUES('artifact_multi','dep_multi','{}',?,1,1,?,'verified',1,1,?,'2099-01-01T00:00:00Z','2026-08-07T00:00:00Z')")
        .bind("a".repeat(64)).bind(&archive_digest).bind(&archive_digest).execute(&pool).await.unwrap();

    let env_digest = "e".repeat(64);
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_multi','app_multi','api.env','api','dotenv-v1',1,?)")
        .bind(&env_digest).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_multi_v1','env_multi',1,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,?)")
        .bind(&env_digest).execute(&pool).await.unwrap();
    for (sync, target, node, agent) in [
        ("sync_multi_b", "target_b", "node_b", "agent_b"),
        ("sync_multi_c", "target_c", "node_c", "agent_c"),
    ] {
        sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status) VALUES(?,'env_multi_v1',?,?,?,'pending')")
            .bind(sync).bind(target).bind(node).bind(agent).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest,deleted_at) VALUES('env_deleted','app_multi','worker.env','worker','dotenv-v1',2,?,'2026-08-07T00:00:00Z')")
        .bind(Sha256::digest([]).iter().map(|byte| format!("{byte:02x}")).collect::<String>()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_deleted_v2','env_deleted',2,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,?)")
        .bind(Sha256::digest([]).iter().map(|byte| format!("{byte:02x}")).collect::<String>()).execute(&pool).await.unwrap();
    for (sync, target, node, agent) in [
        ("sync_deleted_b", "target_b", "node_b", "agent_b"),
        ("sync_deleted_c", "target_c", "node_c", "agent_c"),
    ] {
        sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status,action) VALUES(?,'env_deleted_v2',?,?,?,'pending','delete')")
            .bind(sync).bind(target).bind(node).bind(agent).execute(&pool).await.unwrap();
    }

    ensure_deployment_task(&state, "dep_multi").await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE stage='release' AND agent_id='agent_b'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    sqlx::query("UPDATE application_env_syncs SET status='succeeded',actual_version=1 WHERE id='sync_multi_b'")
        .execute(&pool).await.unwrap();
    ensure_deployment_task(&state, "dep_multi").await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE stage='release' AND agent_id='agent_b'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    sqlx::query("UPDATE application_env_syncs SET status='succeeded',actual_version=2 WHERE id='sync_deleted_b'")
        .execute(&pool).await.unwrap();
    ensure_deployment_task(&state, "dep_multi").await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE stage='release' AND agent_id='agent_b'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE stage='release' AND agent_id='agent_c'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let release_b: String = sqlx::query_scalar(
        "SELECT id FROM agent_tasks WHERE deployment_id='dep_multi' AND target_run_id='run_b'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_b'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='running' WHERE id=?")
        .bind(&release_b)
        .execute(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_b",
        2,
        &Message::TaskResult(TaskResult {
            task_id: release_b,
            sequence: 1,
            status: TaskTerminalStatus::Failed,
            exit_code: Some(1),
            error_code: Some("release_failed".to_owned()),
            summary: Some("B 发布失败".to_owned()),
            data: None,
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_as::<_, (String, String)>(
            "SELECT status,phase FROM deployments WHERE id='dep_multi'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        ("running".to_owned(), "targets_pending".to_owned())
    );
    sqlx::query("UPDATE deployment_target_runs SET status='succeeded' WHERE id='run_b'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE nodes SET status='online' WHERE id='node_c'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE application_env_syncs SET status='succeeded',actual_version=1 WHERE id='sync_multi_c'")
        .execute(&pool).await.unwrap();
    sqlx::query("UPDATE application_env_syncs SET status='succeeded',actual_version=2 WHERE id='sync_deleted_c'")
        .execute(&pool).await.unwrap();
    ensure_deployment_task(&state, "dep_multi").await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE stage='release' AND agent_id='agent_b'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE stage='release' AND agent_id='agent_c'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    sqlx::query("UPDATE deployment_target_runs SET status='failed' WHERE id='run_c'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM deployment_target_runs WHERE id='run_b'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "succeeded"
    );
    sqlx::query(
        "UPDATE deployments SET status='failed',phase='targets_failed' WHERE id='dep_multi'",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,retry_of_id,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('dep_retry','app_multi','target_b','admin','dep_multi','queued','targets_pending','idem-retry','hash2','snapshot',?)")
        .bind(snapshot.to_string()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,source_run_id,target_snapshot_json,status,phase,env_gate_status,finished_at) VALUES('retry_b','dep_retry','target_b','node_b','agent_b','run_b','{}','reused','reused','not_required','2026-08-07T00:00:00Z')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,source_run_id,artifact_id,target_snapshot_json,status,env_gate_status) VALUES('retry_c','dep_retry','target_c','node_c','agent_c','run_c','artifact_multi','{}','pending','not_required')").execute(&pool).await.unwrap();
    ensure_deployment_task(&state, "dep_retry").await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='dep_retry' AND stage='prepare'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='dep_retry' AND stage='release' AND agent_id='agent_b'").fetch_one(&pool).await.unwrap(), 0);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='dep_retry' AND stage='release' AND agent_id='agent_c'").fetch_one(&pool).await.unwrap(), 1);
}

#[tokio::test]
async fn deployment_snapshot_is_persisted_as_an_idempotent_agent_task() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    let repeated = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    assert_eq!(repeated, task_id);
    let (status, digest, payload_json, idempotency_key): (String, String, String, String) =
        sqlx::query_as(
            "SELECT status,payload_digest,payload_json,idempotency_key FROM agent_tasks WHERE id=?",
        )
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "queued");
    assert!(digest.starts_with("sha256:"));
    assert_eq!(idempotency_key, "deployment:deployment_agent");
    let TaskPayload::DeploymentExecute(payload) = serde_json::from_str(&payload_json).unwrap()
    else {
        panic!("期望部署任务");
    };
    assert_eq!(payload.work_root, "/srv/apps");
    assert_eq!(payload.script_path, "/srv/apps/deploy.sh");
    assert_eq!(payload.argument_tokens, ["--release-version", "1.0.0"]);
    assert_eq!(
        payload.environment_file_references[0].file_path,
        "/srv/secrets/token"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn legacy_agent_task_is_failed_instead_of_remaining_queued() {
    let (state, pool) = fixture(true).await;
    sqlx::query("UPDATE agents SET protocol_version=10 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    let payload = TaskPayload::DeploymentPrepare(DeploymentPrepareTask {
        deployment_id: "deployment_agent".into(),
        source_policy: SourcePolicy::Branch,
        repository_url: "git@git.example.test:deploy-go/example.git".into(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        checkout_dir: "/srv/tasks/task_v2/checkout".into(),
        work_root: "/srv/tasks/task_v2".into(),
        output_dir: "/srv/tasks/task_v2/staging".into(),
        environment: Environment::Test,
        release_version: "20260806183000".into(),
        modules: vec!["api".into()],
        make_target: MakeTarget::DeployGoPrepare,
        git_credential_lease_id: None,
        timeout_seconds: 900,
        artifact_upload: None,
    });
    let payload_json = serde_json::to_string(&payload).unwrap();
    let digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_v2','agent_runtime','deployment_agent','prepare','deployment_prepare','deployment:deployment_agent:prepare',?,?,'queued','2099-08-06T03:10:00Z')")
        .bind(&digest)
        .bind(&payload_json)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(requeue_expired_deliveries(&state).await.unwrap(), 0);
    let (status, result_json): (String, String) =
        sqlx::query_as("SELECT status,result_json FROM agent_tasks WHERE id='task_v2'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result_json).unwrap()["error_code"],
        "agent_protocol_unsupported"
    );
    let (deployment_status, result_summary): (String, String) =
        sqlx::query_as("SELECT status,result_summary FROM deployments WHERE id='deployment_agent'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(deployment_status, "failed");
    assert!(result_summary.contains("agent_protocol_unsupported"));
}

#[tokio::test]
async fn legacy_running_release_is_interrupted_and_revokes_download_lease() {
    let (state, pool) = fixture(true).await;
    sqlx::query("UPDATE agents SET protocol_version=10 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE deployments SET status='running',phase='deploying' WHERE id='deployment_agent'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let archive_digest = "a".repeat(64);
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_json,manifest_digest,total_size,file_count,storage_key,status,upload_offset,upload_size,archive_digest,expires_at,verified_at) VALUES('artifact_legacy','deployment_agent','{}','digest',1,1,?,'verified',1,1,?,'2099-08-06T03:10:00Z','2026-08-06T03:00:00Z')")
        .bind(&archive_digest)
        .bind(&archive_digest)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,artifact_id,status) VALUES('run_legacy','deployment_agent','target_agent','node_agent','agent_runtime','artifact_legacy','downloading')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,target_run_id,stage,kind,idempotency_key,payload_digest,payload_json,status,lease_expires_at,deadline_at) VALUES('task_legacy_release','agent_runtime','deployment_agent','run_legacy','release','deployment_release','deployment:deployment_agent:release','digest','{}','running','2099-08-06T03:10:00Z','2099-08-06T03:10:00Z')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO artifact_leases(id,artifact_id,agent_id,target_run_id,purpose,manifest_digest,status,expires_at) VALUES('lease_legacy_release','artifact_legacy','agent_runtime','run_legacy','artifact_download','digest','active','2099-08-06T03:10:00Z')")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(requeue_expired_deliveries(&state).await.unwrap(), 0);
    let (status, lease_expires_at, result_json): (String, Option<String>, String) =
        sqlx::query_as("SELECT status,lease_expires_at,result_json FROM agent_tasks WHERE id='task_legacy_release'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "interrupted");
    assert!(lease_expires_at.is_none());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result_json).unwrap()["error_code"],
        "agent_protocol_unsupported"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM artifact_leases WHERE id='lease_legacy_release'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "revoked"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM deployment_target_runs WHERE id='run_legacy'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "failed"
    );
}

#[tokio::test]
async fn legacy_running_prepare_revokes_upload_lease_and_fails_uploading_artifact() {
    let (state, pool) = fixture(true).await;
    sqlx::query("UPDATE agents SET protocol_version=10 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE deployments SET status='running',phase='preparing' WHERE id='deployment_agent'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_digest,total_size,file_count,status,expires_at) VALUES('artifact_legacy_upload','deployment_agent','digest',1,1,'uploading','2099-08-06T03:10:00Z')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO artifact_leases(id,artifact_id,agent_id,purpose,manifest_digest,status,expires_at) VALUES('lease_legacy_upload','artifact_legacy_upload','agent_runtime','artifact_upload','digest','active','2099-08-06T03:10:00Z')")
        .execute(&pool)
        .await
        .unwrap();
    let payload = TaskPayload::DeploymentPrepare(DeploymentPrepareTask {
        deployment_id: "deployment_agent".into(),
        source_policy: SourcePolicy::Branch,
        repository_url: "git@git.example.test:deploy-go/example.git".into(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        checkout_dir: "/srv/tasks/task_legacy_upload/checkout".into(),
        work_root: "/srv/tasks/task_legacy_upload".into(),
        output_dir: "/srv/tasks/task_legacy_upload/staging".into(),
        environment: Environment::Test,
        release_version: "20260806183000".into(),
        modules: vec!["api".into()],
        make_target: MakeTarget::DeployGoPrepare,
        git_credential_lease_id: None,
        timeout_seconds: 900,
        artifact_upload: Some(ArtifactUploadRequest {
            authorization_id: "lease_legacy_upload".into(),
        }),
    });
    let payload_json = serde_json::to_string(&payload).unwrap();
    let digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_legacy_prepare','agent_runtime','deployment_agent','prepare','deployment_prepare','deployment:deployment_agent:prepare',?,?,'running','2099-08-06T03:10:00Z')")
        .bind(&digest)
        .bind(&payload_json)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(requeue_expired_deliveries(&state).await.unwrap(), 0);
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM agent_tasks WHERE id='task_legacy_prepare'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "interrupted"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM artifact_leases WHERE id='lease_legacy_upload'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "revoked"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM deployment_artifacts WHERE id='artifact_legacy_upload'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "failed"
    );
}

#[tokio::test]
async fn v11_agent_missing_required_capability_cannot_create_script_task() {
    let (state, pool) = fixture(true).await;
    sqlx::query(
        "UPDATE agents SET capabilities_json='[\"pty_terminal\"]' WHERE id='agent_runtime'",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(
        enqueue_deployment(&state, "deployment_agent")
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='deployment_agent'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let (status, result_summary): (String, String) =
        sqlx::query_as("SELECT status,result_summary FROM deployments WHERE id='deployment_agent'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert!(result_summary.contains("agent_capability_unavailable"));
}

#[tokio::test]
async fn queued_task_for_v11_agent_missing_required_capability_is_failed() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query(
        "UPDATE agents SET capabilities_json='[\"pty_terminal\"]' WHERE id='agent_runtime'",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(requeue_expired_deliveries(&state).await.unwrap(), 0);
    let (status, result_json): (String, String) =
        sqlx::query_as("SELECT status,result_json FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result_json).unwrap()["error_code"],
        "agent_capability_unavailable"
    );
}

#[tokio::test]
async fn queued_task_for_revoked_agent_fails_without_dispatch() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET revoked_at='2026-08-03T00:00:00Z' WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();

    assert!(!try_dispatch(&state, &task_id).await.unwrap());
    let task: (String, String) =
        sqlx::query_as("SELECT status,result_json FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task.0, "failed");
    assert!(task.1.contains("agent_identity_invalid"));
}

#[tokio::test]
async fn legacy_agent_cannot_create_a_script_deployment_task() {
    let (state, pool) = fixture(true).await;
    sqlx::query("UPDATE agents SET protocol_version=10 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        enqueue_deployment(&state, "deployment_agent")
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='deployment_agent'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let (status, result_summary): (String, String) =
        sqlx::query_as("SELECT status,result_summary FROM deployments WHERE id='deployment_agent'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert!(result_summary.contains("agent_protocol_unsupported"));
}

#[tokio::test]
async fn legacy_agent_cannot_create_a_two_stage_prepare_task() {
    let (state, pool) = fixture(true).await;
    sqlx::query("UPDATE agents SET protocol_version=10 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_targets SET execution_mode='two_stage' WHERE id='target_agent'")
        .execute(&pool)
        .await
        .unwrap();
    let snapshot = json!({
        "execution_mode":"two_stage",
        "target":{"node_id":"node_agent","environment":"test","timeout_seconds":60},
        "source":{
            "repository_url":"https://git.example.test/app.git",
            "resolved_commit_sha":"0123456789abcdef0123456789abcdef01234567",
            "build_agent_id":"agent_runtime",
            "git_credential_id":null
        },
        "two_stage":{"release_version":"release-1","modules":["api"]}
    });
    sqlx::query("UPDATE deployments SET snapshot_json=? WHERE id='deployment_agent'")
        .bind(snapshot.to_string())
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        ensure_deployment_task(&state, "deployment_agent")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='deployment_agent'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let (status, result_summary): (String, String) =
        sqlx::query_as("SELECT status,result_summary FROM deployments WHERE id='deployment_agent'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert!(result_summary.contains("agent_protocol_unsupported"));
}

#[tokio::test]
async fn missing_node_roots_rejects_dispatch_without_creating_a_task() {
    let (state, pool) = fixture(false).await;
    assert!(
        enqueue_deployment(&state, "deployment_agent")
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn current_connection_events_advance_task_deployment_and_logs_once() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='delivered' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    let digest: String = sqlx::query_scalar("SELECT payload_digest FROM agent_tasks WHERE id=?")
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::ReconcileReport(ReconcileReport {
            tasks: vec![ReconciledTask {
                task_id: task_id.clone(),
                payload_digest: digest.clone(),
                state: ReconciledTaskState::Accepted,
                last_sequence: 0,
                result: None,
            }],
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "queued"
    );
    sqlx::query("UPDATE agent_tasks SET status='delivered' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::TaskAck(TaskAck {
            task_id: task_id.clone(),
            payload_digest: digest,
            disposition: TaskAckDisposition::Accepted,
            error_code: None,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::TaskState(TaskState {
            task_id: task_id.clone(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    let output = Message::TaskOutput(TaskOutput {
        task_id: task_id.clone(),
        sequence: 2,
        stream: OutputStream::Stdout,
        text: "deployment output".to_owned(),
    });
    handle_agent_message(&state, "agent_runtime", 2, &output)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::TaskResult(TaskResult {
            task_id: task_id.clone(),
            sequence: 3,
            status: TaskTerminalStatus::Succeeded,
            exit_code: Some(0),
            error_code: None,
            summary: Some("部署完成".to_owned()),
            data: None,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(&state, "agent_runtime", 2, &output)
        .await
        .unwrap();

    let task: (String, i64) =
        sqlx::query_as("SELECT status,last_sequence FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task, ("succeeded".to_owned(), 3));
    let deployment: (String, Option<i64>, bool) = sqlx::query_as(
        "SELECT status,exit_code,protocol_complete FROM deployments WHERE id='deployment_agent'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(deployment, ("succeeded".to_owned(), Some(0), true));
    let logs: Vec<String> = sqlx::query_scalar(
        "SELECT content FROM deployment_logs WHERE deployment_id='deployment_agent'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(logs, ["deployment output"]);
}

#[tokio::test]
async fn stale_connection_and_sequence_gaps_are_rejected() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    let state_message = Message::TaskState(TaskState {
        task_id,
        sequence: 2,
        state: TaskLifecycleState::Running,
    });
    assert!(
        handle_agent_message(&state, "agent_runtime", 1, &state_message)
            .await
            .is_err()
    );
    assert!(
        handle_agent_message(&state, "agent_runtime", 2, &state_message)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn expired_delivery_lease_returns_to_queue_for_retry() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='delivered',lease_expires_at='2026-08-03T00:00:00Z' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(requeue_expired_deliveries(&state).await.unwrap(), 1);
    let task: (String, Option<String>) =
        sqlx::query_as("SELECT status,lease_expires_at FROM agent_tasks WHERE id=?")
            .bind(task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task, ("queued".to_owned(), None));
}

#[tokio::test]
async fn reconnect_reconcile_restores_exact_state_and_interrupts_mismatch() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='delivered' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    let digest: String = sqlx::query_scalar("SELECT payload_digest FROM agent_tasks WHERE id=?")
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::ReconcileReport(ReconcileReport {
            tasks: vec![ReconciledTask {
                task_id: task_id.clone(),
                payload_digest: digest,
                state: ReconciledTaskState::Running,
                last_sequence: 0,
                result: None,
            }],
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM deployments WHERE id='deployment_agent'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "running"
    );

    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::ReconcileReport(ReconcileReport {
            tasks: vec![ReconciledTask {
                task_id: task_id.clone(),
                payload_digest: "sha256:different-payload".to_owned(),
                state: ReconciledTaskState::Running,
                last_sequence: 0,
                result: None,
            }],
        }),
    )
    .await
    .unwrap();
    let states: (String, String) = sqlx::query_as(
        "SELECT t.status,d.status FROM agent_tasks t JOIN deployments d ON d.id=t.deployment_id WHERE t.id=?",
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(states, ("interrupted".to_owned(), "interrupted".to_owned()));
}

#[tokio::test]
async fn reconnect_reconcile_accepts_agent_sequence_ahead_and_continues_stream() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='running',last_sequence=71 WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    let digest: String = sqlx::query_scalar("SELECT payload_digest FROM agent_tasks WHERE id=?")
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::ReconcileReport(ReconcileReport {
            tasks: vec![ReconciledTask {
                task_id: task_id.clone(),
                payload_digest: digest,
                state: ReconciledTaskState::Running,
                last_sequence: 73,
                result: None,
            }],
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::TaskOutput(TaskOutput {
            task_id: task_id.clone(),
            sequence: 74,
            stream: OutputStream::Stdout,
            text: "reconnected output".to_owned(),
        }),
    )
    .await
    .unwrap();

    let task: (String, i64) =
        sqlx::query_as("SELECT status,last_sequence FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task, ("running".to_owned(), 74));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT content FROM deployment_logs WHERE task_id=? AND task_sequence=74"
        )
        .bind(task_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "reconnected output"
    );
}

#[tokio::test]
async fn cancel_before_delivery_finishes_locally_and_all_remote_tasks_stay_canceling() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    assert!(
        !request_deployment_cancel(&state, "deployment_agent")
            .await
            .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "canceled"
    );

    sqlx::query("UPDATE agent_tasks SET status='delivered',finished_at=NULL WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) SELECT 'task_second',agent_id,deployment_id,kind,'deployment:deployment_agent:second',payload_digest,payload_json,'running',deadline_at FROM agent_tasks WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployments SET status='running',phase='deploying',finished_at=NULL WHERE id='deployment_agent'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !request_deployment_cancel(&state, "deployment_agent")
            .await
            .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "canceling"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_tasks WHERE id='task_second'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        "canceling"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM deployments WHERE id='deployment_agent'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "canceling"
    );
}

#[tokio::test]
async fn agent_output_redacts_secret_paths_and_obeys_server_budget() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='running' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO system_settings(key,value_json,version) VALUES('runtime',?,1)")
        .bind(json!({"max_concurrent_deployments":2,"max_log_bytes":12,"log_retention_days":30,"version":1}).to_string())
        .execute(&pool).await.unwrap();
    let output = Message::TaskOutput(TaskOutput {
        task_id: task_id.clone(),
        sequence: 1,
        stream: OutputStream::Stderr,
        text: "secret=/srv/secrets/token suffix".to_owned(),
    });
    handle_agent_message(&state, "agent_runtime", 2, &output)
        .await
        .unwrap();
    handle_agent_message(&state, "agent_runtime", 2, &output)
        .await
        .unwrap();
    let log: (String, bool) = sqlx::query_as(
        "SELECT content,truncated FROM deployment_logs WHERE deployment_id='deployment_agent'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!log.0.contains("/srv/secrets/token"));
    assert!(log.0.len() <= 12);
    assert!(log.1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployment_events WHERE deployment_id='deployment_agent' AND diagnostic_code='log_budget_exceeded'")
            .fetch_one(&pool).await.unwrap(),
        1
    );
}

#[tokio::test]
async fn image_multi_target_fans_out_release_without_prepare_and_filters_env_files() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_image_multi','Image Multi','image-multi','active')")
        .execute(&pool)
        .await
        .unwrap();
    for (node, agent) in [("node_i1", "agent_i1"), ("node_i2", "agent_i2")] {
        sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?,'/srv/apps','/srv/secrets','online')")
            .bind(node)
            .bind(node)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES(?,?, '2026-08-11T00:00:00Z','2026-08-11T00:00:00Z','0.3.0',11,'[\"pty_terminal\",\"privileged_release\"]')")
            .bind(agent)
            .bind(node)
            .execute(&pool)
            .await
            .unwrap();
    }
    let spec = json!({"template":"redis","image":"redis:7-alpine","host_port":6379,"env_files":["compose.env","redis.env"]});
    for (target, node) in [("target_i1", "node_i1"), ("target_i2", "node_i2")] {
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,privileged_release,image_spec_json,status) VALUES(?, 'app_image_multi',?,'prod','image','',60,1,?,'active')")
            .bind(target)
            .bind(node)
            .bind(spec.to_string())
            .execute(&pool)
            .await
            .unwrap();
    }
    let target = |target_id: &str, node_id: &str, agent_id: &str| {
        json!({
            "target_id": target_id,
            "node_id": node_id,
            "agent_id": agent_id,
            "target": {"node_id":node_id,"environment":"prod","timeout_seconds":60}
        })
    };
    let snapshot = json!({
        "application_id":"app_image_multi",
        "execution_mode":"image",
        "image": {
            "release_version":"20260811120000",
            "commit_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "modules":["redis"],
            "image_spec": spec,
            "checkout_tree_digest":"checkout-digest"
        },
        "targets":[target("target_i1","node_i1","agent_i1"),target("target_i2","node_i2","agent_i2")],
        "multi_target_dispatch_version":3
    });
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('dep_image_multi','app_image_multi','target_i1','admin','queued','targets_pending','idem-image-multi','hash','snapshot',?)")
        .bind(snapshot.to_string())
        .execute(&pool)
        .await
        .unwrap();
    for (run, target_id, node_id, agent_id) in [
        ("run_i1", "target_i1", "node_i1", "agent_i1"),
        ("run_i2", "target_i2", "node_i2", "agent_i2"),
    ] {
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,target_snapshot_json,status,env_gate_status) VALUES(?,'dep_image_multi',?,?,?,'{}','pending','not_required')")
            .bind(run)
            .bind(target_id)
            .bind(node_id)
            .bind(agent_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_json,manifest_digest,total_size,file_count,storage_key,status,upload_offset,upload_size,archive_digest,expires_at,verified_at) VALUES('artifact_image_multi','dep_image_multi','{}',?,1,1,?,'verified',1,1,?,'2099-01-01T00:00:00Z','2026-08-11T00:00:00Z')")
        .bind("m".repeat(64))
        .bind("a".repeat(64))
        .bind("a".repeat(64))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_target_runs SET artifact_id='artifact_image_multi' WHERE deployment_id='dep_image_multi'")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_selected_multi','app_image_multi','redis.env','redis','dotenv-v1',1,'selected-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_compose_multi','app_image_multi','compose.env','compose','dotenv-v1',1,'compose-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_ignored_multi','app_image_multi','ignored.env','ignored','dotenv-v1',1,'ignored-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_selected_multi_v1','env_selected_multi',1,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,'selected-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_compose_multi_v1','env_compose_multi',1,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,'compose-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_ignored_multi_v1','env_ignored_multi',1,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,'ignored-digest')")
        .execute(&pool)
        .await
        .unwrap();
    for (target, node, agent) in [
        ("target_i1", "node_i1", "agent_i1"),
        ("target_i2", "node_i2", "agent_i2"),
    ] {
        sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status,actual_version) VALUES(?,'env_selected_multi_v1',?,?,?,'succeeded',1)")
            .bind(format!("sync_selected_{target}"))
            .bind(target)
            .bind(node)
            .bind(agent)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status,actual_version) VALUES(?,'env_compose_multi_v1',?,?,?,'succeeded',1)")
            .bind(format!("sync_compose_{target}"))
            .bind(target)
            .bind(node)
            .bind(agent)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status) VALUES(?,'env_ignored_multi_v1',?,?,?,'pending')")
            .bind(format!("sync_ignored_{target}"))
            .bind(target)
            .bind(node)
            .bind(agent)
            .execute(&pool)
            .await
            .unwrap();
    }

    let state = AppState::new(pool.clone()).with_cross_node_artifacts_enabled(true);
    let reusable_artifacts: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT artifact.id,artifact.manifest_digest,artifact.archive_digest FROM deployment_target_runs run JOIN deployment_artifacts artifact ON artifact.id=run.artifact_id WHERE run.deployment_id='dep_image_multi' AND run.status='pending' AND artifact.status='verified' AND artifact.expires_at>?",
    )
    .bind("2026-08-01T00:00:00Z")
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(reusable_artifacts.len(), 2);
    let gate_rows: Vec<(String, i64, String, Option<i64>)> = sqlx::query_as(
        "SELECT file.file_name,file.current_version,sync.status,sync.actual_version FROM application_env_files file JOIN deployment_targets target ON target.application_id=file.application_id LEFT JOIN application_env_versions version ON version.env_file_id=file.id AND version.env_version=file.current_version LEFT JOIN application_env_syncs sync ON sync.env_version_id=version.id AND sync.target_id=target.id WHERE target.id='target_i1' ORDER BY file.file_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(gate_rows.len(), 3);
    ensure_deployment_task(&state, "dep_image_multi")
        .await
        .unwrap();
    let prepare_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='dep_image_multi' AND stage='prepare'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(prepare_count, 0);
    let releases: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id,status,payload_json FROM agent_tasks WHERE deployment_id='dep_image_multi' AND stage='release' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(releases.len(), 2);
    for (_, _, payload_json) in releases {
        let TaskPayload::DeploymentRelease(release) =
            serde_json::from_str::<TaskPayload>(&payload_json).unwrap()
        else {
            panic!("expected release payload")
        };
        assert_eq!(
            release.checkout_mode,
            deploy_go_agent_protocol::ReleaseCheckoutMode::Artifact
        );
        assert!(release.privileged);
        assert_eq!(release.repository_url, None);
        assert_eq!(release.git_credential_lease_id, None);
        assert_eq!(release.required_env.len(), 2);
        assert!(
            release
                .required_env
                .iter()
                .any(|item| item.file_name == "compose.env")
        );
        assert!(
            release
                .required_env
                .iter()
                .any(|item| item.file_name == "redis.env")
        );
    }
    let deployment: (String, String) =
        sqlx::query_as("SELECT status,phase FROM deployments WHERE id='dep_image_multi'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(deployment, ("running".to_owned(), "deploying".to_owned()));
}

#[tokio::test]
async fn image_release_requires_v11_privileged_agent_and_selected_env_sync() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_image_old','Image Old','image-old','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_image_old','node','/srv/apps','/srv/secrets','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_image_old','node_image_old','2026-08-11T00:00:00Z','2026-08-11T00:00:00Z','0.2.0',7,'[\"privileged_release\"]')")
        .execute(&pool)
        .await
        .unwrap();
    let spec = json!({"template":"redis","image":"redis:7-alpine","host_port":6379,"env_files":["compose.env","redis.env"]});
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,privileged_release,image_spec_json,status) VALUES('target_image_old','app_image_old','node_image_old','prod','image','',60,1,?,'active')")
        .bind(spec.to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_old','app_image_old','redis.env','redis','dotenv-v1',1,'digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_old_compose','app_image_old','compose.env','compose','dotenv-v1',1,'compose-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_old_v1','env_old',1,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,'digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_old_compose_v1','env_old_compose',1,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,'compose-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status) VALUES('sync_old','env_old_v1','target_image_old','node_image_old','agent_image_old','pending')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status) VALUES('sync_old_compose','env_old_compose_v1','target_image_old','node_image_old','agent_image_old','pending')")
        .execute(&pool)
        .await
        .unwrap();
    let snapshot = json!({
        "target_id":"target_image_old",
        "application_name":"Image Old",
        "node_name":"node",
        "execution_mode":"image",
        "release_strategy":"automatic",
        "parameters":{},
        "_artifact_id":"artifact_image_old",
        "_artifact_manifest_digest":"m".repeat(64),
        "_artifact_archive_digest":"a".repeat(64),
        "image": {
            "release_version":"20260811120000",
            "commit_sha":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "modules":["redis"],
            "image_spec": spec,
            "checkout_tree_digest":"checkout-digest"
        },
        "target": {
            "application_id":"app_image_old",
            "node_id":"node_image_old",
            "environment":"prod",
            "script_path":"",
            "timeout_seconds":60,
            "image_spec": spec
        }
    });
    async fn insert_deployment(pool: &sqlx::SqlitePool, id: &str, snapshot: &serde_json::Value) {
        sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES(?,'target_image_old','admin','queued','queued',?,?,?,?)")
            .bind(id)
            .bind(format!("idem-{id}"))
            .bind(format!("hash-{id}"))
            .bind(format!("snapshot-{id}"))
            .bind(snapshot.to_string())
            .execute(pool)
            .await
            .unwrap();
    }
    insert_deployment(&pool, "dep_image_old", &snapshot).await;
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_json,manifest_digest,total_size,file_count,storage_key,status,upload_offset,upload_size,archive_digest,expires_at,verified_at) VALUES('artifact_image_old','dep_image_old','{}',?,1,1,?,'verified',1,1,?,'2099-01-01T00:00:00Z','2026-08-11T00:00:00Z')")
        .bind("m".repeat(64))
        .bind("a".repeat(64))
        .bind("a".repeat(64))
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    ensure_deployment_task(&state, "dep_image_old")
        .await
        .unwrap();
    let release_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='dep_image_old' AND stage='release'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(release_count, 0);
    let (status, result_summary): (String, String) =
        sqlx::query_as("SELECT status,result_summary FROM deployments WHERE id='dep_image_old'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert!(result_summary.contains("privileged_release_protocol_unsupported"));

    sqlx::query("UPDATE agents SET protocol_version=11,capabilities_json='[\"pty_terminal\",\"privileged_release\"]' WHERE id='agent_image_old'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployments SET status='queued',phase='queued',result_summary=NULL,protocol_complete=0,finished_at=NULL WHERE id='dep_image_old'")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "UPDATE application_env_syncs SET status='succeeded',actual_version=1 WHERE id='sync_old'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE application_env_syncs SET status='succeeded',actual_version=1 WHERE id='sync_old_compose'",
    )
    .execute(&pool)
    .await
    .unwrap();
    ensure_deployment_task(&state, "dep_image_old")
        .await
        .unwrap();
    let payload_json: String = sqlx::query_scalar(
        "SELECT payload_json FROM agent_tasks WHERE deployment_id='dep_image_old' AND stage='release'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let TaskPayload::DeploymentRelease(release) =
        serde_json::from_str::<TaskPayload>(&payload_json).unwrap()
    else {
        panic!("expected release payload")
    };
    assert_eq!(
        release.checkout_mode,
        deploy_go_agent_protocol::ReleaseCheckoutMode::Artifact
    );
    assert_eq!(release.required_env.len(), 2);
    assert!(
        release
            .required_env
            .iter()
            .any(|item| item.file_name == "compose.env")
    );
    assert!(
        release
            .required_env
            .iter()
            .any(|item| item.file_name == "redis.env")
    );
    let deployment: (String, String) =
        sqlx::query_as("SELECT status,phase FROM deployments WHERE id='dep_image_old'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(deployment, ("running".to_owned(), "deploying".to_owned()));
}

#[tokio::test]
async fn image_release_waits_when_required_env_file_is_missing() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_image_missing','Image Missing','image-missing','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_image_missing','node','/srv/apps','/srv/secrets','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_image_missing','node_image_missing','2026-08-11T00:00:00Z','2026-08-11T00:00:00Z','0.3.0',11,'[\"pty_terminal\",\"privileged_release\"]')")
        .execute(&pool)
        .await
        .unwrap();
    let spec = json!({"template":"redis","image":"redis:7-alpine","host_port":6379,"env_files":["compose.env","redis.env"]});
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,privileged_release,image_spec_json,status) VALUES('target_image_missing','app_image_missing','node_image_missing','prod','image','',60,1,?,'active')")
        .bind(spec.to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_missing_redis','app_image_missing','redis.env','redis','dotenv-v1',1,'digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_missing_redis_v1','env_missing_redis',1,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,'digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status,actual_version) VALUES('sync_missing_redis','env_missing_redis_v1','target_image_missing','node_image_missing','agent_image_missing','succeeded',1)")
        .execute(&pool)
        .await
        .unwrap();
    let snapshot = json!({
        "target_id":"target_image_missing",
        "execution_mode":"image",
        "_artifact_id":"artifact_image_missing",
        "_artifact_manifest_digest":"m".repeat(64),
        "_artifact_archive_digest":"a".repeat(64),
        "image": {
            "release_version":"20260811130000",
            "commit_sha":"c".repeat(40),
            "modules":["redis"],
            "image_spec": spec,
            "checkout_tree_digest":"checkout-digest"
        },
        "target": {
            "application_id":"app_image_missing",
            "node_id":"node_image_missing",
            "environment":"prod",
            "script_path":"",
            "timeout_seconds":60,
            "image_spec": spec
        }
    });
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('dep_image_missing','target_image_missing','admin','queued','queued','idem-missing','hash-missing','snapshot-missing',?)")
        .bind(snapshot.to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_json,manifest_digest,total_size,file_count,storage_key,status,upload_offset,upload_size,archive_digest,expires_at,verified_at) VALUES('artifact_image_missing','dep_image_missing','{}',?,1,1,?,'verified',1,1,?,'2099-01-01T00:00:00Z','2026-08-11T00:00:00Z')")
        .bind("m".repeat(64))
        .bind("a".repeat(64))
        .bind("a".repeat(64))
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    ensure_deployment_task(&state, "dep_image_missing")
        .await
        .unwrap();
    let release_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='dep_image_missing' AND stage='release'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(release_count, 0);

    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_missing_compose','app_image_missing','compose.env','compose','dotenv-v1',1,'compose-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_missing_compose_v1','env_missing_compose',1,'chacha20poly1305-application-env-v1',X'01',X'000000000000000000000000',1,'compose-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status,actual_version) VALUES('sync_missing_compose','env_missing_compose_v1','target_image_missing','node_image_missing','agent_image_missing','succeeded',1)")
        .execute(&pool)
        .await
        .unwrap();
    ensure_deployment_task(&state, "dep_image_missing")
        .await
        .unwrap();
    let payload_json: String = sqlx::query_scalar(
        "SELECT payload_json FROM agent_tasks WHERE deployment_id='dep_image_missing' AND stage='release'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let TaskPayload::DeploymentRelease(release) =
        serde_json::from_str::<TaskPayload>(&payload_json).unwrap()
    else {
        panic!("expected release payload")
    };
    assert_eq!(release.required_env.len(), 2);

    sqlx::query("UPDATE application_env_files SET deleted_at='2026-08-11T00:00:00Z' WHERE id='env_missing_compose'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM agent_tasks WHERE deployment_id='dep_image_missing' AND stage='release'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE deployments SET status='queued',phase='queued' WHERE id='dep_image_missing'",
    )
    .execute(&pool)
    .await
    .unwrap();
    ensure_deployment_task(&state, "dep_image_missing")
        .await
        .unwrap();
    let release_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='dep_image_missing' AND stage='release'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(release_count, 0);
}

#[tokio::test]
async fn archived_node_is_excluded_from_deployment_dispatch() {
    let (state, pool) = fixture(true).await;
    sqlx::query("UPDATE nodes SET archived_at='2026-08-03T00:00:00Z' WHERE id='node_agent'")
        .execute(&pool)
        .await
        .unwrap();

    // 入队时归档节点不作为可用 Agent 目标。
    assert!(
        enqueue_deployment(&state, "deployment_agent")
            .await
            .is_err()
    );
    let task_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='deployment_agent'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(task_count, 0);
    // 归档语义是停止调度：部署保持 queued 等待（类似离线节点），不失败化。
    let status: String =
        sqlx::query_scalar("SELECT status FROM deployments WHERE id='deployment_agent'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "queued");

    // 恢复后可以正常入队派发。
    sqlx::query("UPDATE nodes SET archived_at=NULL WHERE id='node_agent'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE deployments SET status='queued',phase='queued',result_summary=NULL WHERE id='deployment_agent'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(enqueue_deployment(&state, "deployment_agent").await.is_ok());
}
