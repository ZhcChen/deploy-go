use std::time::Duration;

use chrono::Utc;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use ulid::Ulid;

use crate::{
    AppState,
    crypto::EncryptedSecret,
    error::{ApiError, ApiResult},
    execution_spec,
    executor::deployment::{ExecutionContext, OutputChunk},
    settings,
};

const MAX_LINE_BYTES: usize = 64 * 1024;
const HARD_MAX_LOG_BYTES: u64 = 1024 * 1024 * 1024;
const EVENT_PREFIX: &str = "DEPLOY_EVENT ";

#[derive(sqlx::FromRow)]
struct RuntimeRow {
    deployment_id: String,
    snapshot_json: String,
    host: String,
    port: i64,
    username: String,
    work_root: String,
    trusted_host_key: Option<String>,
    credential_id: Option<String>,
    algorithm: Option<String>,
    encrypted_private_key: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    key_version: Option<i64>,
    application_status: String,
    target_status: String,
    node_status: String,
}

#[derive(Default)]
struct LogState {
    sequence: i64,
    total: u64,
    finished: Option<String>,
    protocol_conflict: bool,
    line_truncated: bool,
    budget_exceeded: bool,
    stdout_buffer: Vec<u8>,
    stderr_buffer: Vec<u8>,
}

pub async fn recover(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query("UPDATE deployments SET status='interrupted',phase='interrupted',result_summary='API 重启时远端状态未知',finished_at=?,updated_at=?,version=version+1 WHERE status IN ('running','canceling')")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn process_one(state: &AppState) -> ApiResult<Option<String>> {
    let Some(id) = claim_next(state.pool())
        .await
        .map_err(|_| ApiError::internal("worker"))?
    else {
        return Ok(None);
    };
    let result = execute_claimed(state, &id).await;
    if let Err(error) = result {
        finish_internal_error(state.pool(), &id).await?;
        tracing::warn!(deployment_id = %id, error = ?error, "部署执行失败");
    }
    Ok(Some(id))
}

pub async fn run_worker(state: AppState) {
    if let Err(error) = recover(state.pool()).await {
        tracing::error!(error = %error, "部署恢复失败");
        return;
    }
    let mut workers = tokio::task::JoinSet::new();
    let mut last_retention = tokio::time::Instant::now() - Duration::from_secs(3600);
    loop {
        if last_retention.elapsed() >= Duration::from_secs(3600) {
            if let Err(error) = purge_expired_output(&state).await {
                tracing::warn!(error = ?error, "部署日志保留清理失败");
            }
            last_retention = tokio::time::Instant::now();
        }
        let limit = settings::load(state.pool(), "worker")
            .await
            .unwrap_or_default()
            .max_concurrent_deployments as usize;
        while workers.len() < limit {
            let state = state.clone();
            workers.spawn(async move { process_one(&state).await });
        }
        let claimed = matches!(workers.join_next().await, Some(Ok(Ok(Some(_)))));
        if !claimed {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

pub async fn purge_expired_output(state: &AppState) -> ApiResult<u64> {
    let days = settings::load(state.pool(), "retention")
        .await?
        .log_retention_days;
    let modifier = format!("-{days} days");
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal("retention"))?;
    sqlx::query("DELETE FROM deployment_events WHERE deployment_id IN (SELECT id FROM deployments WHERE status IN ('succeeded','failed','canceled','interrupted') AND datetime(finished_at) < datetime('now', ?))")
        .bind(&modifier).execute(&mut *transaction).await.map_err(|_| ApiError::internal("retention"))?;
    let deleted = sqlx::query("DELETE FROM deployment_logs WHERE deployment_id IN (SELECT id FROM deployments WHERE status IN ('succeeded','failed','canceled','interrupted') AND datetime(finished_at) < datetime('now', ?))")
        .bind(&modifier).execute(&mut *transaction).await.map_err(|_| ApiError::internal("retention"))?.rows_affected();
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("retention"))?;
    Ok(deleted)
}

async fn claim_next(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query_scalar("UPDATE deployments SET status='running',phase='executing',started_at=?,updated_at=?,version=version+1 WHERE id=(SELECT d.id FROM deployments d WHERE d.status='queued' AND NOT EXISTS (SELECT 1 FROM deployments active WHERE active.target_id=d.target_id AND active.status IN ('running','canceling')) ORDER BY d.queued_at,d.id LIMIT 1) AND status='queued' RETURNING id")
        .bind(&now)
        .bind(&now)
        .fetch_optional(pool)
        .await;
    match result {
        Ok(id) => Ok(id),
        Err(error) if error.to_string().contains("UNIQUE constraint failed") => Ok(None),
        Err(error) => Err(error),
    }
}

async fn execute_claimed(state: &AppState, id: &str) -> ApiResult<()> {
    let row: RuntimeRow = sqlx::query_as("SELECT d.id AS deployment_id,d.snapshot_json,n.host,n.port,n.username,n.work_root,n.trusted_host_key,n.ssh_credential_id AS credential_id,c.algorithm,c.encrypted_private_key,c.nonce,c.key_version,a.status AS application_status,t.status AS target_status,n.status AS node_status FROM deployments d JOIN deployment_targets t ON t.id=d.target_id JOIN applications a ON a.id=t.application_id JOIN nodes n ON n.id=t.node_id LEFT JOIN ssh_credentials c ON c.id=n.ssh_credential_id WHERE d.id=?")
        .bind(id)
        .fetch_one(state.pool())
        .await
        .map_err(|_| ApiError::internal("worker"))?;
    let context = build_context(state, &row, true)?;
    let redactions = context
        .environment
        .iter()
        .map(|(_, value)| value.clone())
        .collect::<Vec<_>>();
    let (output_tx, output_rx) = tokio::sync::mpsc::channel(32);
    let execution = state.deployment_executor().execute(&context, output_tx);
    let persistence = consume_output(state.pool(), id, output_rx, &redactions);
    let (result, logs) = tokio::join!(execution, persistence);
    let logs = logs?;
    match result {
        Ok(exit_code) => finish_result(state.pool(), id, exit_code, logs).await,
        Err(error) => finish_execution_error(state.pool(), id, error.code).await,
    }
}

fn build_context(
    state: &AppState,
    row: &RuntimeRow,
    enforce_preconditions: bool,
) -> ApiResult<ExecutionContext> {
    if enforce_preconditions
        && (row.application_status != "active"
            || row.target_status != "active"
            || row.node_status != "online")
    {
        return Err(ApiError::conflict(
            "deployment_precondition_changed",
            "部署前置条件已经变化",
            "worker",
        ));
    }
    let snapshot: Value =
        serde_json::from_str(&row.snapshot_json).map_err(|_| ApiError::internal("worker"))?;
    let target = snapshot
        .get("target")
        .ok_or_else(|| ApiError::internal("worker"))?;
    let parameters = snapshot
        .get("parameters")
        .ok_or_else(|| ApiError::internal("worker"))?;
    let credential_id = row
        .credential_id
        .as_deref()
        .ok_or_else(|| ApiError::internal("worker"))?;
    let algorithm = row
        .algorithm
        .as_deref()
        .ok_or_else(|| ApiError::internal("worker"))?;
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::internal("worker"))?;
    let private_key = ring
        .decrypt(
            credential_id,
            algorithm,
            &EncryptedSecret {
                ciphertext: row
                    .encrypted_private_key
                    .clone()
                    .ok_or_else(|| ApiError::internal("worker"))?,
                nonce: row
                    .nonce
                    .clone()
                    .ok_or_else(|| ApiError::internal("worker"))?,
                key_version: row
                    .key_version
                    .ok_or_else(|| ApiError::internal("worker"))?,
            },
        )
        .map_err(|_| ApiError::internal("worker"))?;
    let environment = target
        .get("secret_file_references")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|item| {
            Ok((
                item.get("environment_key")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::internal("worker"))?
                    .to_owned(),
                item.get("file_path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::internal("worker"))?
                    .to_owned(),
            ))
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(ExecutionContext {
        deployment_id: row.deployment_id.clone(),
        host: row.host.clone(),
        port: u16::try_from(row.port).map_err(|_| ApiError::internal("worker"))?,
        username: row.username.clone(),
        work_root: row.work_root.clone(),
        script_path: target
            .get("script_path")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::internal("worker"))?
            .to_owned(),
        argument_tokens: execution_spec::parameter_tokens(parameters, "worker")?,
        environment,
        trusted_host_key: row
            .trusted_host_key
            .clone()
            .ok_or_else(|| ApiError::internal("worker"))?,
        private_key,
        timeout: Duration::from_secs(
            target
                .get("timeout_seconds")
                .and_then(Value::as_u64)
                .ok_or_else(|| ApiError::internal("worker"))?,
        ),
    })
}

pub async fn cancel_remote(state: &AppState, id: &str) -> ApiResult<()> {
    let row: RuntimeRow = sqlx::query_as("SELECT d.id AS deployment_id,d.snapshot_json,n.host,n.port,n.username,n.work_root,n.trusted_host_key,n.ssh_credential_id AS credential_id,c.algorithm,c.encrypted_private_key,c.nonce,c.key_version,a.status AS application_status,t.status AS target_status,n.status AS node_status FROM deployments d JOIN deployment_targets t ON t.id=d.target_id JOIN applications a ON a.id=t.application_id JOIN nodes n ON n.id=t.node_id LEFT JOIN ssh_credentials c ON c.id=n.ssh_credential_id WHERE d.id=?")
        .bind(id).fetch_one(state.pool()).await.map_err(|_| ApiError::internal("cancel"))?;
    let context = match build_context(state, &row, false) {
        Ok(context) => context,
        Err(error) => {
            finish(state.pool(), id, "interrupted", "无法构造远端取消请求").await?;
            return Err(error);
        }
    };
    match state.deployment_executor().cancel(&context).await {
        Ok(()) => Ok(()),
        Err(_) => {
            finish(state.pool(), id, "interrupted", "无法确认远端取消结果").await?;
            Err(ApiError::conflict(
                "cancel_unconfirmed",
                "无法确认远端取消结果",
                "cancel",
            ))
        }
    }
}

async fn consume_output(
    pool: &SqlitePool,
    id: &str,
    mut output: tokio::sync::mpsc::Receiver<OutputChunk>,
    redactions: &[String],
) -> ApiResult<LogState> {
    let max_bytes = settings::load(pool, "worker")
        .await?
        .max_log_bytes
        .min(HARD_MAX_LOG_BYTES);
    let mut logs = LogState::default();
    while let Some(chunk) = output.recv().await {
        let mut transaction = pool
            .begin()
            .await
            .map_err(|_| ApiError::internal("worker"))?;
        persist_chunk(
            &mut transaction,
            id,
            &mut logs,
            max_bytes,
            chunk,
            redactions,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|_| ApiError::internal("worker"))?;
    }
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("worker"))?;
    flush_partial_lines(&mut transaction, id, &mut logs, max_bytes, redactions).await?;
    if logs.line_truncated {
        persist_diagnostic(&mut transaction, id, "line_truncated", json!({})).await?;
    }
    if logs.budget_exceeded {
        persist_diagnostic(&mut transaction, id, "log_budget_exceeded", json!({})).await?;
    }
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("worker"))?;
    Ok(logs)
}

