use chrono::{Duration, Utc};
use deploy_go_agent_protocol::{
    AgentCapability, ArtifactDownloadRequest, ArtifactPrepared, ArtifactUploadAuthorized,
    ArtifactUploadRequest, DeploymentExecuteTask, DeploymentPrepareTask, DeploymentReleaseTask,
    EnvSyncAction, EnvSyncTask, Environment, EnvironmentFileReference, ImageDeploySpec, MakeTarget,
    Message, OutputStream, ReconcileReport, ReconciledTaskState, ReleaseAuthorizationRequest,
    ReleaseAuthorizationResponse, RequiredEnvVersion, SecretLeaseRequest, SecretLeaseResponse,
    SourcePolicy, SystemInspectTask, TaskAck, TaskAckDisposition, TaskDispatch, TaskLifecycleState,
    TaskOutput, TaskPayload, TaskProgress, TaskResult, TaskState, TaskTerminalStatus,
};
use deploy_go_release_authorization::{AUDIENCE, Claims, FileDigest, SCHEMA_VERSION};
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

fn agent_internal(error: impl std::fmt::Debug) -> ApiError {
    tracing::warn!(error = ?error, "agent_dispatch internal");
    ApiError::internal("agent_dispatch")
}

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

#[derive(sqlx::FromRow)]
struct TerminalTaskRow {
    stage: Option<String>,
    deployment_id: Option<String>,
    target_run_id: Option<String>,
    multi_target: i64,
    cancel_requested_at: Option<String>,
    release_strategy: String,
}

#[derive(sqlx::FromRow)]
struct ExistingArtifactAuthorization {
    status: String,
    manifest_digest: String,
    total_size: i64,
    file_count: i64,
    upload_size: Option<i64>,
    archive_digest: Option<String>,
}

#[derive(sqlx::FromRow)]
struct PendingEnvSync {
    sync_id: String,
    version_id: String,
    application_slug: String,
    file_name: String,
    env_version: i64,
    digest: String,
    action: String,
}

