use deploy_go_api::db;
use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

const MIGRATION_0001: &str = include_str!("../migrations/0001_initial_schema.sql");
const MIGRATION_0002: &str = include_str!("../migrations/0002_user_preferences.sql");
const MIGRATION_0003: &str = include_str!("../migrations/0003_node_agents.sql");
const MIGRATION_0004: &str = include_str!("../migrations/0004_agent_auth_rotation.sql");
const MIGRATION_0005: &str = include_str!("../migrations/0005_agent_node_online_status.sql");
const MIGRATION_0006: &str = include_str!("../migrations/0006_agent_node_checks.sql");
const MIGRATION_0007: &str = include_str!("../migrations/0007_agent_environment.sql");

fn write_migrations(directory: &std::path::Path, third: Option<&str>) {
    std::fs::write(directory.join("0001_initial_schema.sql"), MIGRATION_0001).unwrap();
    std::fs::write(directory.join("0002_user_preferences.sql"), MIGRATION_0002).unwrap();
    if let Some(third) = third {
        std::fs::write(directory.join("0003_node_agents.sql"), third).unwrap();
    }
}

fn write_migrations_through_seven(directory: &std::path::Path) {
    for (name, content) in [
        ("0001_initial_schema.sql", MIGRATION_0001),
        ("0002_user_preferences.sql", MIGRATION_0002),
        ("0003_node_agents.sql", MIGRATION_0003),
        ("0004_agent_auth_rotation.sql", MIGRATION_0004),
        ("0005_agent_node_online_status.sql", MIGRATION_0005),
        ("0006_agent_node_checks.sql", MIGRATION_0006),
        ("0007_agent_environment.sql", MIGRATION_0007),
    ] {
        std::fs::write(directory.join(name), content).unwrap();
    }
}

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
        "agents",
        "agent_enrollment_tokens",
        "agent_credential_families",
        "agent_refresh_credentials",
        "agent_access_sessions",
        "agent_tasks",
        "agent_task_events",
        "git_credentials",
        "application_sources",
        "git_ref_discoveries",
        "git_secret_leases",
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

    let agent_columns: Vec<String> = sqlx::query("PRAGMA table_info(agents)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(
        agent_columns
            .iter()
            .any(|column| column == "connection_generation")
    );
    assert!(agent_columns.iter().any(|column| column == "environment"));
    let access_columns: Vec<String> = sqlx::query("PRAGMA table_info(agent_access_sessions)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(
        access_columns
            .iter()
            .any(|column| column == "refresh_credential_id")
    );
}

