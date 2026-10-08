use deploy_go_agent_protocol::{Message, OutputStream, TaskOutput};
use deploy_go_api::agents::dispatcher::{event_receipt, handle_agent_message};
use deploy_go_api::{
    AppState, db,
    deployments::{purge_expired_output, purge_over_capacity_output, recover},
};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn restart_interrupts_uncertain_work_and_preserves_queue() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&pool)
        .await
        .unwrap();
    for (id, status) in [
        ("queued", "queued"),
        ("running", "running"),
        ("canceling", "canceling"),
    ] {
        sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES(?,?,?,?,?,?,?,?)")
            .bind(id).bind(format!("target-{id}")).bind("user").bind(status).bind(status).bind(format!("recovery-key-{id}" )).bind(id).bind("snapshot").execute(&pool).await.unwrap();
    }
    assert_eq!(recover(&pool).await.unwrap(), 2);
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT id,status FROM deployments ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [
            ("canceling".to_owned(), "interrupted".to_owned()),
            ("queued".to_owned(), "queued".to_owned()),
            ("running".to_owned(), "interrupted".to_owned())
        ]
    );
}

async fn log_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

async fn seed_output(pool: &sqlx::SqlitePool, id: &str, status: &str, finished: &str, task: bool) {
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,finished_at) VALUES(?,'target','user',?,?,?,?, 'snapshot',?)")
        .bind(id).bind(status).bind(status).bind(id).bind(id).bind(finished).execute(pool).await.unwrap();
    if task {
        sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at,last_sequence,result_json) VALUES(?,'agent',?,'deployment_execute',?,'digest','{}',?,'2030-01-01',3,?)")
            .bind(id).bind(id).bind(id).bind(status)
            .bind(serde_json::json!({"sequence":3,"status":status,"error_message":"retained failure"}).to_string()).execute(pool).await.unwrap();
        for sequence in 1..=3 {
            sqlx::query("INSERT INTO agent_task_event_receipts(task_id,sequence,source_digest,committed) VALUES(?,?,?,1)")
                .bind(id).bind(sequence).bind(format!("source-{sequence}")).execute(pool).await.unwrap();
        }
        for (sequence, kind, payload) in [
            (1, "output", "正文"),
            (2, "progress", "{}"),
            (3, "result", "{}"),
            (4, "diagnostic", "{}"),
        ] {
            let payload = if kind == "result" {
                serde_json::json!({"sequence":3,"status":status,"error_message":"retained failure"})
                    .to_string()
            } else {
                payload.to_owned()
            };
            sqlx::query(
                "INSERT INTO agent_task_events(task_id,sequence,kind,payload_json) VALUES(?,?,?,?)",
            )
            .bind(id)
            .bind(sequence)
            .bind(kind)
            .bind(payload)
            .execute(pool)
            .await
            .unwrap();
        }
    }
    sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,task_sequence,stream,content) VALUES(?,1,1,'stdout',?)")
        .bind(id).bind("x".repeat(600_000)).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_events(id,deployment_id,event_name,payload_json) VALUES(?,?,'diagnostic','{}')")
        .bind(id).bind(id).execute(pool).await.unwrap();
}

async fn set_budget(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO system_settings(key,value_json,version) VALUES('runtime',?,1)")
        .bind(serde_json::json!({"max_concurrent_deployments":2,"max_log_bytes":52428800,"max_total_log_bytes":1048576,"log_retention_days":30,"version":1}).to_string()).execute(pool).await.unwrap();
}

async fn assert_capacity(pool: &sqlx::SqlitePool) -> i64 {
    let counted: i64 =
        sqlx::query_scalar("SELECT total_bytes FROM deployment_log_capacity WHERE id=1")
            .fetch_one(pool)
            .await
            .unwrap();
    let actual: i64 = sqlx::query_scalar("SELECT (SELECT COALESCE(SUM(length(CAST(content AS BLOB))),0) FROM deployment_logs)+(SELECT COALESCE(SUM(length(CAST(payload_json AS BLOB))),0) FROM agent_task_events WHERE kind='output')").fetch_one(pool).await.unwrap();
    assert_eq!(counted, actual);
    counted
}

