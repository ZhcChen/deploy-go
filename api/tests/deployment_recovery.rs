use deploy_go_api::{
    AppState, db,
    deployments::{purge_expired_output, recover},
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn restart_interrupts_uncertain_work_and_preserves_queue() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&pool)
        .await
        .unwrap();
    for (id, status) in [
        ("queued", "queued"),
        ("running", "running"),
        ("canceling", "canceling"),
    ] {
        sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES(?,?,?,?,?,?,?,?)")
            .bind(id).bind(format!("target-{id}")).bind("user").bind(status).bind(status).bind(format!("recovery-key-{id}" )).bind(id).bind("snapshot").execute(&pool).await.unwrap();
    }
    assert_eq!(recover(&pool).await.unwrap(), 2);
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT id,status FROM deployments ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [
            ("canceling".to_owned(), "interrupted".to_owned()),
            ("queued".to_owned(), "queued".to_owned()),
            ("running".to_owned(), "interrupted".to_owned())
        ]
    );
}

#[tokio::test]
async fn retention_removes_output_but_preserves_deployment_history() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,finished_at) VALUES('old','target','user','failed','failed','retention-old-key','hash','snapshot','2020-01-01T00:00:00Z')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,stream,content) VALUES('old',1,'stdout','old log')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_events(id,deployment_id,log_sequence,event_name,payload_json) VALUES('old-event','old',1,'diagnostic','{}')").execute(&pool).await.unwrap();
    let state = AppState::new(pool.clone());
    assert_eq!(purge_expired_output(&state).await.unwrap(), 1);
    let deployments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployments WHERE id='old'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let logs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployment_logs WHERE deployment_id='old'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployment_events WHERE deployment_id='old'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((deployments, logs, events), (1, 0, 0));
}