async fn finish_result(
    pool: &SqlitePool,
    id: &str,
    exit_code: i32,
    logs: LogState,
) -> ApiResult<()> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("worker"))?;
    let canceled: bool =
        sqlx::query_scalar("SELECT status='canceling' FROM deployments WHERE id=?")
            .bind(id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal("worker"))?;
    let outcome = final_outcome(
        exit_code,
        logs.finished.as_deref(),
        logs.protocol_conflict,
        canceled,
    );
    if outcome.conflict {
        persist_diagnostic(&mut transaction, id, "protocol_conflict", json!({})).await?;
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE deployments SET status=?,phase=?,result_summary=?,exit_code=?,protocol_complete=?,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('running','canceling')")
        .bind(outcome.status).bind(outcome.status).bind(outcome.summary).bind(exit_code).bind(outcome.complete).bind(&now).bind(&now).bind(id)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal("worker"))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("worker"))?;
    Ok(())
}

async fn persist_chunk(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    logs: &mut LogState,
    max_bytes: u64,
    chunk: OutputChunk,
    redactions: &[String],
) -> ApiResult<()> {
    let lines = {
        let buffer = match chunk.stream {
            "stdout" => &mut logs.stdout_buffer,
            "stderr" => &mut logs.stderr_buffer,
            _ => return Err(ApiError::internal("worker")),
        };
        buffer.extend_from_slice(&chunk.bytes);
        drain_complete_lines(buffer)
    };
    for line in lines {
        persist_line(
            transaction,
            id,
            logs,
            max_bytes,
            chunk.stream,
            &line,
            redactions,
        )
        .await?;
    }
    Ok(())
}

