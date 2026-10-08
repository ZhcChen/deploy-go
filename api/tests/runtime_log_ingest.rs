mod common;
use axum::{Router, http::StatusCode};
use common::{json_request, response_json};
use deploy_go_api::{AppState, agents::auth::token_hash, app, db, runtime_logs::RuntimeLogStore};
use deploy_go_runtime_log::{LogPolicy, LogRecord, read_page};
use futures_util::StreamExt;
use serde_json::{Value, json};
use sqlx::{
    Connection, SqliteConnection, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{collections::BTreeMap, os::unix::fs::PermissionsExt, path::PathBuf};

const EPOCH: &str = "01K70000000000000000000000";
const NEXT_EPOCH: &str = "01K70000000000000000000001";
fn record(sequence: u64) -> LogRecord {
    LogRecord {
        sequence,
        timestamp: "2026-10-08T00:00:00Z".into(),
        level: "INFO".into(),
        target: "deploy_go_agent".into(),
        message: "SECRET_MESSAGE https://secret.invalid".into(),
        request_id: Some("req_fixture".into()),
        fields: BTreeMap::from([
            ("status".into(), "running".into()),
            ("token".into(), "SECRET_TOKEN".into()),
            ("error".into(), "SECRET_ERROR".into()),
            ("node_id".into(), "spoofed_node".into()),
        ]),
    }
}
fn batch(start: u64, end: u64, after: u64, minimum: u64, epoch: &str) -> Value {
    json!({"component":"agent","epoch":epoch,"after":after,"minimum":minimum,"evicted_bytes":0,
        "entries":(start..=end).map(record).collect::<Vec<_>>()})
}
async fn seed(pool: &SqlitePool, id: &str) {
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES(?,?,'online')")
        .bind(format!("node_{id}"))
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version) VALUES(?,?,18)")
        .bind(id)
        .bind(format!("node_{id}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_credential_families(id,agent_id) VALUES(?,?)")
        .bind(format!("family_{id}"))
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_refresh_credentials(id,family_id,generation,token_hash,expires_at,token_key_version) VALUES(?,?,1,?,'2099-01-01T00:00:00Z',1)")
        .bind(format!("refresh_{id}")).bind(format!("family_{id}")).bind(token_hash("refresh",id)).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO agent_access_sessions(id,agent_id,family_id,refresh_credential_id,token_hash,expires_at,token_key_version) VALUES(?,?,?,?,?,'2099-01-01T00:00:00Z',1)")
        .bind(format!("access_{id}")).bind(id).bind(format!("family_{id}")).bind(format!("refresh_{id}")).bind(token_hash("access",id)).execute(pool).await.unwrap();
}
async fn fixture(
    policy: LogPolicy,
) -> (
    Router,
    SqlitePool,
    RuntimeLogStore,
    tempfile::TempDir,
    PathBuf,
) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    seed(&pool, "agent_one").await;
    seed(&pool, "agent_two").await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let (store, _layer) = RuntimeLogStore::open(root.clone(), policy).unwrap();
    store.recover(&pool).await.unwrap();
    let router = app(AppState::with_runtime_logs(pool.clone(), store.clone()));
    (router, pool, store, temp, root)
}
async fn post(router: Router, body: Value, token: &str) -> axum::response::Response {
    json_request(
        router,
        "POST",
        "/api/v1/agent/runtime-logs",
        body,
        &[("authorization", &format!("Bearer {token}"))],
    )
    .await
}