#[derive(sqlx::FromRow)]
struct ReleaseEnvRequirement {
    file_name: String,
    current_version: i64,
    current_digest: String,
    deleted_at: Option<String>,
    sync_status: Option<String>,
    actual_version: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct ReleaseAgent {
    id: String,
    work_root: String,
    protocol_version: Option<i64>,
    capabilities_json: Option<String>,
}

pub async fn enqueue_pending_env_syncs_for_agent(
    state: &AppState,
    agent_id: &str,
) -> ApiResult<u64> {
    sqlx::query("UPDATE application_env_syncs SET agent_id=?,updated_at=? WHERE status='pending' AND node_id=(SELECT node_id FROM agents WHERE id=?) AND NOT (agent_id IS ?)")
        .bind(agent_id)
        .bind(Utc::now().to_rfc3339())
        .bind(agent_id)
        .bind(agent_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("env_sync_dispatch"))?;
    if sqlx::query_scalar::<_, Option<i64>>("SELECT protocol_version FROM agents WHERE id=? AND revoked_at IS NULL AND archived_at IS NULL")
        .bind(agent_id)
        .fetch_optional(state.pool())
        .await
        .map_err(|_| ApiError::internal("env_sync_dispatch"))?
        .flatten()
        .unwrap_or_default()
        < 4
    {
        return Ok(0);
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE application_env_syncs SET status='failed',error_code='superseded',error_message='Env 版本已被更新版本替代',updated_at=? WHERE agent_id=? AND status='pending' AND EXISTS (SELECT 1 FROM application_env_versions old_version JOIN application_env_files file ON file.id=old_version.env_file_id WHERE old_version.id=application_env_syncs.env_version_id AND old_version.env_version<>file.current_version)")
        .bind(&now)
        .bind(agent_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("env_sync_dispatch"))?;
    let pending = sqlx::query_as::<_, PendingEnvSync>("SELECT sync.id sync_id,version.id version_id,application.slug application_slug,file.file_name,version.env_version,version.digest,sync.action FROM application_env_syncs sync JOIN application_env_versions version ON version.id=sync.env_version_id JOIN application_env_files file ON file.id=version.env_file_id JOIN applications application ON application.id=file.application_id WHERE sync.agent_id=? AND sync.status='pending' AND version.env_version=file.current_version AND NOT EXISTS (SELECT 1 FROM agent_tasks task WHERE task.env_sync_id=sync.id AND task.status IN ('queued','delivered','accepted','running','canceling')) ORDER BY sync.created_at,sync.id LIMIT 64")
        .bind(agent_id)
        .fetch_all(state.pool())
        .await
        .map_err(|_| ApiError::internal("env_sync_dispatch"))?;
    let mut created = 0;
    for sync in pending {
        let task_id = format!("task_{}", Ulid::new());
        let lease_id = format!("envlease_{}", Ulid::new());
        let action = if sync.action == "delete" {
            EnvSyncAction::Delete
        } else {
            EnvSyncAction::Write
        };
        let payload = TaskPayload::EnvSync(EnvSyncTask {
            env_sync_id: sync.sync_id.clone(),
            application_slug: sync.application_slug,
            file_name: sync.file_name,
            env_version: u64::try_from(sync.env_version)
                .map_err(|_| ApiError::internal("env_sync_dispatch"))?,
            digest: sync.digest,
            lease_id: lease_id.clone(),
            action,
        });
        let payload_json =
            serde_json::to_string(&payload).map_err(|_| ApiError::internal("env_sync_dispatch"))?;
        let payload_digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
        let expires_at = (Utc::now() + Duration::minutes(5)).to_rfc3339();
        let deadline_at = (Utc::now() + Duration::minutes(10)).to_rfc3339();
        let mut transaction = state
            .pool()
            .begin()
            .await
            .map_err(|_| ApiError::internal("env_sync_dispatch"))?;
        let inserted = sqlx::query("INSERT INTO agent_tasks(id,agent_id,env_sync_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) SELECT ?,?,?,'env_sync',?,?,?,'queued',? WHERE EXISTS (SELECT 1 FROM application_env_syncs WHERE id=? AND status='pending')")
            .bind(&task_id)
            .bind(agent_id)
            .bind(&sync.sync_id)
            .bind(format!("env-sync:{}:{task_id}", sync.sync_id))
            .bind(&payload_digest)
            .bind(&payload_json)
            .bind(&deadline_at)
            .bind(&sync.sync_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal("env_sync_dispatch"))?;
        if inserted.rows_affected() != 1 {
            continue;
        }
        sqlx::query("INSERT INTO application_env_secret_leases(id,env_sync_id,env_version_id,agent_id,purpose,status,expires_at) VALUES(?,?,?,?,'application_env','issued',?)")
            .bind(&lease_id)
            .bind(&sync.sync_id)
            .bind(&sync.version_id)
            .bind(agent_id)
            .bind(&expires_at)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal("env_sync_dispatch"))?;
        transaction
            .commit()
            .await
            .map_err(|_| ApiError::internal("env_sync_dispatch"))?;
        if try_dispatch(state, &task_id).await? {
            created += 1;
        }
    }
    Ok(created)
}

pub async fn enqueue_deployment(state: &AppState, deployment_id: &str) -> ApiResult<String> {
    if let Some(existing) =
        sqlx::query_scalar::<_, String>("SELECT id FROM agent_tasks WHERE deployment_id=?")
            .bind(deployment_id)
            .fetch_optional(state.pool())
            .await
            .map_err(agent_internal)?
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
    .map_err(agent_internal)?
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
    let snapshot: Value = serde_json::from_str(&source.snapshot_json).map_err(agent_internal)?;
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
    let payload_json = serde_json::to_string(&payload).map_err(agent_internal)?;
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
    dispatch_pending_env_syncs(state).await?;
    let retry_before = (Utc::now() - Duration::seconds(1)).to_rfc3339();
    if state.cross_node_artifacts_enabled() {
        let mut cursor_at = String::new();
        let mut cursor_id = String::new();
        loop {
            let candidates: Vec<(String, String)> = sqlx::query_as(
                "SELECT d.id,d.queued_at FROM deployments d JOIN applications application ON application.id=d.application_id WHERE json_type(d.snapshot_json,'$.targets') IS NOT NULL AND application.status='active' AND NOT EXISTS (SELECT 1 FROM deployments active WHERE active.target_id=d.target_id AND active.id!=d.id AND active.status IN ('running','canceling')) AND (?='' OR d.queued_at>? OR (d.queued_at=? AND d.id>?)) AND ((d.status='queued' AND d.phase IN ('queued','targets_pending') AND NOT EXISTS (SELECT 1 FROM agent_tasks prepare WHERE prepare.deployment_id=d.id AND prepare.stage='prepare')) OR (d.status='running' AND d.phase IN ('preparing','deploying','targets_running','targets_pending') AND (NOT EXISTS (SELECT 1 FROM agent_tasks stage_task WHERE stage_task.deployment_id=d.id AND stage_task.status IN ('queued','delivered','accepted','running','canceling')) OR EXISTS (SELECT 1 FROM agent_tasks pending WHERE pending.deployment_id=d.id AND pending.status='queued' AND pending.updated_at<=?)))) ORDER BY d.queued_at,d.id LIMIT 16",
            )
            .bind(&cursor_at)
            .bind(&cursor_at)
            .bind(&cursor_at)
            .bind(&cursor_id)
            .bind(&retry_before)
            .fetch_all(state.pool())
            .await
            .map_err(agent_internal)?;
            if candidates.is_empty() {
                break;
            }
            for (deployment_id, queued_at) in candidates {
                cursor_at = queued_at;
                cursor_id = deployment_id.clone();
                if ensure_deployment_task(state, &deployment_id)
                    .await?
                    .is_some()
                {
                    return Ok(Some(deployment_id));
                }
            }
        }
    }
    let candidate: Option<(String, String)> = sqlx::query_as(
        "SELECT d.id,target.execution_mode FROM deployments d JOIN deployment_targets target ON target.id=d.target_id JOIN applications application ON application.id=target.application_id JOIN nodes node ON node.id=target.node_id JOIN agents agent ON agent.node_id=node.id LEFT JOIN agent_tasks task ON task.deployment_id=d.id WHERE json_type(d.snapshot_json,'$.targets') IS NULL AND application.status='active' AND target.status='active' AND node.status='online' AND node.work_root IS NOT NULL AND node.secrets_root IS NOT NULL AND agent.revoked_at IS NULL AND agent.archived_at IS NULL AND NOT EXISTS (SELECT 1 FROM deployments active WHERE active.target_id=d.target_id AND active.id!=d.id AND active.status IN ('running','canceling')) AND ((target.execution_mode='script' AND d.status='queued' AND (task.id IS NULL OR (task.status='queued' AND task.updated_at<=?))) OR (target.execution_mode='two_stage' AND ((d.status='queued' AND d.phase IN ('queued','targets_pending') AND NOT EXISTS (SELECT 1 FROM agent_tasks prepare WHERE prepare.deployment_id=d.id AND prepare.stage='prepare')) OR (d.status='running' AND d.phase IN ('preparing','deploying','targets_running') AND (NOT EXISTS (SELECT 1 FROM agent_tasks stage_task WHERE stage_task.deployment_id=d.id AND stage_task.status IN ('queued','delivered','accepted','running','canceling')) OR EXISTS (SELECT 1 FROM agent_tasks pending WHERE pending.deployment_id=d.id AND pending.status='queued' AND pending.updated_at<=?)))) OR (target.execution_mode='image' AND ((d.status='queued' AND d.phase IN ('queued','targets_pending')) OR (d.status='running' AND d.phase IN ('deploying','targets_running','targets_pending') AND (NOT EXISTS (SELECT 1 FROM agent_tasks stage_task WHERE stage_task.deployment_id=d.id AND stage_task.status IN ('queued','delivered','accepted','running','canceling')) OR EXISTS (SELECT 1 FROM agent_tasks pending WHERE pending.deployment_id=d.id AND pending.status='queued' AND pending.updated_at<=?))))))) ORDER BY d.queued_at,d.id LIMIT 1",
    )
    .bind(&retry_before)
    .bind(&retry_before)
    .bind(&retry_before)
    .fetch_optional(state.pool())
    .await
    .map_err(agent_internal)?;
    let Some((deployment_id, execution_mode)) = candidate else {
        return Ok(None);
    };
    if matches!(execution_mode.as_str(), "two_stage" | "image") {
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

async fn dispatch_pending_env_syncs(state: &AppState) -> ApiResult<()> {
    let agent_ids: Vec<String> = sqlx::query_scalar("SELECT DISTINCT sync.agent_id FROM application_env_syncs sync JOIN agents agent ON agent.id=sync.agent_id JOIN nodes node ON node.id=agent.node_id WHERE sync.status='pending' AND node.status='online' AND agent.protocol_version>=4 AND agent.revoked_at IS NULL AND agent.archived_at IS NULL ORDER BY sync.agent_id LIMIT 32")
        .fetch_all(state.pool())
        .await
        .map_err(|_| ApiError::internal("env_sync_dispatch"))?;
    for agent_id in agent_ids {
        if state.agent_connections().is_connected(&agent_id) {
            enqueue_pending_env_syncs_for_agent(state, &agent_id).await?;
        }
    }
    Ok(())
}

pub async fn ensure_deployment_task(
    state: &AppState,
    deployment_id: &str,
) -> ApiResult<Option<String>> {
    let deployment: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT d.status,d.phase,d.snapshot_json,t.execution_mode FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE d.id=? AND t.execution_mode IN ('two_stage','image')",
    )
    .bind(deployment_id)
    .fetch_optional(state.pool())
    .await
    .map_err(agent_internal)?;
    let Some((status, phase, snapshot_json, execution_mode)) = deployment else {
        return Ok(None);
    };
    let snapshot_value: Value = serde_json::from_str(&snapshot_json).map_err(agent_internal)?;
    if snapshot_value
        .get("targets")
        .and_then(Value::as_array)
        .is_some()
    {
        if !state.cross_node_artifacts_enabled() {
            return Ok(None);
        }
        return ensure_application_deployment_tasks(
            state,
            deployment_id,
            status,
            phase,
            snapshot_value,
        )
        .await;
    }
    if execution_mode == "image" {
        return ensure_image_deployment_task(state, deployment_id, &status, &phase, &snapshot_json)
            .await;
    }
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
    .map_err(agent_internal)?;
    if let Some((task_id, task_status)) = prepare {
        if task_status == "succeeded" {
            if snapshot_value
                .get("release_strategy")
                .and_then(Value::as_str)
                == Some("manual")
                && phase == "awaiting_release"
            {
                return Ok(None);
            }
            let release: Option<(String, String)> = sqlx::query_as(
                "SELECT id,status FROM agent_tasks WHERE deployment_id=? AND stage='release'",
            )
            .bind(deployment_id)
            .fetch_optional(state.pool())
            .await
            .map_err(agent_internal)?;
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

async fn ensure_image_deployment_task(
    state: &AppState,
    deployment_id: &str,
    status: &str,
    phase: &str,
    snapshot_json: &str,
) -> ApiResult<Option<String>> {
    if matches!(
        status,
        "canceled" | "failed" | "succeeded" | "interrupted" | "canceling"
    ) {
        return Ok(None);
    }
    let release: Option<(String, String)> = sqlx::query_as(
        "SELECT id,status FROM agent_tasks WHERE deployment_id=? AND stage='release'",
    )
    .bind(deployment_id)
    .fetch_optional(state.pool())
    .await
    .map_err(agent_internal)?;
    if let Some((release_id, release_status)) = release {
        if matches!(release_status.as_str(), "queued" | "delivered") {
            try_dispatch(state, &release_id).await?;
        }
        return Ok(Some(release_id));
    }
    let terminal_release: Option<String> = sqlx::query_scalar(
        "SELECT id FROM agent_tasks WHERE deployment_id=? AND stage='release' AND status IN ('succeeded','failed','expired','canceled')",
    )
    .bind(deployment_id)
    .fetch_optional(state.pool())
    .await
    .map_err(agent_internal)?;
    if terminal_release.is_some() {
        return Ok(None);
    }
    if status != "queued" && !matches!(phase, "deploying" | "targets_running" | "targets_pending") {
        return Ok(None);
    }
    if let Some(task_id) = create_stage_task(state, deployment_id, "release", snapshot_json).await?
    {
        try_dispatch(state, &task_id).await?;
        return Ok(Some(task_id));
    }
    Ok(None)
}

async fn ensure_application_deployment_tasks(
    state: &AppState,
    deployment_id: &str,
    status: String,
    phase: String,
    snapshot: Value,
) -> ApiResult<Option<String>> {
    let image_mode = snapshot.get("execution_mode").and_then(Value::as_str) == Some("image");
    let prepare: Option<(String, String)> = sqlx::query_as(
        "SELECT id,status FROM agent_tasks WHERE deployment_id=? AND stage='prepare'",
    )
    .bind(deployment_id)
    .fetch_optional(state.pool())
    .await
    .map_err(agent_internal)?;
    if prepare.is_none() {
        let retry_artifact: Option<(String, String, String)> = sqlx::query_as(
            "SELECT artifact.id,artifact.manifest_digest,artifact.archive_digest FROM deployment_target_runs run JOIN deployment_artifacts artifact ON artifact.id=run.artifact_id WHERE run.deployment_id=? AND run.status='pending' AND artifact.status='verified' AND artifact.expires_at>? LIMIT 1",
        )
        .bind(deployment_id)
        .bind(Utc::now().to_rfc3339())
        .fetch_optional(state.pool())
        .await
        .map_err(agent_internal)?;
        if let Some(artifact) = retry_artifact {
            tracing::debug!(
                deployment_id,
                artifact_id = %artifact.0,
                "镜像多目标部署直接调度 release"
            );
            return schedule_application_releases(state, deployment_id, &snapshot, artifact).await;
        }
        if image_mode {
            tracing::debug!(deployment_id, "镜像多目标部署未找到可复用制品，暂不调度");
            return Ok(None);
        }
    }
    if let Some((task_id, task_status)) = prepare {
        if task_status != "succeeded" {
            if matches!(task_status.as_str(), "queued" | "delivered") {
                try_dispatch(state, &task_id).await?;
            }
            return Ok(Some(task_id));
        }
        let artifact: Option<(String, String, String)> = sqlx::query_as(
            "SELECT id,manifest_digest,archive_digest FROM deployment_artifacts WHERE deployment_id=? AND status='verified' AND expires_at>?",
        )
        .bind(deployment_id)
        .bind(Utc::now().to_rfc3339())
        .fetch_optional(state.pool())
        .await
        .map_err(agent_internal)?;
        let Some((artifact_id, manifest_digest, archive_digest)) = artifact else {
            return Ok(Some(task_id));
        };
        if snapshot.get("release_strategy").and_then(Value::as_str) == Some("manual")
            && phase == "awaiting_release"
        {
            return Ok(None);
        }
        return schedule_application_releases(
            state,
            deployment_id,
            &snapshot,
            (artifact_id, manifest_digest, archive_digest),
        )
        .await;
    }
    if !matches!(status.as_str(), "queued" | "running") {
        return Ok(None);
    }
    if image_mode {
        return Ok(None);
    }
    let first_target = snapshot
        .get("targets")
        .and_then(Value::as_array)
        .and_then(|targets| targets.first())
        .cloned()
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let mut normalized = snapshot;
    normalized["target"] = first_target
        .get("target")
        .cloned()
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    normalized["target_id"] = first_target
        .get("target_id")
        .cloned()
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    normalized["_cross_node"] = Value::Bool(true);
    if let Some(task_id) =
        create_stage_task(state, deployment_id, "prepare", &normalized.to_string()).await?
    {
        try_dispatch(state, &task_id).await?;
        return Ok(Some(task_id));
    }
    Ok(None)
}

async fn schedule_application_releases(
    state: &AppState,
    deployment_id: &str,
    snapshot: &Value,
    artifact: (String, String, String),
) -> ApiResult<Option<String>> {
    let (artifact_id, manifest_digest, archive_digest) = artifact;
    let targets = snapshot
        .get("targets")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let mut last = None;
    for target in targets {
        let target_id = target
            .get("target_id")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        let run: Option<(String, String)> = sqlx::query_as(
            "SELECT id,status FROM deployment_target_runs WHERE deployment_id=? AND target_id=?",
        )
        .bind(deployment_id)
        .bind(target_id)
        .fetch_optional(state.pool())
        .await
        .map_err(agent_internal)?;
        let Some((run_id, run_status)) = run else {
            continue;
        };
        if matches!(
            run_status.as_str(),
            "succeeded" | "reused" | "canceled" | "failed"
        ) {
            continue;
        }
        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT id,status FROM agent_tasks WHERE target_run_id=? AND stage='release'",
        )
        .bind(&run_id)
        .fetch_optional(state.pool())
        .await
        .map_err(agent_internal)?;
        if let Some((release_id, release_status)) = existing {
            if matches!(release_status.as_str(), "queued" | "delivered") {
                try_dispatch(state, &release_id).await?;
            }
            last = Some(release_id);
            continue;
        }
        let mut normalized = snapshot.clone();
        normalized["target"] = target
            .get("target")
            .cloned()
            .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        normalized["target_id"] = Value::String(target_id.to_owned());
        normalized["_cross_node"] = Value::Bool(true);
        normalized["_target_run_id"] = Value::String(run_id);
        normalized["_artifact_id"] = Value::String(artifact_id.clone());
        normalized["_artifact_manifest_digest"] = Value::String(manifest_digest.clone());
        normalized["_artifact_archive_digest"] = Value::String(archive_digest.clone());
        if let Some(release_id) =
            create_stage_task(state, deployment_id, "release", &normalized.to_string()).await?
        {
            try_dispatch(state, &release_id).await?;
            last = Some(release_id);
        } else {
            tracing::debug!(deployment_id, target_id, "镜像 release 任务创建被延迟");
        }
    }
    Ok(last)
}

async fn create_stage_task(
    state: &AppState,
    deployment_id: &str,
    stage: &str,
    snapshot_json: &str,
) -> ApiResult<Option<String>> {
    let snapshot: Value = serde_json::from_str(snapshot_json).map_err(agent_internal)?;
    let execution_mode = snapshot
        .get("execution_mode")
        .and_then(Value::as_str)
        .unwrap_or("script");
    let image_mode = execution_mode == "image";
    let target = snapshot
        .get("target")
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let environment = target
        .get("environment")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
    let target_code = target
        .get("target_code")
        .and_then(Value::as_str)
        .unwrap_or(environment)
        .to_owned();
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
    let image_spec = if image_mode {
        let spec_value = snapshot
            .get("image")
            .and_then(|image| image.get("image_spec"))
            .cloned()
            .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        Some(serde_json::from_value::<ImageDeploySpec>(spec_value).map_err(agent_internal)?)
    } else {
        None
    };
    let (repository_url, commit_sha, build_agent_id, git_credential_id, release_version, modules) =
        if image_mode {
            let image = snapshot
                .get("image")
                .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
            let modules = image
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
            (
                None,
                image
                    .get("commit_sha")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::internal("agent_dispatch"))?
                    .to_owned(),
                None,
                None,
                image
                    .get("release_version")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::internal("agent_dispatch"))?
                    .to_owned(),
                modules,
            )
        } else {
            let source = snapshot
                .get("source")
                .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
            let two_stage = snapshot
                .get("two_stage")
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
            (
                Some(
                    source
                        .get("repository_url")
                        .and_then(Value::as_str)
                        .ok_or_else(|| ApiError::internal("agent_dispatch"))?
                        .to_owned(),
                ),
                source
                    .get("resolved_commit_sha")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::internal("agent_dispatch"))?
                    .to_owned(),
                Some(
                    source
                        .get("build_agent_id")
                        .and_then(Value::as_str)
                        .ok_or_else(|| ApiError::internal("agent_dispatch"))?
                        .to_owned(),
                ),
                source
                    .get("git_credential_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                two_stage
                    .get("release_version")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::internal("agent_dispatch"))?
                    .to_owned(),
                modules,
            )
        };
    let cross_node = snapshot.get("_cross_node").and_then(Value::as_bool) == Some(true);
    // 特权发布是平台固定能力：release 阶段不再读取目标开关，也不存在关闭配置。
    let privileged_release = stage == "release";
    let deployment_snapshot_hash = if privileged_release {
        Some(
            sqlx::query_scalar::<_, String>("SELECT snapshot_hash FROM deployments WHERE id=?")
                .bind(deployment_id)
                .fetch_one(state.pool())
                .await
                .map_err(agent_internal)?,
        )
    } else {
        None
    };
    let (application_slug, required_env, env_managed) = if stage == "release" {
        let target_id = snapshot
            .get("target_id")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        let Some(gate) = load_release_env_gate(
            state,
            target_id,
            image_spec.as_ref().map(|spec| spec.env_files.as_slice()),
        )
        .await?
        else {
            tracing::debug!(deployment_id, target_id, "镜像 release Env 门禁未就绪");
            if let Some(run_id) = snapshot.get("_target_run_id").and_then(Value::as_str) {
                sqlx::query("UPDATE deployment_target_runs SET env_gate_status='pending',phase='env_sync',updated_at=?,version=version+1 WHERE id=? AND status='pending'")
                    .bind(Utc::now().to_rfc3339())
                    .bind(run_id)
                    .execute(state.pool())
                    .await
                    .map_err(agent_internal)?;
            }
            return Ok(None);
        };
        gate
    } else {
        (String::new(), Vec::new(), false)
    };
    let minimum_protocol = if image_mode {
        8
    } else if privileged_release {
        7
    } else if env_managed {
        4
    } else if cross_node {
        3
    } else {
        2
    };

    let (agent_id, work_root) = if stage == "prepare" {
        let build_agent_id = build_agent_id.ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        let agent: Option<(String, String)> = sqlx::query_as(
            "SELECT a.id,n.work_root FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.id=? AND a.revoked_at IS NULL AND a.archived_at IS NULL AND a.protocol_version>=? AND n.status='online' AND n.work_root IS NOT NULL",
        )
        .bind(build_agent_id)
        .bind(minimum_protocol)
        .fetch_optional(state.pool())
        .await
        .map_err(agent_internal)?;
        let Some((agent_id, work_root)) = agent else {
            return Ok(None);
        };
        (agent_id, work_root)
    } else {
        let target_node_id = target
            .get("node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        let agent: Option<ReleaseAgent> = sqlx::query_as(
            "SELECT a.id,n.work_root,a.protocol_version,a.capabilities_json FROM nodes n JOIN agents a ON a.node_id=n.id WHERE n.id=? AND n.status='online' AND n.work_root IS NOT NULL AND a.revoked_at IS NULL AND a.archived_at IS NULL",
        )
        .bind(target_node_id)
        .fetch_optional(state.pool())
        .await
        .map_err(agent_internal)?;
        let Some(agent) = agent else {
            return Ok(None);
        };
        if image_mode {
            if let Err((code, summary)) = image_release_compatibility(
                agent.protocol_version,
                agent.capabilities_json.as_deref(),
            ) {
                if let Some(run_id) = snapshot.get("_target_run_id").and_then(Value::as_str) {
                    fail_target_run_before_dispatch(state, run_id, code, summary).await?;
                } else {
                    fail_deployment_before_dispatch(state, deployment_id, code, summary).await?;
                }
                return Ok(None);
            }
        } else if privileged_release {
            if let Err((code, summary)) = privileged_release_compatibility(
                agent.protocol_version,
                agent.capabilities_json.as_deref(),
            ) {
                if let Some(run_id) = snapshot.get("_target_run_id").and_then(Value::as_str) {
                    fail_target_run_before_dispatch(state, run_id, code, summary).await?;
                } else {
                    fail_deployment_before_dispatch(state, deployment_id, code, summary).await?;
                }
                return Ok(None);
            }
        } else if agent.protocol_version.unwrap_or_default() < minimum_protocol {
            return Ok(None);
        }
        (agent.id, agent.work_root)
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
    let lease_id = (git_credential_id.is_some()).then(|| format!("lease_{}", Ulid::new()));
    let artifact_authorization_id =
        (stage == "prepare" && cross_node).then(|| format!("artifact_lease_{}", Ulid::new()));
    let download_lease_id = (stage == "release" && (image_mode || cross_node))
        .then(|| format!("artifact_lease_{}", Ulid::new()));
    let artifact_binding = if stage == "release" && (image_mode || cross_node) {
        if let (Some(id), Some(manifest), Some(archive)) = (
            snapshot.get("_artifact_id").and_then(Value::as_str),
            snapshot
                .get("_artifact_manifest_digest")
                .and_then(Value::as_str),
            snapshot
                .get("_artifact_archive_digest")
                .and_then(Value::as_str),
        ) {
            Some((id.to_owned(), manifest.to_owned(), archive.to_owned()))
        } else if image_mode {
            Some(load_image_artifact(state, deployment_id).await?)
        } else {
            None
        }
    } else {
        None
    };
    let mut transaction = state.pool().begin().await.map_err(agent_internal)?;
    let target_run_id = if stage == "release" {
        let target_id = snapshot
            .get("target_id")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        let node_id = target
            .get("node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::internal("agent_dispatch"))?;
        let candidate_id = format!("run_{}", Ulid::new());
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,target_snapshot_json,status,env_gate_status) VALUES(?,?,?,?,?,?,'pending','not_required') ON CONFLICT(deployment_id,target_id) DO NOTHING")
            .bind(&candidate_id)
            .bind(deployment_id)
            .bind(target_id)
            .bind(node_id)
            .bind(&agent_id)
            .bind(target.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "创建或复用 target run 失败");
                ApiError::internal("agent_dispatch")
            })?;
        Some(
            sqlx::query_scalar::<_, String>(
                "SELECT id FROM deployment_target_runs WHERE deployment_id=? AND target_id=?",
            )
            .bind(deployment_id)
            .bind(target_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "读取 target run 失败");
                ApiError::internal("agent_dispatch")
            })?,
        )
    } else {
        None
    };
    let privileged_context = if privileged_release {
        Some(deploy_go_agent_protocol::PrivilegedReleaseContext {
            target_run_id: target_run_id
                .clone()
                .ok_or_else(|| ApiError::internal("agent_dispatch"))?,
            target_id: snapshot
                .get("target_id")
                .and_then(Value::as_str)
                .ok_or_else(|| ApiError::internal("agent_dispatch"))?
                .to_owned(),
            node_id: target
                .get("node_id")
                .and_then(Value::as_str)
                .ok_or_else(|| ApiError::internal("agent_dispatch"))?
                .to_owned(),
            agent_id: String::new(),
            snapshot_hash: deployment_snapshot_hash
                .clone()
                .ok_or_else(|| ApiError::internal("agent_dispatch"))?,
        })
    } else {
        None
    };
    let payload = if stage == "prepare" {
        TaskPayload::DeploymentPrepare(DeploymentPrepareTask {
            deployment_id: deployment_id.to_owned(),
            source_policy: SourcePolicy::Branch,
            repository_url: repository_url.clone().unwrap_or_default(),
            commit_sha: commit_sha.to_owned(),
            checkout_dir: checkout_dir.to_string_lossy().into_owned(),
            work_root: work_root.clone(),
            output_dir: staging_dir.to_string_lossy().into_owned(),
            environment: protocol_environment,
            release_version: release_version.to_owned(),
            modules: modules.clone(),
            make_target: MakeTarget::DeployGoPrepare,
            git_credential_lease_id: lease_id.clone(),
            timeout_seconds,
            artifact_upload: artifact_authorization_id
                .map(|authorization_id| ArtifactUploadRequest { authorization_id }),
        })
    } else {
        TaskPayload::DeploymentRelease(DeploymentReleaseTask {
            deployment_id: deployment_id.to_owned(),
            target_code: target_code.to_owned(),
            work_root,
            checkout_dir: checkout_dir.to_string_lossy().into_owned(),
            artifact_dir: staging_dir.to_string_lossy().into_owned(),
            environment: protocol_environment,
            release_version: release_version.to_owned(),
            commit_sha: commit_sha.to_owned(),
            modules: modules.clone(),
            make_target: MakeTarget::DeployGoRelease,
            timeout_seconds,
            cancel_file: String::new(),
            privileged: privileged_release,
            privileged_context: privileged_context.map(|mut context| {
                context.agent_id = agent_id.clone();
                context
            }),
            artifact_download: if image_mode || cross_node {
                Some(ArtifactDownloadRequest {
                    target_run_id: target_run_id
                        .clone()
                        .unwrap_or_else(|| "missing".to_owned()),
                    lease_id: download_lease_id
                        .clone()
                        .ok_or_else(|| ApiError::internal("agent_dispatch"))?,
                    archive_digest: artifact_binding
                        .as_ref()
                        .map(|binding| binding.2.clone())
                        .ok_or_else(|| ApiError::internal("agent_dispatch"))?,
                    manifest_digest: artifact_binding
                        .as_ref()
                        .map(|binding| binding.1.clone())
                        .ok_or_else(|| ApiError::internal("agent_dispatch"))?,
                })
            } else {
                None
            },
            repository_url: if image_mode {
                None
            } else if cross_node {
                Some(repository_url.clone().unwrap_or_default())
            } else {
                None
            },
            git_credential_lease_id: if image_mode {
                None
            } else {
                (stage == "release" && cross_node)
                    .then(|| lease_id.clone())
                    .flatten()
            },
            application_slug: (!required_env.is_empty()).then_some(application_slug),
            required_env,
            image_spec: image_spec.clone(),
        })
    };
    let payload_json = serde_json::to_string(&payload).map_err(agent_internal)?;
    let payload_digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    let deadline_at =
        (Utc::now() + Duration::seconds(i64::from(timeout_seconds) + 60)).to_rfc3339();
    let now = Utc::now().to_rfc3339();
    let insert = sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,target_run_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,?,?,?,?,?,?,?,?,'queued',?)")
        .bind(&task_id)
        .bind(&agent_id)
        .bind(deployment_id)
        .bind(target_run_id.as_deref())
        .bind(stage)
        .bind(if stage == "prepare" {
            "deployment_prepare"
        } else {
            "deployment_release"
        })
        .bind(if stage == "release" && cross_node {
            format!(
                "deployment:{deployment_id}:release:{}",
                target_run_id.as_deref().unwrap_or("missing")
            )
        } else {
            format!("deployment:{deployment_id}:{stage}")
        })
        .bind(&payload_digest)
        .bind(&payload_json)
        .bind(&deadline_at)
        .execute(&mut *transaction)
        .await;
    if let Err(error) = insert {
        if error.to_string().contains("UNIQUE constraint failed") {
            drop(transaction);
            let existing: Option<String> = if stage == "release" && cross_node {
                sqlx::query_scalar(
                    "SELECT id FROM agent_tasks WHERE target_run_id=? AND stage='release'",
                )
                .bind(target_run_id.as_deref())
                .fetch_optional(state.pool())
                .await
                .map_err(agent_internal)?
            } else {
                sqlx::query_scalar("SELECT id FROM agent_tasks WHERE deployment_id=? AND stage=?")
                    .bind(deployment_id)
                    .bind(stage)
                    .fetch_optional(state.pool())
                    .await
                    .map_err(agent_internal)?
            };
            return Ok(Some(existing.unwrap_or(task_id)));
        }
        tracing::error!(error = %error, "创建 Agent 阶段任务失败");
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
            .map_err(|error| {
                tracing::error!(error = %error, "创建 artifact download lease 失败");
                ApiError::internal("agent_dispatch")
            })?;
    }
    if let (Some(download_lease_id), Some(target_run_id), Some(artifact_binding)) = (
        download_lease_id.as_deref(),
        target_run_id.as_deref(),
        artifact_binding.as_ref(),
    ) {
        let artifact_expiry: String = sqlx::query_scalar(
            "SELECT expires_at FROM deployment_artifacts WHERE id=? AND status='verified'",
        )
        .bind(&artifact_binding.0)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "读取 artifact expiry 失败");
            ApiError::internal("agent_dispatch")
        })?;
        let lease_expiry = artifact_expiry.min(deadline_at.clone());
        sqlx::query("INSERT INTO artifact_leases(id,artifact_id,agent_id,target_run_id,purpose,manifest_digest,status,expires_at) VALUES(?,?,?,?,'artifact_download',?,'active',?)")
            .bind(download_lease_id)
            .bind(&artifact_binding.0)
            .bind(&agent_id)
            .bind(target_run_id)
            .bind(&artifact_binding.1)
            .bind(lease_expiry)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "创建 artifact download lease 失败");
                ApiError::internal("agent_dispatch")
            })?;
        sqlx::query("UPDATE deployment_target_runs SET agent_id=?,artifact_id=?,status='downloading',phase='artifact_download',env_gate_status='ready',updated_at=?,version=version+1 WHERE id=? AND status='pending'")
            .bind(&agent_id)
            .bind(&artifact_binding.0)
            .bind(&now)
            .bind(target_run_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "更新 target run 下载状态失败");
                ApiError::internal("agent_dispatch")
            })?;
    }
    let updated = if stage == "prepare" {
        sqlx::query("UPDATE deployments SET status='running',phase='preparing',updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running')")
            .bind(&now)
            .bind(deployment_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "更新 deployment prepare 状态失败");
                ApiError::internal("agent_dispatch")
            })?
    } else {
        sqlx::query("UPDATE deployments SET status='running',phase='deploying',updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running') AND phase IN ('queued','preparing','deploying','targets_pending','targets_running')")
            .bind(&now)
            .bind(deployment_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "更新 deployment release 状态失败");
                ApiError::internal("agent_dispatch")
            })?
    };
    if updated.rows_affected() != 1 {
        drop(transaction);
        return Ok(Some(task_id));
    }
    transaction.commit().await.map_err(|error| {
        tracing::error!(error = %error, deployment_id, "提交 Agent 阶段任务失败");
        ApiError::internal("agent_dispatch")
    })?;
    Ok(Some(task_id))
}