#[tokio::test]
async fn counters_track_utf8_updates_deletes_and_transaction_rollback() {
    let pool = log_pool().await;
    seed_output(&pool, "task", "failed", "2020-01-01", true).await;
    assert_eq!(assert_capacity(&pool).await, 600_006);
    sqlx::query("UPDATE deployment_logs SET content='中文' WHERE deployment_id='task'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(assert_capacity(&pool).await, 12);
    sqlx::query(
        "UPDATE agent_task_events SET payload_json='abc' WHERE task_id='task' AND sequence=1",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(assert_capacity(&pool).await, 9);
    sqlx::query(
        "UPDATE agent_task_events SET kind='diagnostic' WHERE task_id='task' AND sequence=1",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(assert_capacity(&pool).await, 6);
    sqlx::query("UPDATE agent_task_events SET kind='output' WHERE task_id='task' AND sequence=1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(assert_capacity(&pool).await, 9);
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("DELETE FROM deployment_logs")
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("DELETE FROM agent_task_events WHERE kind='output'")
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    assert_eq!(assert_capacity(&pool).await, 9);
    assert_eq!(
        purge_expired_output(&AppState::new(pool.clone()))
            .await
            .unwrap(),
        1
    );
    assert_eq!(assert_capacity(&pool).await, 0);
}

#[tokio::test]
async fn migration_backfills_existing_output_copies() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let all = sqlx::migrate!();
    let prior = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            all.iter()
                .filter(|migration| migration.version < 40)
                .cloned()
                .collect(),
        ),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    };
    db::migrate_with(&pool, &prior).await.unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&pool)
        .await
        .unwrap();
    seed_output(&pool, "prior", "failed", "2020-01-01", true).await;
    db::migrate(&pool).await.unwrap();
    assert_eq!(assert_capacity(&pool).await, 600_006);
}

#[tokio::test]
async fn equal_finish_times_use_id_order_and_all_associated_tasks_must_be_safe() {
    let pool = log_pool().await;
    set_budget(&pool).await;
    seed_output(&pool, "a", "succeeded", "2020-01-01", false).await;
    seed_output(&pool, "b", "failed", "2020-01-01", true).await;
    let state = AppState::new(pool.clone());
    assert_eq!(purge_over_capacity_output(&state).await.unwrap(), 1);
    let remaining: String = sqlx::query_scalar("SELECT deployment_id FROM deployment_logs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, "b");
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('second','agent','b','deployment_execute','second','digest','{}','queued','2030-01-01')").execute(&pool).await.unwrap();
    assert_eq!(purge_expired_output(&state).await.unwrap(), 0);
    assert_eq!(assert_capacity(&pool).await, 600_006);
}

#[tokio::test]
async fn capacity_reclaims_oldest_whole_deployment_and_preserves_replay_evidence() {
    let pool = log_pool().await;
    set_budget(&pool).await;
    for (id, status, finished, task) in [
        ("old-failed", "failed", "2020-01-01", true),
        ("middle", "succeeded", "2021-01-01", false),
        ("newest", "succeeded", "2022-01-01", true),
    ] {
        seed_output(&pool, id, status, finished, task).await;
    }
    let result: String =
        sqlx::query_scalar("SELECT result_json FROM agent_tasks WHERE id='old-failed'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        purge_over_capacity_output(&AppState::new(pool.clone()))
            .await
            .unwrap(),
        2
    );
    let remaining: Vec<String> =
        sqlx::query_scalar("SELECT deployment_id FROM deployment_logs ORDER BY deployment_id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, ["newest"]);
    assert_eq!(assert_capacity(&pool).await, 600_006);
    let receipts: Vec<(i64, String, i64)> = sqlx::query_as("SELECT sequence,source_digest,committed FROM agent_task_event_receipts WHERE task_id='old-failed' ORDER BY sequence").fetch_all(&pool).await.unwrap();
    assert_eq!(
        receipts,
        [
            (1, "source-1".into(), 1),
            (2, "source-2".into(), 1),
            (3, "source-3".into(), 1)
        ]
    );
    let after: String =
        sqlx::query_scalar("SELECT result_json FROM agent_tasks WHERE id='old-failed'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(result, after);
    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM agent_task_events WHERE task_id='old-failed' ORDER BY sequence",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(kinds, ["progress", "result", "diagnostic"]);
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployment_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(events, 3);
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,connection_generation) VALUES('agent','node',18,1)").execute(&pool).await.unwrap();
    let replay = Message::TaskOutput(TaskOutput {
        task_id: "old-failed".into(),
        sequence: 1,
        stream: OutputStream::Stdout,
        text: "正文".into(),
    });
    let digest = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&replay).unwrap())
    );
    sqlx::query("UPDATE agent_task_event_receipts SET source_digest=? WHERE task_id='old-failed' AND sequence=1")
        .bind(&digest).execute(&pool).await.unwrap();
    let state = AppState::new(pool.clone());
    assert!(
        handle_agent_message(&state, "agent", 1, &replay)
            .await
            .unwrap()
    );
    let receipt = event_receipt(&state, "agent", &replay)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(receipt.message_digest, digest);
    assert_eq!(receipt.sequence, 1);
    assert_eq!(assert_capacity(&pool).await, 600_006);
}

