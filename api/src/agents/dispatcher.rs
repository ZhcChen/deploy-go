use chrono::{Duration, Utc};
use deploy_go_agent_protocol::{
    DeploymentExecuteTask, DeploymentPrepareTask, DeploymentReleaseTask, Environment,
    EnvironmentFileReference, MakeTarget, Message, OutputStream, ReconcileReport,
    ReconciledTaskState, SecretLeaseRequest, SecretLeaseResponse, SourcePolicy, SystemInspectTask,
    TaskAck, TaskAckDisposition, TaskDispatch, TaskLifecycleState, TaskOutput, TaskPayload,
    TaskProgress, TaskResult, TaskState, TaskTerminalStatus,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use ulid::Ulid;

use crate::{
    AppState,
    crypto::EncryptedSecret,
    error::{ApiError, ApiResult},
    execution_spec, settings,
};

const REFS_RESULT_TTL_SECONDS: i64 = 900;
const MAX_REFS: usize = 1024;

#[derive(sqlx::FromRow)]
struct DeploymentTaskSource {
    deployment_id: String,
    snapshot_json: String,
    agent_id: String,
    work_root: Option<String>,
    secrets_root: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DispatchRow {
    agent_id: String,
    idempotency_key: String,
    payload_digest: String,
    payload_json: String,
    deadline_at: String,
    kind: String,
    protocol_version: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct CredentialLeaseRow {
    git_credential_id: String,
    algorithm: String,
    encrypted_private_key: Vec<u8>,
    nonce: Vec<u8>,
    key_version: i64,
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
    let candidate: Option<(String, String)> = sqlx::query_as(
        "SELECT d.id,target.execution_mode FROM deployments d JOIN deployment_targets target ON target.id=d.target_id JOIN applications application ON application.id=target.application_id JOIN nodes node ON node.id=target.node_id JOIN agents agent ON agent.node_id=node.id LEFT JOIN agent_tasks task ON task.deployment_id=d.id WHERE application.status='active' AND target.status='active' AND node.status='online' AND node.work_root IS NOT NULL AND node.secrets_root IS NOT NULL AND agent.revoked_at IS NULL AND agent.archived_at IS NULL AND NOT EXISTS (SELECT 1 FROM deployments active WHERE active.target_id=d.target_id AND active.id!=d.id AND active.status IN ('running','canceling')) AND ((target.execution_mode='script' AND d.status='queued' AND (task.id IS NULL OR (task.status='queued' AND task.updated_at<=?))) OR (target.execution_mode='two_stage' AND ((d.status='queued' AND d.phase='queued' AND NOT EXISTS (SELECT 1 FROM agent_tasks prepare WHERE prepare.deployment_id=d.id AND prepare.stage='prepare')) OR (d.status='running' AND d.phase IN ('preparing','deploying') AND (NOT EXISTS (SELECT 1 FROM agent_tasks stage_task WHERE stage_task.deployment_id=d.id AND stage_task.status IN ('queued','delivered','accepted','running','canceling')) OR EXISTS (SELECT 1 FROM agent_tasks pending WHERE pending.deployment_id=d.id AND pending.status='queued' AND pending.updated_at<=?)))))) ORDER BY d.queued_at,d.id LIMIT 1",
    )
    .bind(&retry_before)
    .bind(&retry_before)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let Some((deployment_id, execution_mode)) = candidate else {
        return Ok(None);
    };
    if execution_mode == "two_stage" {
        if ensure_deployment_task(state, &deployment_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
    } else {
        enqueue_deployment(state, &deployment_id).await?;
    }
    Ok(Some(deployment_id))
}

pub async fn ensure_deployment_task(
    state: &AppState,
    deployment_id: &str,
) -> ApiResult<Option<String>> {
    let deployment: Option<(String, String, String)> = sqlx::query_as(
        "SELECT d.status,d.phase,d.snapshot_json FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE d.id=? AND t.execution_mode='two_stage'",
    )
    .bind(deployment_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let Some((status, _phase, snapshot_json)) = deployment else {
        return Ok(None);
    };
    if matches!(
        status.as_str(),
        "canceled" | "failed" | "succeeded" | "interrupted" | "canceling"
    ) {
        return Ok(None);
    }
    let prepare: Option<(String, String)> = sqlx::query_as(
        "SELECT id,status FROM agent_tasks WHERE deployment_id=? AND stage='prepare'",
    )
    .bind(deployment_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_dispatch"))?;
    if let Some((task_id, task_status)) = prepare {
        if task_status == "succeeded" {
            let release: Option<(String, String)> = sqlx::query_as(
                "SELECT id,status FROM agent_tasks WHERE deployment_id=? AND stage='release'",
            )
            .bind(deployment_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_dispatch"))?;
            if let Some((release_id, release_status)) = release {
                if matches!(release_status.as_str(), "queued" | "delivered") {
                    try_dispatch(state, &release_id).await?;
                }
                return Ok(Some(release_id));
            }
            if let Some(task_id) =
                create_stage_task(state, deployment_id, "release", &snapshot_json).await?
            {
                try_dispatch(state, &task_id).await?;
                return Ok(Some(task_id));
            }
            return Ok(None);
        }
        if matches!(
            task_status.as_str(),
            "queued" | "delivered" | "accepted" | "running" | "canceling"
        ) {
            try_dispatch(state, &task_id).await?;
            return Ok(Some(task_id));
        }
        return Ok(None);
    }
    if status != "queued" {
        return Ok(None);
    }
    if let Some(task_id) =
        create_stage_task(state, deployment_id, "prepare", &snapshot_json).await?
    {
        try_dispatch(state, &task_id).await?;
        return Ok(Some(task_id));
    }
    Ok(None)
}

async fn create_stage_task(
    state: &AppState,
    deployment_id: &str,
    stage: &str,
    snapshot_json: &str,
) -> ApiResult<Option<String>> {
    let snapshot: Value =
        serde_json::from_str(snapshot_json).map_err(|_| ApiError::internal("agent_dispatch"))?;
    let target = snapshot
        .get("target")
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let environment = target
        .get("environment")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let timeout_seconds = target
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let protocol_environment = match environment {
        "dev" => Environment::Dev,
        "test" => Environment::Test,
        "staging" => Environment::Staging,
        "prod" => Environment::Production,
        _ => {
            return Err(ApiError::internal("agent_dispatch"));
        }
    };
    let source = snapshot
        .get("source")
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let repository_url = source
        .get("repository_url")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let commit_sha = source
        .get("resolved_commit_sha")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let build_agent_id = source
        .get("build_agent_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let git_credential_id = source.get("git_credential_id").and_then(Value::as_str);
    let two_stage = snapshot
        .get("two_stage")
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let release_version = two_stage
        .get("release_version")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let modules = two_stage
        .get("modules")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .map(Value::as_str)
                .map(|value| value.map(str::to_owned))
                .collect::<Option<Vec<String>>>()
        })
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;

    let (agent_id, work_root) = if stage == "prepare" {
        let agent: Option<(String, String)> = sqlx::query_as(
            "SELECT a.id,n.work_root FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.id=? AND a.revoked_at IS NULL AND a.archived_at IS NULL AND a.protocol_version>=2 AND n.status='online' AND n.work_root IS NOT NULL",
        )
        .bind(build_agent_id)
        .fetch_optional(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
        let Some((agent_id, work_root)) = agent else {
            return Ok(None);
        };
        (agent_id, work_root)
    } else {
        let target_node_id = target
            .get("node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        let agent: Option<(String, String)> = sqlx::query_as(
            "SELECT a.id,n.work_root FROM nodes n JOIN agents a ON a.node_id=n.id WHERE n.id=? AND n.status='online' AND n.work_root IS NOT NULL AND a.revoked_at IS NULL AND a.archived_at IS NULL AND a.protocol_version>=2",
        )
        .bind(target_node_id)
        .fetch_optional(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
        let Some((agent_id, work_root)) = agent else {
            return Ok(None);
        };
        (agent_id, work_root)
    };
    if work_root.is_empty() {
        return Ok(None);
    }
    let deployment_root = PathBuf::from(&work_root)
        .join("deployments")
        .join(deployment_id);
    let checkout_dir = deployment_root.join("checkout");
    let staging_dir = deployment_root.join("staging");
    let task_id = format!("task_{}", Ulid::new());
    let lease_id = (stage == "prepare" && git_credential_id.is_some())
        .then(|| format!("lease_{}", Ulid::new()));
    let payload = if stage == "prepare" {
        TaskPayload::DeploymentPrepare(DeploymentPrepareTask {
            deployment_id: deployment_id.to_owned(),
            source_policy: SourcePolicy::Branch,
            repository_url: repository_url.to_owned(),
            commit_sha: commit_sha.to_owned(),
            checkout_dir: checkout_dir.to_string_lossy().into_owned(),
            work_root: work_root.clone(),
            output_dir: staging_dir.to_string_lossy().into_owned(),
            environment: protocol_environment,
            release_version: release_version.to_owned(),
            modules,
            make_target: MakeTarget::DeployGoPrepare,
            git_credential_lease_id: lease_id.clone(),
            timeout_seconds,
        })
    } else {
        TaskPayload::DeploymentRelease(DeploymentReleaseTask {
            deployment_id: deployment_id.to_owned(),
            target_code: environment.to_owned(),
            work_root,
            checkout_dir: checkout_dir.to_string_lossy().into_owned(),
            artifact_dir: staging_dir.to_string_lossy().into_owned(),
            environment: protocol_environment,
            release_version: release_version.to_owned(),
            commit_sha: commit_sha.to_owned(),
            modules,
            make_target: MakeTarget::DeployGoRelease,
            timeout_seconds,
            cancel_file: String::new(),
        })
    };
    let payload_json =
        serde_json::to_string(&payload).map_err(|_| ApiError::internal("agent_dispatch"))?;
    let payload_digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    let deadline_at =
        (Utc::now() + Duration::seconds(i64::from(timeout_seconds) + 60)).to_rfc3339();
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let insert = sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,?,?,?,?,?,?,?,'queued',?)")
        .bind(&task_id)
        .bind(&agent_id)
        .bind(deployment_id)
        .bind(stage)
        .bind(if stage == "prepare" {
            "deployment_prepare"
        } else {
            "deployment_release"
        })
        .bind(format!("deployment:{deployment_id}:{stage}"))
        .bind(&payload_digest)
        .bind(&payload_json)
        .bind(&deadline_at)
        .execute(&mut *transaction)
        .await;
    if let Err(error) = insert {
        if error.to_string().contains("UNIQUE constraint failed") {
            drop(transaction);
            let existing: Option<String> =
                sqlx::query_scalar("SELECT id FROM agent_tasks WHERE deployment_id=? AND stage=?")
                    .bind(deployment_id)
                    .bind(stage)
                    .fetch_optional(state.pool())
                    .await
                    .map_err(|_| ApiError::internal("agent_dispatch"))?;
            return Ok(Some(existing.unwrap_or(task_id)));
        }
        return Err(ApiError::internal("agent_dispatch"));
    }
    if let (Some(lease_id), Some(credential_id)) = (lease_id.as_deref(), git_credential_id) {
        sqlx::query("INSERT INTO git_secret_leases(id,task_id,git_credential_id,payload_digest,purpose,status,expires_at) VALUES(?,?,?,?,'git_credential','issued',?)")
            .bind(lease_id)
            .bind(&task_id)
            .bind(credential_id)
            .bind(&payload_digest)
            .bind((Utc::now() + Duration::seconds(60)).to_rfc3339())
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal("agent_dispatch"))?;
    }
    let updated = if stage == "prepare" {
        sqlx::query("UPDATE deployments SET status='running',phase='preparing',updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running')")
            .bind(&now)
            .bind(deployment_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal("agent_dispatch"))?
    } else {
        sqlx::query("UPDATE deployments SET status='running',phase='deploying',updated_at=?,version=version+1 WHERE id=? AND status='running' AND phase IN ('preparing','deploying')")
            .bind(&now)
            .bind(deployment_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal("agent_dispatch"))?
    };
    if updated.rows_affected() != 1 {
        drop(transaction);
        return Ok(Some(task_id));
    }
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
    Ok(Some(task_id))
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
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE deployments SET status='canceled',phase='canceled',cancel_requested_at=COALESCE(cancel_requested_at,?),finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running','canceling') AND NOT EXISTS (SELECT 1 FROM agent_tasks t WHERE t.deployment_id=? AND t.status IN ('queued','delivered','accepted','running','canceling'))")
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .bind(deployment_id)
            .bind(deployment_id)
            .execute(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_cancel"))?;
        return Ok(false);
    };
    let now = Utc::now().to_rfc3339();
    if status == "queued" {
        sqlx::query("UPDATE agent_tasks SET status='canceled',finished_at=?,result_json=?,updated_at=? WHERE id=? AND status='queued'")
            .bind(&now).bind(serde_json::json!({"error_code":"canceled_before_delivery"}).to_string()).bind(&now).bind(&task_id)
            .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_cancel"))?;
        sqlx::query("UPDATE deployments SET status='canceled',phase='canceled',cancel_requested_at=COALESCE(cancel_requested_at,?),finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running','canceling')")
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .bind(deployment_id)
            .execute(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_cancel"))?;
        return Ok(false);
    }
    if !matches!(
        status.as_str(),
        "delivered" | "accepted" | "running" | "canceling"
    ) {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE deployments SET status='canceled',phase='canceled',cancel_requested_at=COALESCE(cancel_requested_at,?),finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running','canceling') AND NOT EXISTS (SELECT 1 FROM agent_tasks t WHERE t.deployment_id=? AND t.status IN ('queued','delivered','accepted','running','canceling'))")
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .bind(deployment_id)
            .bind(deployment_id)
            .execute(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_cancel"))?;
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
    let row: Option<DispatchRow> = sqlx::query_as(
        "SELECT t.agent_id,t.idempotency_key,t.payload_digest,t.payload_json,t.deadline_at,t.kind,a.protocol_version FROM agent_tasks t JOIN agents a ON a.id=t.agent_id WHERE t.id=? AND t.status IN ('queued','delivered')",
    )
    .bind(task_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let Some(row) = row else {
        return Ok(false);
    };
    if matches!(
        row.kind.as_str(),
        "git_refs_query" | "deployment_prepare" | "deployment_release"
    ) && row.protocol_version.unwrap_or_default() < 2
    {
        return Ok(false);
    }
    let payload = serde_json::from_str::<TaskPayload>(&row.payload_json)
        .map_err(|_| ApiError::internal("agent_dispatch"))?;
    let message = Message::TaskDispatch(TaskDispatch {
        task_id: task_id.to_owned(),
        idempotency_key: row.idempotency_key,
        deadline_at: row.deadline_at,
        payload_digest: row.payload_digest,
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
        .send(&row.agent_id, message)
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
    expire_secret_leases(state).await?;
    Ok(result.rows_affected())
}

pub async fn expire_secret_leases(state: &AppState) -> ApiResult<u64> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE git_secret_leases SET status='expired' WHERE status='issued' AND expires_at<=?",
    )
    .bind(&now)
    .execute(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_lease"))?;
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
        Message::TaskProgress(progress) => {
            handle_progress(state, agent_id, connection_generation, progress).await?;
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
        Message::SecretLeaseRequest(request) => {
            handle_secret_lease_request(state, agent_id, connection_generation, request).await?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

async fn handle_progress(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    progress: &TaskProgress,
) -> ApiResult<()> {
    ensure_current_connection(state, agent_id, generation).await?;
    let deployment_id: Option<String> = sqlx::query_scalar(
        "SELECT deployment_id FROM agent_tasks WHERE id=? AND agent_id=? AND deployment_id IS NOT NULL",
    )
    .bind(&progress.task_id)
    .bind(agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_event"))?;
    let Some(deployment_id) = deployment_id else {
        return Ok(());
    };
    if deployment_id != progress.event.deploy_id {
        return Err(ApiError::conflict(
            "agent_event_conflict",
            "Agent 进度事件与任务部署不一致",
            "agent_event",
        ));
    }
    let inserted = persist_sequenced_event(
        state,
        agent_id,
        generation,
        &progress.task_id,
        progress.sequence,
        "progress",
        serde_json::to_value(&progress.event).map_err(|_| ApiError::internal("agent_event"))?,
    )
    .await?;
    if !inserted {
        return Ok(());
    }
    let event =
        serde_json::to_value(&progress.event).map_err(|_| ApiError::internal("agent_event"))?;
    let event_name = event
        .get("event")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_event"))?;
    let status = event
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_event"))?;
    sqlx::query("INSERT INTO deployment_events(id,deployment_id,event_name,status,payload_json) SELECT ?,deployment_id,?,?,? FROM agent_tasks WHERE id=? AND deployment_id IS NOT NULL")
        .bind(format!("event_{}", Ulid::new()))
        .bind(event_name)
        .bind(status)
        .bind(event.to_string())
        .bind(&progress.task_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_event"))?;
    if progress.event.stage == deploy_go_agent_protocol::DeploymentStage::Release
        && event_name.starts_with("deploy.verification.")
    {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE deployments SET phase='verifying',updated_at=?,version=version+1 WHERE id=(SELECT deployment_id FROM agent_tasks WHERE id=?) AND status='running'")
            .bind(&now)
            .bind(&progress.task_id)
            .execute(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
    }
    Ok(())
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

async fn handle_secret_lease_request(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    request: &SecretLeaseRequest,
) -> ApiResult<()> {
    ensure_current_connection(state, agent_id, generation).await?;
    let response = resolve_secret_lease(state, agent_id, request).await?;
    if state
        .agent_connections()
        .send(agent_id, Message::SecretLeaseResponse(response))
        .await
        .is_err()
    {
        reissue_secret_lease(state, &request.lease_id).await?;
        return Err(ApiError::conflict(
            "agent_lease_delivery_failed",
            "密钥租约响应投递失败",
            "agent_lease",
        ));
    }
    Ok(())
}

async fn reissue_secret_lease(state: &AppState, lease_id: &str) -> ApiResult<()> {
    sqlx::query("UPDATE git_secret_leases SET status='issued',granted_at=NULL,expires_at=? WHERE id=? AND status='granted'")
        .bind((Utc::now() + Duration::seconds(60)).to_rfc3339())
        .bind(lease_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_lease"))?;
    Ok(())
}

pub async fn resolve_secret_lease(
    state: &AppState,
    agent_id: &str,
    request: &SecretLeaseRequest,
) -> ApiResult<SecretLeaseResponse> {
    if request.task_id.len() > 128
        || request.lease_id.len() > 128
        || request.payload_digest.len() > 256
    {
        return lease_rejection(state, request, "invalid_request").await;
    }
    let task: Option<(String, String, String)> = sqlx::query_as(
        "SELECT t.payload_digest,t.payload_json,t.status FROM agent_tasks t WHERE t.id=? AND t.agent_id=? AND t.status IN ('delivered','accepted','running','canceling')",
    )
    .bind(&request.task_id)
    .bind(agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_lease"))?;
    let Some((payload_digest, payload_json, _)) = task else {
        return lease_rejection(state, request, "task_not_active").await;
    };
    if payload_digest != request.payload_digest {
        return lease_rejection(state, request, "payload_mismatch").await;
    }
    let payload = serde_json::from_str::<TaskPayload>(&payload_json)
        .map_err(|_| ApiError::internal("agent_lease"))?;
    let configured_lease = match &payload {
        TaskPayload::GitRefsQuery(task) => task.git_credential_lease_id.clone(),
        TaskPayload::DeploymentPrepare(task) => task.git_credential_lease_id.clone(),
        _ => None,
    };
    if configured_lease.as_deref() != Some(request.lease_id.as_str()) {
        return lease_rejection(state, request, "lease_not_bound").await;
    }

    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal("agent_lease"))?;
    let granted = sqlx::query(
        "UPDATE git_secret_leases SET status='granted',granted_at=? WHERE id=? AND task_id=? AND status='issued' AND expires_at>?",
    )
    .bind(&now)
    .bind(&request.lease_id)
    .bind(&request.task_id)
    .bind(&now)
    .execute(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal("agent_lease"))?;
    if granted.rows_affected() != 1 {
        transaction
            .rollback()
            .await
            .map_err(|_| ApiError::internal("agent_lease"))?;
        return lease_rejection(state, request, "lease_not_available").await;
    }
    let credential: Option<CredentialLeaseRow> = sqlx::query_as(
        "SELECT l.git_credential_id,g.algorithm,g.encrypted_private_key,g.nonce,g.key_version FROM git_secret_leases l JOIN git_credentials g ON g.id=l.git_credential_id WHERE l.id=?",
    )
    .bind(&request.lease_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal("agent_lease"))?;
    let Some(credential) = credential else {
        transaction
            .rollback()
            .await
            .map_err(|_| ApiError::internal("agent_lease"))?;
        return lease_rejection(state, request, "credential_unavailable").await;
    };
    let ring = state.master_key_ring().ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "master_key_not_configured",
            "服务未配置主密钥，无法签发 Git 凭证租约",
            "agent_lease",
        )
    })?;
    let plaintext = match ring.decrypt(
        &credential.git_credential_id,
        &credential.algorithm,
        &EncryptedSecret {
            ciphertext: credential.encrypted_private_key,
            nonce: credential.nonce,
            key_version: credential.key_version,
        },
    ) {
        Ok(plaintext) => plaintext,
        Err(_) => {
            transaction
                .rollback()
                .await
                .map_err(|_| ApiError::internal("agent_lease"))?;
            return lease_rejection(state, request, "decryption_failed").await;
        }
    };
    let private_key =
        String::from_utf8(plaintext.to_vec()).map_err(|_| ApiError::internal("agent_lease"))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("agent_lease"))?;
    Ok(SecretLeaseResponse {
        lease_id: request.lease_id.clone(),
        private_key,
        expires_at: now,
        error_code: None,
    })
}

async fn lease_rejection(
    _state: &AppState,
    request: &SecretLeaseRequest,
    code: &str,
) -> ApiResult<SecretLeaseResponse> {
    Ok(SecretLeaseResponse {
        lease_id: request.lease_id.clone(),
        private_key: String::new(),
        expires_at: Utc::now().to_rfc3339(),
        error_code: Some(code.to_owned()),
    })
}

async fn restore_task_state(
    state: &AppState,
    task_id: &str,
    task_status: &str,
    deployment_status: &str,
    phase: &str,
) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    let stage: Option<String> = sqlx::query_scalar("SELECT stage FROM agent_tasks WHERE id=?")
        .bind(task_id)
        .fetch_optional(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_reconcile"))?
        .flatten();
    let resolved_phase = match (stage.as_deref(), task_status) {
        (Some("prepare"), "accepted" | "running") => "preparing",
        (Some("release"), "accepted" | "running") => "deploying",
        _ => phase,
    };
    sqlx::query("UPDATE agent_tasks SET status=?,updated_at=? WHERE id=?")
        .bind(task_status)
        .bind(&now)
        .bind(task_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_reconcile"))?;
    sqlx::query("UPDATE deployments SET status=?,phase=?,updated_at=?,version=version+1 WHERE id=(SELECT deployment_id FROM agent_tasks WHERE id=?) AND status IN ('queued','running','canceling')")
        .bind(deployment_status).bind(resolved_phase).bind(&now).bind(task_id).execute(state.pool()).await
        .map_err(|_| ApiError::internal("agent_reconcile"))?;
    update_refs_discovery_state(state, task_id, task_status).await?;
    Ok(())
}

async fn interrupt_task(state: &AppState, task_id: &str, summary: &str) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE agent_tasks SET status='interrupted',finished_at=?,result_json=?,updated_at=? WHERE id=? AND status IN ('delivered','accepted','running','canceling')")
        .bind(&now).bind(serde_json::json!({"error_code":"reconcile_mismatch"}).to_string()).bind(&now).bind(task_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_reconcile"))?;
    finish_git_ref_discovery_for_task(
        state,
        task_id,
        "interrupted",
        Some("reconcile_mismatch"),
        None,
    )
    .await?;
    expire_task_secret_leases(state, task_id).await?;
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
        finish_git_ref_discovery_for_task(
            state,
            &ack.task_id,
            "failed",
            ack.error_code.as_deref(),
            None,
        )
        .await?;
        expire_task_secret_leases(state, &ack.task_id).await?;
    }
    Ok(())
}

async fn handle_output(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    output: &TaskOutput,
) -> ApiResult<()> {
    let (output, truncated, budget_exceeded) = sanitize_output(state, output).await?;
    let inserted = persist_sequenced_event(
        state,
        agent_id,
        generation,
        &output.task_id,
        output.sequence,
        "output",
        serde_json::to_value(&output).map_err(|_| ApiError::internal("agent_event"))?,
    )
    .await?;
    let stream = match output.stream {
        OutputStream::Stdout => "stdout",
        OutputStream::Stderr => "stderr",
    };
    if inserted && !output.text.is_empty() {
        insert_deployment_log(
            state,
            &output.task_id,
            output.sequence,
            stream,
            &output.text,
            truncated,
        )
        .await?;
    }
    if budget_exceeded {
        sqlx::query("INSERT INTO deployment_events(id,deployment_id,event_name,payload_json,diagnostic_code) SELECT ?,deployment_id,'diagnostic','{}','log_budget_exceeded' FROM agent_tasks WHERE id=? AND deployment_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM deployment_events existing WHERE existing.deployment_id=agent_tasks.deployment_id AND existing.diagnostic_code='log_budget_exceeded')")
            .bind(format!("event_{}", Ulid::new())).bind(&output.task_id)
            .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    }
    Ok(())
}

async fn insert_deployment_log(
    state: &AppState,
    task_id: &str,
    task_sequence: u64,
    stream: &str,
    content: &str,
    truncated: bool,
) -> ApiResult<()> {
    let task_sequence = i64::try_from(task_sequence).map_err(|_| ApiError::internal("agent_event"))?;
    for _ in 0..3 {
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO deployment_logs(deployment_id,task_id,sequence,task_sequence,stream,content,truncated) SELECT deployment_id,?,(SELECT COALESCE(MAX(sequence),0)+1 FROM deployment_logs WHERE deployment_id=agent_tasks.deployment_id),?,?,?,? FROM agent_tasks WHERE id=? AND deployment_id IS NOT NULL",
        )
        .bind(task_id)
        .bind(task_sequence)
        .bind(stream)
        .bind(content)
        .bind(truncated)
        .bind(task_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_event"))?;
        if inserted.rows_affected() == 1 {
            return Ok(());
        }
    }
    Ok(())
}

async fn sanitize_output(
    state: &AppState,
    output: &TaskOutput,
) -> ApiResult<(TaskOutput, bool, bool)> {
    let source: Option<(Option<String>, String)> =
        sqlx::query_as("SELECT deployment_id,payload_json FROM agent_tasks WHERE id=?")
            .bind(&output.task_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
    let Some((deployment_id, payload_json)) = source else {
        return Err(ApiError::not_found("agent_event"));
    };
    let mut text = output.text.clone();
    if let Ok(TaskPayload::DeploymentExecute(payload)) = serde_json::from_str(&payload_json) {
        for reference in payload.environment_file_references {
            text = text.replace(&reference.file_path, "[REDACTED]");
        }
    }
    if let Ok(TaskPayload::GitRefsQuery(payload)) =
        serde_json::from_str::<TaskPayload>(&payload_json)
        && !payload.repository_url.is_empty()
    {
        text = text.replace(&payload.repository_url, "[REDACTED]");
    }
    let Some(deployment_id) = deployment_id else {
        let mut sanitized = output.clone();
        sanitized.text = text;
        return Ok((sanitized, false, false));
    };
    let used: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(LENGTH(CAST(content AS BLOB))),0) FROM deployment_logs WHERE deployment_id=? AND sequence!=?")
        .bind(&deployment_id)
        .bind(i64::try_from(output.sequence).map_err(|_| ApiError::internal("agent_event"))?)
        .fetch_one(state.pool()).await
        .map_err(|_| ApiError::internal("agent_event"))?;
    let maximum = settings::load(state.pool(), "agent_event")
        .await?
        .max_log_bytes;
    let remaining = maximum.saturating_sub(u64::try_from(used).unwrap_or(u64::MAX));
    let truncated = text.len() as u64 > remaining;
    if truncated {
        text = truncate_utf8(&text, usize::try_from(remaining).unwrap_or(usize::MAX)).to_owned();
    }
    let mut sanitized = output.clone();
    sanitized.text = text;
    Ok((sanitized, truncated, truncated))
}

fn truncate_utf8(value: &str, maximum: usize) -> &str {
    if value.len() <= maximum {
        return value;
    }
    let mut end = maximum;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
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
    let stage: Option<String> =
        sqlx::query_scalar("SELECT stage FROM agent_tasks WHERE id=? AND agent_id=?")
            .bind(&task_state.task_id)
            .bind(agent_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?
            .flatten();
    let resolved_phase = match (stage.as_deref(), task_status) {
        (Some("prepare"), "accepted" | "running") => "preparing",
        (Some("release"), "accepted" | "running") => "deploying",
        _ => phase,
    };
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE agent_tasks SET status=?,started_at=CASE WHEN ?='running' THEN COALESCE(started_at,?) ELSE started_at END,updated_at=? WHERE id=? AND agent_id=?")
        .bind(task_status).bind(task_status).bind(&now).bind(&now).bind(&task_state.task_id).bind(agent_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    sqlx::query("UPDATE deployments SET status=?,phase=?,started_at=COALESCE(started_at,?),updated_at=?,version=version+1 WHERE id=(SELECT deployment_id FROM agent_tasks WHERE id=?) AND status IN ('queued','running','canceling')")
        .bind(deployment_status).bind(resolved_phase).bind(&now).bind(&now).bind(&task_state.task_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    update_refs_discovery_state(state, &task_state.task_id, task_status).await?;
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
    finish_git_ref_discovery_for_task(
        state,
        &result.task_id,
        status,
        result.error_code.as_deref(),
        result.data.as_ref(),
    )
    .await?;
    expire_task_secret_leases(state, &result.task_id).await?;
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
) -> ApiResult<bool> {
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
            Ok(false)
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
    Ok(true)
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

async fn expire_task_secret_leases(state: &AppState, task_id: &str) -> ApiResult<()> {
    sqlx::query(
        "UPDATE git_secret_leases SET status='expired' WHERE task_id=? AND status='issued'",
    )
    .bind(task_id)
    .execute(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_lease"))?;
    Ok(())
}

async fn update_refs_discovery_state(
    state: &AppState,
    task_id: &str,
    task_status: &str,
) -> ApiResult<()> {
    if !matches!(task_status, "accepted" | "running" | "canceling") {
        return Ok(());
    }
    sqlx::query(
        "UPDATE git_ref_discoveries SET status='running',finished_at=NULL WHERE task_id=? AND status='queued'",
    )
    .bind(task_id)
    .execute(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_event"))?;
    Ok(())
}

async fn finish_git_ref_discovery_for_task(
    state: &AppState,
    task_id: &str,
    task_status: &str,
    error_code: Option<&str>,
    data: Option<&Value>,
) -> ApiResult<()> {
    let discovery_id: Option<String> =
        sqlx::query_scalar("SELECT id FROM git_ref_discoveries WHERE task_id=?")
            .bind(task_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
    let Some(discovery_id) = discovery_id else {
        return Ok(());
    };
    let now = Utc::now().to_rfc3339();
    if task_status == "succeeded" {
        let refs = data
            .and_then(|data| data.get("refs"))
            .and_then(Value::as_array)
            .filter(|refs| refs.len() <= MAX_REFS)
            .ok_or_else(|| ApiError::internal("agent_event"))?;
        let sanitized = Value::Array(
            refs.iter()
                .filter(|item| {
                    item.get("name")
                        .and_then(Value::as_str)
                        .zip(item.get("ref").and_then(Value::as_str))
                        .zip(item.get("sha").and_then(Value::as_str))
                        .is_some_and(|((name, reference), sha)| valid_git_ref(name, reference, sha))
                })
                .cloned()
                .collect(),
        );
        sqlx::query("UPDATE git_ref_discoveries SET status='succeeded',refs_json=?,error_code=NULL,expires_at=?,finished_at=? WHERE id=?")
            .bind(sanitized.to_string())
            .bind((Utc::now() + Duration::seconds(REFS_RESULT_TTL_SECONDS)).to_rfc3339())
            .bind(&now)
            .bind(&discovery_id)
            .execute(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
    } else {
        sqlx::query("UPDATE git_ref_discoveries SET status=?,refs_json='[]',error_code=?,expires_at=?,finished_at=? WHERE id=?")
            .bind(if task_status == "failed" {
                "failed"
            } else {
                "expired"
            })
            .bind(sanitize_refs_error(error_code))
            .bind(&now)
            .bind(&now)
            .bind(&discovery_id)
            .execute(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
    }
    Ok(())
}

fn valid_git_ref(name: &str, reference: &str, sha: &str) -> bool {
    valid_git_ref_name(name) && reference == format!("refs/heads/{name}") && valid_sha(sha)
}

fn valid_git_ref_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('-')
        && !name.starts_with('/')
        && !name.starts_with("refs/")
        && !name.ends_with('/')
        && !name.ends_with('.')
        && !name.ends_with(".lock")
        && !name.contains("..")
        && !name.contains("@{")
        && !name.contains("//")
        && !name.contains('\\')
        && !name.contains(':')
        && !name.contains('?')
        && !name.contains('*')
        && !name.contains('[')
        && !name.chars().any(char::is_control)
        && !name.chars().any(char::is_whitespace)
}

fn valid_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sanitize_refs_error(error_code: Option<&str>) -> &'static str {
    match error_code {
        Some("git_timeout") => "git_ref_discovery_timeout",
        Some("git_authentication_failed") => "git_authentication_failed",
        Some("git_repository_unreachable") => "git_repository_unreachable",
        Some("git_command_failed") | Some("git_invalid_repository") | Some("git_io_error") => {
            "git_repository_unreachable"
        }
        Some(code) if code.starts_with("secret_lease_") => "secret_lease_failed",
        _ => "git_ref_discovery_failed",
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
    let task: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT stage,deployment_id FROM agent_tasks WHERE id=?")
            .bind(task_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
    let Some((stage, deployment_id)) = task else {
        return Ok(());
    };
    let Some(deployment_id) = deployment_id else {
        return Ok(());
    };
    if status == "succeeded" && stage.as_deref() == Some("prepare") {
        sqlx::query("UPDATE deployments SET phase='deploying',updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running')")
            .bind(&now)
            .bind(deployment_id)
            .execute(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
        return Ok(());
    }
    if status == "succeeded" && stage.as_deref() == Some("release") {
        sqlx::query("UPDATE deployments SET status='succeeded',phase='succeeded',result_summary=?,exit_code=?,protocol_complete=1,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running','canceling')")
            .bind(summary).bind(exit_code).bind(&now).bind(&now).bind(deployment_id)
            .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
        return Ok(());
    }
    if stage.is_some() {
        sqlx::query("UPDATE deployments SET status=?,phase=?,result_summary=?,exit_code=?,protocol_complete=1,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running','canceling')")
            .bind(status).bind(status).bind(summary).bind(exit_code).bind(&now).bind(&now).bind(deployment_id)
            .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
        return Ok(());
    }
    sqlx::query("UPDATE deployments SET status=?,phase=?,result_summary=?,exit_code=?,protocol_complete=1,finished_at=?,updated_at=?,version=version+1 WHERE id=(SELECT deployment_id FROM agent_tasks WHERE id=?) AND status IN ('queued','running','canceling')")
        .bind(status).bind(status).bind(summary).bind(exit_code).bind(&now).bind(&now).bind(task_id)
        .execute(state.pool()).await.map_err(|_| ApiError::internal("agent_event"))?;
    Ok(())
}
