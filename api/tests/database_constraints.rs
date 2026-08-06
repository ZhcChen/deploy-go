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

#[tokio::test]
async fn git_credential_name_and_fingerprint_are_unique_and_status_is_checked() {
    let pool = database().await;
    sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status) VALUES ('git-1', 'Primary', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:one', X'01', X'02', 1, 'active')")
        .execute(&pool).await.unwrap();

    let duplicate_name = sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status) VALUES ('git-2', 'primary', 'ed25519', 'ssh-ed25519 BBBB', 'SHA256:two', X'01', X'02', 1, 'active')")
        .execute(&pool).await;
    assert!(duplicate_name.is_err());
    let bad_status = sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status) VALUES ('git-3', 'Other', 'ed25519', 'ssh-ed25519 CCCC', 'SHA256:three', X'01', X'02', 1, 'deleted')")
        .execute(&pool).await;
    assert!(bad_status.is_err());
}

#[tokio::test]
async fn application_source_is_unique_per_application_and_binds_credential_and_agent() {
    let pool = database().await;
    insert_user(&pool, "admin-1", "administrator")
        .await
        .unwrap();
    sqlx::query("INSERT INTO applications (id, name, slug, status) VALUES ('app-1', 'app', 'app', 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status) VALUES ('git-1', 'primary', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:one', X'01', X'02', 1, 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes (id, name, work_root, secrets_root, status) VALUES ('node-1', 'node', '/var/lib/deploy-go-agent/apps', '/var/lib/deploy-go-agent/secrets', 'online')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO agents (id, node_id, environment) VALUES ('agent-1', 'node-1', 'test')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO application_sources (id, application_id, repository_url, git_credential_id, build_agent_id, deployment_branch, status) VALUES ('source-1', 'app-1', 'git@github.com:example/app.git', 'git-1', 'agent-1', 'main', 'verified')")
        .execute(&pool).await.unwrap();
    let duplicate = sqlx::query("INSERT INTO application_sources (id, application_id, repository_url, git_credential_id, build_agent_id, deployment_branch, status) VALUES ('source-2', 'app-1', 'git@github.com:example/other.git', NULL, NULL, 'main', 'draft')")
        .execute(&pool).await;
    assert!(duplicate.is_err());
    let missing_agent = sqlx::query("INSERT INTO application_sources (id, application_id, repository_url, git_credential_id, build_agent_id, deployment_branch, status) VALUES ('source-3', 'app-1', 'git@github.com:example/app.git', 'git-1', 'missing-agent', 'main', 'verified')")
        .execute(&pool).await;
    assert!(missing_agent.is_err());
}

