mod common;

use deploy_go_api::{db, terminals::store};
use sqlx::sqlite::SqlitePoolOptions;

async fn fixture() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect("sqlite::memory:?cache=shared")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status,display_name) VALUES('usr_admin','admin','x','administrator','active','Admin')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status,work_root,secrets_root) VALUES('node_one','Node One','online','/work','/secrets')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,protocol_version,capabilities_json) VALUES('agent_one','node_one','2026-08-07T00:00:00Z',11,'[\"pty_terminal\"]')")
        .execute(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn database_allows_only_one_active_session_per_node() {
    let pool = fixture().await;
    store::create_session(
        &pool,
        "term_one",
        "node_one",
        "agent_one",
        "usr_admin",
        "req_one",
    )
    .await
    .unwrap();
    let error = store::create_session(
        &pool,
        "term_two",
        "node_one",
        "agent_one",
        "usr_admin",
        "req_two",
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        store::CreateSessionError::ActiveSessionConflict
    ));
}

#[tokio::test]
async fn expired_unattached_session_does_not_permanently_lock_the_node() {
    let pool = fixture().await;
    store::create_session(
        &pool,
        "term_stale",
        "node_one",
        "agent_one",
        "usr_admin",
        "req_stale",
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE terminal_sessions SET started_at='2000-01-01T00:00:00Z' WHERE id='term_stale'",
    )
    .execute(&pool)
    .await
    .unwrap();

    store::create_session(
        &pool,
        "term_next",
        "node_one",
        "agent_one",
        "usr_admin",
        "req_next",
    )
    .await
    .unwrap();

    let stale = store::find_session(&pool, "term_stale")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stale.status, "interrupted");
    assert_eq!(stale.exit_reason.as_deref(), Some("attach_timeout"));
}

#[tokio::test]
async fn revoking_agent_converges_active_sessions() {
    let pool = fixture().await;
    store::create_session(
        &pool,
        "term_one",
        "node_one",
        "agent_one",
        "usr_admin",
        "req_one",
    )
    .await
    .unwrap();
    store::close_sessions_for_agent(&pool, "agent_one", "agent_identity_revoked")
        .await
        .unwrap();
    let second = store::find_session(&pool, "term_one")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.status, "closed");
    assert_eq!(
        second.exit_reason.as_deref(),
        Some("agent_identity_revoked")
    );
}

#[tokio::test]
async fn terminal_schema_has_no_input_or_output_body_columns() {
    let pool = fixture().await;
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('terminal_sessions')")
            .fetch_all(&pool)
            .await
            .unwrap();
    for forbidden in ["input", "output", "content", "command", "transcript"] {
        assert!(!columns.iter().any(|column| column == forbidden));
    }
}

#[tokio::test]
async fn interrupted_is_a_terminal_session_state() {
    let pool = fixture().await;
    store::create_session(
        &pool,
        "term_one",
        "node_one",
        "agent_one",
        "usr_admin",
        "req_one",
    )
    .await
    .unwrap();
    sqlx::query("UPDATE terminal_sessions SET status='interrupted',finished_at='2026-08-07T00:01:00Z',exit_reason='api_restarted' WHERE id='term_one'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        store::find_session(&pool, "term_one")
            .await
            .unwrap()
            .unwrap()
            .status,
        "interrupted"
    );
}
