use std::{convert::Infallible, time::Duration};

use async_stream::stream;
use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use ulid::Ulid;
use utoipa::ToSchema;

mod runtime;
pub use runtime::{process_one, purge_expired_output, recover, run_worker};

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    execution_spec::{self, TargetSnapshotInput},
    grants,
};

#[derive(Clone, Serialize, ToSchema)]
pub struct DeploymentResponse {
    pub id: String,
    pub application_id: String,
    pub target_id: String,
    pub requested_by: String,
    pub retry_of_id: Option<String>,
    pub status: String,
    pub phase: String,
    pub snapshot_hash: String,
    pub result_summary: Option<String>,
    pub exit_code: Option<i64>,
    pub protocol_complete: bool,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub cancel_requested_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
    pub execution_mode: String,
    pub release_strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<String>>,
    pub stage_tasks: Vec<DeploymentStageTaskSummary>,
    pub target_runs: Vec<DeploymentTargetRunResponse>,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct DeploymentTargetRunResponse {
    pub id: String,
    pub target_id: String,
    pub node_id: String,
    pub agent_id: Option<String>,
    pub source_run_id: Option<String>,
    pub status: String,
    pub phase: String,
    pub env_gate_status: String,
    pub result_summary: Option<String>,
    pub error_code: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct DeploymentStageTaskSummary {
    pub stage: String,
    pub task_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewRequest {
    parameters: Value,
    #[serde(default = "default_release_strategy")]
    release_strategy: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfirmRequest {
    parameters: Value,
    snapshot_hash: String,
    release_version: Option<String>,
    #[serde(default = "default_release_strategy")]
    release_strategy: String,
}

#[derive(Serialize, ToSchema)]
pub struct DeploymentPreviewResponse {
    target_id: String,
    application_id: String,
    application_name: String,
    node_id: String,
    node_name: String,
    environment: String,
    execution_mode: String,
    release_strategy: String,
    script_path: String,
    parameters: Value,
    snapshot_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deployment_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modules: Option<Vec<String>>,
}

#[derive(Serialize, ToSchema)]
pub struct ApplicationDeploymentPreviewResponse {
    application_id: String,
    application_name: String,
    execution_mode: String,
    release_strategy: String,
    parameters: Value,
    snapshot_hash: String,
    targets: Vec<DeploymentTargetPreviewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deployment_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modules: Option<Vec<String>>,
}

#[derive(Serialize, ToSchema)]
pub struct DeploymentTargetPreviewResponse {
    target_id: String,
    node_id: String,
    node_name: String,
    agent_id: String,
    agent_online: bool,
    env_gate_status: String,
    script_path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LogQuery {
    after: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeploymentEventQuery {
    limit: Option<u32>,
    after: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeploymentListQuery {
    limit: Option<u32>,
    after: Option<String>,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct DeploymentLogResponse {
    sequence: i64,
    task_id: Option<String>,
    stage: Option<String>,
    task_sequence: Option<i64>,
    stream: String,
    content: String,
    truncated: bool,
    created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct DeploymentEventResponse {
    id: String,
    event_name: String,
    status: Option<String>,
    stage: Option<String>,
    module: Option<String>,
    module_name: Option<String>,
    step_id: Option<String>,
    step: Option<String>,
    failure_stage: Option<String>,
    message: Option<String>,
    created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct DeploymentEventListResponse {
    items: Vec<DeploymentEventResponse>,
    next_cursor: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DeploymentEventRow {
    id: String,
    event_name: String,
    status: Option<String>,
    payload_json: String,
    created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct DeploymentListResponse {
    items: Vec<DeploymentResponse>,
    next_cursor: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DeploymentRow {
    id: String,
    application_id: String,
    target_id: String,
    requested_by: String,
    retry_of_id: Option<String>,
    status: String,
    phase: String,
    snapshot_hash: String,
    result_summary: Option<String>,
    exit_code: Option<i64>,
    protocol_complete: bool,
    queued_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    cancel_requested_at: Option<String>,
    created_at: String,
    updated_at: String,
    version: i64,
    execution_mode: String,
    snapshot_json: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DeploymentTargetRunRow {
    deployment_id: String,
    id: String,
    target_id: String,
    node_id: String,
    agent_id: Option<String>,
    source_run_id: Option<String>,
    status: String,
    phase: String,
    env_gate_status: String,
    result_summary: Option<String>,
    error_code: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct DeploymentStageTaskRow {
    deployment_id: String,
    stage: String,
    task_id: String,
    status: String,
    result_json: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct TargetExecutionRow {
    target_id: String,
    application_id: String,
    application_name: String,
    application_status: String,
    node_id: String,
    node_name: String,
    node_status: String,
    agent_id: Option<String>,
    work_root: Option<String>,
    secrets_root: Option<String>,
    environment: String,
    execution_mode: String,
    script_path: String,
    parameter_schema: String,
    timeout_seconds: i64,
    verification_config: String,
    privileged_release: bool,
    target_status: String,
    target_version: i64,
}

struct PreviewData {
    response: DeploymentPreviewResponse,
    snapshot: Value,
}

struct ApplicationPreviewData {
    response: ApplicationDeploymentPreviewResponse,
    snapshot: Value,
    target_runs: Vec<TargetRunSnapshot>,
}

struct TargetRunSnapshot {
    target_id: String,
    node_id: String,
    agent_id: String,
    snapshot: Value,
}

struct TwoStageSourceInfo {
    source_id: String,
    repository_url: String,
    git_credential_id: Option<String>,
    build_agent_id: String,
    source_version: i64,
    deployment_branch: String,
    resolved_commit_sha: String,
    refs_discovery_id: String,
}

struct TwoStageParameters {
    release_version: String,
    modules: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct VerifiedSourceRow {
    id: String,
    repository_url: String,
    git_credential_id: Option<String>,
    build_agent_id: String,
    source_version: i64,
    deployment_branch: String,
}

#[derive(sqlx::FromRow)]
struct RefDiscoveryRow {
    id: String,
    refs_json: String,
    expires_at: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/applications/{id}/deployment-preview",
            post(application_preview),
        )
        .route("/applications/{id}/deployments", post(application_confirm))
        .route("/deployment-targets/{id}/deployment-preview", post(preview))
        .route("/deployment-targets/{id}/deployments", post(confirm))
        .route("/deployments", get(list))
        .route("/deployments/{id}", get(show))
        .route("/deployments/{id}/events", get(events))
        .route("/deployments/{id}/logs", get(logs))
        .route("/deployments/{id}/cancel", post(cancel))
        .route("/deployments/{id}/retry", post(retry))
        .route("/deployments/{id}/release", post(release))
}

pub(crate) fn default_release_strategy() -> String {
    "automatic".to_owned()
}

fn validate_release_strategy(value: &str, execution_mode: &str, request_id: &str) -> ApiResult<()> {
    if !matches!(value, "automatic" | "manual") {
        return Err(ApiError::validation(
            "发布策略必须是 automatic 或 manual",
            request_id,
        ));
    }
    if execution_mode != "two_stage" && value != "automatic" {
        return Err(ApiError::validation(
            "只有两阶段部署支持手动发布",
            request_id,
        ));
    }
    Ok(())
}

#[utoipa::path(operation_id = "application_deployments_preview", post, path = "/api/v1/applications/{id}/deployment-preview", params(("id" = String, Path)), request_body = PreviewRequest, responses((status = 200, body = ApplicationDeploymentPreviewResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn application_preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<PreviewRequest>,
) -> ApiResult<Json<ApplicationDeploymentPreviewResponse>> {
    let preview = build_application_preview(
        &state,
        &actor,
        &id,
        &payload.parameters,
        &payload.release_strategy,
        None,
        request_id.as_str(),
    )
    .await?;
    Ok(Json(preview.response))
}

#[utoipa::path(operation_id = "application_deployments_confirm", post, path = "/api/v1/applications/{id}/deployments", params(("id" = String, Path)), request_body = ConfirmRequest, responses((status = 200, body = DeploymentResponse), (status = 201, body = DeploymentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn application_confirm(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<ConfirmRequest>,
) -> ApiResult<(StatusCode, Json<DeploymentResponse>)> {
    actor.verify_csrf(&headers, request_id.as_str())?;
    let key = validate_idempotency_key(&headers, request_id.as_str())?;
    let (status, response) = create_application_deployment(
        &state,
        &actor,
        None,
        &id,
        &payload.parameters,
        Some(&payload.snapshot_hash),
        &payload.release_strategy,
        payload.release_version.as_deref(),
        &format!("application-confirm:{key}"),
        request_id.as_str(),
    )
    .await?;
    Ok((status, Json(response)))
}

#[allow(clippy::too_many_arguments)] // 部署创建输入字段较多，集中校验后统一落库
pub(crate) async fn create_application_deployment(
    state: &AppState,
    actor: &AuthUser,
    external_api_key_id: Option<&str>,
    application_id: &str,
    parameters: &Value,
    snapshot_hash: Option<&str>,
    release_strategy: &str,
    release_version: Option<&str>,
    stored_key: &str,
    request_id: &str,
) -> ApiResult<(StatusCode, DeploymentResponse)> {
    grants::require_application_access(state.pool(), actor, application_id, request_id).await?;
    let preview = build_application_preview(
        state,
        actor,
        application_id,
        parameters,
        release_strategy,
        release_version,
        request_id,
    )
    .await?;
    let snapshot_hash = snapshot_hash
        .map(str::to_owned)
        .unwrap_or_else(|| preview.response.snapshot_hash.clone());
    let request_hash = digest_json(&json!({
        "application_id": application_id,
        "parameters": parameters,
        "snapshot_hash": &snapshot_hash,
        "release_strategy": release_strategy,
        "release_version": release_version,
    }));
    if let Some(response) = find_idempotent(
        state.pool(),
        &actor.id,
        stored_key,
        &request_hash,
        request_id,
    )
    .await?
    {
        return Ok((StatusCode::OK, response));
    }
    if preview.response.snapshot_hash != snapshot_hash {
        return Err(ApiError::conflict(
            "deployment_snapshot_changed",
            "应用部署配置已经变化，请重新确认",
            request_id,
        ));
    }
    let deployment_id = format!("deployment_{}", Ulid::new());
    let representative = preview
        .target_runs
        .first()
        .ok_or_else(|| ApiError::internal(request_id))?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let insert = sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,external_api_key_id,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES(?,?,?,?,?, 'queued','targets_pending',?,?,?,?)")
        .bind(&deployment_id).bind(application_id).bind(&representative.target_id).bind(&actor.id).bind(external_api_key_id)
        .bind(stored_key).bind(&request_hash).bind(&snapshot_hash)
        .bind(preview.snapshot.to_string()).execute(&mut *transaction).await;
    if let Err(error) = insert {
        if error.to_string().contains("UNIQUE constraint failed") {
            drop(transaction);
            if let Some(response) = find_idempotent(
                state.pool(),
                &actor.id,
                stored_key,
                &request_hash,
                request_id,
            )
            .await?
            {
                return Ok((StatusCode::OK, response));
            }
            return Err(ApiError::conflict(
                "idempotency_conflict",
                "幂等键已经被并发请求使用",
                request_id,
            ));
        }
        return Err(ApiError::internal(request_id));
    }
    for run in preview.target_runs {
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,target_snapshot_json,status,phase,env_gate_status) VALUES(?,?,?,?,?,?,'pending','pending','not_required')")
            .bind(format!("run_{}", Ulid::new())).bind(&deployment_id).bind(run.target_id)
            .bind(run.node_id).bind(run.agent_id).bind(run.snapshot.to_string())
            .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
    }
    audit::record(&mut transaction, Some(&actor.id), "deployment.create", "deployment", &deployment_id, request_id, json!({"application_id":application_id,"target_count":preview.response.targets.len(),"snapshot_hash":&snapshot_hash})).await.map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok((
        StatusCode::CREATED,
        find(state.pool(), &deployment_id, request_id).await?,
    ))
}

#[utoipa::path(operation_id = "deployments_release", post, path = "/api/v1/deployments/{id}/release", params(("id" = String, Path), ("X-CSRF-Token" = String, Header)), responses((status = 200, body = DeploymentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn release(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Json<DeploymentResponse>> {
    actor.verify_csrf(&headers, request_id.as_str())?;
    require_access(&state, &actor, &id, request_id.as_str()).await?;
    let row: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT status,phase,snapshot_json,COALESCE(application_id,(SELECT application_id FROM deployment_targets WHERE id=deployments.target_id)) FROM deployments WHERE id=?",
    )
    .bind(&id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (status, phase, snapshot_json, application_id) =
        row.ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let snapshot: Value = serde_json::from_str(&snapshot_json)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if snapshot.get("release_strategy").and_then(Value::as_str) != Some("manual") {
        return Err(ApiError::conflict(
            "deployment_not_manual_release",
            "该部署不是手动发布模式",
            request_id.as_str(),
        ));
    }
    if status == "running"
        && matches!(
            phase.as_str(),
            "deploying" | "targets_pending" | "targets_running"
        )
    {
        return Ok(Json(find(state.pool(), &id, request_id.as_str()).await?));
    }
    if status != "running" || phase != "awaiting_release" {
        return Err(ApiError::conflict(
            "deployment_not_awaiting_release",
            "部署当前不在等待发布阶段",
            request_id.as_str(),
        ));
    }
    let prepare_succeeded: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM agent_tasks WHERE deployment_id=? AND stage='prepare' AND status='succeeded')",
    )
    .bind(&id)
    .fetch_one(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if !prepare_succeeded {
        return Err(ApiError::conflict(
            "deployment_prepare_incomplete",
            "prepare 尚未成功完成",
            request_id.as_str(),
        ));
    }
    if snapshot.get("targets").and_then(Value::as_array).is_some() {
        let artifact_ready: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM deployment_artifacts WHERE deployment_id=? AND status='verified')",
        )
        .bind(&id)
        .fetch_one(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
        if !artifact_ready {
            return Err(ApiError::conflict(
                "deployment_artifact_not_ready",
                "部署制品不存在或已经失效",
                request_id.as_str(),
            ));
        }
    }
    let blockers: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT run.target_id,node.id,node.name FROM deployment_target_runs run JOIN nodes node ON node.id=run.node_id WHERE run.deployment_id=? AND EXISTS (SELECT 1 FROM application_env_files file LEFT JOIN application_env_versions version ON version.env_file_id=file.id AND version.env_version=file.current_version LEFT JOIN application_env_syncs sync ON sync.env_version_id=version.id AND sync.target_id=run.target_id WHERE file.application_id=? AND file.deleted_at IS NULL AND (sync.status IS NULL OR sync.status!='succeeded' OR sync.actual_version IS NULL OR sync.actual_version!=file.current_version)) ORDER BY node.name,run.target_id",
    )
    .bind(&id)
    .bind(&application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if !blockers.is_empty() {
        return Err(ApiError::conflict(
            "deployment_env_not_ready",
            "部分目标节点的 Env 尚未同步完成",
            request_id.as_str(),
        )
        .with_details(json!({"targets": blockers.into_iter().map(|(target_id,node_id,node_name)| json!({"target_id":target_id,"node_id":node_id,"node_name":node_name})).collect::<Vec<_>>()})));
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let updated = sqlx::query("UPDATE deployments SET phase='deploying',updated_at=?,version=version+1 WHERE id=? AND status='running' AND phase='awaiting_release'")
        .bind(Utc::now().to_rfc3339()).bind(&id).execute(&mut *transaction).await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if updated.rows_affected() == 1 {
        sqlx::query("UPDATE deployment_artifacts SET expires_at=?,updated_at=?,version=version+1 WHERE deployment_id=? AND status='verified'")
            .bind((Utc::now() + chrono::Duration::hours(24)).to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .bind(&id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        audit::record(
            &mut transaction,
            Some(&actor.id),
            "deployment.release",
            "deployment",
            &id,
            request_id.as_str(),
            json!({"release_strategy":"manual"}),
        )
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    }
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "deployments_preview", post, path = "/api/v1/deployment-targets/{id}/deployment-preview", params(("id" = String, Path)), request_body = PreviewRequest, responses((status = 200, body = DeploymentPreviewResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<PreviewRequest>,
) -> ApiResult<Json<DeploymentPreviewResponse>> {
    let preview = build_preview(
        &state,
        &actor,
        &id,
        &payload.parameters,
        &payload.release_strategy,
        None,
        request_id.as_str(),
    )
    .await?;
    Ok(Json(preview.response))
}

#[utoipa::path(operation_id = "deployments_confirm", post, path = "/api/v1/deployment-targets/{id}/deployments", params(("id" = String, Path)), request_body = ConfirmRequest, responses((status = 200, body = DeploymentResponse), (status = 201, body = DeploymentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn confirm(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<ConfirmRequest>,
) -> ApiResult<(StatusCode, Json<DeploymentResponse>)> {
    actor.verify_csrf(&headers, request_id.as_str())?;
    let idempotency_key = validate_idempotency_key(&headers, request_id.as_str())?;
    let (status, response) = create_target_deployment(
        &state,
        &actor,
        None,
        &id,
        &payload.parameters,
        Some(&payload.snapshot_hash),
        &payload.release_strategy,
        payload.release_version.as_deref(),
        &format!("confirm:{idempotency_key}"),
        request_id.as_str(),
    )
    .await?;
    Ok((status, Json(response)))
}

#[allow(clippy::too_many_arguments)] // 部署创建输入字段较多，集中校验后统一落库
pub(crate) async fn create_target_deployment(
    state: &AppState,
    actor: &AuthUser,
    external_api_key_id: Option<&str>,
    target_id: &str,
    parameters: &Value,
    snapshot_hash: Option<&str>,
    release_strategy: &str,
    release_version: Option<&str>,
    stored_idempotency_key: &str,
    request_id: &str,
) -> ApiResult<(StatusCode, DeploymentResponse)> {
    let preview = build_preview(
        state,
        actor,
        target_id,
        parameters,
        release_strategy,
        release_version,
        request_id,
    )
    .await?;
    let snapshot_hash = snapshot_hash
        .map(str::to_owned)
        .unwrap_or_else(|| preview.response.snapshot_hash.clone());
    if preview.response.snapshot_hash != snapshot_hash {
        return Err(ApiError::conflict(
            "deployment_snapshot_changed",
            "部署目标配置已经变化，请重新确认",
            request_id,
        ));
    }
    let request_hash = digest_json(
        &json!({"target_id":target_id,"parameters":parameters,"snapshot_hash":&snapshot_hash,"release_strategy":release_strategy}),
    );
    if let Some((existing_id, existing_hash)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id, request_hash FROM deployments WHERE requested_by=? AND idempotency_key=?",
    )
    .bind(&actor.id)
    .bind(stored_idempotency_key)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?
    {
        if existing_hash != request_hash {
            return Err(ApiError::conflict(
                "idempotency_conflict",
                "幂等键已用于不同部署请求",
                request_id,
            ));
        }
        return Ok((
            StatusCode::OK,
            find(state.pool(), &existing_id, request_id).await?,
        ));
    }
    let deployment_id = format!("deployment_{}", Ulid::new());
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let insert = sqlx::query("INSERT INTO deployments (id,application_id,target_id,requested_by,external_api_key_id,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES (?, ?, ?, ?, ?, 'queued', 'queued', ?, ?, ?, ?)")
        .bind(&deployment_id).bind(&preview.response.application_id).bind(target_id).bind(&actor.id).bind(external_api_key_id).bind(stored_idempotency_key).bind(&request_hash).bind(&snapshot_hash).bind(preview.snapshot.to_string())
        .execute(&mut *transaction).await;
    if let Err(error) = insert {
        if error.to_string().contains("UNIQUE constraint failed") {
            drop(transaction);
            let existing: Option<(String, String)> = sqlx::query_as("SELECT id,request_hash FROM deployments WHERE requested_by=? AND idempotency_key=?")
                .bind(&actor.id).bind(stored_idempotency_key).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id))?;
            if let Some((existing_id, existing_hash)) = existing
                && existing_hash == request_hash
            {
                return Ok((
                    StatusCode::OK,
                    find(state.pool(), &existing_id, request_id).await?,
                ));
            }
            return Err(ApiError::conflict(
                "idempotency_conflict",
                "幂等键已经被并发请求使用",
                request_id,
            ));
        }
        return Err(ApiError::internal(request_id));
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment.create",
        "deployment",
        &deployment_id,
        request_id,
        json!({"target_id":target_id,"snapshot_hash":&snapshot_hash}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok((
        StatusCode::CREATED,
        find(state.pool(), &deployment_id, request_id).await?,
    ))
}

#[utoipa::path(operation_id = "deployments_list", get, path = "/api/v1/deployments", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = DeploymentListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<DeploymentListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<DeploymentListResponse>> {
    let limit = query.limit.unwrap_or(50);
    if !(1..=200).contains(&limit) {
        return Err(ApiError::validation(
            "limit 必须介于 1 和 200",
            request_id.as_str(),
        ));
    }
    let cursor = query
        .after
        .as_deref()
        .map(decode_list_cursor)
        .transpose()
        .map_err(|_| ApiError::validation("列表游标格式不正确", request_id.as_str()))?;
    let fetch_limit = i64::from(limit) + 1;
    let mut rows = match (actor.identity.as_str(), cursor.as_ref()) {
        ("administrator", None) => {
            sqlx::query_as::<_, DeploymentRow>(DEPLOYMENT_SELECT_ALL)
                .bind(fetch_limit)
                .fetch_all(state.pool())
                .await
        }
        ("administrator", Some((created_at, id))) => {
            sqlx::query_as::<_, DeploymentRow>(DEPLOYMENT_SELECT_ALL_AFTER)
                .bind(created_at)
                .bind(created_at)
                .bind(id)
                .bind(fetch_limit)
                .fetch_all(state.pool())
                .await
        }
        (_, None) => {
            sqlx::query_as::<_, DeploymentRow>(DEPLOYMENT_SELECT_GRANTED)
                .bind(&actor.id)
                .bind(fetch_limit)
                .fetch_all(state.pool())
                .await
        }
        (_, Some((created_at, id))) => {
            sqlx::query_as::<_, DeploymentRow>(DEPLOYMENT_SELECT_GRANTED_AFTER)
                .bind(&actor.id)
                .bind(created_at)
                .bind(created_at)
                .bind(id)
                .bind(fetch_limit)
                .fetch_all(state.pool())
                .await
        }
    }
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next_cursor = has_more
        .then(|| {
            rows.last()
                .map(|row| encode_list_cursor(&row.created_at, &row.id))
        })
        .flatten();
    let mut stage_tasks = load_stage_tasks(state.pool(), &rows, request_id.as_str()).await?;
    let mut target_runs = load_target_runs(state.pool(), &rows, request_id.as_str()).await?;
    let items = rows
        .into_iter()
        .map(|row| {
            let row_id = row.id.clone();
            row.into_response(
                stage_tasks.remove(&row_id).unwrap_or_default(),
                target_runs.remove(&row_id).unwrap_or_default(),
            )
        })
        .collect();
    Ok(Json(DeploymentListResponse { items, next_cursor }))
}

#[utoipa::path(operation_id = "deployments_show", get, path = "/api/v1/deployments/{id}", params(("id" = String, Path)), responses((status = 200, body = DeploymentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<DeploymentResponse>> {
    let application_id: Option<String> = sqlx::query_scalar("SELECT COALESCE(d.application_id,t.application_id) FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE d.id=?")
        .bind(&id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    let application_id = application_id.ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "deployments_events", get, path = "/api/v1/deployments/{id}/events", params(("id" = String, Path), ("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = DeploymentEventListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeploymentEventQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<DeploymentEventListResponse>> {
    require_access(&state, &actor, &id, request_id.as_str()).await?;
    let limit = query.limit.unwrap_or(100);
    if !(1..=200).contains(&limit) {
        return Err(ApiError::validation(
            "limit 必须介于 1 和 200",
            request_id.as_str(),
        ));
    }
    let cursor = query
        .after
        .as_deref()
        .map(decode_list_cursor)
        .transpose()
        .map_err(|_| ApiError::validation("事件游标格式不正确", request_id.as_str()))?;
    let fetch_limit = i64::from(limit) + 1;
    let mut rows = match cursor.as_ref() {
        None => sqlx::query_as::<_, DeploymentEventRow>("SELECT id,event_name,status,payload_json,created_at FROM deployment_events WHERE deployment_id=? ORDER BY created_at,id LIMIT ?")
            .bind(&id)
            .bind(fetch_limit)
            .fetch_all(state.pool())
            .await,
        Some((created_at, event_id)) => sqlx::query_as::<_, DeploymentEventRow>("SELECT id,event_name,status,payload_json,created_at FROM deployment_events WHERE deployment_id=? AND (created_at>? OR (created_at=? AND id>?)) ORDER BY created_at,id LIMIT ?")
            .bind(&id)
            .bind(created_at)
            .bind(created_at)
            .bind(event_id)
            .bind(fetch_limit)
            .fetch_all(state.pool())
            .await,
    }
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next_cursor = has_more
        .then(|| {
            rows.last()
                .map(|row| encode_list_cursor(&row.created_at, &row.id))
        })
        .flatten();
    let items = rows
        .into_iter()
        .map(DeploymentEventRow::into_response)
        .collect();
    Ok(Json(DeploymentEventListResponse { items, next_cursor }))
}

#[utoipa::path(operation_id = "deployments_logs", get, path = "/api/v1/deployments/{id}/logs", params(("id" = String, Path), ("after" = Option<i64>, Query)), responses((status = 200, content_type = "text/event-stream"), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>>> {
    require_access(&state, &actor, &id, request_id.as_str()).await?;
    let header_after = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| ApiError::validation("Last-Event-ID 格式不正确", request_id.as_str()))?;
    if header_after.is_some() && query.after.is_some() && header_after != query.after {
        return Err(ApiError::validation("日志游标不一致", request_id.as_str()));
    }
    let mut after = header_after.or(query.after).unwrap_or(0);
    if after < 0 {
        return Err(ApiError::validation(
            "日志游标不能为负数",
            request_id.as_str(),
        ));
    }
    let pool = state.pool().clone();
    let actor_id = actor.id.clone();
    let session_id = actor.session_id.clone();
    let administrator = actor.identity == "administrator";
    let application_id: String = sqlx::query_scalar("SELECT COALESCE(d.application_id,t.application_id) FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE d.id=?")
        .bind(&id).fetch_one(&pool).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    let bounds: (Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT MIN(sequence),MAX(sequence) FROM deployment_logs WHERE deployment_id=?",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if bounds.1.is_some_and(|maximum| after > maximum)
        || bounds
            .0
            .is_some_and(|minimum| after > 0 && after < minimum - 1)
    {
        return Err(ApiError::validation(
            "日志游标无效或已经过期",
            request_id.as_str(),
        ));
    }
    let output = stream! {
        loop {
            let session_active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users u JOIN sessions s ON s.user_id=u.id WHERE u.id=? AND u.status='active' AND u.system_account=0 AND s.id=? AND s.revoked_at IS NULL AND s.expires_at>strftime('%Y-%m-%dT%H:%M:%fZ','now'))")
                .bind(&actor_id).bind(&session_id).fetch_one(&pool).await.unwrap_or(false);
            let granted = if administrator { true } else {
                sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM user_application_grants WHERE user_id=? AND application_id=?)")
                    .bind(&actor_id).bind(&application_id).fetch_one(&pool).await.unwrap_or(false)
            };
            if !session_active || !granted {
                yield Ok(Event::default().event("authorization-revoked").data("日志访问授权已经失效"));
                break;
            }
            let rows = match sqlx::query_as::<_, DeploymentLogResponse>("SELECT log.sequence,log.task_id,task.stage,log.task_sequence,log.stream,log.content,log.truncated,log.created_at FROM deployment_logs log LEFT JOIN agent_tasks task ON task.id=log.task_id WHERE log.deployment_id=? AND log.sequence>? ORDER BY log.sequence LIMIT 200")
                .bind(&id).bind(after).fetch_all(&pool).await {
                    Ok(rows) => rows,
                    Err(_) => {
                        yield Ok(Event::default().event("stream-error").data("日志读取暂时失败，请按游标重连"));
                        break;
                    }
                };
            for row in rows {
                after = row.sequence;
                let data = serde_json::to_string(&row).unwrap_or_else(|_| "{}".to_owned());
                yield Ok(Event::default().id(row.sequence.to_string()).event("log").data(data));
            }
            let Ok(status) = sqlx::query_scalar::<_, String>("SELECT status FROM deployments WHERE id=?")
                .bind(&id).fetch_one(&pool).await else { continue; };
            let terminal = matches!(status.as_str(), "succeeded" | "failed" | "canceled" | "interrupted");
            let Ok(pending) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployment_logs WHERE deployment_id=? AND sequence>?")
                .bind(&id).bind(after).fetch_one(&pool).await else { continue; };
            if terminal && pending == 0 {
                yield Ok(Event::default().event("terminal").data(json!({"status":status,"last_event_id":after}).to_string()));
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    };
    Ok(Sse::new(output).keep_alive(KeepAlive::default()))
}

#[utoipa::path(operation_id = "deployments_cancel", post, path = "/api/v1/deployments/{id}/cancel", params(("id" = String, Path)), responses((status = 200, body = DeploymentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Json<DeploymentResponse>> {
    actor.verify_csrf(&headers, request_id.as_str())?;
    require_access(&state, &actor, &id, request_id.as_str()).await?;
    Ok(Json(
        cancel_deployment(&state, &actor, &id, request_id.as_str()).await?,
    ))
}

pub(crate) async fn cancel_deployment(
    state: &AppState,
    actor: &AuthUser,
    id: &str,
    request_id: &str,
) -> ApiResult<DeploymentResponse> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let status: String = sqlx::query_scalar("SELECT status FROM deployments WHERE id=?")
        .bind(id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let active_agent_task: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_tasks WHERE deployment_id=? AND status IN ('delivered','accepted','running','canceling'))")
        .bind(id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    match status.as_str() {
        "queued" if !active_agent_task => {
            sqlx::query("UPDATE deployments SET status='canceled',phase='canceled',cancel_requested_at=?,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status='queued'")
                .bind(&now).bind(&now).bind(&now).bind(id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
            sqlx::query("UPDATE agent_tasks SET status='canceled',finished_at=?,result_json=?,updated_at=? WHERE deployment_id=? AND status='queued'")
                .bind(&now).bind(json!({"error_code":"canceled_before_delivery"}).to_string()).bind(&now).bind(id)
                .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
            sqlx::query("UPDATE deployment_target_runs SET status='canceled',phase='canceled',result_summary='部署在下发前取消',finished_at=?,updated_at=?,version=version+1 WHERE deployment_id=? AND status='pending'")
                .bind(&now).bind(&now).bind(id).execute(&mut *transaction).await
                .map_err(|_| ApiError::internal(request_id))?;
        }
        "queued" | "running" => {
            sqlx::query("UPDATE deployments SET status='canceling',phase='canceling',cancel_requested_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running')")
                .bind(&now).bind(&now).bind(id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
        }
        "canceling" => {}
        _ => {
            return Err(ApiError::conflict(
                "deployment_not_cancelable",
                "部署当前不可取消",
                request_id,
            ));
        }
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment.cancel",
        "deployment",
        id,
        request_id,
        json!({}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    if status == "running" || active_agent_task {
        runtime::cancel_remote(state, id).await?;
    }
    find(state.pool(), id, request_id).await
}

#[utoipa::path(operation_id = "deployments_retry", post, path = "/api/v1/deployments/{id}/retry", params(("id" = String, Path)), responses((status = 201, body = DeploymentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn retry(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<(StatusCode, Json<DeploymentResponse>)> {
    actor.verify_csrf(&headers, request_id.as_str())?;
    require_access(&state, &actor, &id, request_id.as_str()).await?;
    let key = validate_idempotency_key(&headers, request_id.as_str())?;
    let stored_key = format!("retry:{key}");
    let original: (String, String, String, String, String) = sqlx::query_as("SELECT target_id,COALESCE(application_id,(SELECT application_id FROM deployment_targets WHERE id=deployments.target_id)),snapshot_json,snapshot_hash,status FROM deployments WHERE id=?")
        .bind(&id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let original_snapshot: Value =
        serde_json::from_str(&original.2).map_err(|_| ApiError::internal(request_id.as_str()))?;
    if original_snapshot
        .get("targets")
        .and_then(Value::as_array)
        .is_some()
    {
        return retry_application_deployment(
            &state,
            &actor,
            &id,
            &original,
            &stored_key,
            request_id.as_str(),
        )
        .await;
    }
    if !matches!(original.4.as_str(), "failed" | "canceled" | "interrupted") {
        return Err(ApiError::conflict(
            "deployment_not_retryable",
            "部署当前不可重试",
            request_id.as_str(),
        ));
    }
    let preview = if original_snapshot
        .get("execution_mode")
        .and_then(Value::as_str)
        == Some("two_stage")
    {
        preview_from_snapshot(&original_snapshot, &original.3)?
    } else {
        let parameters = original_snapshot
            .get("parameters")
            .ok_or_else(|| ApiError::internal(request_id.as_str()))?;
        build_preview(
            &state,
            &actor,
            &original.0,
            parameters,
            original_snapshot
                .get("release_strategy")
                .and_then(Value::as_str)
                .unwrap_or("automatic"),
            None,
            request_id.as_str(),
        )
        .await?
    };
    let new_id = format!("deployment_{}", Ulid::new());
    let request_hash =
        digest_json(&json!({"retry_of_id":id,"snapshot_hash":preview.response.snapshot_hash}));
    if let Some((existing_id, existing_hash)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,request_hash FROM deployments WHERE requested_by=? AND idempotency_key=?",
    )
    .bind(&actor.id)
    .bind(&stored_key)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    {
        if existing_hash == request_hash {
            return Ok((
                StatusCode::OK,
                Json(find(state.pool(), &existing_id, request_id.as_str()).await?),
            ));
        }
        return Err(ApiError::conflict(
            "idempotency_conflict",
            "幂等键已经被使用",
            request_id.as_str(),
        ));
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,retry_of_id,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES(?,?,?,?,?,'queued','queued',?,?,?,?)")
        .bind(&new_id).bind(&original.1).bind(&original.0).bind(&actor.id).bind(&id).bind(&stored_key).bind(&request_hash).bind(&preview.response.snapshot_hash).bind(preview.snapshot.to_string())
        .execute(&mut *transaction).await.map_err(|error| if error.to_string().contains("UNIQUE constraint failed") { ApiError::conflict("idempotency_conflict", "幂等键已经被使用", request_id.as_str()) } else { ApiError::internal(request_id.as_str()) })?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment.retry",
        "deployment",
        &new_id,
        request_id.as_str(),
        json!({"retry_of_id":id}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(find(state.pool(), &new_id, request_id.as_str()).await?),
    ))
}

async fn retry_application_deployment(
    state: &AppState,
    actor: &AuthUser,
    original_id: &str,
    original: &(String, String, String, String, String),
    stored_key: &str,
    request_id: &str,
) -> ApiResult<(StatusCode, Json<DeploymentResponse>)> {
    if let Some(response) =
        find_retry_idempotent(state.pool(), &actor.id, stored_key, original_id, request_id).await?
    {
        return Ok((StatusCode::OK, Json(response)));
    }
    let runs: Vec<(String, String, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT id,target_id,node_id,agent_id,status,target_snapshot_json FROM deployment_target_runs WHERE deployment_id=? ORDER BY target_id",
    )
    .bind(original_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if runs.is_empty()
        || runs
            .iter()
            .any(|run| matches!(run.4.as_str(), "downloading" | "running"))
        || !runs
            .iter()
            .any(|run| matches!(run.4.as_str(), "failed" | "canceled" | "expired"))
    {
        return Err(ApiError::conflict(
            "deployment_not_retryable",
            "部署当前没有可重试的失败目标",
            request_id,
        ));
    }
    let request_hash = digest_json(&json!({
        "retry_of_id": original_id,
        "snapshot_hash": original.3,
        "target_runs": runs.iter().map(|run| (&run.1, &run.4)).collect::<Vec<_>>(),
    }));
    if let Some(response) = find_idempotent(
        state.pool(),
        &actor.id,
        stored_key,
        &request_hash,
        request_id,
    )
    .await?
    {
        return Ok((StatusCode::OK, Json(response)));
    }
    let new_id = format!("deployment_{}", Ulid::new());
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let retry_artifact: Option<(String, String, String)> = sqlx::query_as(
        "SELECT id,status,expires_at FROM deployment_artifacts WHERE deployment_id=?",
    )
    .bind(original_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let retry_artifact_id = match retry_artifact {
        Some((artifact_id, status, expires_at))
            if status == "verified" && expires_at > Utc::now().to_rfc3339() =>
        {
            artifact_id
        }
        Some(_) => {
            return Err(ApiError::conflict(
                "deployment_artifact_not_reusable",
                "原部署制品已失效，需要重新构建并执行全量部署",
                request_id,
            ));
        }
        None => {
            return Err(ApiError::conflict(
                "deployment_artifact_not_reusable",
                "原部署没有可复用制品，需要重新构建并执行全量部署",
                request_id,
            ));
        }
    };
    let insert = sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,retry_of_id,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES(?,?,?,?,?,'queued','targets_pending',?,?,?,?)")
        .bind(&new_id).bind(&original.1).bind(&original.0).bind(&actor.id).bind(original_id)
        .bind(stored_key).bind(&request_hash).bind(&original.3).bind(&original.2)
        .execute(&mut *transaction).await;
    if let Err(error) = insert {
        if error.to_string().contains("UNIQUE constraint failed") {
            drop(transaction);
            if let Some(response) =
                find_retry_idempotent(state.pool(), &actor.id, stored_key, original_id, request_id)
                    .await?
            {
                return Ok((StatusCode::OK, Json(response)));
            }
            return Err(ApiError::conflict(
                "idempotency_conflict",
                "幂等键已经被并发请求使用",
                request_id,
            ));
        }
        return Err(ApiError::internal(request_id));
    }
    for (source_run_id, target_id, node_id, agent_id, status, target_snapshot) in runs {
        let reused = matches!(status.as_str(), "succeeded" | "reused");
        sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,source_run_id,artifact_id,target_snapshot_json,status,phase,env_gate_status,finished_at) VALUES(?,?,?,?,?,?,?,?,?, ?, 'not_required', CASE WHEN ? THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') ELSE NULL END)")
            .bind(format!("run_{}", Ulid::new())).bind(&new_id).bind(target_id).bind(node_id)
            .bind(agent_id).bind(&source_run_id).bind(if reused { None } else { Some(retry_artifact_id.as_str()) }).bind(target_snapshot)
            .bind(if reused { "reused" } else { "pending" })
            .bind(if reused { "reused" } else { "pending" }).bind(reused)
            .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment.retry",
        "deployment",
        &new_id,
        request_id,
        json!({"retry_of_id":original_id,"application_id":original.1}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok((
        StatusCode::CREATED,
        Json(find(state.pool(), &new_id, request_id).await?),
    ))
}

async fn require_access(
    state: &AppState,
    actor: &AuthUser,
    id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let application_id: Option<String> = sqlx::query_scalar("SELECT COALESCE(d.application_id,t.application_id) FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE d.id=?")
        .bind(id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id))?;
    let application_id = application_id.ok_or_else(|| ApiError::not_found(request_id))?;
    grants::require_application_access(state.pool(), actor, &application_id, request_id).await
}

async fn build_preview(
    state: &AppState,
    actor: &AuthUser,
    target_id: &str,
    parameters: &Value,
    release_strategy: &str,
    release_version: Option<&str>,
    request_id: &str,
) -> ApiResult<PreviewData> {
    build_preview_with_availability(
        state,
        actor,
        target_id,
        parameters,
        release_strategy,
        release_version,
        request_id,
        true,
    )
    .await
}

#[allow(clippy::too_many_arguments)] // preview 聚合字段较多，拆分会增加临时结构体
async fn build_preview_with_availability(
    state: &AppState,
    actor: &AuthUser,
    target_id: &str,
    parameters: &Value,
    release_strategy: &str,
    release_version: Option<&str>,
    request_id: &str,
    require_online: bool,
) -> ApiResult<PreviewData> {
    let row: TargetExecutionRow = sqlx::query_as("SELECT t.id AS target_id,t.application_id,a.name AS application_name,a.status AS application_status,t.node_id,n.name AS node_name,n.status AS node_status,agent.id AS agent_id,n.work_root,n.secrets_root,t.environment,t.execution_mode,t.script_path,t.parameter_schema,t.timeout_seconds,t.verification_config,t.privileged_release,t.status AS target_status,t.version AS target_version FROM deployment_targets t JOIN applications a ON a.id=t.application_id JOIN nodes n ON n.id=t.node_id LEFT JOIN agents agent ON agent.node_id=n.id AND agent.revoked_at IS NULL AND agent.archived_at IS NULL WHERE t.id=?")
        .bind(target_id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))?;
    grants::require_application_access(state.pool(), actor, &row.application_id, request_id)
        .await?;
    if row.application_status != "active" {
        return Err(ApiError::conflict(
            "application_archived",
            "应用已归档",
            request_id,
        ));
    }
    if row.target_status != "active" {
        return Err(ApiError::conflict(
            "target_disabled",
            "部署目标已停用",
            request_id,
        ));
    }
    if (require_online && row.node_status != "online")
        || (!require_online && !matches!(row.node_status.as_str(), "online" | "offline"))
        || row.agent_id.is_none()
        || row.work_root.as_deref().is_none_or(str::is_empty)
        || row.secrets_root.as_deref().is_none_or(str::is_empty)
    {
        return Err(ApiError::conflict(
            "node_not_deployable",
            "目标节点 Agent 当前不可部署",
            request_id,
        ));
    }
    let schema: Value =
        serde_json::from_str(&row.parameter_schema).map_err(|_| ApiError::internal(request_id))?;
    let managed_parameters = if row.execution_mode == "two_stage" {
        with_managed_release_version(parameters, release_version, request_id)?
    } else {
        parameters.clone()
    };
    execution_spec::validate_parameter_values(&schema, &managed_parameters, request_id)?;
    let verification: Value = serde_json::from_str(&row.verification_config)
        .map_err(|_| ApiError::internal(request_id))?;
    let refs: Vec<(String,String)> = sqlx::query_as("SELECT environment_key,file_path FROM secret_file_references WHERE deployment_target_id=? ORDER BY environment_key").bind(target_id).fetch_all(state.pool()).await.map_err(|_| ApiError::internal(request_id))?;
    let target_snapshot = execution_spec::target_snapshot(TargetSnapshotInput {
        application_id: &row.application_id,
        node_id: &row.node_id,
        environment: &row.environment,
        script_path: &row.script_path,
        parameter_schema: &schema,
        timeout_seconds: row.timeout_seconds,
        verification_config: &verification,
        secret_refs: &refs,
        privileged_release: row.privileged_release,
        version: row.target_version,
    });
    if row.execution_mode == "two_stage" {
        validate_release_strategy(release_strategy, &row.execution_mode, request_id)?;
        return build_two_stage_preview(
            state.pool(),
            row,
            target_snapshot,
            &managed_parameters,
            release_strategy,
            request_id,
        )
        .await;
    }
    validate_release_strategy(release_strategy, &row.execution_mode, request_id)?;
    let snapshot_hash = execution_spec::snapshot_hash(&target_snapshot);
    let response = DeploymentPreviewResponse {
        target_id: row.target_id,
        application_id: row.application_id,
        application_name: row.application_name,
        node_id: row.node_id,
        node_name: row.node_name,
        environment: row.environment,
        execution_mode: "script".to_owned(),
        release_strategy: "automatic".to_owned(),
        script_path: row.script_path,
        parameters: parameters.clone(),
        snapshot_hash,
        source_policy: None,
        deployment_branch: None,
        resolved_commit_sha: None,
        release_version: None,
        modules: None,
    };
    Ok(PreviewData {
        snapshot: json!({"target":target_snapshot,"parameters":parameters}),
        response,
    })
}

async fn build_application_preview(
    state: &AppState,
    actor: &AuthUser,
    application_id: &str,
    parameters: &Value,
    release_strategy: &str,
    release_version: Option<&str>,
    request_id: &str,
) -> ApiResult<ApplicationPreviewData> {
    grants::require_application_access(state.pool(), actor, application_id, request_id).await?;
    let application: Option<(String, String)> =
        sqlx::query_as("SELECT name,status FROM applications WHERE id=?")
            .bind(application_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    let (application_name, status) = application.ok_or_else(|| ApiError::not_found(request_id))?;
    if status != "active" {
        return Err(ApiError::conflict(
            "application_archived",
            "应用已归档",
            request_id,
        ));
    }
    let targets: Vec<(String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT target.id,target.node_id,node.name,agent.id,node.status FROM deployment_targets target JOIN nodes node ON node.id=target.node_id LEFT JOIN agents agent ON agent.node_id=node.id AND agent.revoked_at IS NULL AND agent.archived_at IS NULL WHERE target.application_id=? AND target.status='active' ORDER BY target.id",
    )
    .bind(application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if targets.is_empty() {
        return Err(ApiError::conflict(
            "application_has_no_active_targets",
            "应用没有可用的部署目标",
            request_id,
        ));
    }
    let mut previews = Vec::with_capacity(targets.len());
    let mut target_runs = Vec::with_capacity(targets.len());
    let mut target_snapshots = Vec::with_capacity(targets.len());
    let managed_release_version = release_version
        .or_else(|| parameters.get("release-version").and_then(Value::as_str))
        .map(str::to_owned)
        .unwrap_or_else(generate_release_version);
    let mut first: Option<PreviewData> = None;
    for (target_id, node_id, node_name, agent_id, node_status) in targets {
        let agent_id = agent_id.ok_or_else(|| {
            ApiError::conflict(
                "target_agent_not_available",
                "启用的部署目标缺少可用 Agent",
                request_id,
            )
        })?;
        let preview = build_preview_with_availability(
            state,
            actor,
            &target_id,
            parameters,
            release_strategy,
            Some(&managed_release_version),
            request_id,
            false,
        )
        .await?;
        if first
            .as_ref()
            .is_some_and(|item| item.response.execution_mode != preview.response.execution_mode)
        {
            return Err(ApiError::conflict(
                "mixed_target_execution_modes",
                "同一应用的启用目标必须使用相同执行模式",
                request_id,
            ));
        }
        let target_snapshot = preview
            .snapshot
            .get("target")
            .cloned()
            .ok_or_else(|| ApiError::internal(request_id))?;
        previews.push(DeploymentTargetPreviewResponse {
            target_id: target_id.clone(),
            node_id: node_id.clone(),
            node_name,
            agent_id: agent_id.clone(),
            agent_online: node_status == "online",
            env_gate_status: preview_env_gate_status(state.pool(), &target_id, request_id).await?,
            script_path: preview.response.script_path.clone(),
        });
        target_runs.push(TargetRunSnapshot {
            target_id: target_id.clone(),
            node_id: node_id.clone(),
            agent_id: agent_id.clone(),
            snapshot: target_snapshot.clone(),
        });
        target_snapshots.push(json!({
            "target_id": target_id,
            "node_id": node_id,
            "agent_id": agent_id,
            "target": target_snapshot,
        }));
        if first.is_none() {
            first = Some(preview);
        }
    }
    let first = first.ok_or_else(|| ApiError::internal(request_id))?;
    let mut snapshot = json!({
        "application_id": application_id,
        "application_name": application_name,
        "execution_mode": first.response.execution_mode,
        "release_strategy": release_strategy,
        "parameters": parameters,
        "targets": target_snapshots,
        "multi_target_dispatch_version": 3,
    });
    for key in ["source", "two_stage"] {
        if let Some(value) = first.snapshot.get(key) {
            snapshot[key] = value.clone();
        }
    }
    let snapshot_hash = digest_json(&snapshot);
    Ok(ApplicationPreviewData {
        response: ApplicationDeploymentPreviewResponse {
            application_id: application_id.to_owned(),
            application_name,
            execution_mode: first.response.execution_mode,
            release_strategy: release_strategy.to_owned(),
            parameters: parameters.clone(),
            snapshot_hash,
            targets: previews,
            deployment_branch: first.response.deployment_branch,
            resolved_commit_sha: first.response.resolved_commit_sha,
            release_version: first.response.release_version,
            modules: first.response.modules,
        },
        snapshot,
        target_runs,
    })
}

async fn preview_env_gate_status(
    pool: &sqlx::SqlitePool,
    target_id: &str,
    request_id: &str,
) -> ApiResult<String> {
    let rows: Vec<(i64, String, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT file.current_version,COALESCE(sync.status,'pending'),sync.actual_version,file.deleted_at FROM application_env_files file JOIN deployment_targets target ON target.application_id=file.application_id LEFT JOIN application_env_versions version ON version.env_file_id=file.id AND version.env_version=file.current_version LEFT JOIN application_env_syncs sync ON sync.env_version_id=version.id AND sync.target_id=target.id WHERE target.id=? ORDER BY file.file_name COLLATE NOCASE,file.id",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if rows.is_empty() {
        return Ok("not_required".to_owned());
    }
    if rows.iter().any(|(_, status, _, _)| status == "failed") {
        return Ok("failed".to_owned());
    }
    if rows
        .iter()
        .all(|(version, status, actual, _)| status == "succeeded" && actual == &Some(*version))
    {
        return Ok("ready".to_owned());
    }
    Ok("pending".to_owned())
}

async fn build_two_stage_preview(
    pool: &sqlx::SqlitePool,
    row: TargetExecutionRow,
    target_snapshot: Value,
    parameters: &Value,
    release_strategy: &str,
    request_id: &str,
) -> ApiResult<PreviewData> {
    let source = resolve_two_stage_source(pool, &row.application_id, request_id).await?;
    let two_stage = extract_two_stage_parameters(parameters, request_id)?;
    let source_snapshot = json!({
        "source_id": source.source_id,
        "source_version": source.source_version,
        "source_policy": "branch",
        "repository_url": source.repository_url,
        "git_credential_id": source.git_credential_id,
        "build_agent_id": source.build_agent_id,
        "deployment_branch": source.deployment_branch,
        "requested_ref": format!("refs/heads/{}", source.deployment_branch),
        "resolved_commit_sha": source.resolved_commit_sha,
        "refs_discovery_id": source.refs_discovery_id,
    });
    let snapshot = json!({
        "target": target_snapshot,
        "target_id": row.target_id,
        "application_name": row.application_name,
        "node_name": row.node_name,
        "execution_mode": "two_stage",
        "release_strategy": release_strategy,
        "source": source_snapshot,
        "two_stage": {
            "release_version": two_stage.release_version,
            "modules": two_stage.modules,
        },
        "parameters": parameters,
    });
    let snapshot_hash = digest_json(&snapshot);
    Ok(PreviewData {
        response: DeploymentPreviewResponse {
            target_id: row.target_id,
            application_id: row.application_id,
            application_name: row.application_name,
            node_id: row.node_id,
            node_name: row.node_name,
            environment: row.environment,
            execution_mode: "two_stage".to_owned(),
            release_strategy: release_strategy.to_owned(),
            script_path: row.script_path,
            parameters: parameters.clone(),
            snapshot_hash,
            source_policy: Some("branch".to_owned()),
            deployment_branch: Some(source.deployment_branch),
            resolved_commit_sha: Some(source.resolved_commit_sha),
            release_version: Some(two_stage.release_version),
            modules: Some(two_stage.modules),
        },
        snapshot,
    })
}

async fn resolve_two_stage_source(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<TwoStageSourceInfo> {
    let source: Option<VerifiedSourceRow> = sqlx::query_as(
        "SELECT id,repository_url,git_credential_id,build_agent_id,source_version,deployment_branch FROM application_sources WHERE application_id=? AND status='verified'",
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some(source) = source else {
        return Err(ApiError::conflict(
            "git_source_not_verified",
            "应用尚未配置并固定 Git 分支来源",
            request_id,
        ));
    };
    let source_id = source.id;
    let repository_url = source.repository_url;
    let git_credential_id = source.git_credential_id;
    let build_agent_id = source.build_agent_id;
    let source_version = source.source_version;
    let deployment_branch = source.deployment_branch;
    if deployment_branch.is_empty() {
        return Err(ApiError::conflict(
            "git_branch_not_verified",
            "应用 Git 来源尚未固定部署分支",
            request_id,
        ));
    }
    let discovery: Option<RefDiscoveryRow> = sqlx::query_as(
        "SELECT id,refs_json,expires_at FROM git_ref_discoveries WHERE application_source_id=? AND source_version=? AND status='succeeded' ORDER BY created_at DESC,id DESC LIMIT 1",
    )
    .bind(&source_id)
    .bind(source_version)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some(discovery) = discovery else {
        return Err(ApiError::conflict(
            "git_ref_discovery_missing",
            "当前来源版本没有可用的分支发现结果",
            request_id,
        ));
    };
    let discovery_id = discovery.id;
    let refs_json = discovery.refs_json;
    let expires_at = discovery.expires_at;
    if expires_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|expires_at| expires_at <= chrono::Utc::now())
    {
        return Err(ApiError::conflict(
            "git_ref_discovery_expired",
            "分支发现结果已过期，请先刷新应用来源",
            request_id,
        ));
    }
    let refs: Vec<crate::application_sources::GitRefResponse> =
        serde_json::from_str(&refs_json).map_err(|_| ApiError::internal(request_id))?;
    let resolved = refs
        .iter()
        .find(|reference| reference.name == deployment_branch)
        .ok_or_else(|| {
            ApiError::conflict(
                "git_branch_not_found",
                "固定分支不在最近的分支发现结果中",
                request_id,
            )
        })?;
    Ok(TwoStageSourceInfo {
        source_id,
        repository_url,
        git_credential_id,
        build_agent_id,
        source_version,
        deployment_branch,
        resolved_commit_sha: resolved.sha.clone(),
        refs_discovery_id: discovery_id,
    })
}

fn extract_two_stage_parameters(
    parameters: &Value,
    request_id: &str,
) -> ApiResult<TwoStageParameters> {
    let release_version = parameters
        .get("release-version")
        .and_then(Value::as_str)
        .filter(|value| {
            !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
        })
        .ok_or_else(|| ApiError::validation("两阶段部署缺少 release-version 参数", request_id))?
        .to_owned();
    let modules = match parameters.get("modules") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .filter(|value| {
                        !value.is_empty()
                            && value.len() <= 64
                            && !value.chars().any(char::is_control)
                    })
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        ApiError::validation("两阶段部署 modules 参数格式不正确", request_id)
                    })
            })
            .collect::<ApiResult<Vec<String>>>()?,
        Some(Value::String(value)) => value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                if value.len() <= 64 && !value.chars().any(char::is_control) {
                    Ok(value.to_owned())
                } else {
                    Err(ApiError::validation(
                        "两阶段部署 modules 参数格式不正确",
                        request_id,
                    ))
                }
            })
            .collect::<ApiResult<Vec<String>>>()?,
        _ => {
            return Err(ApiError::validation(
                "两阶段部署缺少 modules 参数",
                request_id,
            ));
        }
    };
    if modules.is_empty() || modules.len() > 32 {
        return Err(ApiError::validation(
            "两阶段部署 modules 必须包含 1 到 32 个模块",
            request_id,
        ));
    }
    Ok(TwoStageParameters {
        release_version,
        modules,
    })
}

fn with_managed_release_version(
    parameters: &Value,
    release_version: Option<&str>,
    request_id: &str,
) -> ApiResult<Value> {
    let mut managed = parameters
        .as_object()
        .cloned()
        .ok_or_else(|| ApiError::validation("部署参数必须是对象", request_id))?;
    let release_version = release_version
        .or_else(|| parameters.get("release-version").and_then(Value::as_str))
        .map(str::to_owned)
        .unwrap_or_else(generate_release_version);
    if release_version.is_empty()
        || release_version.len() > 128
        || release_version.chars().any(char::is_control)
    {
        return Err(ApiError::validation("发布版本格式不正确", request_id));
    }
    managed.insert("release-version".to_owned(), Value::String(release_version));
    Ok(Value::Object(managed))
}

fn generate_release_version() -> String {
    Utc::now().format("%Y%m%d%H%M%S%3f").to_string()
}

fn preview_from_snapshot(snapshot: &Value, snapshot_hash: &str) -> ApiResult<PreviewData> {
    let release_strategy = snapshot
        .get("release_strategy")
        .and_then(Value::as_str)
        .unwrap_or("automatic");
    let target = snapshot
        .get("target")
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let source = snapshot
        .get("source")
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let two_stage = snapshot
        .get("two_stage")
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let parameters = snapshot
        .get("parameters")
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let target_id = snapshot
        .get("target_id")
        .or_else(|| target.get("target_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let application_name = snapshot
        .get("application_name")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let node_name = snapshot
        .get("node_name")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let application_id = target
        .get("application_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let node_id = target
        .get("node_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let environment = target
        .get("environment")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let script_path = target
        .get("script_path")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let deployment_branch = source
        .get("deployment_branch")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let resolved_commit_sha = source
        .get("resolved_commit_sha")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    let release_version = two_stage
        .get("release_version")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
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
        .ok_or_else(|| ApiError::internal("deployments_retry"))?;
    Ok(PreviewData {
        response: DeploymentPreviewResponse {
            target_id: target_id.to_owned(),
            application_id: application_id.to_owned(),
            application_name: application_name.to_owned(),
            node_id: node_id.to_owned(),
            node_name: node_name.to_owned(),
            environment: environment.to_owned(),
            execution_mode: "two_stage".to_owned(),
            release_strategy: release_strategy.to_owned(),
            script_path: script_path.to_owned(),
            parameters: parameters.clone(),
            snapshot_hash: snapshot_hash.to_owned(),
            source_policy: Some("branch".to_owned()),
            deployment_branch: Some(deployment_branch.to_owned()),
            resolved_commit_sha: Some(resolved_commit_sha.to_owned()),
            release_version: Some(release_version.to_owned()),
            modules: Some(modules),
        },
        snapshot: snapshot.clone(),
    })
}

pub(crate) fn validate_idempotency_key(headers: &HeaderMap, request_id: &str) -> ApiResult<String> {
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if (16..=128).contains(&key.len()) && key.bytes().all(|byte| byte.is_ascii_graphic()) {
        Ok(key.to_owned())
    } else {
        Err(ApiError::validation(
            "Idempotency-Key 格式不正确",
            request_id,
        ))
    }
}
fn digest_json(value: &Value) -> String {
    format!("{:x}", Sha256::digest(value.to_string().as_bytes()))
}
fn encode_list_cursor(created_at: &str, id: &str) -> String {
    URL_SAFE_NO_PAD.encode(format!("{created_at}\0{id}"))
}
fn decode_list_cursor(cursor: &str) -> Result<(String, String), ()> {
    let decoded = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| ())?;
    let decoded = String::from_utf8(decoded).map_err(|_| ())?;
    let (created_at, id) = decoded.split_once('\0').ok_or(())?;
    if created_at.is_empty() || id.is_empty() || id.contains('\0') {
        return Err(());
    }
    Ok((created_at.to_owned(), id.to_owned()))
}

impl DeploymentEventRow {
    fn into_response(self) -> DeploymentEventResponse {
        let payload = serde_json::from_str::<Value>(&self.payload_json).unwrap_or(Value::Null);
        let field = |name: &str| payload.get(name).and_then(Value::as_str).map(str::to_owned);
        DeploymentEventResponse {
            id: self.id,
            event_name: self.event_name,
            status: self.status,
            stage: field("stage"),
            module: field("module"),
            module_name: field("module_name"),
            step_id: field("step_id"),
            step: field("step"),
            failure_stage: field("failure_stage"),
            message: field("message"),
            created_at: self.created_at,
        }
    }
}

pub(crate) async fn find(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<DeploymentResponse> {
    let row: DeploymentRow = sqlx::query_as(DEPLOYMENT_SELECT_ONE)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))?;
    let row_id = row.id.clone();
    let mut stage_tasks = load_stage_tasks(pool, std::slice::from_ref(&row), request_id).await?;
    let mut target_runs = load_target_runs(pool, std::slice::from_ref(&row), request_id).await?;
    Ok(row.into_response(
        stage_tasks.remove(&row_id).unwrap_or_default(),
        target_runs.remove(&row_id).unwrap_or_default(),
    ))
}

const DEPLOYMENT_SELECT_ONE: &str = "SELECT d.id,COALESCE(d.application_id,target.application_id) AS application_id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version,COALESCE(target.execution_mode,'script') AS execution_mode,d.snapshot_json FROM deployments d LEFT JOIN deployment_targets target ON target.id=d.target_id WHERE d.id=?";
const DEPLOYMENT_SELECT_ALL: &str = "SELECT d.id,COALESCE(d.application_id,target.application_id) AS application_id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version,COALESCE(target.execution_mode,'script') AS execution_mode,d.snapshot_json FROM deployments d LEFT JOIN deployment_targets target ON target.id=d.target_id ORDER BY d.created_at DESC,d.id DESC LIMIT ?";
const DEPLOYMENT_SELECT_ALL_AFTER: &str = "SELECT d.id,COALESCE(d.application_id,target.application_id) AS application_id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version,COALESCE(target.execution_mode,'script') AS execution_mode,d.snapshot_json FROM deployments d LEFT JOIN deployment_targets target ON target.id=d.target_id WHERE d.created_at<? OR (d.created_at=? AND d.id<?) ORDER BY d.created_at DESC,d.id DESC LIMIT ?";
const DEPLOYMENT_SELECT_GRANTED: &str = "SELECT d.id,COALESCE(d.application_id,target.application_id) AS application_id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version,COALESCE(target.execution_mode,'script') AS execution_mode,d.snapshot_json FROM deployments d JOIN deployment_targets target ON target.id=d.target_id JOIN user_application_grants g ON g.application_id=COALESCE(d.application_id,target.application_id) WHERE g.user_id=? ORDER BY d.created_at DESC,d.id DESC LIMIT ?";
const DEPLOYMENT_SELECT_GRANTED_AFTER: &str = "SELECT d.id,COALESCE(d.application_id,target.application_id) AS application_id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version,COALESCE(target.execution_mode,'script') AS execution_mode,d.snapshot_json FROM deployments d JOIN deployment_targets target ON target.id=d.target_id JOIN user_application_grants g ON g.application_id=COALESCE(d.application_id,target.application_id) WHERE g.user_id=? AND (d.created_at<? OR (d.created_at=? AND d.id<?)) ORDER BY d.created_at DESC,d.id DESC LIMIT ?";

impl DeploymentRow {
    fn into_response(
        self,
        stage_tasks: Vec<DeploymentStageTaskSummary>,
        target_runs: Vec<DeploymentTargetRunResponse>,
    ) -> DeploymentResponse {
        let mut deployment_branch = None;
        let mut resolved_commit_sha = None;
        let mut release_version = None;
        let mut modules = None;
        let mut multi_target = false;
        let mut release_strategy = "automatic".to_owned();
        if let Some(raw) = self.snapshot_json.as_deref()
            && let Ok(snapshot) = serde_json::from_str::<Value>(raw)
        {
            multi_target = snapshot.get("targets").and_then(Value::as_array).is_some();
            release_strategy = snapshot
                .get("release_strategy")
                .and_then(Value::as_str)
                .unwrap_or("automatic")
                .to_owned();
            deployment_branch = snapshot
                .get("source")
                .and_then(|source| source.get("deployment_branch"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            resolved_commit_sha = snapshot
                .get("source")
                .and_then(|source| source.get("resolved_commit_sha"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            if let Some(two_stage) = snapshot.get("two_stage") {
                release_version = two_stage
                    .get("release_version")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                modules = two_stage
                    .get("modules")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    });
            }
        }
        let (status, phase) = if multi_target {
            aggregate_target_runs(&target_runs, &self.status, &self.phase)
        } else {
            (self.status, self.phase)
        };
        DeploymentResponse {
            id: self.id,
            application_id: self.application_id,
            target_id: self.target_id,
            requested_by: self.requested_by,
            retry_of_id: self.retry_of_id,
            status,
            phase,
            snapshot_hash: self.snapshot_hash,
            result_summary: self.result_summary,
            exit_code: self.exit_code,
            protocol_complete: self.protocol_complete,
            queued_at: self.queued_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
            cancel_requested_at: self.cancel_requested_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
            execution_mode: self.execution_mode,
            release_strategy,
            deployment_branch,
            resolved_commit_sha,
            release_version,
            modules,
            stage_tasks,
            target_runs,
        }
    }
}

async fn load_stage_tasks(
    pool: &sqlx::SqlitePool,
    rows: &[DeploymentRow],
    request_id: &str,
) -> ApiResult<std::collections::HashMap<String, Vec<DeploymentStageTaskSummary>>> {
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "SELECT deployment_id,stage,id AS task_id,status,result_json,started_at,finished_at,created_at,updated_at FROM agent_tasks WHERE deployment_id IN ({placeholders}) AND stage IS NOT NULL ORDER BY created_at,id"
    );
    let mut query = sqlx::query_as::<_, DeploymentStageTaskRow>(&sql);
    for id in ids {
        query = query.bind(id);
    }
    let rows = query
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let mut grouped: std::collections::HashMap<String, Vec<DeploymentStageTaskSummary>> =
        std::collections::HashMap::new();
    for row in rows {
        let (exit_code, error_code) = task_result_fields(row.result_json.as_deref());
        grouped
            .entry(row.deployment_id)
            .or_default()
            .push(DeploymentStageTaskSummary {
                stage: row.stage,
                task_id: row.task_id,
                status: row.status,
                exit_code,
                error_code,
                started_at: row.started_at,
                finished_at: row.finished_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
    }
    Ok(grouped)
}

async fn load_target_runs(
    pool: &sqlx::SqlitePool,
    deployments: &[DeploymentRow],
    request_id: &str,
) -> ApiResult<std::collections::HashMap<String, Vec<DeploymentTargetRunResponse>>> {
    let ids: Vec<&str> = deployments.iter().map(|row| row.id.as_str()).collect();
    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "SELECT deployment_id,id,target_id,node_id,agent_id,source_run_id,status,phase,env_gate_status,result_summary,error_code,started_at,finished_at,created_at,updated_at FROM deployment_target_runs WHERE deployment_id IN ({placeholders}) ORDER BY target_id,id"
    );
    let mut query = sqlx::query_as::<_, DeploymentTargetRunRow>(&sql);
    for id in ids {
        query = query.bind(id);
    }
    let rows = query
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let mut grouped = std::collections::HashMap::new();
    for row in rows {
        grouped
            .entry(row.deployment_id)
            .or_insert_with(Vec::new)
            .push(DeploymentTargetRunResponse {
                id: row.id,
                target_id: row.target_id,
                node_id: row.node_id,
                agent_id: row.agent_id,
                source_run_id: row.source_run_id,
                status: row.status,
                phase: row.phase,
                env_gate_status: row.env_gate_status,
                result_summary: row.result_summary,
                error_code: row.error_code,
                started_at: row.started_at,
                finished_at: row.finished_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
    }
    Ok(grouped)
}

fn aggregate_target_runs(
    runs: &[DeploymentTargetRunResponse],
    stored_status: &str,
    stored_phase: &str,
) -> (String, String) {
    if stored_status == "running" && stored_phase == "awaiting_release" {
        return (stored_status.to_owned(), stored_phase.to_owned());
    }
    if stored_status == "canceling" {
        return ("canceling".to_owned(), "canceling".to_owned());
    }
    if runs.is_empty() {
        return (stored_status.to_owned(), stored_phase.to_owned());
    }
    if runs
        .iter()
        .any(|run| matches!(run.status.as_str(), "failed" | "expired"))
    {
        return ("failed".to_owned(), "targets_failed".to_owned());
    }
    if runs
        .iter()
        .all(|run| matches!(run.status.as_str(), "succeeded" | "reused"))
    {
        return ("succeeded".to_owned(), "targets_succeeded".to_owned());
    }
    if runs.iter().any(|run| run.status == "canceled")
        && runs
            .iter()
            .all(|run| matches!(run.status.as_str(), "succeeded" | "reused" | "canceled"))
    {
        return ("canceled".to_owned(), "targets_canceled".to_owned());
    }
    if runs
        .iter()
        .any(|run| matches!(run.status.as_str(), "downloading" | "running"))
    {
        return ("running".to_owned(), "targets_running".to_owned());
    }
    if runs.iter().any(|run| run.status == "canceled") {
        return ("failed".to_owned(), "targets_incomplete".to_owned());
    }
    ("queued".to_owned(), "targets_pending".to_owned())
}

async fn find_idempotent(
    pool: &sqlx::SqlitePool,
    actor_id: &str,
    key: &str,
    request_hash: &str,
    request_id: &str,
) -> ApiResult<Option<DeploymentResponse>> {
    let existing: Option<(String, String)> = sqlx::query_as(
        "SELECT id,request_hash FROM deployments WHERE requested_by=? AND idempotency_key=?",
    )
    .bind(actor_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some((id, existing_hash)) = existing else {
        return Ok(None);
    };
    if existing_hash != request_hash {
        return Err(ApiError::conflict(
            "idempotency_conflict",
            "幂等键已用于不同部署请求",
            request_id,
        ));
    }
    Ok(Some(find(pool, &id, request_id).await?))
}

async fn find_retry_idempotent(
    pool: &sqlx::SqlitePool,
    actor_id: &str,
    key: &str,
    original_id: &str,
    request_id: &str,
) -> ApiResult<Option<DeploymentResponse>> {
    let existing: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT id,retry_of_id FROM deployments WHERE requested_by=? AND idempotency_key=?",
    )
    .bind(actor_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some((id, retry_of_id)) = existing else {
        return Ok(None);
    };
    if retry_of_id.as_deref() != Some(original_id) {
        return Err(ApiError::conflict(
            "idempotency_conflict",
            "幂等键已用于其他部署重试",
            request_id,
        ));
    }
    Ok(Some(find(pool, &id, request_id).await?))
}

fn task_result_fields(result_json: Option<&str>) -> (Option<i64>, Option<String>) {
    let Some(raw) = result_json else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return (None, None);
    };
    (
        value.get("exit_code").and_then(Value::as_i64),
        value
            .get("error_code")
            .and_then(Value::as_str)
            .map(str::to_owned),
    )
}