#[tokio::test]
async fn bearer_identity_safe_projection_and_latest_digest_conflict() {
    let (router, pool, _store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    assert_eq!(
        post(router.clone(), batch(1, 2, 0, 1, EPOCH), "invalid")
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = post(router.clone(), batch(1, 2, 0, 1, EPOCH), "agent_one").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"epoch":EPOCH,"acknowledged_sequence":2})
    );
    let page = read_page(&root, 0, 64, 65536).unwrap();
    assert_eq!(page.entries.len(), 2);
    let text = serde_json::to_string(&page.entries).unwrap();
    assert!(!text.contains("SECRET"));
    assert!(!text.contains("spoofed"));
    assert_eq!(page.entries[0].fields["node_id"], "node_agent_one");
    assert_eq!(page.entries[0].fields["agent_id"], "agent_one");
    assert_eq!(
        post(router.clone(), batch(1, 2, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(read_page(&root, 0, 64, 65536).unwrap().entries.len(), 2);
    let mut conflict = batch(1, 2, 0, 1, EPOCH);
    conflict["entries"][1]["message"] = "different".into();
    assert_eq!(
        post(router.clone(), conflict, "agent_one").await.status(),
        StatusCode::CONFLICT
    );
    let mut spoofed = batch(3, 3, 2, 1, EPOCH);
    spoofed["node_id"] = "node_agent_two".into();
    assert_eq!(
        post(router.clone(), spoofed, "agent_one").await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(router.clone(), batch(1, 1, 0, 1, EPOCH), "agent_two")
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM runtime_log_source_watermarks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    sqlx::query(
        "UPDATE agent_credential_families SET revoked_at='2026-01-01' WHERE agent_id='agent_one'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        post(router, batch(3, 3, 2, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn explicit_source_gap_epoch_switch_and_non_decreasing_watermark() {
    let (router, pool, _store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    assert_eq!(
        post(router.clone(), batch(5, 6, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::OK
    );
    let entry = read_page(&root, 0, 64, 65536).unwrap().entries.remove(0);
    assert_eq!(entry.fields["source_gap_from"], 1);
    assert_eq!(entry.fields["source_gap_to"], 4);
    assert_eq!(entry.fields["source_gap_count"], 4);
    let response = post(router.clone(), batch(5, 5, 0, 5, EPOCH), "agent_one").await;
    assert_eq!(response_json(response).await["acknowledged_sequence"], 6);
    assert_eq!(
        post(router.clone(), batch(1, 1, 1, 1, NEXT_EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(router.clone(), batch(1, 1, 0, 1, NEXT_EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        post(router, batch(7, 7, 6, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let watermark: (String, i64) =
        sqlx::query_as("SELECT epoch,sequence FROM runtime_log_source_watermarks")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(watermark, (NEXT_EPOCH.into(), 1));
}

#[tokio::test]
async fn internal_source_gaps_are_recorded_without_cross_identity_changes() {
    let (router, pool, _store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    let mut body = batch(1, 1, 0, 1, EPOCH);
    body["entries"] = json!([record(1), record(4), record(7)]);
    let response = post(router.clone(), body, "agent_one").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["acknowledged_sequence"], 7);
    let page = read_page(&root, 0, 64, 65536).unwrap();
    assert_eq!(page.entries[1].fields["source_gap_from"], 2);
    assert_eq!(page.entries[1].fields["source_gap_to"], 3);
    assert_eq!(page.entries[2].fields["source_gap_from"], 5);
    assert_eq!(page.entries[2].fields["source_gap_to"], 6);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM runtime_log_source_watermarks WHERE agent_id='agent_two'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let mut backwards = batch(8, 8, 7, 1, EPOCH);
    backwards["entries"] = json!([record(9), record(8)]);
    assert_eq!(
        post(router, backwards, "agent_one").await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn lost_sender_cursor_fast_forwards_to_existing_durable_watermark() {
    let (router, pool, _store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    for start in (1..=1000).step_by(64) {
        let end = (start + 63).min(1000);
        let response = post(
            router.clone(),
            batch(start, end, start - 1, 1, EPOCH),
            "agent_one",
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    let maximum = read_page(&root, 0, 64, 65536).unwrap().maximum;
    let response = post(router, batch(1, 64, 0, 1, EPOCH), "agent_one").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"epoch":EPOCH,"acknowledged_sequence":1000})
    );
    assert_eq!(read_page(&root, 0, 64, 65536).unwrap().maximum, maximum);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT sequence FROM runtime_log_source_watermarks WHERE agent_id='agent_one'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1000
    );
}

#[tokio::test]
async fn recovery_upserts_latest_once_per_source_and_failure_can_fall_back_without_ack() {
    let (router, pool, store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    post(router.clone(), batch(1, 64, 0, 1, EPOCH), "agent_one").await;
    post(router.clone(), batch(65, 128, 64, 1, EPOCH), "agent_one").await;
    post(router.clone(), batch(1, 64, 0, 1, EPOCH), "agent_two").await;
    sqlx::query("CREATE TABLE recovery_writes(attempt INTEGER)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER count_recovery BEFORE INSERT ON runtime_log_source_watermarks BEGIN INSERT INTO recovery_writes VALUES(1); END").execute(&pool).await.unwrap();
    drop(router);
    drop(store);
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let (reopened, _layer) = RuntimeLogStore::open(root.clone(), LogPolicy::CENTRAL).unwrap();
    reopened.recover(&pool).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM recovery_writes")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    std::fs::write(root.join("unexpected"), b"fixture").unwrap();
    // 模拟持久恢复不可用，内存回退不影响健康，但禁止无持久 ACK。
    reopened.fallback_to_memory().await;
    let router = app(AppState::with_runtime_logs(pool.clone(), reopened));
    assert_eq!(
        json_request(router.clone(), "GET", "/healthz", json!({}), &[])
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        json_request(router.clone(), "GET", "/readyz", json!({}), &[])
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        post(router, batch(129, 129, 128, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn rotated_sources_survive_restart_and_retransmission() {
    let policy = LogPolicy {
        segment_bytes: 2048,
        total_bytes: 8192,
    };
    let (router, pool, store, _temp, root) = fixture(policy).await;
    assert_eq!(
        post(router.clone(), batch(1, 1, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::OK
    );
    for sequence in 1..=30 {
        assert_eq!(
            post(
                router.clone(),
                batch(sequence, sequence, sequence - 1, 1, EPOCH),
                "agent_two"
            )
            .await
            .status(),
            StatusCode::OK
        );
    }
    assert!(
        !read_page(&root, 0, 500, 65536)
            .unwrap()
            .entries
            .iter()
            .any(|v| v.fields["agent_id"] == "agent_one")
    );
    drop(router);
    drop(store);
    tokio::task::yield_now().await;
    let (reopened, _layer) = RuntimeLogStore::open(root.clone(), policy).unwrap();
    reopened.recover(&pool).await.unwrap();
    let router = app(AppState::with_runtime_logs(pool.clone(), reopened));
    let before = read_page(&root, 0, 500, 65536).unwrap().maximum;
    assert_eq!(
        post(router, batch(1, 1, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(read_page(&root, 0, 500, 65536).unwrap().maximum, before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT sequence FROM runtime_log_source_watermarks WHERE agent_id='agent_one'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}

#[tokio::test]
async fn durable_append_before_database_commit_is_recovered_before_rotation() {
    let (router, pool, store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    sqlx::query("CREATE TRIGGER fail_watermark BEFORE INSERT ON runtime_log_source_watermarks BEGIN SELECT RAISE(FAIL,'fixture crash window'); END").execute(&pool).await.unwrap();
    assert_eq!(
        post(router.clone(), batch(1, 1, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(read_page(&root, 0, 64, 65536).unwrap().entries.len(), 1);
    assert_eq!(
        post(router.clone(), batch(1, 1, 0, 1, EPOCH), "agent_two")
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(read_page(&root, 0, 64, 65536).unwrap().entries.len(), 1);
    sqlx::query("DROP TRIGGER fail_watermark")
        .execute(&pool)
        .await
        .unwrap();
    drop(router);
    drop(store);
    tokio::task::yield_now().await;
    let (reopened, _layer) = RuntimeLogStore::open(root.clone(), LogPolicy::CENTRAL).unwrap();
    reopened.recover(&pool).await.unwrap();
    let router = app(AppState::with_runtime_logs(pool.clone(), reopened));
    assert_eq!(
        post(router, batch(1, 1, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(read_page(&root, 0, 64, 65536).unwrap().entries.len(), 1);
}

#[tokio::test]
async fn cancelled_request_after_fsync_recovers_before_competing_rotation() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("logs");
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let options = SqliteConnectOptions::new()
        .filename(base.join("fixture.sqlite"))
        .create_if_missing(true)
        .shared_cache(false)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(30));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    seed(&pool, "agent_one").await;
    seed(&pool, "agent_two").await;
    let policy = LogPolicy {
        segment_bytes: 2048,
        total_bytes: 8192,
    };
    let (store, _layer) = RuntimeLogStore::open(root.clone(), policy).unwrap();
    store.recover(&pool).await.unwrap();
    let router = app(AppState::with_runtime_logs(pool.clone(), store));

    // SQLite worker 可能在请求取消后完成已排队语句；拒绝 trigger 保证晚到写入也不提交。
    sqlx::query("CREATE TRIGGER cancel_watermark BEFORE INSERT ON runtime_log_source_watermarks BEGIN SELECT RAISE(FAIL,'fixture cancelled watermark'); END")
        .execute(&pool).await.unwrap();
    let mut blocker = SqliteConnection::connect_with(&options).await.unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut blocker)
        .await
        .unwrap();
    let request_router = router.clone();
    let request =
        tokio::spawn(
            async move { post(request_router, batch(1, 1, 0, 1, EPOCH), "agent_one").await },
        );
    let observed_root = root.clone();
    let persisted = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let path = observed_root.clone();
            let page = tokio::task::spawn_blocking(move || read_page(&path, 0, 64, 65536))
                .await
                .unwrap();
            if let Ok(page) = page {
                if page.entries.len() == 1 {
                    break page;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("日志应在水位写锁释放前完成 fsync");
    assert_eq!(persisted.entries[0].fields["source_sequence"], 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM runtime_log_source_watermarks")
            .fetch_one(&mut blocker)
            .await
            .unwrap(),
        0
    );
    assert!(!request.is_finished(), "请求仍在持久写入与水位提交之间");
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    sqlx::query("ROLLBACK").execute(&mut blocker).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM runtime_log_source_watermarks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );

    // 真实取消已释放存储锁；未恢复水位前，另一来源也不得追加并轮转遗留记录。
    assert_eq!(
        post(router.clone(), batch(1, 30, 0, 1, EPOCH), "agent_two")
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        read_page(&root, 0, 500, 65536).unwrap().maximum,
        persisted.maximum
    );
    sqlx::query("DROP TRIGGER cancel_watermark")
        .execute(&pool)
        .await
        .unwrap();
    let response = post(router.clone(), batch(1, 1, 0, 1, EPOCH), "agent_one").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"epoch":EPOCH,"acknowledged_sequence":1})
    );
    assert_eq!(
        read_page(&root, 0, 500, 65536).unwrap().maximum,
        persisted.maximum
    );
    let watermark: (i64, i64) = sqlx::query_as("SELECT sequence,central_sequence FROM runtime_log_source_watermarks WHERE agent_id='agent_one'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(watermark, (1, persisted.maximum as i64));
    assert_eq!(
        post(router.clone(), batch(1, 30, 0, 1, EPOCH), "agent_two")
            .await
            .status(),
        StatusCode::OK
    );
    let rotated = read_page(&root, 0, 500, 65536).unwrap();
    assert!(
        !rotated
            .entries
            .iter()
            .any(|entry| entry.fields["agent_id"] == "agent_one")
    );
    let response = post(router, batch(1, 1, 0, 1, EPOCH), "agent_one").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["acknowledged_sequence"], 1);
    assert_eq!(
        read_page(&root, 0, 500, 65536).unwrap().maximum,
        rotated.maximum
    );
}

#[tokio::test]
async fn batch_limits_and_concurrent_duplicate_requests() {
    let (router, _pool, _store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    assert_eq!(
        post(router.clone(), batch(1, 65, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut huge = batch(1, 1, 0, 1, EPOCH);
    huge["entries"][0]["message"] = "x".repeat(16384).into();
    assert_eq!(
        post(router.clone(), huge, "agent_one").await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut large = batch(1, 64, 0, 1, EPOCH);
    for entry in large["entries"].as_array_mut().unwrap() {
        entry["message"] = "x".repeat(9000).into();
    }
    assert_eq!(
        post(router.clone(), large, "agent_one").await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (one, two) = tokio::join!(
        post(router.clone(), batch(1, 64, 0, 1, EPOCH), "agent_one"),
        post(router, batch(1, 64, 0, 1, EPOCH), "agent_one")
    );
    assert_eq!(one.status(), StatusCode::OK);
    assert_eq!(two.status(), StatusCode::OK);
    assert_eq!(read_page(&root, 0, 500, 65536).unwrap().entries.len(), 64);
}

#[tokio::test]
async fn storage_refusal_never_acknowledges_and_schemas_include_settings_dto() {
    let (router, pool, _store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    std::fs::write(root.join("unexpected"), b"fixture").unwrap();
    assert_eq!(
        post(router, batch(1, 1, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM runtime_log_source_watermarks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let schema = deploy_go_api::openapi_document();
    assert!(schema["components"]["schemas"]["RuntimeSettingsUpdate"].is_object());
    assert!(schema["paths"]["/api/v1/agent/runtime-logs"]["post"].is_object());
    assert_eq!(
        schema["paths"]["/api/v1/agent/runtime-logs"]["post"]["security"],
        json!([{ "agentBearerAuth":[] }])
    );
}

#[tokio::test]
async fn admin_sse_filters_sources_resumes_and_revokes_live_access() {
    let (router, pool, _store, _temp, _root) = fixture(LogPolicy::CENTRAL).await;
    assert_eq!(
        json_request(
            router.clone(),
            "GET",
            "/api/v1/runtime-logs",
            json!({}),
            &[]
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let (cookie, _) = common::admin_session(router.clone()).await;
    post(router.clone(), batch(1, 2, 0, 1, EPOCH), "agent_one").await;
    post(router.clone(), batch(1, 1, 0, 1, EPOCH), "agent_two").await;
    let response = json_request(
        router.clone(),
        "GET",
        "/api/v1/runtime-logs?node_id=node_agent_one&component=agent&after=1",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut events = response.into_body().into_data_stream();
    let data = events.next().await.unwrap().unwrap();
    let text = String::from_utf8(data.to_vec()).unwrap();
    assert!(text.contains("event: log"));
    assert!(text.contains("id: 2"));
    assert!(text.contains("node_agent_one"));
    assert!(!text.contains("node_agent_two"));
    let stats = events.next().await.unwrap().unwrap();
    assert!(
        String::from_utf8(stats.to_vec())
            .unwrap()
            .contains("event: stats")
    );
    sqlx::query("UPDATE sessions SET revoked_at='2026-01-01'")
        .execute(&pool)
        .await
        .unwrap();
    let revoked = tokio::time::timeout(std::time::Duration::from_secs(2), events.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        String::from_utf8(revoked.to_vec())
            .unwrap()
            .contains("authorization-revoked")
    );
    let invalid = json_request(
        router,
        "GET",
        "/api/v1/runtime-logs?after=99",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn slow_sse_reader_observes_retention_gap_and_filtered_cursor_advances() {
    let policy = LogPolicy {
        segment_bytes: 2048,
        total_bytes: 8192,
    };
    let (router, _pool, _store, _temp, root) = fixture(policy).await;
    let (cookie, _) = common::admin_session(router.clone()).await;
    for sequence in 1..=2 {
        post(
            router.clone(),
            batch(sequence, sequence, sequence - 1, 1, EPOCH),
            "agent_one",
        )
        .await;
    }
    let response = json_request(
        router.clone(),
        "GET",
        "/api/v1/runtime-logs?component=agent&after=1",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let mut events = response.into_body().into_data_stream();
    let first = events.next().await.unwrap().unwrap();
    assert!(String::from_utf8(first.to_vec()).unwrap().contains("id: 2"));
    let stats = events.next().await.unwrap().unwrap();
    assert!(
        String::from_utf8(stats.to_vec())
            .unwrap()
            .contains("event: stats")
    );
    // 消费者暂停期间轮转；下一次读取明确展示缺口，不依赖内存缓存回查。
    for sequence in 3..=30 {
        post(
            router.clone(),
            batch(sequence, sequence, sequence - 1, 1, EPOCH),
            "agent_one",
        )
        .await;
    }
    assert!(read_page(&root, 0, 500, 65536).unwrap().minimum.unwrap() > 3);
    let gap = events.next().await.unwrap().unwrap();
    assert!(
        String::from_utf8(gap.to_vec())
            .unwrap()
            .contains("event: gap")
    );
    let log = events.next().await.unwrap().unwrap();
    assert!(
        String::from_utf8(log.to_vec())
            .unwrap()
            .contains("event: log")
    );
    let response = json_request(
        router,
        "GET",
        "/api/v1/runtime-logs?node_id=absent&after=1",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let mut filtered = response.into_body().into_data_stream();
    let gap = filtered.next().await.unwrap().unwrap();
    assert!(
        String::from_utf8(gap.to_vec())
            .unwrap()
            .contains("event: gap")
    );
    let stats = filtered.next().await.unwrap().unwrap();
    assert!(
        String::from_utf8(stats.to_vec())
            .unwrap()
            .contains("event: stats")
    );
}

#[tokio::test]
async fn central_metadata_sequence_gap_is_reported_without_resetting_source_cursor() {
    let (router, pool, store, _temp, root) = fixture(LogPolicy::CENTRAL).await;
    assert_eq!(
        post(router.clone(), batch(1, 2, 0, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::OK
    );
    drop(router);
    drop(store);
    tokio::task::yield_now().await;
    let path = root.join("state.json");
    let mut metadata: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    metadata["next_sequence"] = 10.into();
    std::fs::write(path, serde_json::to_vec(&metadata).unwrap()).unwrap();
    let (reopened, _layer) = RuntimeLogStore::open(root.clone(), LogPolicy::CENTRAL).unwrap();
    reopened.recover(&pool).await.unwrap();
    let router = app(AppState::with_runtime_logs(pool.clone(), reopened));
    assert_eq!(
        post(router.clone(), batch(3, 3, 2, 1, EPOCH), "agent_one")
            .await
            .status(),
        StatusCode::OK
    );
    let page = read_page(&root, 0, 64, 65536).unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|record| record.sequence)
            .collect::<Vec<_>>(),
        [1, 2, 10]
    );
    let (cookie, _) = common::admin_session(router.clone()).await;
    let response = json_request(
        router,
        "GET",
        "/api/v1/runtime-logs?after=1",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let mut events = response.into_body().into_data_stream();
    let gap = String::from_utf8(events.next().await.unwrap().unwrap().to_vec()).unwrap();
    assert!(gap.contains("event: gap"));
    assert!(gap.contains("\"from_sequence\":3"));
    assert!(gap.contains("\"to_sequence\":9"));
}
