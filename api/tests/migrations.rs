use deploy_go_api::db;
use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

#[tokio::test]
async fn user_preferences_migration_upgrades_the_previous_schema() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(":memory:")
                .foreign_keys(true),
        )
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0001_initial_schema.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, username, password_hash, identity, status) VALUES ('usr_existing', 'existing', 'hash', 'administrator', 'active')")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::raw_sql(include_str!("../migrations/0002_user_preferences.sql"))
        .execute(&pool)
        .await
        .unwrap();

    let display_name: Option<String> =
        sqlx::query_scalar("SELECT display_name FROM users WHERE id = 'usr_existing'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(display_name, None);
    sqlx::query("INSERT INTO user_preferences (user_id) VALUES ('usr_existing')")
        .execute(&pool)
        .await
        .unwrap();
    let version: i64 =
        sqlx::query_scalar("SELECT version FROM user_preferences WHERE user_id = 'usr_existing'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(version, 1);

    sqlx::query("INSERT INTO users (id, username, password_hash, identity, status, email) VALUES ('usr_email', 'email-user', 'hash', 'user', 'active', 'Case@Example.com')")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        sqlx::query("INSERT INTO users (id, username, password_hash, identity, status, email) VALUES ('usr_duplicate', 'duplicate', 'hash', 'user', 'active', 'case@example.com')")
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE user_preferences SET follow_logs = 2 WHERE user_id = 'usr_existing'")
            .execute(&pool)
            .await
            .is_err()
    );
    sqlx::query("DELETE FROM users WHERE id = 'usr_existing'")
        .execute(&pool)
        .await
        .unwrap();
    let preferences: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_preferences WHERE user_id = 'usr_existing'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(preferences, 0);

    let csrf_tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'session_csrf_tokens'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(csrf_tables, 1);
}

#[tokio::test]
async fn migrations_upgrade_empty_database_and_are_repeatable() {
    let directory = tempfile::tempdir().unwrap();
    let options = SqliteConnectOptions::new()
        .filename(directory.path().join("migration-test.db"))
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    db::migrate(&pool).await.unwrap();
    db::migrate(&pool).await.unwrap();

    let tables = sqlx::query("SELECT name FROM sqlite_schema WHERE type = 'table'")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();

    for expected in [
        "users",
        "sessions",
        "user_application_grants",
        "ssh_credentials",
        "nodes",
        "node_checks",
        "applications",
        "deployment_targets",
        "secret_file_references",
        "deployments",
        "deployment_logs",
        "deployment_events",
        "audit_logs",
        "system_settings",
        "user_preferences",
        "session_csrf_tokens",
    ] {
        assert!(
            tables.iter().any(|table| table == expected),
            "missing {expected}"
        );
    }

    let user_columns: Vec<String> = sqlx::query("PRAGMA table_info(users)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(user_columns.iter().any(|column| column == "display_name"));
    assert!(user_columns.iter().any(|column| column == "email"));
}
