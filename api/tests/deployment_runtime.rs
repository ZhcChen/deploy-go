use deploy_go_api::{
    AppState, db,
    deployments::{process_one, recover, run_worker},
};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

async fn fixture(node_status: &str) -> (AppState, sqlx::SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_runtime','Runtime App','runtime-app','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_runtime','Runtime Node','/srv/apps','/srv/secrets',?)").bind(node_status).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version) VALUES('agent_runtime','node_runtime','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.1.0',1)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_runtime','app_runtime','node_runtime','test','/srv/apps/deploy.sh',60,'active')").execute(&pool).await.unwrap();
    let snapshot = json!({"target":{"application_id":"app_runtime","node_id":"node_runtime","environment":"test","script_path":"/srv/apps/deploy.sh","parameter_schema":{"type":"object","additionalProperties":false},"timeout_seconds":60,"verification_config":{},"secret_file_references":[],"version":1},"parameters":{}});
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment_runtime','target_runtime','admin','queued','queued','request-runtime-0001','hash','snapshot',?)").bind(snapshot.to_string()).execute(&pool).await.unwrap();
    (AppState::new(pool.clone()), pool)
}

#[tokio::test]
async fn worker_enqueues_agent_task_without_marking_deployment_running() {
    let (state, pool) = fixture("online").await;
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some("deployment_runtime")
    );
    let deployment_status: String =
        sqlx::query_scalar("SELECT status FROM deployments WHERE id='deployment_runtime'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let task: (String, String) = sqlx::query_as(
        "SELECT kind,status FROM agent_tasks WHERE deployment_id='deployment_runtime'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(deployment_status, "queued");
    assert_eq!(task, ("deployment_execute".to_owned(), "queued".to_owned()));
}

#[tokio::test]
async fn offline_agent_is_not_claimed_or_converted_to_ssh_execution() {
    let (state, pool) = fixture("offline").await;
    assert_eq!(process_one(&state).await.unwrap(), None);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM deployments WHERE id='deployment_runtime'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "queued"
    );
}

#[tokio::test]
async fn restart_preserves_agent_backed_work_and_interrupts_unowned_work() {
    let (_state, pool) = fixture("online").await;
    sqlx::query(
        "UPDATE deployments SET status='running',phase='executing' WHERE id='deployment_runtime'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_runtime','agent_runtime','deployment_runtime','deployment_execute','deployment:runtime','sha256:0123456789abcdef','{}','running','2099-08-03T00:00:00Z')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,target_code,script_path,timeout_seconds,status) SELECT 'target_unowned',application_id,node_id,'staging','staging',script_path,timeout_seconds,status FROM deployment_targets WHERE id='target_runtime'").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) SELECT 'deployment_unowned','target_unowned',requested_by,'running','executing','request-runtime-0002','hash2',snapshot_hash,snapshot_json FROM deployments WHERE id='deployment_runtime'").execute(&pool).await.unwrap();
    assert_eq!(recover(&pool).await.unwrap(), 1);
    let statuses: Vec<(String, String)> =
        sqlx::query_as("SELECT id,status FROM deployments ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        statuses,
        vec![
            ("deployment_runtime".to_owned(), "running".to_owned()),
            ("deployment_unowned".to_owned(), "interrupted".to_owned())
        ]
    );
}

#[tokio::test]
async fn worker_stops_after_shutdown_without_abort() {
    let (state, _pool) = fixture("offline").await;
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    shutdown_tx.send(true).unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        run_worker(state, shutdown_rx),
    )
    .await
    .expect("worker 应在收到关闭信号后正常退出");
}
