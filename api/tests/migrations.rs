use deploy_go_api::db;
use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

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
    ] {
        assert!(
            tables.iter().any(|table| table == expected),
            "missing {expected}"
        );
    }
}
