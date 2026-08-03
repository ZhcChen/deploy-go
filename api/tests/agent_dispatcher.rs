use deploy_go_agent_protocol::TaskPayload;
use deploy_go_api::{AppState, agents::dispatcher::enqueue_deployment, db};
use serde_json::json;
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
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version) VALUES('agent_runtime','node_agent','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.1.0',1)").execute(&pool).await.unwrap();
    let schema = json!({"type":"object","properties":{"release-version":{"type":"string"}},"required":["release-version"],"additionalProperties":false});
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,parameter_schema,timeout_seconds,verification_config,status) VALUES('target_agent','app_agent','node_agent','test','/srv/apps/deploy.sh',?,60,'{}','active')").bind(schema.to_string()).execute(&pool).await.unwrap();
    let snapshot = json!({"target":{"application_id":"app_agent","node_id":"node_agent","environment":"test","script_path":"/srv/apps/deploy.sh","parameter_schema":schema,"timeout_seconds":60,"verification_config":{},"secret_file_references":[{"environment_key":"TOKEN_FILE","file_path":"/srv/secrets/token"}],"version":1},"parameters":{"release-version":"1.0.0"}});
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment_agent','target_agent','admin','queued','queued','request-agent-0001','hash','snapshot',?)").bind(snapshot.to_string()).execute(&pool).await.unwrap();
    (AppState::new(pool.clone()), pool)
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