#[tokio::test]
async fn two_stage_migration_preserves_legacy_tasks_and_enforces_stage_constraints() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    write_migrations_through_seven(&old_migrations);
    let database_path = directory.path().join("two-stage-upgrade.db");
    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(old_migrations)
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO users (id, username, password_hash, identity, status) VALUES ('usr_legacy', 'legacy', 'hash', 'administrator', 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes (id, name, work_root, secrets_root, status) VALUES ('node_legacy', 'legacy-node', '/var/lib/deploy-go-agent/apps', '/var/lib/deploy-go-agent/secrets', 'online')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents (id, node_id, environment) VALUES ('agent_legacy', 'node_legacy', 'test')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications (id, name, slug, status) VALUES ('app_legacy', 'legacy-app', 'legacy-app', 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets (id, application_id, node_id, environment, script_path, timeout_seconds, status) VALUES ('target_legacy', 'app_legacy', 'node_legacy', 'test', '/srv/deploy.sh', 60, 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments (id, target_id, requested_by, status, phase, idempotency_key, request_hash, snapshot_hash) VALUES ('deployment_legacy', 'target_legacy', 'usr_legacy', 'running', 'executing', 'legacy-key', 'request', 'snapshot')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_legacy', 'agent_legacy', 'deployment_legacy', 'deployment_execute', 'legacy-task', 'digest', '{}', 'running', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,stream,content) VALUES('deployment_legacy',1,'stdout','legacy log')")
        .execute(&pool).await.unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let preserved: (String, Option<String>, String) = sqlx::query_as(
        "SELECT kind, stage, deployment_id FROM agent_tasks WHERE id = 'task_legacy'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        preserved,
        (
            "deployment_execute".into(),
            None,
            "deployment_legacy".into()
        )
    );

    let (task_id, task_sequence): (Option<String>, i64) = sqlx::query_as(
        "SELECT task_id,task_sequence FROM deployment_logs WHERE deployment_id='deployment_legacy' AND sequence=1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((task_id, task_sequence), (None, 1));

    let execution_mode: String = sqlx::query_scalar(
        "SELECT execution_mode FROM deployment_targets WHERE id = 'target_legacy'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(execution_mode, "script");

    sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_prepare', 'agent_legacy', 'deployment_legacy', 'prepare', 'deployment_prepare', 'prepare-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_release', 'agent_legacy', 'deployment_legacy', 'release', 'deployment_release', 'release-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    let duplicate_stage = sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_prepare_2', 'agent_legacy', 'deployment_legacy', 'prepare', 'deployment_prepare', 'prepare-2', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(duplicate_stage.is_err());
    let missing_stage = sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_bad_stage', 'agent_legacy', 'deployment_legacy', 'deployment_prepare', 'prepare-bad', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(missing_stage.is_err());
    let bad_kind = sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_bad_kind', 'agent_legacy', 'deployment_legacy', 'prepare', 'deployment_execute', 'prepare-bad-kind', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(bad_kind.is_err());
    sqlx::query("INSERT INTO agent_tasks (id, agent_id, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_refs', 'agent_legacy', 'git_refs_query', 'refs-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();

    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn node_agent_migration_preserves_a_related_legacy_database() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    write_migrations(&old_migrations, None);
    let database_path = directory.path().join("upgrade.db");
    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(old_migrations)
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    seed_related_legacy_data(&pool).await;
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let preserved: (String, String, String) = sqlx::query_as("SELECT n.id,t.id,d.id FROM nodes n JOIN deployment_targets t ON t.node_id=n.id JOIN deployments d ON d.target_id=t.id WHERE n.id='node_existing'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        preserved,
        (
            "node_existing".into(),
            "target_existing".into(),
            "dep_existing".into()
        )
    );
    let checks: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM node_checks WHERE node_id='node_existing'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(checks, 1);
    let foreign_key_errors = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(foreign_key_errors.is_empty());
    sqlx::query("INSERT INTO nodes (id,name,status) VALUES ('node_agent','agent-node','offline')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE nodes SET status='online' WHERE id='node_agent'")
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn failed_node_rebuild_rolls_back_without_damaging_legacy_data() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    let broken_migrations = directory.path().join("broken-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    std::fs::create_dir(&broken_migrations).unwrap();
    write_migrations(&old_migrations, None);
    let broken = format!(
        "{}\nINSERT INTO table_that_does_not_exist VALUES (1);",
        include_str!("../migrations/0003_node_agents.sql")
    );
    write_migrations(&broken_migrations, Some(&broken));
    let database_path = directory.path().join("rollback.db");
    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(old_migrations)
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    seed_related_legacy_data(&pool).await;

    assert!(
        db::migrate_with(
            &pool,
            &sqlx::migrate::Migrator::new(broken_migrations)
                .await
                .unwrap(),
        )
        .await
        .is_err()
    );
    let node_name: String = sqlx::query_scalar("SELECT name FROM nodes WHERE id='node_existing'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(node_name, "legacy-node");
    let agents_table: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='agents'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(agents_table, 0);
    let migration_record: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 3")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(migration_record, 0);
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(foreign_keys, 1);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}

async fn seed_related_legacy_data(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO users (id,username,password_hash,identity,status) VALUES ('usr_existing','existing','hash','administrator','active')")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO ssh_credentials (id,name,algorithm,public_key,fingerprint,encrypted_private_key,nonce,key_version) VALUES ('cred_existing','legacy-key','ed25519','ssh-ed25519 AAAA','SHA256:legacy',X'01',X'02',1)")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO nodes (id,name,host,port,username,ssh_credential_id,work_root,secrets_root,status) VALUES ('node_existing','legacy-node','127.0.0.1',22,'deploy','cred_existing','/srv/apps','/srv/secrets','online')")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO node_checks (id,node_id,status) VALUES ('check_existing','node_existing','succeeded')")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO applications (id,name,slug,status) VALUES ('app_existing','legacy-app','legacy-app','active')")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets (id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES ('target_existing','app_existing','node_existing','production','/srv/apps/deploy.sh',60,'active')")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO deployments (id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES ('dep_existing','target_existing','usr_existing','succeeded','finished','idempotency-legacy','request','snapshot')")
        .execute(pool).await.unwrap();
}
