use axum::{
    Json, Router,
    extract::{Extension, Path, State},
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
    auth::{AuthUser, hash_password, validate_credentials},
    error::{ApiError, ApiResult},
};

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub identity: String,
    pub status: String,
    pub version: i64,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct CreateUserRequest {
    username: String,
    password: String,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct UpdateStatusRequest {
    status: String,
    version: i64,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ResetPasswordRequest {
    password: String,
    version: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list).post(create))
        .route("/users/{id}/status", patch(update_status))
        .route("/users/{id}/password", post(reset_password))
}

#[utoipa::path(get, path = "/api/v1/users", responses((status = 200), (status = 401), (status = 403)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    user.require_administrator(request_id.as_str())?;
    let users = sqlx::query_as::<_, UserResponse>(
        "SELECT id, username, identity, status, version FROM users ORDER BY created_at, id LIMIT 200",
    )
    .fetch_all(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(json!({"items": users, "next_cursor": null})))
}

#[utoipa::path(post, path = "/api/v1/users", request_body = CreateUserRequest, responses((status = 201, body = UserResponse), (status = 401), (status = 403), (status = 409)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    Json(payload): Json<CreateUserRequest>,
) -> ApiResult<(StatusCode, Json<UserResponse>)> {
    user.require_administrator(request_id.as_str())?;
    user.verify_csrf(&headers, request_id.as_str())?;
    validate_credentials(&payload.username, &payload.password, request_id.as_str())?;
    let id = format!("usr_{}", Ulid::new());
    let password_hash = hash_password(&payload.password, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("INSERT INTO users (id, username, password_hash, identity, status) VALUES (?, ?, ?, 'user', 'active')")
        .bind(&id).bind(payload.username.trim()).bind(password_hash).execute(&mut *transaction).await;
    if let Err(error) = result {
        if error.to_string().contains("UNIQUE constraint failed") {
            return Err(ApiError::conflict(
                "username_exists",
                "用户名已经存在",
                request_id.as_str(),
            ));
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
            identity: "user".to_owned(),
            status: "active".to_owned(),
            version: 1,
        }),
    ))
}

#[utoipa::path(patch, path = "/api/v1/users/{id}/status", params(("id" = String, Path)), request_body = UpdateStatusRequest, responses((status = 200, body = UserResponse), (status = 401), (status = 403), (status = 404), (status = 409)))]
pub(crate) async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    Json(payload): Json<UpdateStatusRequest>,
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

#[utoipa::path(post, path = "/api/v1/users/{id}/password", params(("id" = String, Path)), request_body = ResetPasswordRequest, responses((status = 204), (status = 401), (status = 403), (status = 404), (status = 409)))]
pub(crate) async fn reset_password(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    Json(payload): Json<ResetPasswordRequest>,
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
    sqlx::query_as("SELECT id, username, identity, status, version FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))
}
