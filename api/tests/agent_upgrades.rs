use chrono::Utc;
use deploy_go_api::{agents::upgrades, db};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

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
    sqlx::query("INSERT INTO agents(id,node_id,environment,last_seen_at,protocol_version,architecture,capabilities_json) VALUES('agent-upgrade','node-upgrade','test',?,17,'x86_64','[\"agent_upgrade_v1\"]')")
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
