use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
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
    grants,
};

#[derive(Clone, Serialize, ToSchema)]
pub struct RuntimeStatusResponse {
    runtime_status_id: String,
    application_id: String,
    target_id: String,
    target_code: String,
    status: String,
    payload: Option<Value>,
    error_code: Option<String>,
    error_message: Option<String>,
    requested_by: Option<String>,
    requested_at: String,
    observed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct RuntimeStatusRow {
    runtime_status_id: String,
    application_id: String,
    target_id: String,
    target_code: String,
    status: String,
    payload_json: Option<String>,
    error_code: Option<String>,
    error_message: Option<String>,
    requested_by: Option<String>,
    requested_at: String,
    observed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
pub(crate) struct RuntimeStatusQuery {
    target_id: Option<String>,
}

#[derive(Clone)]
struct RuntimeStatusTarget {
    target_id: String,
    target_code: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/applications/{application_id}/runtime-status",
        get(show).post(read),
    )
}

#[utoipa::path(operation_id = "runtime_status_show", get, path = "/api/v1/applications/{application_id}/runtime-status", params(("application_id" = String, Path), ("target_id" = Option<String>, Query)), responses((status = 200, body = RuntimeStatusResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Query(query): Query<RuntimeStatusQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<RuntimeStatusResponse>> {
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    let target = resolve_target(
        state.pool(),
        &application_id,
        query.target_id.as_deref(),
        request_id.as_str(),
    )
    .await?;
    Ok(Json(
        find_status(
            state.pool(),
            &application_id,
            &target.target_id,
            request_id.as_str(),
        )
        .await?,
    ))
}

#[utoipa::path(operation_id = "runtime_status_read", post, path = "/api/v1/applications/{application_id}/runtime-status", params(("application_id" = String, Path), ("target_id" = Option<String>, Query)), responses((status = 202, body = RuntimeStatusResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn read(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Query(query): Query<RuntimeStatusQuery>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<(StatusCode, Json<RuntimeStatusResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let target = resolve_target(
        state.pool(),
        &application_id,
        query.target_id.as_deref(),
        request_id.as_str(),
    )
    .await?;
    let runtime_status_id = upsert_pending(
        &state,
        &application_id,
        &target,
        &actor,
        request_id.as_str(),
    )
    .await?;
    crate::agents::dispatcher::enqueue_runtime_status_probe(&state, &runtime_status_id).await?;
    let mut audit_transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut audit_transaction,
        Some(&actor.id),
        "application.runtime_status.read",
        "application",
        &application_id,
        request_id.as_str(),
        json!({"target_id":target.target_id,"target_code":target.target_code,"runtime_status_id":runtime_status_id,"status":"pending"}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit_transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::ACCEPTED,
        Json(
            find_status(
                state.pool(),
                &application_id,
                &target.target_id,
                request_id.as_str(),
            )
            .await?,
        ),
    ))
}

async fn resolve_target(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    target_id: Option<&str>,
    request_id: &str,
) -> ApiResult<RuntimeStatusTarget> {
    if let Some(target_id) = target_id {
        if target_id.is_empty()
            || target_id.len() > 128
            || !target_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ApiError::validation("target_id 参数格式不正确", request_id));
        }
        let row: Option<(String, String, String)> = sqlx::query_as(
            "SELECT t.id,t.target_code,t.status FROM deployment_targets t JOIN applications a ON a.id=t.application_id WHERE t.id=? AND t.application_id=? AND t.status='active' AND a.status='active'",
        )
        .bind(target_id)
        .bind(application_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
        let Some((target_id, target_code, status)) = row else {
            return Err(ApiError::not_found(request_id));
        };
        if status != "active" {
            return Err(ApiError::not_found(request_id));
        }
        return Ok(RuntimeStatusTarget {
            target_id,
            target_code,
        });
    }
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT t.id,t.target_code FROM deployment_targets t JOIN applications a ON a.id=t.application_id WHERE t.application_id=? AND t.status='active' AND a.status='active' ORDER BY t.created_at,t.id",
    )
    .bind(application_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    match rows.len() {
        0 => Err(ApiError::not_found(request_id)),
        1 => {
            let (target_id, target_code) = rows.into_iter().next().expect("rows len == 1");
            Ok(RuntimeStatusTarget {
                target_id,
                target_code,
            })
        }
        _ => Err(ApiError::validation(
            "应用存在多个启用目标，请通过 target_id 指定目标",
            request_id,
        )),
    }
}

async fn upsert_pending(
    state: &AppState,
    application_id: &str,
    target: &RuntimeStatusTarget,
    actor: &AuthUser,
    request_id: &str,
) -> ApiResult<String> {
    let existing: Option<(String, String)> = sqlx::query_as(
        "SELECT runtime_status_id,status FROM application_runtime_statuses WHERE target_id=? ORDER BY created_at DESC,runtime_status_id DESC LIMIT 1",
    )
    .bind(&target.target_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if existing
        .as_ref()
        .is_some_and(|(_, status)| matches!(status.as_str(), "pending" | "running"))
    {
        return Err(ApiError::conflict(
            "runtime_status_in_progress",
            "运行时状态读取正在进行，请等待完成后再试",
            request_id,
        ));
    }
    let runtime_status_id = format!("status_{}", Ulid::new());
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO application_runtime_statuses (runtime_status_id,application_id,target_id,status,requested_by,requested_at) VALUES (?,?,?,'pending',?,?)")
        .bind(&runtime_status_id)
        .bind(application_id)
        .bind(&target.target_id)
        .bind(&actor.id)
        .bind(&now)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(runtime_status_id)
}

async fn find_status(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    target_id: &str,
    request_id: &str,
) -> ApiResult<RuntimeStatusResponse> {
    let row: Option<RuntimeStatusRow> = sqlx::query_as(
        "SELECT rs.runtime_status_id,rs.application_id,rs.target_id,t.target_code,rs.status,rs.payload_json,rs.error_code,rs.error_message,rs.requested_by,rs.requested_at,rs.observed_at,rs.created_at,rs.updated_at FROM application_runtime_statuses rs JOIN deployment_targets t ON t.id=rs.target_id WHERE rs.application_id=? AND rs.target_id=? ORDER BY rs.created_at DESC,rs.runtime_status_id DESC LIMIT 1",
    )
    .bind(application_id)
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let row = row.ok_or_else(|| ApiError::not_found(request_id))?;
    Ok(RuntimeStatusResponse {
        runtime_status_id: row.runtime_status_id,
        application_id: row.application_id,
        target_id: row.target_id,
        target_code: row.target_code,
        status: row.status,
        payload: row
            .payload_json
            .as_deref()
            .and_then(|value| serde_json::from_str(value).ok()),
        error_code: row.error_code,
        error_message: row.error_message,
        requested_by: row.requested_by,
        requested_at: row.requested_at,
        observed_at: row.observed_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}