async fn flush_partial_lines(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    logs: &mut LogState,
    max_bytes: u64,
    redactions: &[String],
) -> ApiResult<()> {
    for (stream, line) in [
        ("stdout", std::mem::take(&mut logs.stdout_buffer)),
        ("stderr", std::mem::take(&mut logs.stderr_buffer)),
    ] {
        if !line.is_empty() {
            persist_line(transaction, id, logs, max_bytes, stream, &line, redactions).await?;
        }
    }
    Ok(())
}

fn drain_complete_lines(buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let Some(last_newline) = buffer.iter().rposition(|byte| *byte == b'\n') else {
        return Vec::new();
    };
    let tail = buffer.split_off(last_newline + 1);
    let complete = std::mem::replace(buffer, tail);
    complete[..complete.len() - 1]
        .split(|byte| *byte == b'\n')
        .map(<[u8]>::to_vec)
        .collect()
}

async fn persist_line(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    logs: &mut LogState,
    max_bytes: u64,
    stream: &'static str,
    bytes: &[u8],
    redactions: &[String],
) -> ApiResult<()> {
    if logs.total >= max_bytes {
        logs.budget_exceeded = true;
        return Ok(());
    }
    let remaining = max_bytes - logs.total;
    let allowed = usize::try_from(remaining)
        .unwrap_or(usize::MAX)
        .min(MAX_LINE_BYTES);
    let end = bytes.len().min(allowed);
    let mut content = String::from_utf8_lossy(&bytes[..end]).into_owned();
    for secret in redactions.iter().filter(|secret| !secret.is_empty()) {
        content = content.replace(secret, "[REDACTED]");
    }
    let truncated = end < bytes.len();
    logs.line_truncated |= truncated;
    logs.budget_exceeded |= truncated && remaining <= MAX_LINE_BYTES as u64;
    logs.sequence += 1;
    logs.total += content.len() as u64;
    sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,stream,content,truncated) VALUES(?,?,?,?,?)")
        .bind(id).bind(logs.sequence).bind(stream).bind(&content).bind(truncated)
        .execute(&mut **transaction).await.map_err(|_| ApiError::internal("worker"))?;
    if !truncated && let Some(raw) = content.strip_prefix(EVENT_PREFIX) {
        persist_event(transaction, id, logs.sequence, raw, logs).await?;
    }
    if std::str::from_utf8(bytes).is_err() {
        persist_diagnostic(transaction, id, "invalid_utf8", json!({"stream":stream})).await?;
    }
    Ok(())
}