#[tokio::test]
async fn concurrent_cleanup_and_writes_on_same_database_keep_counters_consistent() {
    use sqlx::sqlite::SqliteConnectOptions;
    let directory = tempfile::tempdir().unwrap();
    let options = SqliteConnectOptions::new()
        .filename(directory.path().join("logs.db"))
        .create_if_missing(true)
        .foreign_keys(false)
        .busy_timeout(std::time::Duration::from_secs(10));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&pool)
        .await
        .unwrap();
    set_budget(&pool).await;
    seed_output(&pool, "old", "failed", "2020-01-01", true).await;
    seed_output(&pool, "live", "running", "2020-01-01", true).await;
    let concurrent = SqlitePoolOptions::new()
        .max_connections(3)
        .connect_with(options)
        .await
        .unwrap();
    let state = AppState::new(concurrent.clone());
    let writes = async {
        for index in 0..20 {
            sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,task_sequence,stream,content) VALUES('live',?,?,'stdout','活动')")
                .bind(index + 2).bind(index + 2).execute(&concurrent).await.unwrap();
            sqlx::query("UPDATE deployment_logs SET content=? WHERE deployment_id='live'")
                .bind(format!("活动输出-{index}"))
                .execute(&concurrent)
                .await
                .unwrap();
            sqlx::query("DELETE FROM deployment_logs WHERE deployment_id='live' AND sequence=?")
                .bind(index + 2)
                .execute(&concurrent)
                .await
                .unwrap();
            sqlx::query("UPDATE agent_task_events SET payload_json=? WHERE task_id='live' AND kind='output'")
                .bind(format!("活动副本-{index}")).execute(&concurrent).await.unwrap();
        }
    };
    let (capacity, retention, ()) = tokio::join!(
        purge_over_capacity_output(&state),
        purge_expired_output(&state),
        writes
    );
    capacity.unwrap();
    retention.unwrap();
    assert_capacity(&concurrent).await;
    let logs: Vec<String> = sqlx::query_scalar("SELECT deployment_id FROM deployment_logs")
        .fetch_all(&concurrent)
        .await
        .unwrap();
    assert_eq!(logs, ["live"]);
    let result: String = sqlx::query_scalar("SELECT result_json FROM agent_tasks WHERE id='old'")
        .fetch_one(&concurrent)
        .await
        .unwrap();
    assert!(result.contains("retained failure"));
    let receipts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_task_event_receipts WHERE task_id='old'")
            .fetch_one(&concurrent)
            .await
            .unwrap();
    assert_eq!(receipts, 3);
}

#[tokio::test]
async fn standalone_task_output_is_counted_and_safely_reclaimed() {
    let pool = log_pool().await;
    set_budget(&pool).await;
    let result = serde_json::json!({"sequence":3,"status":"failed"}).to_string();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at,finished_at,last_sequence,result_json) VALUES('inspect','agent','system_inspect','inspect','digest','{}','failed','2030-01-01','2020-01-01',3,?)")
        .bind(&result).execute(&pool).await.unwrap();
    for sequence in 1..=3 {
        sqlx::query("INSERT INTO agent_task_event_receipts(task_id,sequence,source_digest,committed) VALUES('inspect',?,?,1)")
            .bind(sequence).bind(format!("source-{sequence}")).execute(&pool).await.unwrap();
    }
    for (sequence, kind, payload) in [
        (1, "output", "x".repeat(1100000)),
        (2, "progress", "{}".into()),
        (3, "result", result),
    ] {
        sqlx::query("INSERT INTO agent_task_events(task_id,sequence,kind,payload_json) VALUES('inspect',?,?,?)")
            .bind(sequence).bind(kind).bind(payload).execute(&pool).await.unwrap();
    }
    assert!(assert_capacity(&pool).await > 1048576);
    sqlx::query(
        "UPDATE agent_task_event_receipts SET committed=0 WHERE task_id='inspect' AND sequence=2",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    assert_eq!(purge_over_capacity_output(&state).await.unwrap(), 0);
    sqlx::query(
        "UPDATE agent_task_event_receipts SET committed=1 WHERE task_id='inspect' AND sequence=2",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(purge_over_capacity_output(&state).await.unwrap(), 1);
    assert!(assert_capacity(&pool).await < 1048576);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_task_event_receipts WHERE task_id='inspect'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        3
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_task_events WHERE task_id='inspect'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );
}

