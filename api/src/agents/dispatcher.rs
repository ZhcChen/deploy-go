use chrono::{Duration, Utc};
use deploy_go_agent_protocol::{
    DeploymentExecuteTask, EnvironmentFileReference, Message, TaskDispatch, TaskPayload,
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