async fn persist_event(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    sequence: i64,
    raw: &str,
    logs: &mut LogState,
) -> ApiResult<()> {
    let parsed: Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(_) => return persist_diagnostic(transaction, id, "malformed_event", json!({})).await,
    };
    let Some(name) = parsed.get("event").and_then(Value::as_str) else {
        return persist_diagnostic(transaction, id, "malformed_event", json!({})).await;
    };
    let valid_envelope = parsed.get("schema_version").and_then(Value::as_i64) == Some(1)
        && parsed
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .is_some()
        && matches!(
            parsed.get("status").and_then(Value::as_str),
            Some("running" | "succeeded" | "failed" | "canceled")
        )
        && parsed
            .get("deploy_id")
            .and_then(Value::as_str)
            .is_none_or(|deploy_id| deploy_id == id);
    if !valid_envelope {
        return persist_diagnostic(transaction, id, "malformed_event", json!({})).await;
    }
    let known = matches!(
        name,
        "deploy.started"
            | "deploy.preflight.started"
            | "deploy.preflight.succeeded"
            | "deploy.preflight.failed"
            | "deploy.step.started"
            | "deploy.step.succeeded"
            | "deploy.step.failed"
            | "deploy.verification.started"
            | "deploy.verification.succeeded"
            | "deploy.verification.failed"
            | "deploy.finished"
    );
    let diagnostic = (!known).then_some("unknown_event");
    let status = parsed.get("status").and_then(Value::as_str);
    if name == "deploy.finished" {
        if logs
            .finished
            .as_deref()
            .is_some_and(|current| Some(current) != status)
        {
            logs.protocol_conflict = true;
            // Keep the failure-side fact when scripts emit contradictory terminal events.
            if logs.finished.as_deref() != Some("failed") {
                logs.finished = status.map(str::to_owned);
            }
            return Ok(());
        }
        logs.finished = status.map(str::to_owned);
    }
    sqlx::query("INSERT INTO deployment_events(id,deployment_id,log_sequence,event_name,status,payload_json,diagnostic_code) VALUES(?,?,?,?,?,?,?)")
        .bind(format!("event_{}", Ulid::new())).bind(id).bind(sequence).bind(name).bind(status).bind(parsed.to_string()).bind(diagnostic)
        .execute(&mut **transaction).await.map_err(|_| ApiError::internal("worker"))?;
    Ok(())
}

