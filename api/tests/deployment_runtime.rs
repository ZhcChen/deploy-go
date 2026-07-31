mod common;

use async_trait::async_trait;
use deploy_go_api::{
    AppState,
    crypto::MasterKeyRing,
    db,
    deployments::process_one,
    executor::{
        deployment::{DeploymentExecutor, ExecutionContext, ExecutionResult, OutputChunk},
        ssh::ProbeError,
    },
};
use serde_json::json;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone)]
struct MockExecutor {
    result: ExecutionResult,
}

#[async_trait]
impl DeploymentExecutor for MockExecutor {
    async fn execute(
        &self,
        context: &ExecutionContext,
        output: tokio::sync::mpsc::Sender<OutputChunk>,
    ) -> Result<i32, ProbeError> {
        assert_eq!(context.argument_tokens, ["--release-version", "1.0.0"]);
        for chunk in self.result.chunks.clone() {
            output.send(chunk).await.unwrap();
        }
        Ok(self.result.exit_code)
    }

    async fn cancel(&self, _context: &ExecutionContext) -> Result<(), ProbeError> {
        Ok(())
    }
}

async fn fixture(result: ExecutionResult) -> (AppState, SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let ring = MasterKeyRing::from_raw(1, [7; 32], None).unwrap();
    let encrypted = ring
        .encrypt("cred_runtime", "ed25519", b"PRIVATE KEY")
        .unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_runtime','Runtime','runtime','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO ssh_credentials(id,name,algorithm,public_key,fingerprint,encrypted_private_key,nonce,key_version) VALUES('cred_runtime','Runtime Key','ed25519','ssh-ed25519 AAAA','SHA256:runtime',?,?,?)")
        .bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,host,port,username,ssh_credential_id,work_root,secrets_root,status,trusted_host_key) VALUES('node_runtime','Runtime Node','fixture.invalid',22,'deploy','cred_runtime','/srv/apps','/srv/secrets','online','fixture.invalid ssh-ed25519 AAAA')").execute(&pool).await.unwrap();
    let schema = json!({"type":"object","properties":{"release-version":{"type":"string"}},"required":["release-version"],"additionalProperties":false});
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,parameter_schema,timeout_seconds,verification_config,status) VALUES('target_runtime','app_runtime','node_runtime','test','/srv/apps/deploy.sh',?,60,'{}','active')").bind(schema.to_string()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO secret_file_references(id,deployment_target_id,environment_key,file_path) VALUES('secret_runtime','target_runtime','TOKEN_FILE','/srv/secrets/token')").execute(&pool).await.unwrap();
    let snapshot = json!({"target":{"application_id":"app_runtime","node_id":"node_runtime","environment":"test","script_path":"/srv/apps/deploy.sh","parameter_schema":schema,"timeout_seconds":60,"verification_config":{},"secret_file_references":[{"environment_key":"TOKEN_FILE","file_path":"/srv/secrets/token"}],"version":1},"parameters":{"release-version":"1.0.0"}});
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment_runtime','target_runtime','admin','queued','queued','runtime-request-0001','hash','snapshot',?)").bind(snapshot.to_string()).execute(&pool).await.unwrap();
    let state = AppState::new(pool.clone())
        .with_master_key_ring(ring)
        .with_deployment_executor(MockExecutor { result });
    (state, pool)
}

