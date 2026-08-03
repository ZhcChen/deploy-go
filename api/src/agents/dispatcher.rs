use chrono::{Duration, Utc};
use deploy_go_agent_protocol::{
    DeploymentExecuteTask, EnvironmentFileReference, Message, OutputStream, ReconcileReport,
    ReconciledTaskState, SystemInspectTask, TaskAck, TaskAckDisposition, TaskDispatch,
    TaskLifecycleState, TaskOutput, TaskPayload, TaskResult, TaskState, TaskTerminalStatus,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use ulid::Ulid;

use crate::{
    AppState,
    error::{ApiError, ApiResult},
    execution_spec,
};

#[derive(sqlx::FromRow)]
struct DeploymentTaskSource {
    deployment_id: String,
    snapshot_json: String,
    agent_id: String,
    work_root: Option<String>,
    secrets_root: Option<String>,
}

pub async fn enqueue_deployment(state: &AppState, deployment_id: &str) -> ApiResult<String> {
    if let Some(existing) =
        sqlx::query_scalar::<_, String>("SELECT id FROM agent_tasks WHERE deployment_id=?")
            .bind(deployment_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_dispatch"))?
    {
        try_dispatch(state, &existing).await?;
        return Ok(existing);
    }
    let source = sqlx::query_as::<_, DeploymentTaskSource>(
        "SELECT d.id AS deployment_id,d.snapshot_json,a.id AS agent_id,n.work_root,n.secrets_root FROM deployments d JOIN deployment_targets t ON t.id=d.target_id JOIN nodes n ON n.id=t.node_id JOIN agents a ON a.node_id=n.id WHERE d.id=? AND d.status='queued' AND n.status='online' AND a.revoked_at IS NULL AND a.archived_at IS NULL",
    )
    .bind(deployment_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_dispatch"))?
    .ok_or_else(|| ApiError::conflict("agent_not_available", "目标节点 Agent 当前不可用", "agent_dispatch"))?;
    let work_root = source.work_root.ok_or_else(|| {
        ApiError::conflict(
            "agent_work_root_missing",
            "节点尚未配置工作根目录",
            "agent_dispatch",
        )
    })?;
    let secrets_root = source.secrets_root.ok_or_else(|| {
        ApiError::conflict(
            "agent_secrets_root_missing",
            "节点尚未配置敏感文件根目录",
            "agent_dispatch",
        )
    })?;
    let snapshot: Value = serde_json::from_str(&source.snapshot_json)
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let target = snapshot
        .get("target")
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let parameters = snapshot
        .get("parameters")
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let script_path = target
        .get("script_path")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    execution_spec::validate_script_path(&work_root, script_path, "agent_dispatch")?;
    let references = target
        .get("secret_file_references")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|reference| {
            let environment_key = reference
                .get("environment_key")
                .and_then(Value::as_str)
                .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
            let file_path = reference
                .get("file_path")
                .and_then(Value::as_str)
                .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
            execution_spec::validate_secret_path(&secrets_root, file_path, "agent_dispatch")?;
            Ok(EnvironmentFileReference {
                environment_key: environment_key.to_owned(),
                file_path: file_path.to_owned(),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    let timeout_seconds = target
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let payload = TaskPayload::DeploymentExecute(DeploymentExecuteTask {
        deployment_id: source.deployment_id.clone(),
        work_root,
        script_path: script_path.to_owned(),
        argument_tokens: execution_spec::parameter_tokens(parameters, "agent_dispatch")?,
        environment_file_references: references,
        timeout_seconds,
        wrapper_version: "1".to_owned(),
    });
    let payload_json =
        serde_json::to_string(&payload).map_err(|_| ApiError::internal("agent_dispatch"))?;
    let payload_digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    let task_id = format!("task_{}", Ulid::new());
    let deadline_at =
        (Utc::now() + Duration::seconds(i64::from(timeout_seconds) + 60)).to_rfc3339();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,?,?,'deployment_execute',?, ?,?,'queued',?)")
        .bind(&task_id)
        .bind(&source.agent_id)
        .bind(&source.deployment_id)
        .bind(format!("deployment:{}", source.deployment_id))
        .bind(&payload_digest)
        .bind(&payload_json)
        .bind(&deadline_at)
        .execute(state.pool())
        .await
        .map_err(|error| {
            if error.to_string().contains("UNIQUE constraint failed") {
                ApiError::conflict("agent_task_conflict", "部署任务已经入队", "agent_dispatch")
            } else {
                ApiError::internal("agent_dispatch")
            }
        })?;
    try_dispatch(state, &task_id).await?;
    Ok(task_id)
}

pub async fn dispatch_next_deployment(state: &AppState) -> ApiResult<Option<String>> {
    requeue_expired_deliveries(state).await?;
    let retry_before = (Utc::now() - Duration::seconds(1)).to_rfc3339();
    let deployment_id: Option<String> = sqlx::query_scalar(
        "SELECT d.id FROM deployments d JOIN deployment_targets target ON target.id=d.target_id JOIN applications application ON application.id=target.application_id JOIN nodes node ON node.id=target.node_id JOIN agents agent ON agent.node_id=node.id LEFT JOIN agent_tasks task ON task.deployment_id=d.id WHERE d.status='queued' AND application.status='active' AND target.status='active' AND node.status='online' AND node.work_root IS NOT NULL AND node.secrets_root IS NOT NULL AND agent.revoked_at IS NULL AND agent.archived_at IS NULL AND NOT EXISTS (SELECT 1 FROM deployments active WHERE active.target_id=d.target_id AND active.id!=d.id AND active.status IN ('running','canceling')) AND (task.id IS NULL OR (task.status='queued' AND task.updated_at<=?)) ORDER BY d.queued_at,d.id LIMIT 1",
    )
    .bind(retry_before)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let Some(deployment_id) = deployment_id else {
        return Ok(None);
    };
    enqueue_deployment(state, &deployment_id).await?;
    Ok(Some(deployment_id))
}

pub async fn enqueue_node_inspect(
    state: &AppState,
    node_id: &str,
    check_id: &str,
) -> ApiResult<String> {
    let source: Option<(String, String, String)> = sqlx::query_as(
        "SELECT agent.id,node.work_root,node.secrets_root FROM nodes node JOIN agents agent ON agent.node_id=node.id WHERE node.id=? AND node.status='online' AND node.work_root IS NOT NULL AND node.secrets_root IS NOT NULL AND agent.revoked_at IS NULL AND agent.archived_at IS NULL",
    )
    .bind(node_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_node_check"))?;
    let Some((agent_id, work_root, secrets_root)) = source else {
        return Err(ApiError::conflict(
            "agent_not_available",
            "节点 Agent 当前不可检查",
            "agent_node_check",
        ));
    };
    let payload = TaskPayload::SystemInspect(SystemInspectTask {
        work_root,
        secrets_root,
    });
    let payload_json =
        serde_json::to_string(&payload).map_err(|_| ApiError::internal("agent_node_check"))?;
    let payload_digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    let task_id = format!("task_{}", Ulid::new());
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,node_check_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,?,?,'system_inspect',?,?,?,'queued',?)")
        .bind(&task_id).bind(&agent_id).bind(check_id).bind(format!("node-check:{check_id}"))
        .bind(&payload_digest).bind(&payload_json).bind((Utc::now() + Duration::minutes(2)).to_rfc3339())
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_node_check"))?;
    try_dispatch(state, &task_id).await?;
    Ok(task_id)
}

pub async fn request_deployment_cancel(state: &AppState, deployment_id: &str) -> ApiResult<bool> {
    let task: Option<(String, String, String)> =
        sqlx::query_as("SELECT id,agent_id,status FROM agent_tasks WHERE deployment_id=?")
            .bind(deployment_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_cancel"))?;
    let Some((task_id, agent_id, status)) = task else {
        return Ok(false);
    };
    let now = Utc::now().to_rfc3339();
    if status == "queued" {
        sqlx::query("UPDATE agent_tasks SET status='canceled',finished_at=?,result_json=?,updated_at=? WHERE id=? AND status='queued'")
            .bind(&now).bind(serde_json::json!({"error_code":"canceled_before_delivery"}).to_string()).bind(&now).bind(&task_id)
            .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_cancel"))?;
        return Ok(false);
    }
    if !matches!(
        status.as_str(),
        "delivered" | "accepted" | "running" | "canceling"
    ) {
        return Ok(false);
    }
    sqlx::query("UPDATE agent_tasks SET status='canceling',updated_at=? WHERE id=? AND status IN ('delivered','accepted','running','canceling')")
        .bind(&now).bind(&task_id).execute(state.pool()).await
        .map_err(|_| ApiError::internal("agent_cancel"))?;
    let sent = state
        .agent_connections()
        .send(
            &agent_id,
            Message::TaskCancel(deploy_go_agent_protocol::TaskCancel {
                task_id,
                reason: "deployment_cancel_requested".to_owned(),
            }),
        )
        .await
        .is_ok();
    Ok(sent)
}

pub async fn try_dispatch(state: &AppState, task_id: &str) -> ApiResult<bool> {
    let row: Option<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT agent_id,idempotency_key,payload_digest,payload_json,deadline_at FROM agent_tasks WHERE id=? AND status IN ('queued','delivered')",
    )
    .bind(task_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let Some((agent_id, idempotency_key, payload_digest, payload_json, deadline_at)) = row else {
        return Ok(false);
    };
    let payload = serde_json::from_str::<TaskPayload>(&payload_json)
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let message = Message::TaskDispatch(TaskDispatch {
        task_id: task_id.to_owned(),
        idempotency_key,
        deadline_at,
        payload_digest,
        task: payload,
    });
    let now = Utc::now();
    sqlx::query("UPDATE agent_tasks SET status='delivered',delivered_at=COALESCE(delivered_at,?),lease_expires_at=?,updated_at=? WHERE id=? AND status IN ('queued','delivered')")
        .bind(now.to_rfc3339())
        .bind((now + Duration::seconds(30)).to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(task_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
    if state
        .agent_connections()
        .send(&agent_id, message)
        .await
        .is_err()
    {
        sqlx::query("UPDATE agent_tasks SET status='queued',lease_expires_at=NULL,updated_at=? WHERE id=? AND status='delivered'")
            .bind(Utc::now().to_rfc3339())
            .bind(task_id)
            .execute(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_dispatch"))?;
        return Ok(false);
    }
    Ok(true)
}

pub async fn requeue_expired_deliveries(state: &AppState) -> ApiResult<u64> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query("UPDATE agent_tasks SET status='queued',lease_expires_at=NULL,updated_at=? WHERE status='delivered' AND lease_expires_at IS NOT NULL AND lease_expires_at<=?")
        .bind(&now)
        .bind(&now)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
    Ok(result.rows_affected())
}

pub async fn active_task_ids(state: &AppState, agent_id: &str) -> ApiResult<Vec<String>> {
    requeue_expired_deliveries(state).await?;
    sqlx::query_scalar("SELECT id FROM agent_tasks WHERE agent_id=? AND status IN ('delivered','accepted','running','canceling') ORDER BY created_at,id")
        .bind(agent_id)
        .fetch_all(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_reconcile"))
}

pub async fn dispatch_queued_for_agent(state: &AppState, agent_id: &str) -> ApiResult<u64> {
    requeue_expired_deliveries(state).await?;
    let task_ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM agent_tasks WHERE agent_id=? AND status='queued' ORDER BY created_at,id",
    )
    .bind(agent_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let mut dispatched = 0;
    for task_id in task_ids {
        if try_dispatch(state, &task_id).await? {
            dispatched += 1;
        }
    }
    Ok(dispatched)
}

pub async fn handle_agent_message(
    state: &AppState,
    agent_id: &str,
    connection_generation: i64,
    message: &Message,
) -> ApiResult<bool> {
    match message {
        Message::TaskAck(ack) => {
            handle_ack(state, agent_id, connection_generation, ack).await?;
            Ok(true)
        }
        Message::TaskOutput(output) => {
            handle_output(state, agent_id, connection_generation, output).await?;
            Ok(true)
        }
        Message::TaskState(task_state) => {
            handle_state(state, agent_id, connection_generation, task_state).await?;
            Ok(true)
        }
        Message::TaskResult(result) => {
            handle_result(state, agent_id, connection_generation, result).await?;
            Ok(true)
        }
        Message::ReconcileReport(report) => {
            handle_reconcile_report(state, agent_id, connection_generation, report).await?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

async fn handle_reconcile_report(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    report: &ReconcileReport,
) -> ApiResult<()> {
    ensure_current_connection(state, agent_id, generation).await?;
    for task in &report.tasks {
        let expected: Option<(String, i64, String)> = sqlx::query_as(
            "SELECT payload_digest,last_sequence,status FROM agent_tasks WHERE id=? AND agent_id=? AND status IN ('delivered','accepted','running','canceling')",
        )
        .bind(&task.task_id)
        .bind(agent_id)
        .fetch_optional(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_reconcile"))?;
        let Some((digest, last_sequence, expected_status)) = expected else {
            continue;
        };
        let sequence_matches = u64::try_from(last_sequence).ok() == Some(task.last_sequence);
        if digest != task.payload_digest
            || !sequence_matches
            || task.state == ReconciledTaskState::Unknown
        {
            interrupt_task(state, &task.task_id, "Agent 恢复对账不一致").await?;
            continue;
        }
        if expected_status == "canceling" && task.state != ReconciledTaskState::Terminal {
            restore_task_state(state, &task.task_id, "canceling", "canceling", "canceling").await?;
            state
                .agent_connections()
                .send(
                    agent_id,
                    Message::TaskCancel(deploy_go_agent_protocol::TaskCancel {
                        task_id: task.task_id.clone(),
                        reason: "deployment_cancel_requested".to_owned(),
                    }),
                )
                .await
                .map_err(|_| {
                    ApiError::conflict(
                        "agent_cancel_delivery_failed",
                        "Agent 取消请求投递失败",
                        "agent_reconcile",
                    )
                })?;
            continue;
        }
        match task.state {
            ReconciledTaskState::Accepted => {
                restore_task_state(state, &task.task_id, "accepted", "running", "accepted").await?;
            }
            ReconciledTaskState::Running => {
                restore_task_state(state, &task.task_id, "running", "running", "executing").await?;
            }
            ReconciledTaskState::Terminal => {
                let Some(result) = &task.result else {
                    interrupt_task(state, &task.task_id, "Agent 未提供最终结果").await?;
                    continue;
                };
                handle_result(state, agent_id, generation, result).await?;
            }
            ReconciledTaskState::Unknown => unreachable!(),
        }
    }
    Ok(())
}

async fn restore_task_state(
    state: &AppState,
    task_id: &str,
    task_status: &str,
    deployment_status: &str,
    phase: &str,
) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE agent_tasks SET status=?,updated_at=? WHERE id=?")
        .bind(task_status)
        .bind(&now)
        .bind(task_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_reconcile"))?;
    sqlx::query("UPDATE deployments SET status=?,phase=?,updated_at=?,version=version+1 WHERE id=(SELECT deployment_id FROM agent_tasks WHERE id=?) AND status IN ('queued','running','canceling')")
        .bind(deployment_status).bind(phase).bind(&now).bind(task_id).execute(state.pool()).await
        .map_err(|_| ApiError::internal("agent_reconcile"))?;
    Ok(())
}

async fn interrupt_task(state: &AppState, task_id: &str, summary: &str) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE agent_tasks SET status='interrupted',finished_at=?,result_json=?,updated_at=? WHERE id=? AND status IN ('delivered','accepted','running','canceling')")
        .bind(&now).bind(serde_json::json!({"error_code":"reconcile_mismatch"}).to_string()).bind(&now).bind(task_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_reconcile"))?;
    finish_deployment_for_task(state, task_id, "interrupted", summary, None).await
}

async fn handle_ack(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    ack: &TaskAck,
) -> ApiResult<()> {
    ensure_current_connection(state, agent_id, generation).await?;
    let now = Utc::now().to_rfc3339();
    let status = match ack.disposition {
        TaskAckDisposition::Accepted | TaskAckDisposition::Duplicate => "accepted",
        TaskAckDisposition::Rejected => "failed",
    };
    let updated = sqlx::query("UPDATE agent_tasks SET status=?,acknowledged_at=COALESCE(acknowledged_at,?),finished_at=CASE WHEN ?='failed' THEN ? ELSE finished_at END,result_json=CASE WHEN ?='failed' THEN ? ELSE result_json END,updated_at=? WHERE id=? AND agent_id=? AND payload_digest=? AND status IN ('delivered','accepted')")
        .bind(status)
        .bind(&now)
        .bind(status)
        .bind(&now)
        .bind(status)
        .bind(serde_json::json!({"error_code":ack.error_code}).to_string())
        .bind(&now)
        .bind(&ack.task_id)
        .bind(agent_id)
        .bind(&ack.payload_digest)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_event"))?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "agent_task_ack_invalid",
            "Agent 任务 ACK 无效",
            "agent_event",
        ));
    }
    if status == "failed" {
        finish_deployment_for_task(state, &ack.task_id, "failed", "Agent 拒绝任务", None).await?;
        finish_node_check_for_task(state, &ack.task_id, None, ack.error_code.as_deref()).await?;
    }
    Ok(())
}

async fn handle_output(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    output: &TaskOutput,
) -> ApiResult<()> {
    persist_sequenced_event(
        state,
        agent_id,
        generation,
        &output.task_id,
        output.sequence,
        "output",
        serde_json::to_value(output).map_err(|_| ApiError::internal("agent_event"))?,
    )
    .await?;
    let stream = match output.stream {
        OutputStream::Stdout => "stdout",
        OutputStream::Stderr => "stderr",
    };
    sqlx::query("INSERT OR IGNORE INTO deployment_logs(deployment_id,sequence,stream,content,truncated) SELECT deployment_id,?,?,?,0 FROM agent_tasks WHERE id=? AND deployment_id IS NOT NULL")
        .bind(i64::try_from(output.sequence).map_err(|_| ApiError::internal("agent_event"))?)
        .bind(stream)
        .bind(&output.text)
        .bind(&output.task_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_event"))?;
    Ok(())
}

async fn handle_state(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    task_state: &TaskState,
) -> ApiResult<()> {
    persist_sequenced_event(
        state,
        agent_id,
        generation,
        &task_state.task_id,
        task_state.sequence,
        "state",
        serde_json::to_value(task_state).map_err(|_| ApiError::internal("agent_event"))?,
    )
    .await?;
    let (task_status, deployment_status, phase) = match task_state.state {
        TaskLifecycleState::Accepted => ("accepted", "running", "accepted"),
        TaskLifecycleState::Running => ("running", "running", "executing"),
        TaskLifecycleState::Canceling => ("canceling", "canceling", "canceling"),
    };
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE agent_tasks SET status=?,started_at=CASE WHEN ?='running' THEN COALESCE(started_at,?) ELSE started_at END,updated_at=? WHERE id=? AND agent_id=?")
        .bind(task_status).bind(task_status).bind(&now).bind(&now).bind(&task_state.task_id).bind(agent_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    sqlx::query("UPDATE deployments SET status=?,phase=?,started_at=COALESCE(started_at,?),updated_at=?,version=version+1 WHERE id=(SELECT deployment_id FROM agent_tasks WHERE id=?) AND status IN ('queued','running','canceling')")
        .bind(deployment_status).bind(phase).bind(&now).bind(&now).bind(&task_state.task_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    Ok(())
}

async fn handle_result(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    result: &TaskResult,
) -> ApiResult<()> {
    persist_sequenced_event(
        state,
        agent_id,
        generation,
        &result.task_id,
        result.sequence,
        "result",
        serde_json::to_value(result).map_err(|_| ApiError::internal("agent_event"))?,
    )
    .await?;
    let status = match result.status {
        TaskTerminalStatus::Succeeded => "succeeded",
        TaskTerminalStatus::Failed => "failed",
        TaskTerminalStatus::Canceled => "canceled",
        TaskTerminalStatus::Interrupted => "interrupted",
    };
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE agent_tasks SET status=?,finished_at=?,result_json=?,updated_at=? WHERE id=? AND agent_id=?")
        .bind(status).bind(&now).bind(serde_json::to_string(result).map_err(|_| ApiError::internal("agent_event"))?).bind(&now).bind(&result.task_id).bind(agent_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    finish_node_check_for_task(
        state,
        &result.task_id,
        result.data.as_ref(),
        result.error_code.as_deref(),
    )
    .await?;
    finish_deployment_for_task(
        state,
        &result.task_id,
        status,
        result.summary.as_deref().unwrap_or(status),
        result.exit_code,
    )
    .await
}

async fn finish_node_check_for_task(
    state: &AppState,
    task_id: &str,
    data: Option<&Value>,
    error_code: Option<&str>,
) -> ApiResult<()> {
    let source: Option<(String, String)> = sqlx::query_as(
        "SELECT check_row.id,check_row.node_id FROM node_checks check_row JOIN agent_tasks task ON task.node_check_id=check_row.id WHERE task.id=? AND check_row.status='running'",
    )
    .bind(task_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_node_check"))?;
    let Some((check_id, node_id)) = source else {
        return Ok(());
    };
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal("agent_node_check"))?;
    if let Some(data) = data {
        let os_name = data
            .get("os_name")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty());
        let architecture = data
            .get("architecture")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty());
        let disk = data
            .get("disk_available_bytes")
            .and_then(Value::as_u64)
            .and_then(|value| i64::try_from(value).ok());
        let accessible = data.get("work_root_accessible").and_then(Value::as_bool) == Some(true)
            && data.get("secrets_root_accessible").and_then(Value::as_bool) == Some(true);
        if let (Some(os_name), Some(architecture), Some(disk)) = (os_name, architecture, disk)
            && accessible
        {
            sqlx::query("UPDATE node_checks SET status='succeeded',os_name=?,architecture=?,disk_available_bytes=?,capabilities_json=?,finished_at=? WHERE id=? AND status='running'")
                .bind(os_name).bind(architecture).bind(disk).bind(data.to_string()).bind(&now).bind(&check_id)
                .execute(&mut *transaction).await.map_err(|_| ApiError::internal("agent_node_check"))?;
            sqlx::query("UPDATE nodes SET checked_at=?,updated_at=?,version=version+1 WHERE id=?")
                .bind(&now)
                .bind(&now)
                .bind(&node_id)
                .execute(&mut *transaction)
                .await
                .map_err(|_| ApiError::internal("agent_node_check"))?;
            transaction
                .commit()
                .await
                .map_err(|_| ApiError::internal("agent_node_check"))?;
            return Ok(());
        }
    }
    sqlx::query("UPDATE node_checks SET status='failed',failure_code=?,failure_message='Agent 节点检查失败',finished_at=? WHERE id=? AND status='running'")
        .bind(error_code.unwrap_or("invalid_inspection_result")).bind(&now).bind(&check_id)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal("agent_node_check"))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("agent_node_check"))?;
    Ok(())
}

async fn persist_sequenced_event(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    task_id: &str,
    sequence: u64,
    kind: &str,
    payload: Value,
) -> ApiResult<()> {
    ensure_current_connection(state, agent_id, generation).await?;
    let sequence = i64::try_from(sequence).map_err(|_| ApiError::internal("agent_event"))?;
    let payload_json = payload.to_string();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal("agent_event"))?;
    let last: Option<i64> =
        sqlx::query_scalar("SELECT last_sequence FROM agent_tasks WHERE id=? AND agent_id=?")
            .bind(task_id)
            .bind(agent_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
    let Some(last) = last else {
        return Err(ApiError::not_found("agent_event"));
    };
    if sequence <= last {
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT payload_json FROM agent_task_events WHERE task_id=? AND sequence=? AND kind=?",
        )
        .bind(task_id)
        .bind(sequence)
        .bind(kind)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal("agent_event"))?;
        return if existing.as_deref() == Some(payload_json.as_str()) {
            Ok(())
        } else {
            Err(ApiError::conflict(
                "agent_event_conflict",
                "Agent 事件序号冲突",
                "agent_event",
            ))
        };
    }
    if sequence != last + 1 {
        return Err(ApiError::conflict(
            "agent_event_gap",
            "Agent 事件序号不连续",
            "agent_event",
        ));
    }
    sqlx::query(
        "INSERT INTO agent_task_events(task_id,sequence,kind,payload_json) VALUES(?,?,?,?)",
    )
    .bind(task_id)
    .bind(sequence)
    .bind(kind)
    .bind(payload_json)
    .execute(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal("agent_event"))?;
    sqlx::query("UPDATE agent_tasks SET last_sequence=?,updated_at=? WHERE id=? AND agent_id=? AND last_sequence=?")
        .bind(sequence).bind(Utc::now().to_rfc3339()).bind(task_id).bind(agent_id).bind(last)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal("agent_event"))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("agent_event"))?;
    Ok(())
}

async fn ensure_current_connection(
    state: &AppState,
    agent_id: &str,
    generation: i64,
) -> ApiResult<()> {
    let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agents WHERE id=? AND connection_generation=? AND revoked_at IS NULL AND archived_at IS NULL)")
        .bind(agent_id).bind(generation).fetch_one(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    if current {
        Ok(())
    } else {
        Err(ApiError::unauthorized("agent_event"))
    }
}

async fn finish_deployment_for_task(
    state: &AppState,
    task_id: &str,
    status: &str,
    summary: &str,
    exit_code: Option<i32>,
) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE deployments SET status=?,phase=?,result_summary=?,exit_code=?,protocol_complete=1,finished_at=?,updated_at=?,version=version+1 WHERE id=(SELECT deployment_id FROM agent_tasks WHERE id=?) AND status IN ('queued','running','canceling')")
        .bind(status).bind(status).bind(summary).bind(exit_code).bind(&now).bind(&now).bind(task_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    Ok(())
}