async fn persist_diagnostic(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    code: &str,
    payload: Value,
) -> ApiResult<()> {
    sqlx::query("INSERT INTO deployment_events(id,deployment_id,event_name,payload_json,diagnostic_code) VALUES(?,?, 'diagnostic', ?, ?)")
        .bind(format!("event_{}", Ulid::new())).bind(id).bind(payload.to_string()).bind(code)
        .execute(&mut **transaction).await.map_err(|_| ApiError::internal("worker"))?;
    Ok(())
}

struct Outcome {
    status: &'static str,
    complete: bool,
    summary: &'static str,
    conflict: bool,
}

fn final_outcome(
    exit_code: i32,
    finished: Option<&str>,
    existing_conflict: bool,
    canceled: bool,
) -> Outcome {
    let exit_conflict = matches!(finished, Some("succeeded")) && exit_code != 0
        || matches!(finished, Some("failed")) && exit_code == 0
        || matches!(finished, Some("canceled")) && !canceled;
    let conflict = existing_conflict || exit_conflict;
    if canceled && !matches!(finished, Some("succeeded" | "failed")) {
        return Outcome {
            status: "canceled",
            complete: matches!(finished, Some("canceled")),
            summary: "部署已取消",
            conflict,
        };
    }
    if conflict || exit_code != 0 || matches!(finished, Some("failed")) {
        return Outcome {
            status: "failed",
            complete: finished.is_some() && !conflict,
            summary: "部署脚本执行失败",
            conflict,
        };
    }
    if matches!(finished, Some("succeeded")) {
        Outcome {
            status: "succeeded",
            complete: true,
            summary: "部署成功",
            conflict,
        }
    } else {
        Outcome {
            status: "succeeded",
            complete: false,
            summary: "脚本成功退出，但部署协议不完整",
            conflict,
        }
    }
}

async fn finish_execution_error(pool: &SqlitePool, id: &str, code: &str) -> ApiResult<()> {
    let status = if matches!(code, "timeout" | "process_io_failed") {
        "interrupted"
    } else {
        "failed"
    };
    finish(pool, id, status, code).await
}
async fn finish_internal_error(pool: &SqlitePool, id: &str) -> ApiResult<()> {
    finish(pool, id, "interrupted", "部署执行上下文不可用").await
}
async fn finish(pool: &SqlitePool, id: &str, status: &str, summary: &str) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE deployments SET status=?,phase=?,result_summary=?,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('running','canceling')")
        .bind(status).bind(status).bind(summary).bind(&now).bind(&now).bind(id).execute(pool).await.map_err(|_|ApiError::internal("worker"))?;
    Ok(())
}
