use deploy_go_api::{agents::store, db};
use sqlx::sqlite::SqlitePoolOptions;

async fn database() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn create_agent_atomically_creates_an_offline_node() {
    let pool = database().await;
    let agent = store::create_with_node(&pool, " production-01 ", "staging")
        .await
        .unwrap();

    assert_eq!(agent.name, "production-01");
    assert_eq!(agent.registered_at, None);
    let node: (String, Option<String>, Option<i64>, String, String, String) =
        sqlx::query_as("SELECT name,host,port,status,work_root,secrets_root FROM nodes WHERE id=?")
            .bind(&agent.node_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        node,
        (
            "production-01".into(),
            None,
            None,
            "offline".into(),
            "/var/lib/deploy-go-agent/apps".into(),
            "/var/lib/deploy-go-agent/secrets".into(),
        )
    );
}

#[tokio::test]
async fn names_and_node_bindings_are_unique() {
    let pool = database().await;
    let agent = store::create_with_node(&pool, "production-01", "staging")
        .await
        .unwrap();

    assert!(matches!(
        store::create_with_node(&pool, "PRODUCTION-01", "staging").await,
        Err(store::CreateAgentError::NameConflict)
    ));
    assert!(matches!(
        store::bind_existing_node(&pool, &agent.node_id, "staging").await,
        Err(store::CreateAgentError::NodeAlreadyBound)
    ));
    assert!(matches!(
        store::bind_existing_node(&pool, "node_missing", "staging").await,
        Err(store::CreateAgentError::NodeNotFound)
    ));
}

#[tokio::test]
async fn concurrent_creation_with_the_same_name_only_succeeds_once() {
    let directory = tempfile::tempdir().unwrap();
    let database_url = format!(
        "sqlite://{}?mode=rwc",
        directory.path().join("agents.db").display()
    );
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let (first, second) = tokio::join!(
        store::create_with_node(&pool, "production-01", "staging"),
        store::create_with_node(&pool, "PRODUCTION-01", "staging")
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let failure = if first.is_err() { first } else { second };
    assert!(matches!(
        failure,
        Err(store::CreateAgentError::NameConflict)
    ));

    let nodes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nodes")
        .fetch_one(&pool)
        .await
        .unwrap();
    let agents: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((nodes, agents), (1, 1));
}

#[tokio::test]
async fn deleting_an_agent_cannot_delete_its_node_or_history() {
    let pool = database().await;
    let agent = store::create_with_node(&pool, "production-01", "staging")
        .await
        .unwrap();

    let error = sqlx::query("DELETE FROM nodes WHERE id=?")
        .bind(&agent.node_id)
        .execute(&pool)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("FOREIGN KEY constraint failed"));

    sqlx::query("UPDATE agents SET revoked_at='2026-08-03T03:00:00Z',archived_at='2026-08-03T03:00:00Z' WHERE id=?")
        .bind(&agent.id)
        .execute(&pool)
        .await
        .unwrap();
    let node_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM nodes WHERE id=?)")
        .bind(&agent.node_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(node_exists);
}

#[tokio::test]
async fn token_and_task_constraints_enforce_one_active_identity() {
    let pool = database().await;
    let agent = store::create_with_node(&pool, "production-01", "staging")
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_enrollment_tokens (id,agent_id,token_hash,expires_at) VALUES ('enroll_1',?,X'0102','2026-08-03T03:30:00Z')")
        .bind(&agent.id).execute(&pool).await.unwrap();
    assert!(sqlx::query("INSERT INTO agent_enrollment_tokens (id,agent_id,token_hash,expires_at) VALUES ('enroll_2',?,X'0304','2026-08-03T03:30:00Z')")
        .bind(&agent.id).execute(&pool).await.is_err());

    sqlx::query("INSERT INTO agent_credential_families (id,agent_id) VALUES ('family_1',?)")
        .bind(&agent.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        sqlx::query("INSERT INTO agent_credential_families (id,agent_id) VALUES ('family_2',?)")
            .bind(&agent.id)
            .execute(&pool)
            .await
            .is_err()
    );

    sqlx::query("INSERT INTO agent_tasks (id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES ('task_1',?,'system_inspect','idem-0123456789abcdef','sha256:01','{}','queued','2026-08-03T03:30:00Z')")
        .bind(&agent.id).execute(&pool).await.unwrap();
    assert!(sqlx::query("INSERT INTO agent_tasks (id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES ('task_2',?,'system_inspect','idem-0123456789abcdef','sha256:02','{}','queued','2026-08-03T03:30:00Z')")
        .bind(&agent.id).execute(&pool).await.is_err());
}
