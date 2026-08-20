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
const MIGRATION_0008: &str = include_str!("../migrations/0008_git_branch_two_stage_deployment.sql");
const MIGRATION_0009: &str = include_str!("../migrations/0009_git_secret_leases.sql");
const MIGRATION_0010: &str = include_str!("../migrations/0010_progress_events.sql");
const MIGRATION_0011: &str = include_str!("../migrations/0011_deployment_log_stage.sql");
const MIGRATION_0012: &str =
    include_str!("../migrations/0012_cross_node_artifacts_and_application_envs.sql");
const MIGRATION_0013: &str = include_str!("../migrations/0013_deployment_application_scope.sql");

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

fn write_migrations_through_eleven(directory: &std::path::Path) {
    write_migrations_through_seven(directory);
    for (name, content) in [
        ("0008_git_branch_two_stage_deployment.sql", MIGRATION_0008),
        ("0009_git_secret_leases.sql", MIGRATION_0009),
        ("0010_progress_events.sql", MIGRATION_0010),
        ("0011_deployment_log_stage.sql", MIGRATION_0011),
    ] {
        std::fs::write(directory.join(name), content).unwrap();
    }
}

fn write_migrations_through_thirteen(directory: &std::path::Path) {
    write_migrations_through_eleven(directory);
    for (name, content) in [
        (
            "0012_cross_node_artifacts_and_application_envs.sql",
            MIGRATION_0012,
        ),
        ("0013_deployment_application_scope.sql", MIGRATION_0013),
    ] {
        std::fs::write(directory.join(name), content).unwrap();
    }
}

