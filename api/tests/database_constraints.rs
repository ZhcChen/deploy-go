use deploy_go_api::db;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

async fn database() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    pool
}

async fn insert_user(pool: &SqlitePool, id: &str, identity: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, identity, status) VALUES (?, ?, 'hash', ?, 'active')",
    )
    .bind(id)
    .bind(format!("user-{id}"))
    .bind(identity)
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
async fn database_allows_only_one_administrator() {
    let pool = database().await;
    insert_user(&pool, "admin-1", "administrator")
        .await
        .unwrap();

    let error = insert_user(&pool, "admin-2", "administrator")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("UNIQUE constraint failed"));
}

#[tokio::test]
async fn bound_ssh_credential_cannot_be_deleted() {
    let pool = database().await;
    sqlx::query("INSERT INTO ssh_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version) VALUES ('cred-1', 'primary', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:test', X'01', X'02', 1)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes (id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status) VALUES ('node-1', 'node', '127.0.0.1', 22, 'deploy', 'cred-1', '/srv/apps', '/srv/secrets', 'unchecked')")
        .execute(&pool).await.unwrap();

    let error = sqlx::query("DELETE FROM ssh_credentials WHERE id = 'cred-1'")
        .execute(&pool)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
}

#[tokio::test]
async fn application_grant_is_unique_per_user_and_application() {
    let pool = database().await;
    insert_user(&pool, "user-1", "user").await.unwrap();
    sqlx::query("INSERT INTO applications (id, name, slug, status) VALUES ('app-1', 'app', 'app', 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO user_application_grants (user_id, application_id, granted_by) VALUES ('user-1', 'app-1', 'user-1')")
        .execute(&pool).await.unwrap();

    let error = sqlx::query("INSERT INTO user_application_grants (user_id, application_id, granted_by) VALUES ('user-1', 'app-1', 'user-1')")
        .execute(&pool).await.unwrap_err();
    assert!(error.to_string().contains("UNIQUE constraint failed"));
}

#[tokio::test]
async fn target_allows_multiple_queued_but_only_one_execution_owner() {
    let pool = database().await;
    insert_user(&pool, "admin-1", "administrator")
        .await
        .unwrap();
    sqlx::query("INSERT INTO ssh_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version) VALUES ('cred-1', 'primary', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:test', X'01', X'02', 1)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes (id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status) VALUES ('node-1', 'node', '127.0.0.1', 22, 'deploy', 'cred-1', '/srv/apps', '/srv/secrets', 'online')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications (id, name, slug, status) VALUES ('app-1', 'app', 'app', 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets (id, application_id, node_id, environment, script_path, parameter_schema, timeout_seconds, verification_config, status) VALUES ('target-1', 'app-1', 'node-1', 'prod', '/srv/apps/deploy.sh', '{}', 900, '{}', 'active')")
        .execute(&pool).await.unwrap();

    for (id, key) in [
        ("dep-1", "idempotency-key-0001"),
        ("dep-2", "idempotency-key-0002"),
    ] {
        sqlx::query("INSERT INTO deployments (id, target_id, requested_by, status, phase, idempotency_key, request_hash, snapshot_hash) VALUES (?, 'target-1', 'admin-1', 'queued', 'queued', ?, ?, 'snapshot')")
            .bind(id).bind(key).bind(key).execute(&pool).await.unwrap();
    }

    sqlx::query(
        "UPDATE deployments SET status = 'running', phase = 'executing' WHERE id = 'dep-1'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = sqlx::query(
        "UPDATE deployments SET status = 'running', phase = 'executing' WHERE id = 'dep-2'",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(error.to_string().contains("UNIQUE constraint failed"));
}
