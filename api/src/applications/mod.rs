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
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    grants, pagination,
};

#[derive(Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct ApplicationResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
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
    version: Option<i64>,
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
        sqlx::query_as::<_, ApplicationResponse>("SELECT id, name, slug, description, status, created_at, updated_at, version FROM applications WHERE (created_at>? OR (created_at=? AND id>?)) AND (? IS NULL OR status=?) ORDER BY created_at, id LIMIT ?")
            .bind(&created_at).bind(&created_at).bind(&id).bind(&query.status).bind(&query.status).bind((limit + 1) as i64).fetch_all(state.pool()).await
    } else {
        sqlx::query_as::<_, ApplicationResponse>("SELECT a.id, a.name, a.slug, a.description, a.status, a.created_at, a.updated_at, a.version FROM applications a JOIN user_application_grants g ON g.application_id=a.id WHERE g.user_id=? AND (a.created_at>? OR (a.created_at=? AND a.id>?)) AND (? IS NULL OR a.status=?) ORDER BY a.created_at, a.id LIMIT ?")
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
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO applications (id, name, slug, description, status) VALUES (?, ?, ?, ?, 'active')")
        .bind(&id).bind(payload.name.trim()).bind(&payload.slug).bind(payload.description.trim())
        .execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application.create",
        "application",
        &id,
        request_id.as_str(),
        json!({"name":payload.name.trim(),"slug":payload.slug}),
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
    find(state.pool(), &id, request_id.as_str()).await?;
    let version = payload
        .version
        .ok_or_else(|| ApiError::validation("编辑应用必须提供 version", request_id.as_str()))?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE applications SET name=?, slug=?, description=?, updated_at=?, version=version+1 WHERE id=? AND version=?")
        .bind(payload.name.trim()).bind(&payload.slug).bind(payload.description.trim()).bind(Utc::now().to_rfc3339()).bind(&id).bind(version)
        .execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    require_updated(result.rows_affected(), request_id.as_str())?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application.update",
        "application",
        &id,
        request_id.as_str(),
        json!({"name":payload.name.trim(),"slug":payload.slug}),
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
    {
        return Err(ApiError::validation("应用配置格式不正确", request_id));
    }
    Ok(())
}

async fn find(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<ApplicationResponse> {
    sqlx::query_as("SELECT id, name, slug, description, status, created_at, updated_at, version FROM applications WHERE id=?")
        .bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
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
    if error.to_string().contains("UNIQUE constraint failed") {
        ApiError::conflict(
            "application_identity_exists",
            "应用名称或 slug 已存在",
            request_id,
        )
    } else {
        ApiError::internal(request_id)
    }
}
