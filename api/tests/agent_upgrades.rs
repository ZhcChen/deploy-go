use axum::http::StatusCode;
use chrono::Utc;
use common::{admin_session, json_request, response_json, test_app};
use deploy_go_api::{agents::upgrades, db};
use serde_json::json;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

mod common;

async fn pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-upgrade','Upgrade Node','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,last_seen_at,protocol_version,architecture,capabilities_json,connection_generation) VALUES('agent-upgrade','node-upgrade','test',?,17,'x86_64','[\"agent_upgrade_v1\"]',1)")
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    pool
}

#[tokio::test]
async fn enqueue_is_idempotent_and_claim_creates_fenced_lock() {
    let pool = pool().await;
    let digest = format!("sha256:{}", "a".repeat(64));
    let first = upgrades::enqueue(
        &pool,
        "agent-upgrade",
        "node-upgrade",
        Some("0.3.6"),
        "0.3.7",
        &digest,
        Some("x86_64"),
    )
    .await
    .unwrap();
    assert!(first.is_some());
    assert!(
        upgrades::enqueue(
            &pool,
            "agent-upgrade",
            "node-upgrade",
            Some("0.3.6"),
            "0.3.7",
            &digest,
            Some("x86_64"),
        )
        .await
        .unwrap()
        .is_none()
    );

    let now = Utc::now().to_rfc3339();
    upgrades::refresh_waiting_state(&pool, &now).await.unwrap();
    let claim = upgrades::claim_ready_job(&pool, &now, 120)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.node_id, "node-upgrade");
    assert_eq!(claim.lock_epoch, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_maintenance_locks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(
        upgrades::claim_ready_job(&pool, &now, 120)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn unsupported_architecture_is_blocked_without_a_claim() {
    let pool = pool().await;
    let digest = format!("sha256:{}", "b".repeat(64));
    let job = upgrades::enqueue(
        &pool,
        "agent-upgrade",
        "node-upgrade",
        Some("0.3.6"),
        "0.3.7-arm",
        &digest,
        Some("aarch64"),
    )
    .await
    .unwrap()
    .unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM agent_upgrade_jobs WHERE id=?")
        .bind(job)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "blocked_unsupported_architecture");
    assert!(
        upgrades::claim_ready_job(&pool, &Utc::now().to_rfc3339(), 120)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn admin_upgrade_api_lists_and_retries_failed_jobs_with_csrf() {
    let (app, pool) = test_app().await;
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-api','API Node','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,agent_version,architecture) VALUES('agent-api','node-api','test','0.3.6','x86_64')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,error_code,error_summary) VALUES('upgrade_api','agent-api','node-api','0.3.7',?, 'x86_64','failed','upgrade_install_failed','安装失败')")
        .bind(format!("sha256:{}", "c".repeat(64)))
        .execute(&pool)
        .await
        .unwrap();

    let (cookie, csrf) = admin_session(app.clone()).await;
    let listed = json_request(
        app.clone(),
        "GET",
        "/api/v1/agent-upgrades",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(response_json(listed).await[0]["id"], "upgrade_api");

    let missing_csrf = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent-upgrades/upgrade_api/retry",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);

    let retried = json_request(
        app,
        "POST",
        "/api/v1/agent-upgrades/upgrade_api/retry",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(retried.status(), StatusCode::OK);
    assert_eq!(response_json(retried).await["status"], "queued");
}