#[tokio::test]
async fn execution_persists_redacted_logs_events_and_complete_success() {
    let event = r#"DEPLOY_EVENT {"schema_version":1,"event":"deploy.finished","timestamp":"2026-07-31T00:00:00Z","status":"succeeded"}"#;
    let result = ExecutionResult {
        chunks: vec![
            OutputChunk {
                stream: "stdout",
                bytes: format!("using /srv/secrets/token\n{event}\n").into_bytes(),
            },
            OutputChunk {
                stream: "stderr",
                bytes: vec![b'b', b'a', b'd', 0xff, b'\n'],
            },
        ],
        exit_code: 0,
    };
    let (state, pool) = fixture(result).await;
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some("deployment_runtime")
    );
    let deployment: (String, bool) = sqlx::query_as(
        "SELECT status,protocol_complete FROM deployments WHERE id='deployment_runtime'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(deployment, ("succeeded".to_owned(), true));
    let logs: Vec<String> =
        sqlx::query_scalar("SELECT content FROM deployment_logs ORDER BY sequence")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(logs.iter().any(|line| line.contains("[REDACTED]")));
    assert!(logs.iter().all(|line| !line.contains("/srv/secrets/token")));
    let diagnostics: Vec<String> = sqlx::query_scalar(
        "SELECT diagnostic_code FROM deployment_events WHERE diagnostic_code IS NOT NULL",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(diagnostics, ["invalid_utf8"]);
}

#[tokio::test]
async fn nonzero_exit_fails_and_zero_without_finished_is_incomplete() {
    let (failed, pool) = fixture(ExecutionResult {
        chunks: vec![],
        exit_code: 12,
    })
    .await;
    process_one(&failed).await.unwrap();
    let status: String =
        sqlx::query_scalar("SELECT status FROM deployments WHERE id='deployment_runtime'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");

    let (incomplete, pool) = fixture(ExecutionResult {
        chunks: vec![OutputChunk {
            stream: "stdout",
            bytes: b"done\n".to_vec(),
        }],
        exit_code: 0,
    })
    .await;
    process_one(&incomplete).await.unwrap();
    let row: (String, bool) = sqlx::query_as(
        "SELECT status,protocol_complete FROM deployments WHERE id='deployment_runtime'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row, ("succeeded".to_owned(), false));
}

#[tokio::test]
async fn active_target_is_not_claimed_twice() {
    let (state, pool) = fixture(ExecutionResult {
        chunks: vec![],
        exit_code: 0,
    })
    .await;
    sqlx::query("UPDATE deployments SET status='running' WHERE id='deployment_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) SELECT 'deployment_second',target_id,requested_by,'queued','queued','runtime-request-0002','hash2',snapshot_hash,snapshot_json FROM deployments WHERE id='deployment_runtime'").execute(&pool).await.unwrap();
    assert_eq!(process_one(&state).await.unwrap(), None);
}

#[derive(Clone)]
struct PausedExecutor {
    emitted: Arc<Notify>,
    release: Arc<Notify>,
    chunks: Vec<OutputChunk>,
    exit_code: i32,
}

#[async_trait]
impl DeploymentExecutor for PausedExecutor {
    async fn execute(
        &self,
        _context: &ExecutionContext,
        output: tokio::sync::mpsc::Sender<OutputChunk>,
    ) -> Result<i32, ProbeError> {
        for chunk in self.chunks.clone() {
            output.send(chunk).await.unwrap();
        }
        self.emitted.notify_one();
        self.release.notified().await;
        Ok(self.exit_code)
    }

    async fn cancel(&self, _context: &ExecutionContext) -> Result<(), ProbeError> {
        Ok(())
    }
}

#[tokio::test]
async fn output_is_committed_before_execution_finishes_and_terminal_state_is_monotonic() {
    let (base, pool) = fixture(ExecutionResult {
        chunks: vec![],
        exit_code: 0,
    })
    .await;
    let emitted = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let state = base.with_deployment_executor(PausedExecutor {
        emitted: emitted.clone(),
        release: release.clone(),
        chunks: vec![OutputChunk {
            stream: "stdout",
            bytes: b"visible now\n".to_vec(),
        }],
        exit_code: 0,
    });
    let worker_state = state.clone();
    let worker = tokio::spawn(async move { process_one(&worker_state).await.unwrap() });
    emitted.notified().await;
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM deployment_logs WHERE deployment_id='deployment_runtime'",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            if count == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    sqlx::query("UPDATE deployments SET status='interrupted',phase='interrupted' WHERE id='deployment_runtime'").execute(&pool).await.unwrap();
    release.notify_one();
    worker.await.unwrap();
    let status: String =
        sqlx::query_scalar("SELECT status FROM deployments WHERE id='deployment_runtime'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "interrupted");
}

#[tokio::test]
async fn confirmed_running_cancel_accepts_signal_exit_code() {
    let (base, pool) = fixture(ExecutionResult {
        chunks: vec![],
        exit_code: 0,
    })
    .await;
    let emitted = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let state = base.with_deployment_executor(PausedExecutor {
        emitted: emitted.clone(),
        release: release.clone(),
        chunks: vec![],
        exit_code: 130,
    });
    let worker_state = state.clone();
    let worker = tokio::spawn(async move { process_one(&worker_state).await.unwrap() });
    emitted.notified().await;
    sqlx::query(
        "UPDATE deployments SET status='canceling',phase='canceling' WHERE id='deployment_runtime'",
    )
    .execute(&pool)
    .await
    .unwrap();
    release.notify_one();
    worker.await.unwrap();
    let status: String =
        sqlx::query_scalar("SELECT status FROM deployments WHERE id='deployment_runtime'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "canceled");
}

#[derive(Clone)]
struct FailingExecutor(&'static str);

#[async_trait]
impl DeploymentExecutor for FailingExecutor {
    async fn execute(
        &self,
        _: &ExecutionContext,
        _: tokio::sync::mpsc::Sender<OutputChunk>,
    ) -> Result<i32, ProbeError> {
        Err(ProbeError::new(self.0, "fixture failure"))
    }
    async fn cancel(&self, _: &ExecutionContext) -> Result<(), ProbeError> {
        Ok(())
    }
}

#[tokio::test]
async fn uncertain_transport_errors_are_interrupted() {
    for code in ["timeout", "process_io_failed"] {
        let (base, pool) = fixture(ExecutionResult {
            chunks: vec![],
            exit_code: 0,
        })
        .await;
        let state = base.with_deployment_executor(FailingExecutor(code));
        process_one(&state).await.unwrap();
        let status: String =
            sqlx::query_scalar("SELECT status FROM deployments WHERE id='deployment_runtime'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "interrupted");
    }
}

#[tokio::test]
async fn protocol_conflict_uses_failure_side_and_split_utf8_stays_valid() {
    let mut failed = r#"DEPLOY_EVENT {"schema_version":1,"event":"deploy.finished","timestamp":"2026-07-31T00:00:00Z","status":"failed"}"#.as_bytes().to_vec();
    failed.push(b'\n');
    let mut succeeded = r#"DEPLOY_EVENT {"schema_version":1,"event":"deploy.finished","timestamp":"2026-07-31T00:00:01Z","status":"succeeded"}"#.as_bytes().to_vec();
    succeeded.push(b'\n');
    let result = ExecutionResult {
        chunks: vec![
            OutputChunk {
                stream: "stdout",
                bytes: vec![0xc3],
            },
            OutputChunk {
                stream: "stdout",
                bytes: vec![0xa9, b'\n'],
            },
            OutputChunk {
                stream: "stdout",
                bytes: failed,
            },
            OutputChunk {
                stream: "stdout",
                bytes: succeeded,
            },
        ],
        exit_code: 0,
    };
    let (state, pool) = fixture(result).await;
    process_one(&state).await.unwrap();
    let status: String =
        sqlx::query_scalar("SELECT status FROM deployments WHERE id='deployment_runtime'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    let conflicts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_events WHERE diagnostic_code='protocol_conflict'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(conflicts, 1);
    let invalid: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_events WHERE diagnostic_code='invalid_utf8'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(invalid, 0);
}

#[tokio::test]
async fn malformed_unknown_and_output_limits_create_diagnostics() {
    let unknown = r#"DEPLOY_EVENT {"schema_version":1,"event":"deploy.future","timestamp":"2026-07-31T00:00:00Z","status":"running"}"#;
    let result = ExecutionResult {
        chunks: vec![
            OutputChunk {
                stream: "stdout",
                bytes: b"DEPLOY_EVENT not-json\n".to_vec(),
            },
            OutputChunk {
                stream: "stdout",
                bytes: format!("{unknown}\n").into_bytes(),
            },
            OutputChunk {
                stream: "stderr",
                bytes: vec![b'x'; 64 * 1024 + 1],
            },
        ],
        exit_code: 0,
    };
    let (state, pool) = fixture(result).await;
    process_one(&state).await.unwrap();
    let codes:Vec<String>=sqlx::query_scalar("SELECT diagnostic_code FROM deployment_events WHERE diagnostic_code IS NOT NULL ORDER BY diagnostic_code").fetch_all(&pool).await.unwrap();
    assert!(codes.contains(&"malformed_event".to_owned()));
    assert!(codes.contains(&"unknown_event".to_owned()));
    assert!(codes.contains(&"line_truncated".to_owned()));

    let (state, pool) = fixture(ExecutionResult {
        chunks: vec![OutputChunk {
            stream: "stdout",
            bytes: b"0123456789\nmore\n".to_vec(),
        }],
        exit_code: 0,
    })
    .await;
    sqlx::query("INSERT INTO system_settings(key,value_json,version) VALUES('runtime',?,1)").bind(json!({"max_concurrent_deployments":2,"max_log_bytes":5,"log_retention_days":30,"version":1}).to_string()).execute(&pool).await.unwrap();
    process_one(&state).await.unwrap();
    let budget: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_events WHERE diagnostic_code='log_budget_exceeded'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(budget, 1);
}
