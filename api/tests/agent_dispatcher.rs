use deploy_go_agent_protocol::{
    Message, OutputStream, ReconcileReport, ReconciledTask, ReconciledTaskState, TaskAck,
    TaskAckDisposition, TaskLifecycleState, TaskOutput, TaskPayload, TaskResult, TaskState,
    TaskTerminalStatus,
};
use deploy_go_api::{
    AppState,
    agents::dispatcher::{
        enqueue_deployment, handle_agent_message, request_deployment_cancel,
        requeue_expired_deliveries,
    },
    db,
};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

async fn fixture(with_roots: bool) -> (AppState, sqlx::SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_agent','Agent App','agent-app','active')").execute(&pool).await.unwrap();
    if with_roots {
        sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_agent','Agent Node','/srv/apps','/srv/secrets','online')").execute(&pool).await.unwrap();
    } else {
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node_agent','Agent Node','online')")
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version) VALUES('agent_runtime','node_agent','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.1.0',1)").execute(&pool).await.unwrap();
    let schema = json!({"type":"object","properties":{"release-version":{"type":"string"}},"required":["release-version"],"additionalProperties":false});
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,parameter_schema,timeout_seconds,verification_config,status) VALUES('target_agent','app_agent','node_agent','test','/srv/apps/deploy.sh',?,60,'{}','active')").bind(schema.to_string()).execute(&pool).await.unwrap();
    let snapshot = json!({"target":{"application_id":"app_agent","node_id":"node_agent","environment":"test","script_path":"/srv/apps/deploy.sh","parameter_schema":schema,"timeout_seconds":60,"verification_config":{},"secret_file_references":[{"environment_key":"TOKEN_FILE","file_path":"/srv/secrets/token"}],"version":1},"parameters":{"release-version":"1.0.0"}});
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment_agent','target_agent','admin','queued','queued','request-agent-0001','hash','snapshot',?)").bind(snapshot.to_string()).execute(&pool).await.unwrap();
    (AppState::new(pool.clone()), pool)
}

#[tokio::test]
async fn deployment_snapshot_is_persisted_as_an_idempotent_agent_task() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    let repeated = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    assert_eq!(repeated, task_id);
    let (status, digest, payload_json, idempotency_key): (String, String, String, String) =
        sqlx::query_as(
            "SELECT status,payload_digest,payload_json,idempotency_key FROM agent_tasks WHERE id=?",
        )
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "queued");
    assert!(digest.starts_with("sha256:"));
    assert_eq!(idempotency_key, "deployment:deployment_agent");
    let TaskPayload::DeploymentExecute(payload) = serde_json::from_str(&payload_json).unwrap()
    else {
        panic!("期望部署任务");
    };
    assert_eq!(payload.work_root, "/srv/apps");
    assert_eq!(payload.script_path, "/srv/apps/deploy.sh");
    assert_eq!(payload.argument_tokens, ["--release-version", "1.0.0"]);
    assert_eq!(
        payload.environment_file_references[0].file_path,
        "/srv/secrets/token"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn missing_node_roots_rejects_dispatch_without_creating_a_task() {
    let (state, pool) = fixture(false).await;
    assert!(
        enqueue_deployment(&state, "deployment_agent")
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn current_connection_events_advance_task_deployment_and_logs_once() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='delivered' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    let digest: String = sqlx::query_scalar("SELECT payload_digest FROM agent_tasks WHERE id=?")
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::TaskAck(TaskAck {
            task_id: task_id.clone(),
            payload_digest: digest,
            disposition: TaskAckDisposition::Accepted,
            error_code: None,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::TaskState(TaskState {
            task_id: task_id.clone(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    let output = Message::TaskOutput(TaskOutput {
        task_id: task_id.clone(),
        sequence: 2,
        stream: OutputStream::Stdout,
        text: "deployment output".to_owned(),
    });
    handle_agent_message(&state, "agent_runtime", 2, &output)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::TaskResult(TaskResult {
            task_id: task_id.clone(),
            sequence: 3,
            status: TaskTerminalStatus::Succeeded,
            exit_code: Some(0),
            error_code: None,
            summary: Some("部署完成".to_owned()),
        }),
    )
    .await
    .unwrap();
    handle_agent_message(&state, "agent_runtime", 2, &output)
        .await
        .unwrap();

    let task: (String, i64) =
        sqlx::query_as("SELECT status,last_sequence FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task, ("succeeded".to_owned(), 3));
    let deployment: (String, Option<i64>, bool) = sqlx::query_as(
        "SELECT status,exit_code,protocol_complete FROM deployments WHERE id='deployment_agent'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(deployment, ("succeeded".to_owned(), Some(0), true));
    let logs: Vec<String> = sqlx::query_scalar(
        "SELECT content FROM deployment_logs WHERE deployment_id='deployment_agent'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(logs, ["deployment output"]);
}

#[tokio::test]
async fn stale_connection_and_sequence_gaps_are_rejected() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    let state_message = Message::TaskState(TaskState {
        task_id,
        sequence: 2,
        state: TaskLifecycleState::Running,
    });
    assert!(
        handle_agent_message(&state, "agent_runtime", 1, &state_message)
            .await
            .is_err()
    );
    assert!(
        handle_agent_message(&state, "agent_runtime", 2, &state_message)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn expired_delivery_lease_returns_to_queue_for_retry() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='delivered',lease_expires_at='2026-08-03T00:00:00Z' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(requeue_expired_deliveries(&state).await.unwrap(), 1);
    let task: (String, Option<String>) =
        sqlx::query_as("SELECT status,lease_expires_at FROM agent_tasks WHERE id=?")
            .bind(task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task, ("queued".to_owned(), None));
}

#[tokio::test]
async fn reconnect_reconcile_restores_exact_state_and_interrupts_mismatch() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=2 WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='delivered' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    let digest: String = sqlx::query_scalar("SELECT payload_digest FROM agent_tasks WHERE id=?")
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::ReconcileReport(ReconcileReport {
            tasks: vec![ReconciledTask {
                task_id: task_id.clone(),
                payload_digest: digest,
                state: ReconciledTaskState::Running,
                last_sequence: 0,
                result: None,
            }],
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM deployments WHERE id='deployment_agent'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "running"
    );

    handle_agent_message(
        &state,
        "agent_runtime",
        2,
        &Message::ReconcileReport(ReconcileReport {
            tasks: vec![ReconciledTask {
                task_id: task_id.clone(),
                payload_digest: "sha256:different-payload".to_owned(),
                state: ReconciledTaskState::Running,
                last_sequence: 0,
                result: None,
            }],
        }),
    )
    .await
    .unwrap();
    let states: (String, String) = sqlx::query_as(
        "SELECT t.status,d.status FROM agent_tasks t JOIN deployments d ON d.id=t.deployment_id WHERE t.id=?",
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(states, ("interrupted".to_owned(), "interrupted".to_owned()));
}

#[tokio::test]
async fn cancel_before_delivery_finishes_locally_and_delivered_task_stays_canceling() {
    let (state, pool) = fixture(true).await;
    let task_id = enqueue_deployment(&state, "deployment_agent")
        .await
        .unwrap();
    assert!(
        !request_deployment_cancel(&state, "deployment_agent")
            .await
            .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "canceled"
    );

    sqlx::query("UPDATE agent_tasks SET status='delivered',finished_at=NULL WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !request_deployment_cancel(&state, "deployment_agent")
            .await
            .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_tasks WHERE id=?")
            .bind(task_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "canceling"
    );
}