fn privileged_release_compatibility(
    protocol_version: Option<i64>,
    capabilities_json: Option<&str>,
) -> Result<(), (&'static str, &'static str)> {
    if protocol_version.unwrap_or_default() < 7 {
        return Err((
            "privileged_release_protocol_unsupported",
            "目标 Agent 不支持特权 release 控制协议 v7",
        ));
    }
    let capabilities = capabilities_json
        .and_then(|value| serde_json::from_str::<Vec<AgentCapability>>(value).ok())
        .unwrap_or_default();
    if !capabilities.contains(&AgentCapability::PrivilegedRelease) {
        return Err((
            "privileged_release_capability_unavailable",
            "目标 Agent 的特权 release executor 不可用",
        ));
    }
    Ok(())
}

fn image_release_compatibility(
    protocol_version: Option<i64>,
    capabilities_json: Option<&str>,
) -> Result<(), (&'static str, &'static str)> {
    privileged_release_compatibility(protocol_version, capabilities_json)?;
    if protocol_version.unwrap_or_default() < 8 {
        return Err((
            "image_release_protocol_unsupported",
            "目标 Agent 不支持镜像 release 控制协议 v8",
        ));
    }
    Ok(())
}

async fn fail_target_run_before_dispatch(
    state: &AppState,
    run_id: &str,
    error_code: &str,
    summary: &str,
) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    let mut transaction = state.pool().begin().await.map_err(agent_internal)?;
    let deployment_id: Option<String> = sqlx::query_scalar(
        "UPDATE deployment_target_runs SET status='failed',phase='failed',result_summary=?,error_code=?,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status NOT IN ('succeeded','reused','failed','expired','canceled') RETURNING deployment_id",
    )
    .bind(summary)
    .bind(error_code)
    .bind(&now)
    .bind(&now)
    .bind(run_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(agent_internal)?;
    let Some(deployment_id) = deployment_id else {
        transaction.commit().await.map_err(agent_internal)?;
        return Ok(());
    };
    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*),SUM(CASE WHEN status IN ('failed','expired') THEN 1 ELSE 0 END),SUM(CASE WHEN status NOT IN ('succeeded','reused','failed','expired','canceled') THEN 1 ELSE 0 END) FROM deployment_target_runs WHERE deployment_id=?",
    )
    .bind(&deployment_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(agent_internal)?;
    if counts.0 > 0 && counts.2 == 0 && counts.1 > 0 {
        sqlx::query("UPDATE deployments SET status='failed',phase='targets_failed',result_summary='至少一个目标部署失败',protocol_complete=1,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running')")
            .bind(&now)
            .bind(&now)
            .bind(&deployment_id)
            .execute(&mut *transaction)
            .await
            .map_err(agent_internal)?;
    }
    transaction.commit().await.map_err(agent_internal)?;
    Ok(())
}

