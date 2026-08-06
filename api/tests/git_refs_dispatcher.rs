use chrono::{Duration, Utc};
use deploy_go_agent_protocol::{
    GitRefsQueryTask, Message, SecretLeasePurpose, SecretLeaseRequest, TaskLifecycleState,
    TaskPayload, TaskResult, TaskState, TaskTerminalStatus,
};
use deploy_go_api::{
    AppState,
    agents::dispatcher::{
        expire_secret_leases, handle_agent_message, resolve_secret_lease, try_dispatch,
    },
    crypto::MasterKeyRing,
    db,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePoolOptions;

const PRIVATE_KEY: &str =
    "-----BEGIN OPENSSH PRIVATE KEY-----\nfake\n-----END OPENSSH PRIVATE KEY-----";
const REPOSITORY_URL: &str = "git@git.example.test:deploy-go/example.git";

async fn fixture(with_lease: bool) -> (AppState, sqlx::SqlitePool, String) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO nodes (id, name, work_root, secrets_root, status) VALUES ('node_refs', 'refs-node', '/var/lib/deploy-go-agent/apps', '/var/lib/deploy-go-agent/secrets', 'online')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO agents (id, node_id, registered_at, last_seen_at, agent_version, protocol_version, connection_generation) VALUES ('agent_refs', 'node_refs', '2026-08-06T00:00:00Z', '2026-08-06T00:00:00Z', '0.1.0', 2, 1)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO applications (id, name, slug, status) VALUES ('app_refs', 'refs-app', 'refs-app', 'active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_sources (id, application_id, repository_url, build_agent_id, status) VALUES ('source_refs', 'app_refs', ?, 'agent_refs', 'draft')")
        .bind(REPOSITORY_URL)
        .execute(&pool)
        .await
        .unwrap();

    let ring = MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap();
    let encrypted = ring
        .encrypt("git_cred_refs", "ed25519", PRIVATE_KEY.as_bytes())
        .unwrap();
    sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status) VALUES ('git_cred_refs', 'refs-key', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:refs', ?, ?, ?, 'active')")
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce)
        .bind(encrypted.key_version)
        .execute(&pool)
        .await
        .unwrap();

    let payload = TaskPayload::GitRefsQuery(GitRefsQueryTask {
        refs_query_id: "refs_query_001".to_owned(),
        repository_url: REPOSITORY_URL.to_owned(),
        git_credential_lease_id: with_lease.then(|| "lease_001".to_owned()),
        timeout_seconds: 30,
    });
    let payload_json = serde_json::to_string(&payload).unwrap();
    let digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    sqlx::query("INSERT INTO agent_tasks (id, agent_id, kind, idempotency_key, payload_digest, payload_json, status, deadline_at) VALUES ('task_refs', 'agent_refs', 'git_refs_query', 'git-refs:source_refs:refs_query_001', ?, ?, 'running', '2099-01-01T00:00:00Z')")
        .bind(&digest)
        .bind(&payload_json)
        .execute(&pool)
        .await
        .unwrap();
    if with_lease {
        sqlx::query("INSERT INTO git_secret_leases (id, task_id, git_credential_id, payload_digest, purpose, status, expires_at) VALUES ('lease_001', 'task_refs', 'git_cred_refs', ?, 'git_credential', 'issued', ?)")
            .bind(&digest)
            .bind((Utc::now() + Duration::minutes(5)).to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO git_ref_discoveries (id, application_source_id, source_version, task_id, status) VALUES ('refs_query_001', 'source_refs', 1, 'task_refs', 'queued')")
        .execute(&pool)
        .await
        .unwrap();
    (
        AppState::new(pool.clone()).with_master_key_ring(ring),
        pool,
        digest,
    )
}

#[tokio::test]
async fn refs_result_persists_sanitized_discovery_and_expires_lease() {
    let (state, pool, _) = fixture(true).await;
    handle_agent_message(
        &state,
        "agent_refs",
        1,
        &Message::TaskState(TaskState {
            task_id: "task_refs".to_owned(),
            sequence: 1,
            state: TaskLifecycleState::Accepted,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_refs",
        1,
        &Message::TaskState(TaskState {
            task_id: "task_refs".to_owned(),
            sequence: 2,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_refs",
        1,
        &Message::TaskResult(TaskResult {
            task_id: "task_refs".to_owned(),
            sequence: 3,
            status: TaskTerminalStatus::Succeeded,
            exit_code: Some(0),
            error_code: None,
            summary: Some("分支发现完成".to_owned()),
            data: Some(json!({
                "refs": [
                    {"name":"main","ref":"refs/heads/main","sha":"0123456789abcdef0123456789abcdef01234567"},
                    {"name":"develop","ref":"refs/heads/develop","sha":"1123456789abcdef0123456789abcdef01234567"},
                    {"name":"tag-only","ref":"refs/tags/v1","sha":"2223456789abcdef0123456789abcdef01234567"},
                    {"name":"bad-sha","ref":"refs/heads/bad-sha","sha":"not-a-sha"}
                ]
            })),
        }),
    )
    .await
    .unwrap();

    let (status, refs_json, expires_at, finished_at): (String, String, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT status, refs_json, expires_at, finished_at FROM git_ref_discoveries WHERE id='refs_query_001'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "succeeded");
    assert!(expires_at.is_some());
    assert!(finished_at.is_some());
    let refs: serde_json::Value = serde_json::from_str(&refs_json).unwrap();
    let names = refs
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, ["main", "develop"]);
    let lease_status: String =
        sqlx::query_scalar("SELECT status FROM git_secret_leases WHERE id='lease_001'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(lease_status, "expired");
}

#[tokio::test]
async fn refs_failure_records_distinct_error() {
    let (state, pool, _) = fixture(false).await;
    handle_agent_message(
        &state,
        "agent_refs",
        1,
        &Message::TaskState(TaskState {
            task_id: "task_refs".to_owned(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_refs",
        1,
        &Message::TaskResult(TaskResult {
            task_id: "task_refs".to_owned(),
            sequence: 2,
            status: TaskTerminalStatus::Failed,
            exit_code: Some(1),
            error_code: Some("git_command_failed".to_owned()),
            summary: Some("仓库不可达".to_owned()),
            data: None,
        }),
    )
    .await
    .unwrap();

    let (status, error_code): (String, Option<String>) = sqlx::query_as(
        "SELECT status, error_code FROM git_ref_discoveries WHERE id='refs_query_001'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "failed");
    assert_eq!(error_code.as_deref(), Some("git_repository_unreachable"));
}

#[tokio::test]
async fn secret_lease_is_granted_once_and_cannot_be_reused() {
    let (state, pool, digest) = fixture(true).await;
    let request = SecretLeaseRequest {
        task_id: "task_refs".to_owned(),
        lease_id: "lease_001".to_owned(),
        payload_digest: digest,
        purpose: SecretLeasePurpose::GitCredential,
    };
    let first = resolve_secret_lease(&state, "agent_refs", &request)
        .await
        .unwrap();
    assert_eq!(first.error_code, None);
    assert_eq!(first.private_key, PRIVATE_KEY);
    assert!(
        first
            .expires_at
            .parse::<chrono::DateTime<chrono::Utc>>()
            .is_ok()
    );
    let status: String =
        sqlx::query_scalar("SELECT status FROM git_secret_leases WHERE id='lease_001'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "granted");

    let second = resolve_secret_lease(&state, "agent_refs", &request)
        .await
        .unwrap();
    assert_eq!(second.error_code.as_deref(), Some("lease_not_available"));
    assert!(second.private_key.is_empty());
}

#[tokio::test]
async fn secret_lease_rejects_wrong_agent_digest_and_expired_leases() {
    let (state, pool, digest) = fixture(true).await;
    let request = SecretLeaseRequest {
        task_id: "task_refs".to_owned(),
        lease_id: "lease_001".to_owned(),
        payload_digest: digest,
        purpose: SecretLeasePurpose::GitCredential,
    };
    let wrong_agent = resolve_secret_lease(&state, "agent_other", &request)
        .await
        .unwrap();
    assert_eq!(wrong_agent.error_code.as_deref(), Some("task_not_active"));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM git_secret_leases WHERE id='lease_001'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "issued"
    );

    let (state, pool, _digest) = fixture(true).await;
    let request = SecretLeaseRequest {
        task_id: "task_refs".to_owned(),
        lease_id: "lease_001".to_owned(),
        payload_digest: format!("sha256:{}", "0".repeat(64)),
        purpose: SecretLeasePurpose::GitCredential,
    };
    let wrong_digest = resolve_secret_lease(&state, "agent_refs", &request)
        .await
        .unwrap();
    assert_eq!(wrong_digest.error_code.as_deref(), Some("payload_mismatch"));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM git_secret_leases WHERE id='lease_001'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "issued"
    );

    let (state, pool, digest) = fixture(true).await;
    sqlx::query(
        "UPDATE git_secret_leases SET expires_at='2026-08-01T00:00:00Z' WHERE id='lease_001'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let request = SecretLeaseRequest {
        task_id: "task_refs".to_owned(),
        lease_id: "lease_001".to_owned(),
        payload_digest: digest,
        purpose: SecretLeasePurpose::GitCredential,
    };
    let expired = resolve_secret_lease(&state, "agent_refs", &request)
        .await
        .unwrap();
    assert_eq!(expired.error_code.as_deref(), Some("lease_not_available"));
}

#[tokio::test]
async fn stale_issued_leases_are_expired_by_worker_cleanup() {
    let (state, pool, _) = fixture(true).await;
    sqlx::query(
        "UPDATE git_secret_leases SET expires_at='2026-08-01T00:00:00Z' WHERE id='lease_001'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(expire_secret_leases(&state).await.unwrap(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM git_secret_leases WHERE id='lease_001'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "expired"
    );
}

#[tokio::test]
async fn queued_refs_task_waits_for_agent_connection() {
    let (state, pool, _) = fixture(true).await;
    sqlx::query("UPDATE agent_tasks SET status='queued',finished_at=NULL WHERE id='task_refs'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!try_dispatch(&state, "task_refs").await.unwrap());
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_tasks WHERE id='task_refs'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        "queued"
    );
}
