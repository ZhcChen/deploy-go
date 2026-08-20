use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, application_configs, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    execution_spec, grants, pagination,
};

#[derive(Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct ApplicationResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub app_type: String,
    pub type_version: String,
    pub environment: String,
    pub parameter_schema: serde_json::Value,
    pub verification_config: serde_json::Value,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ApplicationListResponse {
    items: Vec<ApplicationResponse>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationListQuery {
    limit: Option<u32>,
    after: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveApplicationRequest {
    name: String,
    slug: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_app_type")]
    app_type: String,
    #[serde(default = "default_type_version")]
    type_version: String,
    environment: String,
    #[serde(default)]
    parameter_schema: Option<serde_json::Value>,
    #[serde(default)]
    verification_config: Option<serde_json::Value>,
    #[serde(default, alias = "template")]
    template_id: Option<String>,
    version: Option<i64>,
}

fn default_app_type() -> String {
    "binary".to_owned()
}

fn default_type_version() -> String {
    "1".to_owned()
}

fn default_parameter_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {},
        "required": [],
        "additionalProperties": false
    })
}

fn default_verification_config() -> serde_json::Value {
    serde_json::json!({
        "type": "http",
        "path": "/healthz",
        "expected_status": 200,
        "timeout_ms": 5000
    })
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationStatusRequest {
    status: String,
    version: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/applications", get(list).post(create))
        .route("/applications/{id}", get(show).patch(update))
        .route("/applications/{id}/status", put(update_status))
}

#[utoipa::path(operation_id = "applications_list", get, path = "/api/v1/applications", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query), ("status" = Option<String>, Query)), responses((status = 200, body = ApplicationListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<ApplicationListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<ApplicationListResponse>> {
    if !matches!(query.status.as_deref(), None | Some("active" | "archived")) {
        return Err(ApiError::validation(
            "应用状态筛选值不正确",
            request_id.as_str(),
        ));
    }
    let page = pagination::ListQuery {
        limit: query.limit,
        after: query.after,
    };
    let limit = pagination::limit(&page, request_id.as_str())?;
    let (created_at, id) = pagination::decode_after(&page, request_id.as_str())?
        .unwrap_or_else(|| ("0000".to_owned(), "".to_owned()));
    let applications = if actor.identity == "administrator" {
        sqlx::query_as::<_, ApplicationResponse>("SELECT id, display_name AS name, slug, description, app_type, type_version, environment, parameter_schema, verification_config, status, created_at, updated_at, version FROM applications WHERE (created_at>? OR (created_at=? AND id>?)) AND (? IS NULL OR status=?) ORDER BY created_at, id LIMIT ?")
            .bind(&created_at).bind(&created_at).bind(&id).bind(&query.status).bind(&query.status).bind((limit + 1) as i64).fetch_all(state.pool()).await
    } else {
        sqlx::query_as::<_, ApplicationResponse>("SELECT a.id, a.display_name AS name, a.slug, a.description, a.app_type, a.type_version, a.environment, a.parameter_schema, a.verification_config, a.status, a.created_at, a.updated_at, a.version FROM applications a JOIN user_application_grants g ON g.application_id=a.id WHERE g.user_id=? AND (a.created_at>? OR (a.created_at=? AND a.id>?)) AND (? IS NULL OR a.status=?) ORDER BY a.created_at, a.id LIMIT ?")
            .bind(&actor.id).bind(&created_at).bind(&created_at).bind(&id).bind(&query.status).bind(&query.status).bind((limit + 1) as i64).fetch_all(state.pool()).await
    }
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (items, next_cursor) =
        pagination::finish(applications, limit, |item| (&item.created_at, &item.id));
    Ok(Json(ApplicationListResponse { items, next_cursor }))
}