async fn fail_deployment_before_dispatch(
    state: &AppState,
    deployment_id: &str,
    error_code: &str,
    summary: &str,
) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    let summary = format!("[{error_code}] {summary}");
    sqlx::query("UPDATE deployments SET status='failed',phase='targets_failed',result_summary=?,protocol_complete=1,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running')")
        .bind(&summary)
        .bind(&now)
        .bind(&now)
        .bind(deployment_id)
        .execute(state.pool())
        .await
        .map_err(agent_internal)?;
    fail_remaining_runs_for_deployment(state, deployment_id, &summary, error_code).await?;
    Ok(())
}

async fn fail_remaining_runs_for_deployment(
    state: &AppState,
    deployment_id: &str,
    summary: &str,
    error_code: &str,
) -> ApiResult<u64> {
    let now = Utc::now().to_rfc3339();
    Ok(sqlx::query(
        "UPDATE deployment_target_runs SET status='failed',phase='failed',result_summary=?,error_code=?,finished_at=COALESCE(finished_at,?),updated_at=?,version=version+1 WHERE deployment_id=? AND status NOT IN ('succeeded','reused','failed','expired','canceled')",
    )
    .bind(summary)
    .bind(error_code)
    .bind(&now)
    .bind(&now)
    .bind(deployment_id)
    .execute(state.pool())
    .await
    .map_err(agent_internal)?
    .rows_affected())
}

pub(crate) async fn terminalize_runs_for_terminal_deployments(state: &AppState) -> ApiResult<u64> {
    let now = Utc::now().to_rfc3339();
    Ok(sqlx::query(
        "UPDATE deployment_target_runs SET
           status=CASE WHEN (SELECT d.status FROM deployments d WHERE d.id=deployment_target_runs.deployment_id)='canceled' THEN 'canceled' ELSE 'failed' END,
           phase=CASE WHEN (SELECT d.status FROM deployments d WHERE d.id=deployment_target_runs.deployment_id)='canceled' THEN 'canceled' ELSE 'failed' END,
           result_summary=CASE WHEN (SELECT d.status FROM deployments d WHERE d.id=deployment_target_runs.deployment_id)='canceled' THEN '部署已取消，未执行目标运行' ELSE '部署已终止，未执行目标运行' END,
           error_code='deployment_terminal',
           finished_at=COALESCE(finished_at,?),
           updated_at=?,
           version=version+1
         WHERE status NOT IN ('succeeded','reused','failed','expired','canceled')
           AND deployment_id IN (SELECT id FROM deployments WHERE status IN ('failed','interrupted','canceled'))",
    )
    .bind(&now)
    .bind(&now)
    .execute(state.pool())
    .await
    .map_err(agent_internal)?
    .rows_affected())
}

