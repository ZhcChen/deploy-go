use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::{
        AuthUser, hash_password, validate_credentials, validate_display_name,
        validate_optional_email,
    },
    error::{ApiError, ApiResult},
    pagination,
};

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub identity: String,
    pub status: String,
    pub version: i64,
}

#[derive(sqlx::FromRow)]
struct UserListRow {
    id: String,
    username: String,
    display_name: String,
    email: Option<String>,
    identity: String,
    status: String,
    version: i64,
    created_at: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateUserRequest {
    username: String,
    password: String,
    display_name: Option<String>,
    email: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateStatusRequest {
    status: String,
    version: i64,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResetPasswordRequest {
    password: String,
    version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct UserListResponse {
    items: Vec<UserResponse>,
    next_cursor: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list).post(create))
        .route("/users/{id}", get(show))
        .route("/users/{id}/status", patch(update_status))
        .route("/users/{id}/password", post(reset_password))
}

#[utoipa::path(operation_id = "users_show", get, path = "/api/v1/users/{id}", params(("id" = String, Path)), responses((status = 200, body = UserResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    user: AuthUser,
) -> ApiResult<Json<UserResponse>> {
    user.require_administrator(request_id.as_str())?;
    Ok(Json(
        find_user(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "users_list", get, path = "/api/v1/users", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = UserListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<pagination::ListQuery>,
    Extension(request_id): Extension<RequestId>,
    user: AuthUser,
) -> ApiResult<Json<UserListResponse>> {
    user.require_administrator(request_id.as_str())?;
    let limit = pagination::limit(&query, request_id.as_str())?;
    let (created_at, id) = pagination::decode_after(&query, request_id.as_str())?
        .unwrap_or_else(|| ("0000".to_owned(), "".to_owned()));
    let users = sqlx::query_as::<_, UserListRow>(
        "SELECT id, username, COALESCE(display_name, username) AS display_name, email, identity, status, version, created_at FROM users WHERE system_account = 0 AND (created_at>? OR (created_at=? AND id>?)) ORDER BY created_at, id LIMIT ?",
    )
    .bind(&created_at).bind(&created_at).bind(&id).bind((limit + 1) as i64)
    .fetch_all(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (users, next_cursor) =
        pagination::finish(users, limit, |item| (&item.created_at, &item.id));
    let users = users
        .into_iter()
        .map(|user| UserResponse {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            email: user.email,
            identity: user.identity,
            status: user.status,
            version: user.version,
        })
        .collect();
    Ok(Json(UserListResponse {
        items: users,
        next_cursor,
    }))
}

#[utoipa::path(operation_id = "users_create", post, path = "/api/v1/users", request_body = CreateUserRequest, responses((status = 201, body = UserResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<CreateUserRequest>,
) -> ApiResult<(StatusCode, Json<UserResponse>)> {
    user.require_administrator(request_id.as_str())?;
    user.verify_csrf(&headers, request_id.as_str())?;
    validate_credentials(&payload.username, &payload.password, request_id.as_str())?;
    let display_name = validate_display_name(
        payload.display_name.as_deref().unwrap_or(&payload.username),
        request_id.as_str(),
    )?;
    let email = validate_optional_email(payload.email.as_deref(), request_id.as_str())?;
    let id = format!("usr_{}", Ulid::new());
    let password_hash = hash_password(&payload.password, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("INSERT INTO users (id, username, password_hash, identity, status, display_name, email) VALUES (?, ?, ?, 'user', 'active', ?, ?)")
        .bind(&id).bind(payload.username.trim()).bind(password_hash).bind(&display_name).bind(&email).execute(&mut *transaction).await;
    if let Err(error) = result {
        if error.to_string().contains("UNIQUE constraint failed") {
            let (code, message) = if error.to_string().contains("users.email") {
                ("email_exists", "邮箱已经存在")
            } else {
                ("username_exists", "用户名已经存在")
            };
            return Err(ApiError::conflict(code, message, request_id.as_str()));
        }
        return Err(ApiError::internal(request_id.as_str()));
    }
    audit::record(
        &mut transaction,
        Some(&user.id),
        "user.create",
        "user",
        &id,
        request_id.as_str(),
        json!({"username":payload.username.trim()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(UserResponse {
            id,
            username: payload.username.trim().to_owned(),
            display_name,
            email,
            identity: "user".to_owned(),
            status: "active".to_owned(),
            version: 1,
        }),
    ))
}

#[utoipa::path(operation_id = "users_update_status", patch, path = "/api/v1/users/{id}/status", params(("id" = String, Path)), request_body = UpdateStatusRequest, responses((status = 200, body = UserResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<UpdateStatusRequest>,
) -> ApiResult<Json<UserResponse>> {
    user.require_administrator(request_id.as_str())?;
    user.verify_csrf(&headers, request_id.as_str())?;
    if !matches!(payload.status.as_str(), "active" | "disabled") {
        return Err(ApiError::validation("用户状态不正确", request_id.as_str()));
    }
    let target = find_user(state.pool(), &id, request_id.as_str()).await?;
    if target.identity == "administrator" && payload.status != "active" {
        return Err(ApiError::conflict(
            "administrator_protected",
            "管理员不能被停用",
            request_id.as_str(),
        ));
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE users SET status = ?, version = version + 1, updated_at = ? WHERE id = ? AND version = ?")
        .bind(&payload.status).bind(Utc::now().to_rfc3339()).bind(&id).bind(payload.version)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "用户已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    if payload.status == "disabled" {
        sqlx::query("UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL")
            .bind(Utc::now().to_rfc3339())
            .bind(&id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    }
    audit::record(
        &mut transaction,
        Some(&user.id),
        "user.status.update",
        "user",
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
    Ok(Json(
        find_user(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "users_reset_password", post, path = "/api/v1/users/{id}/password", params(("id" = String, Path)), request_body = ResetPasswordRequest, responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn reset_password(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<ResetPasswordRequest>,
) -> ApiResult<StatusCode> {
    user.require_administrator(request_id.as_str())?;
    user.verify_csrf(&headers, request_id.as_str())?;
    if !(12..=256).contains(&payload.password.len()) {
        return Err(ApiError::validation(
            "密码长度必须为 12 至 256 个字符",
            request_id.as_str(),
        ));
    }
    let target = find_user(state.pool(), &id, request_id.as_str()).await?;
    let password_hash = hash_password(&payload.password, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE users SET password_hash = ?, version = version + 1, updated_at = ? WHERE id = ? AND version = ?")
        .bind(password_hash).bind(Utc::now().to_rfc3339()).bind(&id).bind(payload.version)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "用户已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    sqlx::query("UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL")
        .bind(Utc::now().to_rfc3339())
        .bind(&id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&user.id),
        "user.password.reset",
        "user",
        &id,
        request_id.as_str(),
        json!({"username":target.username}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn find_user(pool: &sqlx::SqlitePool, id: &str, request_id: &str) -> ApiResult<UserResponse> {
    sqlx::query_as("SELECT id, username, COALESCE(display_name, username) AS display_name, email, identity, status, version FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))
}
