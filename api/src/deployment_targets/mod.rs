use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    execution_spec, grants, pagination,
};

const TARGET_ENVIRONMENT_COMPAT_VALUE: &str = "prod";

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SecretFileReference {
    pub environment_key: String,
    pub file_path: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveTargetRequest {
    node_id: String,
    script_path: String,
    parameter_schema: Value,
    timeout_seconds: i64,
    verification_config: Value,
    #[serde(default = "default_execution_mode")]
    execution_mode: String,
    #[serde(default)]
    secret_file_references: Vec<SecretFileReference>,
    #[serde(default)]
    privileged_release: bool,
    #[serde(default)]
    privileged_release_confirmed: bool,
    version: Option<i64>,
}

fn default_execution_mode() -> String {
    "script".to_owned()
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TargetStatusRequest {
    status: String,
    version: i64,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct DeploymentTargetResponse {
    pub id: String,
    pub application_id: String,
    pub node_id: String,
    #[schema(read_only)]
    pub environment: String,
    pub execution_mode: String,
    pub script_path: String,
    pub parameter_schema: Value,
    pub timeout_seconds: i64,
    pub verification_config: Value,
    pub secret_file_references: Vec<SecretFileReference>,
    pub privileged_release: bool,
    pub status: String,
    pub snapshot_hash: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DeploymentTargetListResponse {
    items: Vec<DeploymentTargetResponse>,
    next_cursor: Option<String>,
}

#[derive(sqlx::FromRow)]
struct TargetRow {
    id: String,
    application_id: String,
    node_id: String,
    environment: String,
    execution_mode: String,
    script_path: String,
    parameter_schema: String,
    timeout_seconds: i64,
    verification_config: String,
    privileged_release: bool,
    status: String,
    created_at: String,
    updated_at: String,
    version: i64,
}

#[derive(sqlx::FromRow)]
struct NodePolicy {
    status: String,
    work_root: String,
    secrets_root: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/applications/{application_id}/targets",
            get(list).post(create),
        )
        .route("/deployment-targets/{id}", get(show).patch(update))
        .route("/deployment-targets/{id}/status", put(update_status))
}

#[utoipa::path(operation_id = "deployment_targets_list", get, path = "/api/v1/applications/{application_id}/targets", params(("application_id" = String, Path), ("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = DeploymentTargetListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Query(query): Query<pagination::ListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<DeploymentTargetListResponse>> {
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    let limit = pagination::limit(&query, request_id.as_str())?;
    let after = pagination::decode_after(&query, request_id.as_str())?;
    let (environment, id) = after.clone().unwrap_or_default();
    let rows = sqlx::query_as::<_, TargetRow>("SELECT id, application_id, node_id, environment, execution_mode, script_path, parameter_schema, timeout_seconds, verification_config, privileged_release, status, created_at, updated_at, version FROM deployment_targets WHERE application_id=? AND (? IS NULL OR environment>? OR (environment=? AND id>?)) ORDER BY environment, id LIMIT ?")
        .bind(&application_id).bind(after.as_ref().map(|_| 1)).bind(&environment).bind(&environment).bind(&id).bind((limit + 1) as i64).fetch_all(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (rows, next_cursor) = pagination::finish(rows, limit, |item| (&item.environment, &item.id));
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(expand(state.pool(), row, request_id.as_str()).await?);
    }
    Ok(Json(DeploymentTargetListResponse { items, next_cursor }))
}

#[utoipa::path(operation_id = "deployment_targets_show", get, path = "/api/v1/deployment-targets/{id}", params(("id" = String, Path)), responses((status = 200, body = DeploymentTargetResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<DeploymentTargetResponse>> {
    let row = find_row(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &row.application_id,
        request_id.as_str(),
    )
    .await?;
    Ok(Json(expand(state.pool(), row, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "deployment_targets_create", post, path = "/api/v1/applications/{application_id}/targets", params(("application_id" = String, Path)), request_body = SaveTargetRequest, responses((status = 201, body = DeploymentTargetResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveTargetRequest>,
) -> ApiResult<(StatusCode, Json<DeploymentTargetResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    ensure_application_active(state.pool(), &application_id, request_id.as_str()).await?;
    let node = validate_target(state.pool(), &payload, request_id.as_str()).await?;
    require_privileged_release_confirmation(&payload, true, request_id.as_str())?;
    validate_two_stage_requirements(state.pool(), &application_id, &payload, request_id.as_str())
        .await?;
    let id = format!("target_{}", Ulid::new());
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO deployment_targets (id, application_id, node_id, environment, execution_mode, script_path, parameter_schema, timeout_seconds, verification_config, privileged_release, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active')")
        .bind(&id).bind(&application_id).bind(&payload.node_id).bind(TARGET_ENVIRONMENT_COMPAT_VALUE).bind(&payload.execution_mode).bind(&payload.script_path)
        .bind(payload.parameter_schema.to_string()).bind(payload.timeout_seconds).bind(payload.verification_config.to_string()).bind(payload.privileged_release)
        .execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    replace_secret_refs(
        &mut transaction,
        &id,
        &payload.secret_file_references,
        &node.secrets_root,
        request_id.as_str(),
    )
    .await?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment_target.create",
        "deployment_target",
        &id,
        request_id.as_str(),
        json!({"application_id":application_id,"node_id":payload.node_id,"privileged_release":payload.privileged_release}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let row = find_row(state.pool(), &id, request_id.as_str()).await?;
    Ok((
        StatusCode::CREATED,
        Json(expand(state.pool(), row, request_id.as_str()).await?),
    ))
}

#[utoipa::path(operation_id = "deployment_targets_update", patch, path = "/api/v1/deployment-targets/{id}", params(("id" = String, Path)), request_body = SaveTargetRequest, responses((status = 200, body = DeploymentTargetResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveTargetRequest>,
) -> ApiResult<Json<DeploymentTargetResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = find_row(state.pool(), &id, request_id.as_str()).await?;
    ensure_application_active(state.pool(), &current.application_id, request_id.as_str()).await?;
    let node = validate_target(state.pool(), &payload, request_id.as_str()).await?;
    require_privileged_release_confirmation(
        &payload,
        payload.privileged_release
            && (!current.privileged_release || current.node_id != payload.node_id),
        request_id.as_str(),
    )?;
    validate_two_stage_requirements(
        state.pool(),
        &current.application_id,
        &payload,
        request_id.as_str(),
    )
    .await?;
    let version = payload
        .version
        .ok_or_else(|| ApiError::validation("编辑部署目标必须提供 version", request_id.as_str()))?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE deployment_targets SET node_id=?, execution_mode=?, script_path=?, parameter_schema=?, timeout_seconds=?, verification_config=?, privileged_release=?, updated_at=?, version=version+1 WHERE id=? AND version=?")
        .bind(&payload.node_id).bind(&payload.execution_mode).bind(&payload.script_path).bind(payload.parameter_schema.to_string())
        .bind(payload.timeout_seconds).bind(payload.verification_config.to_string()).bind(payload.privileged_release).bind(Utc::now().to_rfc3339()).bind(&id).bind(version)
        .execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    require_updated(result.rows_affected(), request_id.as_str())?;
    replace_secret_refs(
        &mut transaction,
        &id,
        &payload.secret_file_references,
        &node.secrets_root,
        request_id.as_str(),
    )
    .await?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment_target.update",
        "deployment_target",
        &id,
        request_id.as_str(),
        json!({"node_id":payload.node_id,"privileged_release_before":current.privileged_release,"privileged_release_after":payload.privileged_release}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let row = find_row(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(expand(state.pool(), row, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "deployment_targets_update_status", put, path = "/api/v1/deployment-targets/{id}/status", params(("id" = String, Path)), request_body = TargetStatusRequest, responses((status = 200, body = DeploymentTargetResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<TargetStatusRequest>,
) -> ApiResult<Json<DeploymentTargetResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    if !matches!(payload.status.as_str(), "active" | "disabled") {
        return Err(ApiError::validation(
            "部署目标状态不正确",
            request_id.as_str(),
        ));
    }
    let current = find_row(state.pool(), &id, request_id.as_str()).await?;
    if payload.status == "active" {
        ensure_application_active(state.pool(), &current.application_id, request_id.as_str())
            .await?;
        ensure_node_online(state.pool(), &current.node_id, request_id.as_str()).await?;
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE deployment_targets SET status=?, updated_at=?, version=version+1 WHERE id=? AND version=?")
        .bind(&payload.status).bind(Utc::now().to_rfc3339()).bind(&id).bind(payload.version).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    require_updated(result.rows_affected(), request_id.as_str())?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment_target.status.update",
        "deployment_target",
        &id,
        request_id.as_str(),
        json!({"status":payload.status}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let row = find_row(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(expand(state.pool(), row, request_id.as_str()).await?))
}

async fn validate_target(
    pool: &sqlx::SqlitePool,
    payload: &SaveTargetRequest,
    request_id: &str,
) -> ApiResult<NodePolicy> {
    if !matches!(payload.execution_mode.as_str(), "script" | "two_stage")
        || !(1..=86_400).contains(&payload.timeout_seconds)
        || payload.secret_file_references.len() > 64
    {
        return Err(ApiError::validation("部署目标基础配置无效", request_id));
    }
    if payload.privileged_release && payload.execution_mode != "two_stage" {
        return Err(ApiError::validation(
            "开启特权发布必须使用两阶段模式",
            request_id,
        ));
    }
    let node: NodePolicy =
        sqlx::query_as("SELECT status, work_root, secrets_root FROM nodes WHERE id=?")
            .bind(&payload.node_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?
            .ok_or_else(|| ApiError::not_found(request_id))?;
    if node.status != "online" {
        return Err(ApiError::conflict(
            "node_not_deployable",
            "节点未通过检查或已停用",
            request_id,
        ));
    }
    execution_spec::validate_script_path(&node.work_root, &payload.script_path, request_id)?;
    execution_spec::validate_parameter_schema(&payload.parameter_schema, request_id)?;
    execution_spec::validate_verification_config(
        &payload.verification_config,
        &node.work_root,
        request_id,
    )?;
    let mut keys = std::collections::HashSet::new();
    for reference in &payload.secret_file_references {
        execution_spec::validate_environment_key(&reference.environment_key, request_id)?;
        execution_spec::validate_secret_path(&node.secrets_root, &reference.file_path, request_id)?;
        if !keys.insert(reference.environment_key.as_str()) {
            return Err(ApiError::validation("敏感文件环境变量键重复", request_id));
        }
    }
    Ok(node)
}

fn require_privileged_release_confirmation(
    payload: &SaveTargetRequest,
    confirmation_required: bool,
    request_id: &str,
) -> ApiResult<()> {
    if confirmation_required && payload.privileged_release && !payload.privileged_release_confirmed
    {
        return Err(ApiError::validation(
            "开启特权发布必须确认 root 信任边界",
            request_id,
        ));
    }
    Ok(())
}

async fn validate_two_stage_requirements(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    payload: &SaveTargetRequest,
    request_id: &str,
) -> ApiResult<()> {
    if payload.execution_mode != "two_stage" {
        return Ok(());
    }
    let source_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM application_sources WHERE application_id=?")
            .bind(application_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    if source_status.as_deref() != Some("verified") {
        return Err(ApiError::conflict(
            "git_source_not_verified",
            "两阶段部署要求应用先配置并固定 Git 分支来源",
            request_id,
        ));
    }
    let protocol_version: Option<i64> = sqlx::query_scalar(
        "SELECT agent.protocol_version FROM agents agent JOIN nodes node ON node.id=agent.node_id WHERE node.id=? AND agent.revoked_at IS NULL AND agent.archived_at IS NULL",
    )
    .bind(&payload.node_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if protocol_version.unwrap_or_default() < 2 {
        return Err(ApiError::conflict(
            "agent_protocol_too_old",
            "两阶段部署要求目标节点 Agent 支持协议 v2",
            request_id,
        ));
    }
    Ok(())
}

async fn replace_secret_refs(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    target_id: &str,
    refs: &[SecretFileReference],
    secrets_root: &str,
    request_id: &str,
) -> ApiResult<()> {
    sqlx::query("DELETE FROM secret_file_references WHERE deployment_target_id=?")
        .bind(target_id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    for reference in refs {
        execution_spec::validate_secret_path(secrets_root, &reference.file_path, request_id)?;
        sqlx::query("INSERT INTO secret_file_references (id, deployment_target_id, environment_key, file_path) VALUES (?, ?, ?, ?)")
            .bind(format!("secret_ref_{}", Ulid::new())).bind(target_id).bind(&reference.environment_key).bind(&reference.file_path)
            .execute(&mut **transaction).await.map_err(|_| ApiError::internal(request_id))?;
    }
    Ok(())
}

async fn expand(
    pool: &sqlx::SqlitePool,
    row: TargetRow,
    request_id: &str,
) -> ApiResult<DeploymentTargetResponse> {
    let refs: Vec<(String, String)> = sqlx::query_as("SELECT environment_key, file_path FROM secret_file_references WHERE deployment_target_id=? ORDER BY environment_key")
        .bind(&row.id).fetch_all(pool).await.map_err(|_| ApiError::internal(request_id))?;
    let parameter_schema: Value =
        serde_json::from_str(&row.parameter_schema).map_err(|_| ApiError::internal(request_id))?;
    let verification_config: Value = serde_json::from_str(&row.verification_config)
        .map_err(|_| ApiError::internal(request_id))?;
    let snapshot = execution_spec::target_snapshot(execution_spec::TargetSnapshotInput {
        application_id: &row.application_id,
        node_id: &row.node_id,
        environment: &row.environment,
        script_path: &row.script_path,
        parameter_schema: &parameter_schema,
        timeout_seconds: row.timeout_seconds,
        verification_config: &verification_config,
        secret_refs: &refs,
        privileged_release: row.privileged_release,
        version: row.version,
    });
    Ok(DeploymentTargetResponse {
        id: row.id,
        application_id: row.application_id,
        node_id: row.node_id,
        environment: row.environment,
        execution_mode: row.execution_mode,
        script_path: row.script_path,
        parameter_schema,
        timeout_seconds: row.timeout_seconds,
        verification_config,
        privileged_release: row.privileged_release,
        secret_file_references: refs
            .into_iter()
            .map(|(environment_key, file_path)| SecretFileReference {
                environment_key,
                file_path,
            })
            .collect(),
        status: row.status,
        snapshot_hash: execution_spec::snapshot_hash(&snapshot),
        created_at: row.created_at,
        updated_at: row.updated_at,
        version: row.version,
    })
}

async fn find_row(pool: &sqlx::SqlitePool, id: &str, request_id: &str) -> ApiResult<TargetRow> {
    sqlx::query_as("SELECT id, application_id, node_id, environment, execution_mode, script_path, parameter_schema, timeout_seconds, verification_config, privileged_release, status, created_at, updated_at, version FROM deployment_targets WHERE id=?")
        .bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}
async fn ensure_application_active(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM applications WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    match status.as_deref() {
        Some("active") => Ok(()),
        Some(_) => Err(ApiError::conflict(
            "application_archived",
            "应用已归档",
            request_id,
        )),
        None => Err(ApiError::not_found(request_id)),
    }
}
async fn ensure_node_online(pool: &sqlx::SqlitePool, id: &str, request_id: &str) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM nodes WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    match status.as_deref() {
        Some("online") => Ok(()),
        Some(_) => Err(ApiError::conflict(
            "node_not_deployable",
            "节点未通过检查或已停用",
            request_id,
        )),
        None => Err(ApiError::not_found(request_id)),
    }
}
fn require_updated(rows: u64, request_id: &str) -> ApiResult<()> {
    if rows == 0 {
        Err(ApiError::conflict(
            "resource_version_conflict",
            "部署目标已经被其他请求修改",
            request_id,
        ))
    } else {
        Ok(())
    }
}
fn map_unique(error: sqlx::Error, request_id: &str) -> ApiError {
    if error.to_string().contains("UNIQUE constraint failed") {
        ApiError::conflict(
            "deployment_target_exists",
            "相同应用、环境和节点的目标已存在",
            request_id,
        )
    } else {
        ApiError::internal(request_id)
    }
}