#[utoipa::path(operation_id = "applications_show", get, path = "/api/v1/applications/{id}", params(("id" = String, Path)), responses((status = 200, body = ApplicationResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<ApplicationResponse>> {
    grants::require_application_access(state.pool(), &actor, &id, request_id.as_str()).await?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "applications_create", post, path = "/api/v1/applications", request_body = SaveApplicationRequest, responses((status = 201, body = ApplicationResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveApplicationRequest>,
) -> ApiResult<(StatusCode, Json<ApplicationResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    validate(&payload, request_id.as_str())?;
    let id = format!("app_{}", Ulid::new());
    let parameter_schema = payload
        .parameter_schema
        .clone()
        .unwrap_or_else(default_parameter_schema);
    let verification_config = payload
        .verification_config
        .clone()
        .unwrap_or_else(default_verification_config);
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO applications (id, name, display_name, slug, description, app_type, type_version, environment, parameter_schema, verification_config, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active')")
        .bind(&id).bind(&id).bind(payload.name.trim()).bind(&payload.slug).bind(payload.description.trim()).bind(&payload.app_type).bind(&payload.type_version).bind(payload.environment.trim()).bind(parameter_schema.to_string()).bind(verification_config.to_string())
        .execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    if let Some(template_id) = payload.template_id.as_deref() {
        let ring = state
            .master_key_ring()
            .ok_or_else(|| ApiError::service_not_ready(request_id.as_str()))?;
        application_configs::clone_template_for_application(
            &mut transaction,
            ring,
            &id,
            template_id,
            Some(&actor.id),
            request_id.as_str(),
        )
        .await?;
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application.create",
        "application",
        &id,
        request_id.as_str(),
        json!({"name":payload.name.trim(),"slug":payload.slug,"app_type":payload.app_type,"type_version":payload.type_version,"environment":payload.environment.trim(),"template_id":payload.template_id}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(find(state.pool(), &id, request_id.as_str()).await?),
    ))
}

#[utoipa::path(operation_id = "applications_update", patch, path = "/api/v1/applications/{id}", params(("id" = String, Path)), request_body = SaveApplicationRequest, responses((status = 200, body = ApplicationResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveApplicationRequest>,
) -> ApiResult<Json<ApplicationResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    validate(&payload, request_id.as_str())?;
    let current = find(state.pool(), &id, request_id.as_str()).await?;
    let version = payload
        .version
        .ok_or_else(|| ApiError::validation("编辑应用必须提供 version", request_id.as_str()))?;
    let parameter_schema = payload
        .parameter_schema
        .clone()
        .unwrap_or_else(|| current.parameter_schema.clone());
    let verification_config = payload
        .verification_config
        .clone()
        .unwrap_or_else(|| current.verification_config.clone());
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE applications SET display_name=?, slug=?, description=?, app_type=?, type_version=?, environment=?, parameter_schema=?, verification_config=?, updated_at=?, version=version+1 WHERE id=? AND version=?")
        .bind(payload.name.trim()).bind(&payload.slug).bind(payload.description.trim()).bind(&payload.app_type).bind(&payload.type_version).bind(payload.environment.trim()).bind(parameter_schema.to_string()).bind(verification_config.to_string()).bind(Utc::now().to_rfc3339()).bind(&id).bind(version)
        .execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    require_updated(result.rows_affected(), request_id.as_str())?;
    if current.environment != payload.environment {
        let target_result = sqlx::query("UPDATE deployment_targets SET environment=?, updated_at=?, version=version+1 WHERE application_id=?")
            .bind(payload.environment.trim()).bind(Utc::now().to_rfc3339()).bind(&id)
            .execute(&mut *transaction).await.map_err(|error| map_target_unique(error, request_id.as_str()))?;
        if target_result.rows_affected() > 0 {
            audit::record(
                &mut transaction,
                Some(&actor.id),
                "deployment_target.environment.sync",
                "application",
                &id,
                request_id.as_str(),
                json!({"environment_before":current.environment,"environment_after":payload.environment.trim(),"targets_updated":target_result.rows_affected()}),
            )
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        }
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application.update",
        "application",
        &id,
        request_id.as_str(),
        json!({"name":payload.name.trim(),"slug":payload.slug,"app_type":payload.app_type,"type_version":payload.type_version,"environment_before":current.environment,"environment_after":payload.environment.trim()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "applications_update_status", put, path = "/api/v1/applications/{id}/status", params(("id" = String, Path)), request_body = ApplicationStatusRequest, responses((status = 200, body = ApplicationResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<ApplicationStatusRequest>,
) -> ApiResult<Json<ApplicationResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    if !matches!(payload.status.as_str(), "active" | "archived") {
        return Err(ApiError::validation("应用状态不正确", request_id.as_str()));
    }
    find(state.pool(), &id, request_id.as_str()).await?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE applications SET status=?, updated_at=?, version=version+1 WHERE id=? AND version=?")
        .bind(&payload.status).bind(Utc::now().to_rfc3339()).bind(&id).bind(payload.version).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    require_updated(result.rows_affected(), request_id.as_str())?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application.status.update",
        "application",
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
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

fn validate(payload: &SaveApplicationRequest, request_id: &str) -> ApiResult<()> {
    if payload.name.trim().is_empty()
        || payload.name.chars().count() > 100
        || payload.name.chars().any(char::is_control)
        || !(3..=64).contains(&payload.slug.len())
        || payload.slug.starts_with('-')
        || payload.slug.ends_with('-')
        || !payload
            .slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || payload.description.chars().count() > 1000
        || payload.description.chars().any(char::is_control)
        || !valid_application_type(&payload.app_type, &payload.type_version)
        || !matches!(
            payload.environment.as_str(),
            "dev" | "test" | "staging" | "prod"
        )
    {
        return Err(ApiError::validation("应用配置格式不正确", request_id));
    }
    if let Some(schema) = payload.parameter_schema.as_ref() {
        execution_spec::validate_parameter_schema(schema, request_id)?;
    }
    if let Some(config) = payload.verification_config.as_ref() {
        // 应用级只校验配置形状；路径必须落在目标节点 work_root 内由目标创建/部署时校验。
        execution_spec::validate_verification_config(config, "/", request_id)?;
    }
    Ok(())
}

async fn find(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<ApplicationResponse> {
    sqlx::query_as("SELECT id, display_name AS name, slug, description, app_type, type_version, environment, parameter_schema, verification_config, status, created_at, updated_at, version FROM applications WHERE id=?")
        .bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}

fn valid_application_type(app_type: &str, type_version: &str) -> bool {
    if type_version.len() > 32
        || !type_version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return false;
    }
    match app_type {
        "binary" => type_version == "1",
        "redis" => type_version == "7",
        "valkey" => type_version == "9",
        "postgres" => matches!(type_version, "16" | "18"),
        "etcd" => type_version == "3.6",
        _ => false,
    }
}
fn require_updated(rows: u64, request_id: &str) -> ApiResult<()> {
    if rows == 0 {
        Err(ApiError::conflict(
            "resource_version_conflict",
            "应用已经被其他请求修改",
            request_id,
        ))
    } else {
        Ok(())
    }
}
fn map_unique(error: sqlx::Error, request_id: &str) -> ApiError {
    let detail = error.to_string();
    if detail.contains("applications.slug") {
        ApiError::conflict("application_slug_exists", "应用 slug 已存在", request_id)
    } else if detail.contains("UNIQUE constraint failed") {
        ApiError::conflict(
            "application_identity_exists",
            "应用名称或 slug 已存在",
            request_id,
        )
    } else {
        ApiError::internal(request_id)
    }
}

fn map_target_unique(error: sqlx::Error, request_id: &str) -> ApiError {
    if error.to_string().contains("UNIQUE constraint failed") {
        ApiError::conflict(
            "deployment_target_environment_conflict",
            "应用环境变更会与同节点历史目标冲突，请先停用或删除重复目标",
            request_id,
        )
    } else {
        ApiError::internal(request_id)
    }
}
