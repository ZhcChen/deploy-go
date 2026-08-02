use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
};
use serde::Serialize;
use serde_json::json;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};

pub async fn require_application_access(
    pool: &sqlx::SqlitePool,
    actor: &AuthUser,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let visible: bool = if actor.identity == "administrator" {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM applications WHERE id = ?)")
            .bind(application_id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?
    } else {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_application_grants WHERE user_id = ? AND application_id = ?)")
            .bind(&actor.id)
            .bind(application_id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?
    };
    if visible {
        Ok(())
    } else {
        Err(ApiError::not_found(request_id))
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/{user_id}/applications", get(list))
        .route(
            "/users/{user_id}/applications/{application_id}",
            put(grant).delete(revoke),
        )
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct ApplicationGrantResponse {
    application_id: String,
    granted_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct ApplicationGrantListResponse {
    items: Vec<ApplicationGrantResponse>,
    next_cursor: Option<String>,
}

#[utoipa::path(operation_id = "grants_list", get, path = "/api/v1/users/{user_id}/applications", params(("user_id" = String, Path)), responses((status = 200, body = ApplicationGrantListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<ApplicationGrantListResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let target_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = ? AND identity = 'user')")
            .bind(&user_id)
            .fetch_one(state.pool())
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if !target_exists {
        return Err(ApiError::not_found(request_id.as_str()));
    }
    let items = sqlx::query_as(
        "SELECT application_id, granted_at FROM user_application_grants WHERE user_id = ? ORDER BY granted_at, application_id",
    )
    .bind(user_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ApplicationGrantListResponse {
        items,
        next_cursor: None,
    }))
}

#[utoipa::path(operation_id = "grants_grant", put, path = "/api/v1/users/{user_id}/applications/{application_id}", params(("user_id" = String, Path), ("application_id" = String, Path)), responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn grant(
    State(state): State<AppState>,
    Path((user_id, application_id)): Path<(String, String)>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<StatusCode> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let target_identity: Option<String> =
        sqlx::query_scalar("SELECT identity FROM users WHERE id = ? AND status = 'active'")
            .bind(&user_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if target_identity.as_deref() != Some("user") {
        return Err(ApiError::not_found(request_id.as_str()));
    }
    let application_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM applications WHERE id = ?)")
            .bind(&application_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if !application_exists {
        return Err(ApiError::not_found(request_id.as_str()));
    }
    let result = sqlx::query("INSERT INTO user_application_grants (user_id, application_id, granted_by) VALUES (?, ?, ?) ON CONFLICT(user_id, application_id) DO NOTHING")
        .bind(&user_id).bind(&application_id).bind(&actor.id).execute(&mut *transaction).await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() == 0 {
        transaction
            .commit()
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        return Ok(StatusCode::NO_CONTENT);
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application.grant",
        "application",
        &application_id,
        request_id.as_str(),
        json!({"user_id":user_id}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(operation_id = "grants_revoke", delete, path = "/api/v1/users/{user_id}/applications/{application_id}", params(("user_id" = String, Path), ("application_id" = String, Path)), responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn revoke(
    State(state): State<AppState>,
    Path((user_id, application_id)): Path<(String, String)>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<StatusCode> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result =
        sqlx::query("DELETE FROM user_application_grants WHERE user_id = ? AND application_id = ?")
            .bind(&user_id)
            .bind(&application_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() == 0 {
        transaction
            .commit()
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        return Ok(StatusCode::NO_CONTENT);
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application.grant.revoke",
        "application",
        &application_id,
        request_id.as_str(),
        json!({"user_id":user_id}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(StatusCode::NO_CONTENT)
}