async fn load_release_env_gate(
    state: &AppState,
    target_id: &str,
    image_env_files: Option<&[String]>,
) -> ApiResult<Option<(String, Vec<RequiredEnvVersion>, bool)>> {
    let application_slug: String = sqlx::query_scalar("SELECT application.slug FROM deployment_targets target JOIN applications application ON application.id=target.application_id WHERE target.id=?")
        .bind(target_id)
        .fetch_one(state.pool())
        .await
        .map_err(|_| ApiError::internal("env_release_gate"))?;
    let rows = sqlx::query_as::<_, ReleaseEnvRequirement>("SELECT file.file_name,file.current_version,file.current_digest,file.deleted_at,sync.status sync_status,sync.actual_version FROM application_env_files file JOIN deployment_targets target ON target.application_id=file.application_id LEFT JOIN application_env_versions version ON version.env_file_id=file.id AND version.env_version=file.current_version LEFT JOIN application_env_syncs sync ON sync.env_version_id=version.id AND sync.target_id=target.id WHERE target.id=? ORDER BY file.file_name COLLATE NOCASE,file.id")
        .bind(target_id)
        .fetch_all(state.pool())
        .await
        .map_err(|_| ApiError::internal("env_release_gate"))?;
    let rows = if let Some(env_files) = image_env_files {
        rows.into_iter()
            .filter(|row| env_files.contains(&row.file_name))
            .collect::<Vec<_>>()
    } else {
        rows
    };
    if let Some(env_files) = image_env_files {
        let present = rows
            .iter()
            .filter(|row| row.deleted_at.is_none())
            .map(|row| row.file_name.as_str())
            .collect::<std::collections::HashSet<_>>();
        if env_files
            .iter()
            .any(|file| !present.contains(file.as_str()))
        {
            return Ok(None);
        }
    }
    let env_managed = !rows.is_empty();
    let mut required = Vec::with_capacity(rows.len());
    for row in rows {
        if row.sync_status.as_deref() != Some("succeeded")
            || row.actual_version != Some(row.current_version)
        {
            return Ok(None);
        }
        required.push(RequiredEnvVersion {
            file_name: row.file_name,
            env_version: u64::try_from(row.current_version)
                .map_err(|_| ApiError::internal("env_release_gate"))?,
            digest: row.current_digest,
            action: if row.deleted_at.is_some() {
                EnvSyncAction::Delete
            } else {
                EnvSyncAction::Write
            },
        });
    }
    Ok(Some((application_slug, required, env_managed)))
}

async fn load_image_artifact(
    state: &AppState,
    deployment_id: &str,
) -> ApiResult<(String, String, String)> {
    sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,manifest_digest,archive_digest FROM deployment_artifacts WHERE deployment_id=? AND status='verified' AND expires_at>?",
    )
    .bind(deployment_id)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(state.pool())
    .await
    .map_err(agent_internal)?
    .ok_or_else(|| {
        ApiError::conflict(
            "deployment_artifact_unavailable",
            "镜像部署制品尚未就绪或已过期",
            "agent_dispatch",
        )
    })
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
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal("agent_cancel"))?;
    sqlx::query("UPDATE agent_tasks SET status='canceled',finished_at=?,result_json=?,updated_at=? WHERE deployment_id=? AND status='queued'")
        .bind(&now)
        .bind(serde_json::json!({"error_code":"canceled_before_delivery"}).to_string())
        .bind(&now)
        .bind(deployment_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal("agent_cancel"))?;
    sqlx::query("UPDATE deployment_target_runs SET status='canceled',phase='canceled',finished_at=?,updated_at=?,version=version+1 WHERE deployment_id=? AND status='pending'")
        .bind(&now)
        .bind(&now)
        .bind(deployment_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal("agent_cancel"))?;
    sqlx::query("UPDATE agent_tasks SET status='canceling',updated_at=? WHERE deployment_id=? AND status IN ('delivered','accepted','running','canceling')")
        .bind(&now)
        .bind(deployment_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal("agent_cancel"))?;
    let has_remote_tasks: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM agent_tasks WHERE deployment_id=? AND status='canceling')",
    )
    .bind(deployment_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal("agent_cancel"))?;
    sqlx::query("UPDATE deployments SET status=?,phase=?,cancel_requested_at=COALESCE(cancel_requested_at,?),finished_at=CASE WHEN ? THEN NULL ELSE ? END,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running','canceling')")
        .bind(if has_remote_tasks { "canceling" } else { "canceled" })
        .bind(if has_remote_tasks { "canceling" } else { "canceled" })
        .bind(&now)
        .bind(has_remote_tasks)
        .bind(&now)
        .bind(&now)
        .bind(deployment_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal("agent_cancel"))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("agent_cancel"))?;

    let tasks: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,agent_id FROM agent_tasks WHERE deployment_id=? AND status='canceling' ORDER BY created_at,id",
    )
    .bind(deployment_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_cancel"))?;
    let mut sent = false;
    for (task_id, agent_id) in tasks {
        sent |= state
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
    }
    Ok(sent)
}

pub async fn try_dispatch(state: &AppState, task_id: &str) -> ApiResult<bool> {
    let row: Option<DispatchRow> = sqlx::query_as(
        "SELECT t.agent_id,t.idempotency_key,t.payload_digest,t.payload_json,t.deadline_at,t.kind,a.protocol_version FROM agent_tasks t JOIN agents a ON a.id=t.agent_id WHERE t.id=? AND t.status IN ('queued','delivered')",
    )
    .bind(task_id)
    .fetch_optional(state.pool())
    .await
    .map_err(agent_internal)?;
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
    let payload = serde_json::from_str::<TaskPayload>(&row.payload_json).map_err(agent_internal)?;
    let requires_v3 = matches!(
        &payload,
        TaskPayload::DeploymentPrepare(task) if task.artifact_upload.is_some()
    ) || matches!(
        &payload,
        TaskPayload::DeploymentRelease(task) if task.artifact_download.is_some()
    );
    if requires_v3 && row.protocol_version.unwrap_or_default() < 3 {
        return Ok(false);
    }
    if matches!(&payload, TaskPayload::EnvSync(_)) && row.protocol_version.unwrap_or_default() < 4 {
        return Ok(false);
    }
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
        .map_err(agent_internal)?;
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
            .map_err(agent_internal)?;
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
        .map_err(agent_internal)?;
    expire_secret_leases(state).await?;
    Ok(result.rows_affected())
}