#[tokio::test]
async fn artifact_upload_sessions_upgrade_a_populated_version_thirteen_database() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    write_migrations_through_thirteen(&old_migrations);
    let database_path = directory.path().join("version-thirteen.db");
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
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('user-13','user13','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node-13','node13','/srv/apps','/srv/secrets','online')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment) VALUES('agent-13','node-13','prod')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO applications(id,name,slug,status) VALUES('app-13','app13','app-13','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target-13','app-13','node-13','prod','/srv/deploy.sh',60,'active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES('dep-13','app-13','target-13','user-13','running','preparing','dep-13','request','snapshot')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_digest,total_size,file_count,status,upload_offset,expires_at) VALUES('artifact-13','dep-13','manifest',1,1,'uploading',7,'2099-01-01T00:00:00Z')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO artifact_leases(id,artifact_id,agent_id,purpose,manifest_digest,status,expires_at) VALUES('lease-13','artifact-13','agent-13','artifact_upload','manifest','active','2099-01-01T00:00:00Z')").execute(&pool).await.unwrap();

    db::migrate(&pool).await.unwrap();

    let facts: (i64, Option<i64>, Option<String>) = sqlx::query_as("SELECT upload_offset,upload_size,archive_digest FROM deployment_artifacts WHERE id='artifact-13'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(facts, (7, None, None));
    let lease_status: String =
        sqlx::query_scalar("SELECT status FROM artifact_leases WHERE id='lease-13'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(lease_status, "active");
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn application_display_name_migration_allows_duplicate_names_and_keeps_slug_unique() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("0034_") {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(name)).unwrap();
    }
    let database_path = directory.path().join("version-thirty-three.db");
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
    sqlx::query(
        "INSERT INTO applications(id,name,slug,status) VALUES('app-a','App A','app-a','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO applications(id,name,slug,status) VALUES('app-b','App B','app-b','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let (name, display_name): (String, String) =
        sqlx::query_as("SELECT name,display_name FROM applications WHERE id='app-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(name, "app-a");
    assert_eq!(display_name, "App A");

    sqlx::query("INSERT INTO applications(id,name,display_name,slug,status) VALUES('app-a2','app-a2','App A','app-a2','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO applications(id,name,slug,status) VALUES('app-d','App D','app-d','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let backfilled: String =
        sqlx::query_scalar("SELECT display_name FROM applications WHERE id='app-d'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(backfilled, "App D");
    let duplicate_slug = sqlx::query("INSERT INTO applications(id,name,display_name,slug,status) VALUES('app-a3','app-a3','App C','app-a','active')")
        .execute(&pool)
        .await;
    assert!(duplicate_slug.is_err());
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
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
        "external_api_keys",
        "external_api_key_applications",
        "configuration_center_credentials",
        "configuration_centers",
        "application_configuration_centers",
        "configuration_center_reveals",
        "configuration_center_identities",
        "configuration_center_kv_mutations",
        "configuration_center_switches",
        "secret_environment_leases",
        "application_template_bindings",
        "application_config_files",
        "application_config_versions",
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
    assert!(user_columns.iter().any(|column| column == "system_account"));

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
    let deployment_columns: Vec<String> = sqlx::query("PRAGMA table_info(deployments)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(
        deployment_columns
            .iter()
            .any(|column| column == "external_api_key_id")
    );
    let application_columns: Vec<String> = sqlx::query("PRAGMA table_info(applications)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(
        application_columns
            .iter()
            .any(|column| column == "environment")
    );
    let application_default: Option<String> = sqlx::query_scalar(
        "SELECT dflt_value FROM pragma_table_info('applications') WHERE name='environment'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(application_default.as_deref(), Some("'prod'"));
}

#[tokio::test]
async fn configuration_center_migration_upgrades_a_populated_version_twenty_nine_database() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap() {
        let entry = entry.unwrap();
        if matches!(
            entry.file_name().to_str(),
            Some("0030_configuration_centers.sql" | "0031_secret_environment_lease_sources.sql")
        ) {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(entry.file_name())).unwrap();
    }
    let database_path = directory.path().join("version-twenty-nine.db");
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
    sqlx::query("INSERT INTO users (id,username,password_hash,identity,status) VALUES ('user-29','user29','hash','administrator','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO applications (id,name,slug,status) VALUES ('app-29','app29','app-29','active')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id='user-29'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(user_count, 1);
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='configuration_centers'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(table_count, 1);
    let lease_columns: Vec<String> = sqlx::query("PRAGMA table_info(secret_environment_leases)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(lease_columns.iter().any(|column| column == "credential_id"));
    assert!(
        lease_columns
            .iter()
            .any(|column| column == "descriptor_digest")
    );
    assert!(
        lease_columns
            .iter()
            .any(|column| column == "public_values_json")
    );
    assert!(
        lease_columns
            .iter()
            .any(|column| column == "credential_variable_name")
    );
    assert!(lease_columns.iter().any(|column| column == "value_digest"));
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn application_configuration_workspace_migration_preserves_a_populated_0031_database() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "0032_application_template_configuration_workspace.sql" {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(entry.file_name())).unwrap();
    }
    let database_path = directory.path().join("version-0031.db");
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
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app-0031','app-0031','app-0031','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_digest) VALUES('env-0031','app-0031','legacy.env','legacy','dotenv-v1','digest')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let tables: Vec<String> = sqlx::query("SELECT name FROM sqlite_schema WHERE type='table'")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    for table in [
        "application_template_bindings",
        "application_config_files",
        "application_config_versions",
    ] {
        assert!(tables.iter().any(|name| name == table), "missing {table}");
    }
    let legacy_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM application_env_files WHERE id='env-0031'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(legacy_count, 1);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
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
    let legacy_run: (String, String, String) = sqlx::query_as(
        "SELECT deployment_id,target_id,node_id FROM deployment_target_runs WHERE id='legacy_run_deployment_legacy'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        legacy_run,
        (
            "deployment_legacy".into(),
            "target_legacy".into(),
            "node_legacy".into()
        )
    );

    sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_prepare', 'agent_legacy', 'deployment_legacy', 'prepare', 'deployment_prepare', 'prepare-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, target_run_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_release', 'agent_legacy', 'deployment_legacy', 'legacy_run_deployment_legacy', 'release', 'deployment_release', 'release-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
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
async fn cross_node_migration_preserves_a_populated_version_eleven_database() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    write_migrations_through_eleven(&old_migrations);
    let database_path = directory.path().join("version-eleven.db");
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

    sqlx::query("INSERT INTO users (id,username,password_hash,identity,status) VALUES ('user-11','user11','hash','administrator','active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes (id,name,work_root,secrets_root,status) VALUES ('node-11','node11','/srv/apps','/srv/secrets','online')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents (id,node_id,environment) VALUES ('agent-11','node-11','prod')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO applications (id,name,slug,status) VALUES ('app-11','app11','app-11','active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets (id,application_id,node_id,environment,script_path,timeout_seconds,status,execution_mode) VALUES ('target-11','app-11','node-11','prod','/srv/deploy.sh',60,'active','two_stage')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments (id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES ('dep-11','target-11','user-11','running','deploying','dep-11-key','request','snapshot')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO git_credentials (id,name,algorithm,public_key,fingerprint,encrypted_private_key,nonce,key_version,status) VALUES ('git-11','git11','ed25519','ssh-ed25519 AAAA','SHA256:11',X'01',X'02',1,'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_sources (id,application_id,repository_url,git_credential_id,build_agent_id,deployment_branch,status) VALUES ('source-11','app-11','git@example.test:app.git','git-11','agent-11','main','verified')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_tasks (id,agent_id,deployment_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES ('prepare-11','agent-11','dep-11','prepare','deployment_prepare','prepare-11','digest','{}','succeeded','2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_tasks (id,agent_id,deployment_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES ('release-11','agent-11','dep-11','release','deployment_release','release-11','digest','{}','running','2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_task_events (task_id,sequence,kind,payload_json) VALUES ('release-11',1,'progress','{}')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_logs (deployment_id,task_id,sequence,task_sequence,stream,content) VALUES ('dep-11','release-11',1,1,'stdout','preserved')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO git_secret_leases (id,task_id,git_credential_id,payload_digest,purpose,status,expires_at) VALUES ('lease-11','prepare-11','git-11','digest','git_credential','issued','2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO git_ref_discoveries (id,application_source_id,source_version,task_id,status) VALUES ('refs-11','source-11',1,'prepare-11','queued')")
        .execute(&pool).await.unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let target_run_id: String =
        sqlx::query_scalar("SELECT target_run_id FROM agent_tasks WHERE id='release-11'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(target_run_id, "legacy_run_dep-11");
    let application_id: String =
        sqlx::query_scalar("SELECT application_id FROM deployments WHERE id='dep-11'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(application_id, "app-11");
    for (table, predicate) in [
        ("agent_task_events", "task_id='release-11'"),
        ("deployment_logs", "task_id='release-11'"),
        ("git_secret_leases", "task_id='prepare-11'"),
        ("git_ref_discoveries", "task_id='prepare-11'"),
    ] {
        let count: i64 =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE {predicate}"))
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1, "{table} row was not preserved");
    }
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

#[tokio::test]
async fn privileged_terminal_migration_preserves_a_populated_version_sixteen_database() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("0017_")
            || name.to_string_lossy().starts_with("0018_")
            || name.to_string_lossy().starts_with("0019_")
            || name.to_string_lossy().starts_with("0020_")
            || name.to_string_lossy().starts_with("0021_")
            || name.to_string_lossy().starts_with("0022_")
            || name.to_string_lossy().starts_with("0024_")
            || name.to_string_lossy().starts_with("0025_")
            || name.to_string_lossy().starts_with("0033_")
            || name.to_string_lossy().starts_with("0034_")
        {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(name)).unwrap();
    }
    let database_path = directory.path().join("version-sixteen.db");
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
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status,display_name) VALUES('user-16','user16','hash','administrator','active','User 16')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status,work_root,secrets_root) VALUES('node-16','node16','online','/srv/apps','/srv/secrets')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,protocol_version,capabilities_json) VALUES('agent-16','node-16','prod',4,'{}')").execute(&pool).await.unwrap();

    db::migrate(&pool).await.unwrap();

    let node: (String, bool) =
        sqlx::query_as("SELECT name,privileged_execution FROM nodes WHERE id='node-16'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(node, ("node16".into(), false));
    let terminal_table: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name='terminal_sessions')").fetch_one(&pool).await.unwrap();
    assert!(terminal_table);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn image_deployment_migration_preserves_targets_and_enables_image_mode() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("0020_")
            || name.to_string_lossy().starts_with("0021_")
            || name.to_string_lossy().starts_with("0022_")
            || name.to_string_lossy().starts_with("0024_")
            || name.to_string_lossy().starts_with("0025_")
            || name.to_string_lossy().starts_with("0033_")
            || name.to_string_lossy().starts_with("0034_")
        {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(name)).unwrap();
    }
    let database_path = directory.path().join("version-nineteen.db");
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
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('user-19','user19','hash','administrator','active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node-19','node19','/srv/apps','/srv/secrets','online')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,protocol_version,capabilities_json) VALUES('agent-19','node-19','prod',8,'[\"privileged_release\"]')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO applications(id,name,slug,status) VALUES('app-19','app19','app-19','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,privileged_release,status) VALUES('target-19','app-19','node-19','prod','two_stage','/srv/deploy.sh',60,1,'active')")
        .execute(&pool).await.unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let preserved: (String, bool, Option<String>) = sqlx::query_as(
        "SELECT execution_mode, privileged_release, image_spec_json FROM deployment_targets WHERE id='target-19'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (preserved.0.as_str(), preserved.1, preserved.2.is_none()),
        ("two_stage", true, true)
    );
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,privileged_release,image_spec_json,target_code,status) VALUES('target-image-19','app-19','node-19','staging','image','',60,1,'{\"template\":\"postgres\",\"image\":\"postgres:18-alpine\",\"host_port\":5432,\"env_files\":[\"postgres.env\"]}','staging','active')")
        .execute(&pool)
        .await
        .unwrap();
    let image_mode: String = sqlx::query_scalar(
        "SELECT execution_mode FROM deployment_targets WHERE id='target-image-19'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(image_mode, "image");
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn application_environment_migration_backfills_agents_and_targets() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("0021_")
            || name.to_string_lossy().starts_with("0022_")
            || name.to_string_lossy().starts_with("0024_")
            || name.to_string_lossy().starts_with("0025_")
            || name.to_string_lossy().starts_with("0033_")
            || name.to_string_lossy().starts_with("0034_")
        {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(name)).unwrap();
    }
    let database_path = directory.path().join("version-twenty.db");
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
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('user-20','user20','hash','administrator','active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node-20','node20','/srv/apps','/srv/secrets','online')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,last_seen_at) VALUES('agent-20','node-20','test','2026-08-12T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO applications(id,name,slug,status) VALUES('app-20','app20','app-20','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target-20','app-20','node-20','prod','two_stage','/srv/deploy.sh',60,'active')")
        .execute(&pool).await.unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let application_environment: String =
        sqlx::query_scalar("SELECT environment FROM applications WHERE id='app-20'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(application_environment, "test");
    let target_environment: String =
        sqlx::query_scalar("SELECT environment FROM deployment_targets WHERE id='target-20'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(target_environment, "test");
    let target_version: i64 =
        sqlx::query_scalar("SELECT version FROM deployment_targets WHERE id='target-20'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(target_version, 2);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn application_environment_migration_keeps_ambiguous_targets_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("0021_")
            || name.to_string_lossy().starts_with("0022_")
            || name.to_string_lossy().starts_with("0024_")
            || name.to_string_lossy().starts_with("0025_")
            || name.to_string_lossy().starts_with("0033_")
            || name.to_string_lossy().starts_with("0034_")
        {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(name)).unwrap();
    }
    let database_path = directory.path().join("version-twenty-conflict.db");
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
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('user-20c','user20c','hash','administrator','active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node-20c','node20c','/srv/apps','/srv/secrets','online')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,last_seen_at) VALUES('agent-20c','node-20c','test','2026-08-12T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app-20c','app20c','app-20c','active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target-20a','app-20c','node-20c','prod','two_stage','/srv/deploy.sh',60,'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target-20b','app-20c','node-20c','production','two_stage','/srv/deploy.sh',60,'active')")
        .execute(&pool).await.unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let application_environment: String =
        sqlx::query_scalar("SELECT environment FROM applications WHERE id='app-20c'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(application_environment, "test");
    let target_environments: Vec<String> = sqlx::query_scalar(
        "SELECT environment FROM deployment_targets WHERE application_id='app-20c' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        target_environments,
        vec!["prod".to_owned(), "production".to_owned()]
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn application_deploy_contract_migration_backfills_application_schema_and_verification() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("0024_")
            || name.to_string_lossy().starts_with("0025_")
        {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(name)).unwrap();
    }
    let database_path = directory.path().join("version-twenty-three.db");
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
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('user-24','user24','hash','administrator','active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node-24','node24','/srv/apps','/srv/secrets','online')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,last_seen_at) VALUES('agent-24','node-24','test','2026-08-13T00:00:00Z')")
        .execute(&pool).await.unwrap();
    for app in [
        "app-24-schema",
        "app-24-default",
        "app-24-mirror",
        "app-24-latest",
    ] {
        sqlx::query("INSERT INTO applications(id,name,slug,status,environment) VALUES(?,?,?, 'active','test')")
            .bind(app)
            .bind(app)
            .bind(app)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,parameter_schema,timeout_seconds,verification_config,privileged_release,status,created_at,updated_at) VALUES('target-24-schema','app-24-schema','node-24','prod','two_stage','/srv/deploy.sh','{\"type\":\"object\",\"properties\":{\"modules\":{\"type\":\"string\"}},\"required\":[\"modules\"]}',60,'{\"type\":\"http\",\"path\":\"/healthz\",\"expected_status\":200,\"timeout_ms\":3000}',1,'active','2026-08-12T00:00:00.000Z','2026-08-12T00:00:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,parameter_schema,timeout_seconds,verification_config,privileged_release,status,created_at,updated_at) VALUES('target-24-mirror','app-24-mirror','node-24','prod','two_stage','/srv/deploy.sh','{}',60,'{}',0,'active','2026-08-12T00:00:00.000Z','2026-08-12T00:00:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,parameter_schema,timeout_seconds,verification_config,privileged_release,status,created_at,updated_at,target_code) VALUES('target-24-old','app-24-latest','node-24','prod','two_stage','/srv/deploy.sh','{\"version\":\"old\"}',60,'{\"version\":\"old\"}',0,'active','2026-08-12T00:00:00.000Z','2026-08-12T00:00:00.000Z','prod')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,parameter_schema,timeout_seconds,verification_config,privileged_release,status,created_at,updated_at,target_code) VALUES('target-24-new','app-24-latest','node-24','staging','two_stage','/srv/deploy.sh','{\"version\":\"new\"}',60,'{\"version\":\"new\"}',0,'active','2026-08-13T00:00:00.000Z','2026-08-13T00:00:00.000Z','staging')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES('dep-24','app-24-latest','target-24-new','user-24','succeeded','finished','dep-24-key','request','snapshot')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();

    let schema: String =
        sqlx::query_scalar("SELECT parameter_schema FROM applications WHERE id='app-24-schema'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let verification: String =
        sqlx::query_scalar("SELECT verification_config FROM applications WHERE id='app-24-schema'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(schema.contains("\"modules\""));
    assert!(verification.contains("\"timeout_ms\":3000"));

    let default_schema: String =
        sqlx::query_scalar("SELECT parameter_schema FROM applications WHERE id='app-24-default'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(default_schema.contains("\"additionalProperties\":false"));
    let mirror_schema: String =
        sqlx::query_scalar("SELECT parameter_schema FROM applications WHERE id='app-24-mirror'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(mirror_schema.contains("\"additionalProperties\":false"));

    let latest_schema: String =
        sqlx::query_scalar("SELECT parameter_schema FROM applications WHERE id='app-24-latest'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(latest_schema.contains("\"version\":\"new\""));
    let latest_verification: String =
        sqlx::query_scalar("SELECT verification_config FROM applications WHERE id='app-24-latest'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(latest_verification.contains("\"version\":\"new\""));

    let target_columns: Vec<String> = sqlx::query("PRAGMA table_info(deployment_targets)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(
        target_columns
            .iter()
            .any(|column| column == "parameter_schema")
    );
    assert!(
        target_columns
            .iter()
            .any(|column| column == "verification_config")
    );
    let target_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployment_targets")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(target_count, 4);
    let non_privileged_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployment_targets WHERE privileged_release=0")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(non_privileged_count, 0);

    let triggers: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_schema WHERE type='trigger' AND name IN ('deployments_application_matches_target_insert','deployments_application_immutable_update')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(triggers, 2);
    assert!(
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES('dep-24-bad','app-24-schema','target-24-new','user-24','queued','pending','dep-24-bad-key','request','snapshot')")
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap()
            .is_empty()
    );
}