#[tokio::test]
async fn both_cleanup_paths_protect_active_legacy_and_incomplete_deliveries() {
    let pool = log_pool().await;
    set_budget(&pool).await;
    for id in [
        "active",
        "legacy",
        "gap",
        "uncommitted",
        "mismatch",
        "missing-result",
        "invalid-result",
        "nonterminal-result",
        "zero",
        "beyond-receipt",
        "beyond-output",
        "result-diverged",
    ] {
        seed_output(&pool, id, "failed", "2020-01-01", true).await;
    }
    for query in [
        "UPDATE agent_tasks SET status='running' WHERE id='active'",
        "DELETE FROM agent_task_event_receipts WHERE task_id='legacy'",
        "DELETE FROM agent_task_event_receipts WHERE task_id='gap' AND sequence=2",
        "UPDATE agent_task_event_receipts SET committed=0 WHERE task_id='uncommitted' AND sequence=2",
        "UPDATE agent_tasks SET result_json='{\"sequence\":2,\"status\":\"failed\"}' WHERE id='mismatch'",
        "UPDATE agent_tasks SET result_json=NULL WHERE id='missing-result'",
        "UPDATE agent_tasks SET result_json='invalid' WHERE id='invalid-result'",
        "UPDATE agent_tasks SET result_json='{\"sequence\":3,\"status\":\"running\"}' WHERE id='nonterminal-result'",
        "UPDATE agent_tasks SET last_sequence=0,result_json='{\"sequence\":0,\"status\":\"failed\"}' WHERE id='zero'",
        "INSERT INTO agent_task_event_receipts(task_id,sequence,source_digest,committed) VALUES('beyond-receipt',4,'uncommitted',0)",
        "INSERT INTO agent_task_events(task_id,sequence,kind,payload_json) VALUES('beyond-output',5,'output','later')",
        "UPDATE agent_task_events SET payload_json='{\"sequence\":3,\"status\":\"failed\",\"error_message\":\"different\"}' WHERE task_id='result-diverged' AND sequence=3",
    ] {
        sqlx::query(query).execute(&pool).await.unwrap();
    }
    let state = AppState::new(pool.clone());
    assert_eq!(purge_expired_output(&state).await.unwrap(), 0);
    assert_eq!(purge_over_capacity_output(&state).await.unwrap(), 0);
    assert!(assert_capacity(&pool).await > 1048576);
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployment_logs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 12);
    // 补齐缺口后可以回收；receipt 和失败结果始终保留供晚到重传查询。
    sqlx::query("INSERT INTO agent_task_event_receipts(task_id,sequence,source_digest,committed) VALUES('gap',2,'source-2',1)").execute(&pool).await.unwrap();
    assert_eq!(purge_expired_output(&state).await.unwrap(), 1);
    assert_eq!(purge_over_capacity_output(&state).await.unwrap(), 0);
}

#[tokio::test]
async fn retention_removes_output_but_preserves_deployment_history() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,finished_at) VALUES('old','target','user','failed','failed','retention-old-key','hash','snapshot','2020-01-01T00:00:00Z')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_logs(deployment_id,task_id,sequence,task_sequence,stream,content) VALUES('old',NULL,1,1,'stdout','old log')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_events(id,deployment_id,log_sequence,event_name,payload_json) VALUES('old-event','old',1,'diagnostic','{}')").execute(&pool).await.unwrap();
    for (id, event, diagnostic) in [
        ("result", "result", None),
        ("progress", "progress", None),
        ("coded", "deploy.failed", Some("failure_code")),
        ("display", "state", None),
    ] {
        sqlx::query("INSERT INTO deployment_events(id,deployment_id,event_name,payload_json,diagnostic_code) VALUES(?,'old',?,'{}',?)")
            .bind(id).bind(event).bind(diagnostic).execute(&pool).await.unwrap();
    }
    let state = AppState::new(pool.clone());
    assert_eq!(purge_expired_output(&state).await.unwrap(), 1);
    let deployments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployments WHERE id='old'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let logs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployment_logs WHERE deployment_id='old'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployment_events WHERE deployment_id='old'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((deployments, logs, events), (1, 0, 4));
}