pub async fn expire_secret_leases(state: &AppState) -> ApiResult<u64> {
    let now = Utc::now().to_rfc3339();
    let git = sqlx::query(
        "UPDATE git_secret_leases SET status='expired' WHERE status='issued' AND expires_at<=?",
    )
    .bind(&now)
    .execute(state.pool())
    .await
    .map_err(|_| ApiError::internal("agent_lease"))?;
    let env = sqlx::query("UPDATE application_env_secret_leases SET status='expired' WHERE status='issued' AND expires_at<=?")
        .bind(&now)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_lease"))?;
    Ok(git.rows_affected() + env.rows_affected())
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
    .map_err(agent_internal)?;
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
        Message::ArtifactPrepared(prepared) => {
            handle_artifact_prepared(state, agent_id, connection_generation, prepared).await?;
            Ok(true)
        }
        Message::ReleaseAuthorizationRequest(request) => {
            handle_release_authorization_request(state, agent_id, connection_generation, request)
                .await?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

async fn handle_release_authorization_request(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    request: &ReleaseAuthorizationRequest,
) -> ApiResult<()> {
    ensure_current_connection(state, agent_id, generation).await?;
    let authorization = authorize_privileged_release(state, agent_id, request).await;
    let response = match authorization {
        Ok(authorization) => ReleaseAuthorizationResponse {
            task_id: request.task_id.clone(),
            authorization_id: request.authorization_id.clone(),
            authorization: Some(authorization),
            error_code: None,
        },
        Err(error_code) => ReleaseAuthorizationResponse {
            task_id: request.task_id.clone(),
            authorization_id: request.authorization_id.clone(),
            authorization: None,
            error_code: Some(error_code),
        },
    };
    state
        .agent_connections()
        .send(agent_id, Message::ReleaseAuthorizationResponse(response))
        .await
        .map(|_| ())
        .map_err(|_| {
            ApiError::conflict(
                "release_authorization_delivery_failed",
                "特权发布授权响应投递失败",
                "release_authorization",
            )
        })
}

#[derive(sqlx::FromRow)]
struct PrivilegedReleaseAuthorizationRow {
    deployment_id: String,
    target_run_id: String,
    payload_digest: String,
    payload_json: String,
    deadline_at: String,
    snapshot_hash: String,
    snapshot_json: String,
    target_id: String,
    node_id: String,
    run_agent_id: Option<String>,
    artifact_id: Option<String>,
    manifest_json: Option<String>,
    manifest_digest: Option<String>,
}

#[derive(serde::Deserialize)]
struct AuthorizationArtifactManifest {
    artifacts: Vec<AuthorizationArtifactEntry>,
}

#[derive(serde::Deserialize)]
struct AuthorizationArtifactEntry {
    path: String,
    sha256: String,
}

async fn authorize_privileged_release(
    state: &AppState,
    agent_id: &str,
    request: &ReleaseAuthorizationRequest,
) -> Result<String, String> {
    let signer = state
        .release_signer()
        .ok_or_else(|| "release_authorization_unavailable".to_owned())?;
    let row: Option<PrivilegedReleaseAuthorizationRow> = sqlx::query_as(
        "SELECT task.deployment_id,task.target_run_id,task.payload_digest,task.payload_json,task.deadline_at,deployment.snapshot_hash,deployment.snapshot_json,run.target_id,run.node_id,run.agent_id AS run_agent_id,run.artifact_id,artifact.manifest_json,artifact.manifest_digest FROM agent_tasks task JOIN deployments deployment ON deployment.id=task.deployment_id JOIN deployment_target_runs run ON run.id=task.target_run_id JOIN deployment_artifacts artifact ON artifact.id=run.artifact_id WHERE task.id=? AND task.agent_id=? AND task.kind='deployment_release' AND task.status IN ('delivered','accepted','running') AND deployment.status='running' AND deployment.cancel_requested_at IS NULL AND run.status IN ('downloading','running') AND run.env_gate_status IN ('ready','not_required') AND artifact.status='verified' AND artifact.expires_at>?",
    )
    .bind(&request.task_id)
    .bind(agent_id)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(state.pool())
    .await
    .map_err(|_| "release_authorization_failed".to_owned())?;
    let row = row.ok_or_else(|| "release_task_inactive".to_owned())?;
    let payload: TaskPayload = serde_json::from_str(&row.payload_json)
        .map_err(|_| "release_task_payload_invalid".to_owned())?;
    let TaskPayload::DeploymentRelease(task) = payload else {
        return Err("release_task_payload_invalid".to_owned());
    };
    if !task.privileged
        || row.target_run_id != request.target_run_id
        || row.target_id != request.target_id
        || row.run_agent_id.as_deref() != Some(agent_id)
        || row.snapshot_hash != request.snapshot_hash
        || task
            .artifact_download
            .as_ref()
            .map(|download| download.target_run_id.as_str())
            != Some(request.target_run_id.as_str())
    {
        return Err("release_authorization_binding_mismatch".to_owned());
    }
    if task.image_spec.is_some() {
        let deployment_snapshot: Value = serde_json::from_str(&row.snapshot_json)
            .map_err(|_| "release_snapshot_invalid".to_owned())?;
        let expected_checkout_digest = deployment_snapshot
            .get("image")
            .and_then(|image| image.get("checkout_tree_digest"))
            .and_then(Value::as_str)
            .ok_or_else(|| "release_snapshot_invalid".to_owned())?;
        if !expected_checkout_digest.eq_ignore_ascii_case(&request.checkout_tree_digest) {
            return Err("release_checkout_mismatch".to_owned());
        }
    }
    let expected_manifest_digest = row
        .manifest_digest
        .as_deref()
        .ok_or_else(|| "release_artifact_not_verified".to_owned())?;
    if row.artifact_id.is_none()
        || !expected_manifest_digest.eq_ignore_ascii_case(&request.artifact_manifest_digest)
    {
        return Err("release_artifact_mismatch".to_owned());
    }
    let manifest: AuthorizationArtifactManifest = serde_json::from_str(
        row.manifest_json
            .as_deref()
            .ok_or_else(|| "release_artifact_not_verified".to_owned())?,
    )
    .map_err(|_| "release_artifact_manifest_invalid".to_owned())?;
    let mut expected_artifacts = manifest
        .artifacts
        .into_iter()
        .map(|item| (item.path, item.sha256.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    let mut actual_artifacts = request
        .artifacts
        .iter()
        .map(|item| (item.relative_path.clone(), item.digest.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    expected_artifacts.sort();
    actual_artifacts.sort();
    if expected_artifacts != actual_artifacts {
        return Err("release_artifact_mismatch".to_owned());
    }
    let mut expected_env = task
        .required_env
        .iter()
        .filter(|item| item.action == EnvSyncAction::Write)
        .map(|item| (item.file_name.clone(), item.digest.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    let mut actual_env = request
        .env_files
        .iter()
        .map(|item| (item.relative_path.clone(), item.digest.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    expected_env.sort();
    actual_env.sort();
    if expected_env != actual_env {
        return Err("release_env_mismatch".to_owned());
    }
    let deadline = chrono::DateTime::parse_from_rfc3339(&row.deadline_at)
        .map_err(|_| "release_deadline_invalid".to_owned())?
        .timestamp();
    let now = Utc::now().timestamp();
    if deadline <= now {
        return Err("release_deadline_expired".to_owned());
    }
    let expires_at = deadline.min(now.saturating_add(300));
    let environment = serde_json::to_value(&task.environment)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| "release_task_payload_invalid".to_owned())?;
    signer
        .sign(&Claims {
            schema_version: SCHEMA_VERSION,
            audience: AUDIENCE.to_owned(),
            authorization_id: request.authorization_id.clone(),
            nonce: format!("release_nonce_{}", Ulid::new()),
            deployment_id: row.deployment_id,
            target_run_id: row.target_run_id,
            target_id: row.target_id,
            node_id: row.node_id,
            agent_id: agent_id.to_owned(),
            snapshot_hash: request.snapshot_hash.clone(),
            commit_sha: task.commit_sha,
            checkout_tree_digest: request.checkout_tree_digest.clone(),
            artifact_manifest_digest: request.artifact_manifest_digest.clone(),
            artifacts: request
                .artifacts
                .iter()
                .map(|item| FileDigest {
                    relative_path: item.relative_path.clone(),
                    digest: item.digest.clone(),
                })
                .collect(),
            env_files: request
                .env_files
                .iter()
                .map(|item| FileDigest {
                    relative_path: item.relative_path.clone(),
                    digest: item.digest.clone(),
                })
                .collect(),
            environment,
            release_version: task.release_version,
            modules: task.modules,
            task_payload_digest: row.payload_digest,
            cancel_file: request.cancel_file.clone(),
            issued_at: now,
            expires_at,
            deadline_at: deadline,
        })
        .map_err(|_| "release_authorization_failed".to_owned())
}

async fn handle_artifact_prepared(
    state: &AppState,
    agent_id: &str,
    generation: i64,
    prepared: &ArtifactPrepared,
) -> ApiResult<()> {
    ensure_current_connection(state, agent_id, generation).await?;
    let response = authorize_artifact_upload(state, agent_id, prepared).await;
    let response = match response {
        Ok(lease_id) => ArtifactUploadAuthorized {
            task_id: prepared.task_id.clone(),
            authorization_id: prepared.authorization_id.clone(),
            lease_id: Some(lease_id),
            error_code: None,
        },
        Err(code) => {
            tracing::warn!(
                agent_id,
                task_id = %prepared.task_id,
                deployment_id = %prepared.deployment_id,
                error_code = %code,
                "artifact upload authorization rejected"
            );
            ArtifactUploadAuthorized {
                task_id: prepared.task_id.clone(),
                authorization_id: prepared.authorization_id.clone(),
                lease_id: None,
                error_code: Some(code),
            }
        }
    };
    state
        .agent_connections()
        .send(agent_id, Message::ArtifactUploadAuthorized(response))
        .await
        .map_err(|_| {
            ApiError::conflict(
                "artifact_authorization_delivery_failed",
                "制品授权响应投递失败",
                "artifact_authorization",
            )
        })?;
    Ok(())
}

async fn authorize_artifact_upload(
    state: &AppState,
    agent_id: &str,
    prepared: &ArtifactPrepared,
) -> Result<String, String> {
    if !state.cross_node_artifacts_enabled() {
        return Err("cross_node_artifacts_disabled".to_owned());
    }
    let task: Option<(String, String, String)> = sqlx::query_as(
        "SELECT deployment_id,payload_json,deadline_at FROM agent_tasks WHERE id=? AND agent_id=? AND kind='deployment_prepare' AND status IN ('delivered','accepted','running')",
    )
    .bind(&prepared.task_id)
    .bind(agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| "artifact_authorization_failed".to_owned())?;
    let Some((deployment_id, payload_json, deadline_at)) = task else {
        return Err("artifact_prepare_task_inactive".to_owned());
    };
    if deployment_id != prepared.deployment_id {
        return Err("artifact_deployment_mismatch".to_owned());
    }
    let payload: TaskPayload = serde_json::from_str(&payload_json)
        .map_err(|_| "artifact_prepare_payload_invalid".to_owned())?;
    let TaskPayload::DeploymentPrepare(task) = payload else {
        return Err("artifact_prepare_payload_invalid".to_owned());
    };
    if task
        .artifact_upload
        .as_ref()
        .map(|item| item.authorization_id.as_str())
        != Some(prepared.authorization_id.as_str())
    {
        return Err("artifact_authorization_not_bound".to_owned());
    }
    validate_prepared_manifest(prepared, &task)?;
    let existing: Option<ExistingArtifactAuthorization> = sqlx::query_as(
        "SELECT artifact.status,artifact.manifest_digest,artifact.total_size,artifact.file_count,artifact.upload_size,artifact.archive_digest FROM artifact_leases lease JOIN deployment_artifacts artifact ON artifact.id=lease.artifact_id WHERE lease.id=? AND lease.agent_id=? AND lease.purpose='artifact_upload'",
    )
    .bind(&prepared.authorization_id)
    .bind(agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| "artifact_authorization_failed".to_owned())?;
    if let Some(existing) = existing {
        if existing.manifest_digest != prepared.manifest_digest
            || u64::try_from(existing.total_size).ok() != Some(prepared.total_size)
            || u32::try_from(existing.file_count).ok() != Some(prepared.file_count)
            || existing
                .upload_size
                .is_some_and(|value| u64::try_from(value).ok() != Some(prepared.archive_size))
            || existing
                .archive_digest
                .as_deref()
                .is_some_and(|value| value != prepared.archive_digest)
        {
            return Err("artifact_authorization_facts_conflict".to_owned());
        }
        if existing.status == "verified" {
            return Err("artifact_already_verified".to_owned());
        }
        return Ok(prepared.authorization_id.clone());
    }
    let artifact_id = format!("artifact_{}", Ulid::new());
    let now = Utc::now();
    let configured_expiry = now
        + Duration::seconds(
            i64::try_from(
                state
                    .artifact_store()
                    .ok_or_else(|| "artifact_store_unavailable".to_owned())?
                    .config()
                    .upload_ttl_seconds,
            )
            .unwrap_or(i64::MAX),
        );
    let deadline = chrono::DateTime::parse_from_rfc3339(&deadline_at)
        .map_err(|_| "artifact_prepare_payload_invalid".to_owned())?
        .with_timezone(&Utc);
    let expires_at = configured_expiry.min(deadline).to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| "artifact_authorization_failed".to_owned())?;
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_json,manifest_digest,total_size,file_count,upload_size,archive_digest,status,expires_at) VALUES(?,?,?,?,?,?,?,?,'uploading',?)")
        .bind(&artifact_id)
        .bind(&deployment_id)
        .bind(&prepared.manifest_json)
        .bind(&prepared.manifest_digest)
        .bind(i64::try_from(prepared.total_size).map_err(|_| "artifact_manifest_invalid".to_owned())?)
        .bind(i64::from(prepared.file_count))
        .bind(i64::try_from(prepared.archive_size).map_err(|_| "artifact_manifest_invalid".to_owned())?)
        .bind(&prepared.archive_digest)
        .bind(&expires_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| "artifact_authorization_failed".to_owned())?;
    sqlx::query("INSERT INTO artifact_leases(id,artifact_id,agent_id,purpose,manifest_digest,status,expires_at) VALUES(?,?,?,'artifact_upload',?,'active',?)")
        .bind(&prepared.authorization_id)
        .bind(&artifact_id)
        .bind(agent_id)
        .bind(&prepared.manifest_digest)
        .bind(&expires_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| "artifact_authorization_failed".to_owned())?;
    transaction
        .commit()
        .await
        .map_err(|_| "artifact_authorization_failed".to_owned())?;
    Ok(prepared.authorization_id.clone())
}

fn validate_prepared_manifest(
    prepared: &ArtifactPrepared,
    task: &DeploymentPrepareTask,
) -> Result<(), String> {
    if format!("{:x}", Sha256::digest(prepared.manifest_json.as_bytes()))
        != prepared.manifest_digest
        || prepared.archive_size == 0
        || prepared.archive_digest.len() != 64
        || !prepared
            .archive_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("artifact_manifest_invalid".to_owned());
    }
    let manifest: Value = serde_json::from_str(&prepared.manifest_json)
        .map_err(|_| "artifact_manifest_invalid".to_owned())?;
    if manifest.get("schema_version").and_then(Value::as_u64) != Some(1)
        || manifest.get("release_version").and_then(Value::as_str)
            != Some(task.release_version.as_str())
        || manifest.get("commit_sha").and_then(Value::as_str) != Some(task.commit_sha.as_str())
    {
        return Err("artifact_manifest_invalid".to_owned());
    }
    let entries = manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or_else(|| "artifact_manifest_invalid".to_owned())?;
    let modules = entries
        .iter()
        .filter_map(|entry| entry.get("module").and_then(Value::as_str))
        .collect::<std::collections::HashSet<_>>();
    let expected = task
        .modules
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    let total = entries
        .iter()
        .filter_map(|entry| entry.get("size").and_then(Value::as_u64))
        .try_fold(0_u64, u64::checked_add)
        .ok_or_else(|| "artifact_manifest_invalid".to_owned())?;
    if entries.is_empty()
        || entries.len() != modules.len()
        || modules != expected
        || total != prepared.total_size
        || usize::try_from(prepared.file_count).ok() != Some(entries.len())
    {
        return Err("artifact_manifest_invalid".to_owned());
    }
    Ok(())
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
        let reported_sequence = i64::try_from(task.last_sequence).ok();
        if digest != task.payload_digest
            || reported_sequence.is_none_or(|sequence| sequence < last_sequence)
            || task.state == ReconciledTaskState::Unknown
        {
            interrupt_task(state, &task.task_id, "Agent 恢复对账不一致").await?;
            continue;
        }
        if reported_sequence.is_some_and(|sequence| sequence > last_sequence) {
            advance_reconciled_sequence(state, &task.task_id, last_sequence, task.last_sequence)
                .await?;
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
                restore_task_state(state, &task.task_id, "queued", "running", "accepted").await?;
                try_dispatch(state, &task.task_id).await?;
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

async fn advance_reconciled_sequence(
    state: &AppState,
    task_id: &str,
    expected_sequence: i64,
    reported_sequence: u64,
) -> ApiResult<()> {
    let reported_sequence =
        i64::try_from(reported_sequence).map_err(|_| ApiError::internal("agent_reconcile"))?;
    let updated = sqlx::query("UPDATE agent_tasks SET last_sequence=?,updated_at=? WHERE id=? AND last_sequence=? AND status IN ('delivered','accepted','running','canceling')")
        .bind(reported_sequence)
        .bind(Utc::now().to_rfc3339())
        .bind(task_id)
        .bind(expected_sequence)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal("agent_reconcile"))?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "agent_reconcile_race",
            "Agent 对账期间任务状态已变化",
            "agent_reconcile",
        ));
    }
    tracing::warn!(
        task_id,
        expected_sequence,
        reported_sequence,
        "Agent 重连时主控缺失部分任务事件，已按 Agent journal 推进序号"
    );
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
        TaskPayload::DeploymentRelease(task) => task.git_credential_lease_id.clone(),
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
        finish_env_sync_for_task(state, &ack.task_id, "failed", ack.error_code.as_deref()).await?;
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
    let task_sequence =
        i64::try_from(task_sequence).map_err(|_| ApiError::internal("agent_event"))?;
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
    if task_status == "running" {
        finish_env_sync_for_task(state, &task_state.task_id, "syncing", None).await?;
    }
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
    finish_env_sync_for_task(state, &result.task_id, status, result.error_code.as_deref()).await?;
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

async fn finish_env_sync_for_task(
    state: &AppState,
    task_id: &str,
    status: &str,
    error_code: Option<&str>,
) -> ApiResult<()> {
    let now = Utc::now().to_rfc3339();
    match status {
        "syncing" => {
            sqlx::query("UPDATE application_env_syncs SET status='syncing',attempt_count=attempt_count+CASE WHEN status='pending' THEN 1 ELSE 0 END,last_attempt_at=?,updated_at=? WHERE id=(SELECT env_sync_id FROM agent_tasks WHERE id=?) AND status IN ('pending','syncing')")
                .bind(&now)
                .bind(&now)
                .bind(task_id)
                .execute(state.pool())
                .await
                .map_err(|_| ApiError::internal("env_sync_result"))?;
        }
        "succeeded" => {
            sqlx::query("UPDATE application_env_syncs SET status='succeeded',actual_version=(SELECT version.env_version FROM application_env_versions version WHERE version.id=application_env_syncs.env_version_id),error_code=NULL,error_message=NULL,last_attempt_at=COALESCE(last_attempt_at,?),synced_at=?,updated_at=? WHERE id=(SELECT env_sync_id FROM agent_tasks WHERE id=?) AND status IN ('pending','syncing')")
                .bind(&now)
                .bind(&now)
                .bind(&now)
                .bind(task_id)
                .execute(state.pool())
                .await
                .map_err(|_| ApiError::internal("env_sync_result"))?;
        }
        "failed" | "canceled" | "interrupted" => {
            let sanitized = match error_code {
                Some("env_sync_digest_mismatch") => "env_sync_digest_mismatch",
                Some("env_sync_unsafe_target") => "env_sync_unsafe_target",
                Some("env_sync_lease_rejected") => "env_sync_lease_rejected",
                Some("env_sync_disabled") => "env_sync_disabled",
                _ => "env_sync_failed",
            };
            sqlx::query("UPDATE application_env_syncs SET status='failed',error_code=?,error_message='Env 同步失败',last_attempt_at=COALESCE(last_attempt_at,?),updated_at=? WHERE id=(SELECT env_sync_id FROM agent_tasks WHERE id=?) AND status IN ('pending','syncing')")
                .bind(sanitized)
                .bind(&now)
                .bind(&now)
                .bind(task_id)
                .execute(state.pool())
                .await
                .map_err(|_| ApiError::internal("env_sync_result"))?;
        }
        _ => {}
    }
    Ok(())
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
    sqlx::query("UPDATE application_env_secret_leases SET status='revoked' WHERE env_sync_id=(SELECT env_sync_id FROM agent_tasks WHERE id=?) AND status='issued'")
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
    let task: Option<TerminalTaskRow> =
        sqlx::query_as("SELECT task.stage,task.deployment_id,task.target_run_id,COALESCE(json_type(deployment.snapshot_json,'$.targets') IS NOT NULL,0) AS multi_target,deployment.cancel_requested_at,COALESCE(json_extract(deployment.snapshot_json,'$.release_strategy'),'automatic') AS release_strategy FROM agent_tasks task LEFT JOIN deployments deployment ON deployment.id=task.deployment_id WHERE task.id=?")
            .bind(task_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal("agent_event"))?;
    let Some(TerminalTaskRow {
        stage,
        deployment_id,
        target_run_id,
        multi_target,
        cancel_requested_at,
        release_strategy,
    }) = task
    else {
        return Ok(());
    };
    let Some(deployment_id) = deployment_id else {
        return Ok(());
    };
    if let Some(target_run_id) = target_run_id.as_deref() {
        let run_status = match status {
            "succeeded" => "succeeded",
            "canceled" => "canceled",
            _ => "failed",
        };
        if multi_target == 0 {
            sqlx::query("UPDATE deployment_target_runs SET status=?,phase=?,result_summary=?,error_code=CASE WHEN ?='succeeded' THEN NULL ELSE ? END,finished_at=?,updated_at=?,version=version+1 WHERE id=?")
                .bind(run_status).bind(run_status).bind(summary).bind(run_status).bind(status)
                .bind(&now).bind(&now).bind(target_run_id).execute(state.pool()).await
                .map_err(|_| ApiError::internal("agent_event"))?;
        } else {
            let mut transaction = state
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::internal("agent_event"))?;
            sqlx::query("UPDATE deployment_target_runs SET status=?,phase=?,result_summary=?,error_code=CASE WHEN ?='succeeded' THEN NULL ELSE ? END,finished_at=?,updated_at=?,version=version+1 WHERE id=?")
            .bind(run_status).bind(run_status).bind(summary).bind(run_status).bind(status)
            .bind(&now).bind(&now).bind(target_run_id).execute(&mut *transaction).await
            .map_err(|_| ApiError::internal("agent_event"))?;
            let counts: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT COUNT(*),SUM(CASE WHEN status IN ('succeeded','reused') THEN 1 ELSE 0 END),SUM(CASE WHEN status IN ('failed','expired') THEN 1 ELSE 0 END),SUM(CASE WHEN status='canceled' THEN 1 ELSE 0 END),SUM(CASE WHEN status NOT IN ('succeeded','reused','failed','expired','canceled') THEN 1 ELSE 0 END),SUM(CASE WHEN status IN ('downloading','running') THEN 1 ELSE 0 END) FROM deployment_target_runs WHERE deployment_id=?",
        ).bind(&deployment_id).fetch_one(&mut *transaction).await
            .map_err(|_| ApiError::internal("agent_event"))?;
            let all_terminal = counts.0 > 0 && counts.4 == 0;
            let (deployment_status, phase, terminal, deployment_summary) = if !all_terminal {
                if cancel_requested_at.is_some() {
                    ("canceling", "canceling", false, "等待目标取消完成")
                } else if counts.5 > 0 {
                    ("running", "targets_running", false, "目标部署执行中")
                } else {
                    ("running", "targets_pending", false, "等待目标部署")
                }
            } else if counts.2 > 0 {
                ("failed", "targets_failed", true, "至少一个目标部署失败")
            } else if counts.0 > 0 && counts.1 == counts.0 {
                ("succeeded", "targets_succeeded", true, "全部目标部署成功")
            } else if counts.0 > 0 && counts.1 + counts.3 == counts.0 && counts.3 > 0 {
                ("canceled", "targets_canceled", true, "目标部署已取消")
            } else {
                ("running", "targets_pending", false, "等待目标部署")
            };
            sqlx::query("UPDATE deployments SET status=?,phase=?,result_summary=?,protocol_complete=?,finished_at=CASE WHEN ? THEN ? ELSE NULL END,updated_at=?,version=version+1 WHERE id=?")
            .bind(deployment_status).bind(phase).bind(deployment_summary).bind(terminal)
            .bind(terminal).bind(&now).bind(&now).bind(&deployment_id)
            .execute(&mut *transaction).await.map_err(|_| ApiError::internal("agent_event"))?;
            transaction
                .commit()
                .await
                .map_err(|_| ApiError::internal("agent_event"))?;
            return Ok(());
        }
    }
    if status == "succeeded" && stage.as_deref() == Some("prepare") {
        let next_phase = if release_strategy == "manual" {
            "awaiting_release"
        } else {
            "deploying"
        };
        sqlx::query("UPDATE deployments SET status='running',phase=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running')")
            .bind(next_phase)
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
        if status != "succeeded" {
            fail_remaining_runs_for_deployment(state, &deployment_id, summary, status).await?;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use deploy_go_release_authorization::ExpectedBinding;
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn privileged_release_requires_v7_and_explicit_capability() {
        assert_eq!(
            privileged_release_compatibility(Some(6), Some(r#"["privileged_release"]"#)),
            Err((
                "privileged_release_protocol_unsupported",
                "目标 Agent 不支持特权 release 控制协议 v7"
            ))
        );
        assert_eq!(
            privileged_release_compatibility(Some(7), Some(r#"["pty_terminal"]"#)),
            Err((
                "privileged_release_capability_unavailable",
                "目标 Agent 的特权 release executor 不可用"
            ))
        );
        assert_eq!(
            privileged_release_compatibility(
                Some(7),
                Some(r#"["pty_terminal","privileged_release"]"#)
            ),
            Ok(())
        );
        assert!(privileged_release_compatibility(Some(7), Some("invalid")).is_err());
    }

    #[tokio::test]
    async fn pre_dispatch_failure_closes_run_and_terminal_deployment() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node','Node','online')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target','app','node','test','two_stage','/unused',60,'active')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment','app','target','admin','running','targets_pending','idem','request','snapshot','{\"targets\":[]}' )").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,target_snapshot_json,status,phase,env_gate_status) VALUES('run','deployment','target','node','{}','pending','pending','not_required')").execute(&pool).await.unwrap();
        let state = AppState::new(pool.clone());

        fail_target_run_before_dispatch(
            &state,
            "run",
            "privileged_release_capability_unavailable",
            "目标 Agent 的特权 release executor 不可用",
        )
        .await
        .unwrap();

        let run: (String, String, String) = sqlx::query_as(
            "SELECT status,error_code,result_summary FROM deployment_target_runs WHERE id='run'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(run.0, "failed");
        assert_eq!(run.1, "privileged_release_capability_unavailable");
        assert_eq!(run.2, "目标 Agent 的特权 release executor 不可用");
        let deployment: (String, String, i64) = sqlx::query_as(
            "SELECT status,phase,protocol_complete FROM deployments WHERE id='deployment'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(deployment, ("failed".into(), "targets_failed".into(), 1));
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 0);
    }

    #[tokio::test]
    async fn deployment_level_failure_closes_pending_runs() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node','Node','online')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target','app','node','test','two_stage','/unused',60,'active')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment','app','target','admin','queued','targets_pending','idem','request','snapshot','{\"targets\":[]}' )").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,target_snapshot_json,status,phase,env_gate_status) VALUES('run','deployment','target','node','{}','pending','pending','not_required')").execute(&pool).await.unwrap();
        let state = AppState::new(pool.clone());

        fail_deployment_before_dispatch(
            &state,
            "deployment",
            "build_agent_unavailable",
            "构建节点不可用",
        )
        .await
        .unwrap();

        let run: (String, String, String) = sqlx::query_as(
            "SELECT status,error_code,result_summary FROM deployment_target_runs WHERE id='run'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(run.0, "failed");
        assert_eq!(run.1, "build_agent_unavailable");
        assert!(run.2.contains("构建节点不可用"));
        let deployment: (String, String, i64) = sqlx::query_as(
            "SELECT status,phase,protocol_complete FROM deployments WHERE id='deployment'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(deployment, ("failed".into(), "targets_failed".into(), 1));
    }

    #[tokio::test]
    async fn prepare_failure_closes_pending_run() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node','Node','online')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,agent_version,protocol_version) VALUES('agent','node','0.2.0',9)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target','app','node','test','two_stage','/unused',60,'active')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment','app','target','admin','running','preparing','idem','request','snapshot','{\"targets\":[]}' )").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,target_snapshot_json,status,phase,env_gate_status) VALUES('run','deployment','target','node','{}','pending','pending','not_required')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task','agent','deployment','prepare','deployment_prepare','idem','digest','{}','failed','2030-01-01T00:00:00Z')").execute(&pool).await.unwrap();
        let state = AppState::new(pool.clone());

        finish_deployment_for_task(&state, "task", "failed", "构建失败", Some(2))
            .await
            .unwrap();

        let run: (String, String, String) = sqlx::query_as(
            "SELECT status,error_code,result_summary FROM deployment_target_runs WHERE id='run'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(run.0, "failed");
        assert_eq!(run.1, "failed");
        assert_eq!(run.2, "构建失败");
        let deployment: (String, String, i64) = sqlx::query_as(
            "SELECT status,phase,protocol_complete FROM deployments WHERE id='deployment'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(deployment, ("failed".into(), "failed".into(), 1));
    }

    #[tokio::test]
    async fn terminal_deployment_reconcile_closes_remaining_runs() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node','Node','online')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target','app','node','test','two_stage','/unused',60,'active')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,target_code,execution_mode,script_path,timeout_seconds,status) VALUES('target2','app','node','staging','test2','two_stage','/unused',60,'active')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment','app','target','admin','failed','failed','idem','request','snapshot','{\"targets\":[]}' )").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,target_snapshot_json,status,phase,env_gate_status) VALUES('run-pending','deployment','target','node','{}','pending','pending','not_required')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,target_snapshot_json,status,phase,env_gate_status,finished_at) VALUES('run-succeeded','deployment','target2','node','{}','succeeded','succeeded','not_required','2026-08-13T00:00:00Z')").execute(&pool).await.unwrap();
        let state = AppState::new(pool.clone());

        terminalize_runs_for_terminal_deployments(&state)
            .await
            .unwrap();

        let pending: (String, String, String) = sqlx::query_as(
            "SELECT status,error_code,result_summary FROM deployment_target_runs WHERE id='run-pending'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(pending.0, "failed");
        assert_eq!(pending.1, "deployment_terminal");
        assert!(pending.2.contains("部署已终止"));
        let succeeded: String = sqlx::query_scalar(
            "SELECT status FROM deployment_target_runs WHERE id='run-succeeded'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(succeeded, "succeeded");
    }

    #[tokio::test]
    async fn privileged_release_authorization_is_bound_to_active_snapshot_and_artifact() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node','Node','online')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,agent_version,protocol_version) VALUES('agent','node','0.1.0',7)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,privileged_release,status) VALUES('target','app','node','test','two_stage','/unused',60,1,'active')").execute(&pool).await.unwrap();
        let snapshot_hash = "a".repeat(64);
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment','app','target','admin','running','deploying','idem','request',?,'{}')")
            .bind(&snapshot_hash)
            .execute(&pool)
            .await
            .unwrap();
        let manifest = json!({"artifacts":[{"path":"api.tar.gz","sha256":"c".repeat(64)}]});
        sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_json,manifest_digest,storage_key,status,upload_offset,upload_size,archive_digest,expires_at,verified_at) VALUES('artifact','deployment',?,?,?,'verified',1,1,?,?,?)")
            .bind(manifest.to_string())
            .bind("b".repeat(64))
            .bind("d".repeat(64))
            .bind("d".repeat(64))
            .bind((Utc::now() + Duration::hours(1)).to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,artifact_id,target_snapshot_json,status,env_gate_status) VALUES('run','deployment','target','node','agent','artifact','{\"privileged_release\":true}','running','ready')")
            .execute(&pool)
            .await
            .unwrap();
        let task = TaskPayload::DeploymentRelease(DeploymentReleaseTask {
            deployment_id: "deployment".into(),
            target_code: "test".into(),
            work_root: "/srv/deploy-go".into(),
            checkout_dir: "/srv/deploy-go/deployments/deployment/checkout".into(),
            artifact_dir: "/srv/deploy-go/deployments/deployment/staging".into(),
            environment: Environment::Test,
            release_version: "release-1".into(),
            commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            modules: vec!["api".into()],
            make_target: MakeTarget::DeployGoRelease,
            timeout_seconds: 600,
            cancel_file: String::new(),
            privileged: true,
            privileged_context: Some(deploy_go_agent_protocol::PrivilegedReleaseContext {
                target_run_id: "run".into(),
                target_id: "target".into(),
                node_id: "node".into(),
                agent_id: "agent".into(),
                snapshot_hash: snapshot_hash.clone(),
            }),
            artifact_download: Some(ArtifactDownloadRequest {
                target_run_id: "run".into(),
                lease_id: "artifact_lease".into(),
                archive_digest: "d".repeat(64),
                manifest_digest: "b".repeat(64),
            }),
            repository_url: Some("https://git.example.test/app.git".into()),
            git_credential_lease_id: None,
            application_slug: None,
            required_env: Vec::new(),
            image_spec: None,
        });
        let payload_json = serde_json::to_string(&task).unwrap();
        let payload_digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
        let deadline = Utc::now() + Duration::minutes(10);
        sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,target_run_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task','agent','deployment','run','release','deployment_release','deployment:release',?,?,'running',?)")
            .bind(&payload_digest)
            .bind(&payload_json)
            .bind(deadline.to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        let signer = deploy_go_release_authorization::ReleaseSigner::from_seed([7_u8; 32]);
        let state = AppState::new(pool.clone()).with_release_signer(signer.clone());
        let request = ReleaseAuthorizationRequest {
            task_id: "task".into(),
            authorization_id: "release_auth_01".into(),
            target_run_id: "run".into(),
            target_id: "target".into(),
            snapshot_hash: snapshot_hash.clone(),
            checkout_tree_digest: "e".repeat(64),
            artifact_manifest_digest: "b".repeat(64),
            artifacts: vec![deploy_go_agent_protocol::ReleaseFileDigest {
                relative_path: "api.tar.gz".into(),
                digest: "c".repeat(64),
            }],
            env_files: Vec::new(),
            cancel_file: "/srv/deploy-go/tasks/task/cancel".into(),
        };

        let token = authorize_privileged_release(&state, "agent", &request)
            .await
            .unwrap();
        signer
            .verifier()
            .verify(
                &token,
                &ExpectedBinding {
                    deployment_id: "deployment",
                    target_run_id: "run",
                    target_id: "target",
                    node_id: "node",
                    agent_id: "agent",
                    snapshot_hash: &snapshot_hash,
                    commit_sha: "0123456789abcdef0123456789abcdef01234567",
                    task_payload_digest: &payload_digest,
                    deadline_at: deadline.timestamp(),
                },
                Utc::now().timestamp(),
            )
            .unwrap();

        // 快照中不再存在 privileged_release 开关；release 固定特权，授权不受该字段影响。
        sqlx::query("UPDATE deployment_target_runs SET target_snapshot_json='{}' WHERE id='run'")
            .execute(&pool)
            .await
            .unwrap();
        authorize_privileged_release(&state, "agent", &request)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn cross_node_dispatch_skips_queued_deployment_when_target_already_active() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO nodes(id,name,status,work_root) VALUES('node','Node','online','/srv/deploy-go')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,agent_version,protocol_version) VALUES('build_agent','node','0.2.0',7)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target','app','node','test','two_stage','/unused',60,'active')")
            .execute(&pool)
            .await
            .unwrap();
        let snapshot = json!({
            "targets": [{
                "target": {
                    "environment": "test",
                    "timeout_seconds": 60,
                    "node_id": "node"
                },
                "target_id": "target"
            }],
            "source": {
                "repository_url": "https://git.example.test/app.git",
                "resolved_commit_sha": "0123456789abcdef0123456789abcdef01234567",
                "build_agent_id": "build_agent"
            },
            "two_stage": {
                "release_version": "release-1",
                "modules": ["api"]
            }
        });
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('active_deployment','app','target','admin','canceling','targets_running','idem_active','request_active','snapshot_active',?)")
            .bind(snapshot.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('queued_deployment','app','target','admin','queued','queued','idem_queued','request_queued','snapshot_queued',?)")
            .bind(snapshot.to_string())
            .execute(&pool)
            .await
            .unwrap();
        let state = AppState::new(pool.clone()).with_cross_node_artifacts_enabled(true);

        let dispatched = dispatch_next_deployment(&state).await.unwrap();

        assert_eq!(dispatched, None);
        let prepare_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='queued_deployment' AND stage='prepare'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(prepare_count, 0);
    }
}