#[tokio::test]
async fn agent_task_kind_requires_unique_matching_execution_fact() {
    let pool = database().await;
    insert_user(&pool, "admin-1", "administrator")
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes (id, name, work_root, secrets_root, status) VALUES ('node-1', 'node', '/var/lib/deploy-go-agent/apps', '/var/lib/deploy-go-agent/secrets', 'online')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO agents (id, node_id, environment) VALUES ('agent-1', 'node-1', 'test')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO applications (id, name, slug, status) VALUES ('app-1', 'app', 'app', 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets (id, application_id, node_id, environment, script_path, timeout_seconds, status) VALUES ('target-1', 'app-1', 'node-1', 'test', '/srv/deploy.sh', 60, 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments (id, target_id, requested_by, status, phase, idempotency_key, request_hash, snapshot_hash) VALUES ('dep-1', 'target-1', 'admin-1', 'running', 'preparing', 'dep-key', 'request', 'snapshot')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_target_runs (id, deployment_id, target_id, node_id, agent_id, status) VALUES ('run-1', 'dep-1', 'target-1', 'node-1', 'agent-1', 'pending')")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-prepare', 'agent-1', 'dep-1', 'prepare', 'deployment_prepare', 'prepare-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    let duplicate = sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-prepare-2', 'agent-1', 'dep-1', 'prepare', 'deployment_prepare', 'prepare-2', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(duplicate.is_err());
    sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, target_run_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-release', 'agent-1', 'dep-1', 'run-1', 'release', 'deployment_release', 'release-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    let duplicate_release = sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, target_run_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-release-2', 'agent-1', 'dep-1', 'run-1', 'release', 'deployment_release', 'release-2', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(duplicate_release.is_err());
    let missing_run = sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-release-no-run', 'agent-1', 'dep-1', 'release', 'deployment_release', 'release-3', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(missing_run.is_err());

    let mismatch = sqlx::query("INSERT INTO agent_tasks (id, agent_id, deployment_id, stage, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-mismatch', 'agent-1', 'dep-1', 'release', 'deployment_prepare', 'mismatch-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(mismatch.is_err());
}

#[tokio::test]
async fn env_sync_task_requires_and_uniquely_binds_a_sync_fact() {
    let pool = database().await;
    insert_user(&pool, "admin-1", "administrator")
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes (id, name, work_root, secrets_root, status) VALUES ('node-1', 'node', '/var/lib/deploy-go-agent/apps', '/var/lib/deploy-go-agent/secrets', 'online')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO agents (id, node_id, environment) VALUES ('agent-1', 'node-1', 'test')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO applications (id, name, slug, status) VALUES ('app-1', 'app', 'app', 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets (id, application_id, node_id, environment, script_path, timeout_seconds, status) VALUES ('target-1', 'app-1', 'node-1', 'test', '/srv/deploy.sh', 60, 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_files (id, application_id, file_name, module, format, current_digest) VALUES ('env-1', 'app-1', 'api.env', 'api', 'dotenv-v1', 'digest')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_versions (id, env_file_id, env_version, algorithm, ciphertext, nonce, key_version, digest) VALUES ('version-1', 'env-1', 1, 'chacha20poly1305-application-env-v1', X'01', X'02', 1, 'digest')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_syncs (id, env_version_id, target_id, node_id, agent_id, status) VALUES ('sync-1', 'version-1', 'target-1', 'node-1', 'agent-1', 'pending')")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO agent_tasks (id, agent_id, env_sync_id, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-sync', 'agent-1', 'sync-1', 'env_sync', 'sync-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    let duplicate = sqlx::query("INSERT INTO agent_tasks (id, agent_id, env_sync_id, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-sync-2', 'agent-1', 'sync-1', 'env_sync', 'sync-2', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(duplicate.is_err());
    let missing_sync = sqlx::query("INSERT INTO agent_tasks (id, agent_id, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-sync-no-fact', 'agent-1', 'env_sync', 'sync-3', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(missing_sync.is_err());
}

#[tokio::test]
async fn artifact_lease_purpose_controls_target_run_binding() {
    let pool = database().await;
    insert_user(&pool, "admin-1", "administrator")
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes (id,name,work_root,secrets_root,status) VALUES ('node-1','node','/srv/apps','/srv/secrets','online')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents (id,node_id,environment) VALUES ('agent-1','node-1','prod')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO applications (id,name,slug,status) VALUES ('app-1','app','app','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO deployment_targets (id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES ('target-1','app-1','node-1','prod','/srv/deploy.sh',60,'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments (id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES ('dep-1','target-1','admin-1','queued','queued','dep-1','request','snapshot')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_artifacts (id,deployment_id,manifest_digest,status,expires_at) VALUES ('artifact-1','dep-1','digest','uploading','2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_target_runs (id,deployment_id,target_id,node_id,agent_id,status) VALUES ('run-1','dep-1','target-1','node-1','agent-1','pending')")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO artifact_leases (id,artifact_id,agent_id,purpose,manifest_digest,status,expires_at) VALUES ('upload-1','artifact-1','agent-1','artifact_upload','digest','active','2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO artifact_leases (id,artifact_id,agent_id,target_run_id,purpose,manifest_digest,status,expires_at) VALUES ('download-1','artifact-1','agent-1','run-1','artifact_download','digest','active','2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    let upload_with_target = sqlx::query("INSERT INTO artifact_leases (id,artifact_id,agent_id,target_run_id,purpose,manifest_digest,status,expires_at) VALUES ('upload-bad','artifact-1','agent-1','run-1','artifact_upload','digest','active','2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(upload_with_target.is_err());
    let download_without_target = sqlx::query("INSERT INTO artifact_leases (id,artifact_id,agent_id,purpose,manifest_digest,status,expires_at) VALUES ('download-bad','artifact-1','agent-1','artifact_download','digest','active','2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(download_without_target.is_err());
}

#[tokio::test]
async fn application_env_objects_versions_and_syncs_are_unique() {
    let pool = database().await;
    sqlx::query("INSERT INTO nodes (id,name,work_root,secrets_root,status) VALUES ('node-1','node','/srv/apps','/srv/secrets','online')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO applications (id,name,slug,status) VALUES ('app-1','app','app','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO deployment_targets (id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES ('target-1','app-1','node-1','prod','/srv/deploy.sh',60,'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_files (id,application_id,file_name,module,format,current_digest) VALUES ('env-1','app-1','api.env','api','dotenv-v1','digest-1')")
        .execute(&pool).await.unwrap();
    let duplicate_file = sqlx::query("INSERT INTO application_env_files (id,application_id,file_name,module,format,current_digest) VALUES ('env-2','app-1','API.ENV','api','dotenv-v1','digest-2')")
        .execute(&pool).await;
    assert!(duplicate_file.is_err());
    sqlx::query("INSERT INTO application_env_versions (id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES ('version-1','env-1',1,'chacha20poly1305-application-env-v1',X'01',X'02',1,'digest-1')")
        .execute(&pool).await.unwrap();
    let duplicate_version = sqlx::query("INSERT INTO application_env_versions (id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES ('version-2','env-1',1,'chacha20poly1305-application-env-v1',X'03',X'04',1,'digest-2')")
        .execute(&pool).await;
    assert!(duplicate_version.is_err());
    sqlx::query("INSERT INTO application_env_syncs (id,env_version_id,target_id,node_id,status) VALUES ('sync-1','version-1','target-1','node-1','pending')")
        .execute(&pool).await.unwrap();
    let duplicate_sync = sqlx::query("INSERT INTO application_env_syncs (id,env_version_id,target_id,node_id,status) VALUES ('sync-2','version-1','target-1','node-1','pending')")
        .execute(&pool).await;
    assert!(duplicate_sync.is_err());
}

#[tokio::test]
async fn git_secret_lease_binds_task_and_credential_and_enforces_status() {
    let pool = database().await;
    insert_user(&pool, "admin-1", "administrator")
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes (id, name, work_root, secrets_root, status) VALUES ('node-1', 'node', '/var/lib/deploy-go-agent/apps', '/var/lib/deploy-go-agent/secrets', 'online')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO agents (id, node_id, environment) VALUES ('agent-1', 'node-1', 'test')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status) VALUES ('git-1', 'primary', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:one', X'01', X'02', 1, 'active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_tasks (id, agent_id, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task-1', 'agent-1', 'git_refs_query', 'refs-1', 'digest', '{}', 'queued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO git_secret_leases (id, task_id, git_credential_id, payload_digest, purpose, status, expires_at) VALUES ('lease-1', 'task-1', 'git-1', 'digest', 'git_credential', 'issued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await.unwrap();

    let bad_status = sqlx::query("INSERT INTO git_secret_leases (id, task_id, git_credential_id, payload_digest, purpose, status, expires_at) VALUES ('lease-2', 'task-1', 'git-1', 'digest', 'git_credential', 'consumed', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(bad_status.is_err());
    let missing_task = sqlx::query("INSERT INTO git_secret_leases (id, task_id, git_credential_id, payload_digest, purpose, status, expires_at) VALUES ('lease-3', 'missing-task', 'git-1', 'digest', 'git_credential', 'issued', '2099-01-01T00:00:00Z')")
        .execute(&pool).await;
    assert!(missing_task.is_err());
    let delete_credential = sqlx::query("DELETE FROM git_credentials WHERE id='git-1'")
        .execute(&pool)
        .await;
    assert!(delete_credential.is_err());
}
