mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use deploy_go_agent_protocol::{
    Message, TaskAck, TaskAckDisposition, TaskLifecycleState, TaskPayload, TaskResult, TaskState,
    TaskTerminalStatus,
};
use deploy_go_api::{
    AppState,
    agents::{
        auth::token_hash,
        dispatcher::{enqueue_pending_env_syncs_for_agent, handle_agent_message},
    },
    app,
    crypto::{APPLICATION_ENV_ALGORITHM, MasterKeyRing},
    db,
};
use sha2::{Digest, Sha256};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tower::ServiceExt;

fn digest(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}

async fn seed_agent(pool: &SqlitePool, suffix: &str, status: &str, token: &str) {
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?,? ,?,?)")
        .bind(format!("node_{suffix}"))
        .bind(format!("Node {suffix}"))
        .bind(format!("/srv/{suffix}/apps"))
        .bind(format!("/srv/{suffix}/secrets"))
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,agent_version,protocol_version,connection_generation) VALUES(?,?, '2026-08-07T00:00:00Z','0.1.0',4,1)")
        .bind(format!("agent_{suffix}"))
        .bind(format!("node_{suffix}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_credential_families(id,agent_id) VALUES(?,?)")
        .bind(format!("family_{suffix}"))
        .bind(format!("agent_{suffix}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_access_sessions(id,family_id,agent_id,token_hash,token_key_version,expires_at) VALUES(?,?,?,?,1,'2099-01-01T00:00:00Z')")
        .bind(format!("access_{suffix}"))
        .bind(format!("family_{suffix}"))
        .bind(format!("agent_{suffix}"))
        .bind(token_hash("access", token))
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn env_sync_uses_single_use_https_lease_and_converges_only_the_reporting_node() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_env_sync','Env Sync','env-sync-production','active')")
        .execute(&pool)
        .await
        .unwrap();
    seed_agent(&pool, "online", "online", "online-token").await;
    seed_agent(&pool, "offline", "offline", "offline-token").await;
    for suffix in ["online", "offline"] {
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES(?,'app_env_sync',?,'prod','/unused',60,'active')")
            .bind(format!("target_{suffix}"))
            .bind(format!("node_{suffix}"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let content = b"SECRET=not-in-task-or-journal\n";
    let env_digest = digest(content);
    let ring = MasterKeyRing::from_raw(1, [9; 32], None).unwrap();
    let encrypted = ring
        .encrypt_application_env("app_env_sync", "env_file", "env_version_1", content)
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_file','app_env_sync','api.env','api','dotenv-v1',1,?)")
        .bind(&env_digest)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_version_1','env_file',1,?,?,?,?,?)")
        .bind(APPLICATION_ENV_ALGORITHM)
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce)
        .bind(encrypted.key_version)
        .bind(&env_digest)
        .execute(&pool)
        .await
        .unwrap();
    for suffix in ["online", "offline"] {
        sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status) VALUES(?,'env_version_1',?,?,?,'pending')")
            .bind(format!("sync_{suffix}"))
            .bind(format!("target_{suffix}"))
            .bind(format!("node_{suffix}"))
            .bind(format!("agent_{suffix}"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let state = AppState::new(pool.clone()).with_master_key_ring(ring.clone());
    enqueue_pending_env_syncs_for_agent(&state, "agent_online")
        .await
        .unwrap();
    let (task_id, payload_digest, payload_json): (String, String, String) = sqlx::query_as(
        "SELECT id,payload_digest,payload_json FROM agent_tasks WHERE env_sync_id='sync_online'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!payload_json.contains("not-in-task-or-journal"));
    let payload: TaskPayload = serde_json::from_str(&payload_json).unwrap();
    let lease_id = match payload {
        TaskPayload::EnvSync(task) => task.lease_id,
        _ => panic!("expected env sync task"),
    };
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE env_sync_id='sync_offline'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );

    let router = app(state.clone());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/agent/application-env-leases/{lease_id}"))
                .header("authorization", "Bearer online-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        to_bytes(response.into_body(), 1024 * 1024).await.unwrap(),
        content.as_slice()
    );
    let replay = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/agent/application-env-leases/{lease_id}"))
                .header("authorization", "Bearer online-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::NOT_FOUND);

    sqlx::query("UPDATE agent_tasks SET status='delivered' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_online",
        1,
        &Message::TaskAck(TaskAck {
            task_id: task_id.clone(),
            payload_digest,
            disposition: TaskAckDisposition::Accepted,
            error_code: None,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_online",
        1,
        &Message::TaskState(TaskState {
            task_id: task_id.clone(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_online",
        1,
        &Message::TaskResult(TaskResult {
            task_id,
            sequence: 2,
            status: TaskTerminalStatus::Succeeded,
            exit_code: None,
            error_code: None,
            summary: None,
            data: Some(serde_json::json!({"env_sync_id":"sync_online","env_version":1,"digest":env_digest})),
        }),
    )
    .await
    .unwrap();
    let online: (String, Option<i64>) = sqlx::query_as(
        "SELECT status,actual_version FROM application_env_syncs WHERE id='sync_online'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(online, ("succeeded".to_owned(), Some(1)));
    let offline: String =
        sqlx::query_scalar("SELECT status FROM application_env_syncs WHERE id='sync_offline'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(offline, "pending");

    let second = b"SECRET=newest-only\n";
    let second_digest = digest(second);
    let second_encrypted = ring
        .encrypt_application_env("app_env_sync", "env_file", "env_version_2", second)
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_version_2','env_file',2,?,?,?,?,?)")
        .bind(APPLICATION_ENV_ALGORITHM)
        .bind(second_encrypted.ciphertext)
        .bind(second_encrypted.nonce)
        .bind(second_encrypted.key_version)
        .bind(&second_digest)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE application_env_files SET current_version=2,current_digest=? WHERE id='env_file'",
    )
    .bind(&second_digest)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status) VALUES('sync_offline_v2','env_version_2','target_offline','node_offline','agent_offline','pending')")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_tasks WHERE agent_id='agent_offline'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    sqlx::query("UPDATE nodes SET status='online' WHERE id='node_offline'")
        .execute(&pool)
        .await
        .unwrap();
    enqueue_pending_env_syncs_for_agent(&state, "agent_offline")
        .await
        .unwrap();
    let old_status: (String, Option<String>) = sqlx::query_as(
        "SELECT status,error_code FROM application_env_syncs WHERE id='sync_offline'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        old_status,
        ("failed".to_owned(), Some("superseded".to_owned()))
    );
    let newest_payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM agent_tasks WHERE env_sync_id='sync_offline_v2'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(newest_payload.contains("\"env_version\":2"));
    assert!(!newest_payload.contains("newest-only"));
}
