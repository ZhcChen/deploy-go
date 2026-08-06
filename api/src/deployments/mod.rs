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

#[derive(Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct DeploymentResponse {
    pub id: String,
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
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewRequest {
    parameters: Value,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfirmRequest {
    parameters: Value,
    snapshot_hash: String,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LogQuery {
    after: Option<i64>,
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
    stream: String,
    content: String,
    truncated: bool,
    created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct DeploymentListResponse {
    items: Vec<DeploymentResponse>,
    next_cursor: Option<String>,
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
    target_status: String,
    target_version: i64,
}

struct PreviewData {
    response: DeploymentPreviewResponse,
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
        .route("/deployment-targets/{id}/deployment-preview", post(preview))
        .route("/deployment-targets/{id}/deployments", post(confirm))
        .route("/deployments", get(list))
        .route("/deployments/{id}", get(show))
        .route("/deployments/{id}/logs", get(logs))
        .route("/deployments/{id}/cancel", post(cancel))
        .route("/deployments/{id}/retry", post(retry))
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
    let stored_idempotency_key = format!("confirm:{idempotency_key}");
    let preview = build_preview(
        &state,
        &actor,
        &id,
        &payload.parameters,
        request_id.as_str(),
    )
    .await?;
    if preview.response.snapshot_hash != payload.snapshot_hash {
        return Err(ApiError::conflict(
            "deployment_snapshot_changed",
            "部署目标配置已经变化，请重新确认",
            request_id.as_str(),
        ));
    }
    let request_hash = digest_json(
        &json!({"target_id":id,"parameters":payload.parameters,"snapshot_hash":payload.snapshot_hash}),
    );
    if let Some((existing_id, existing_hash)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id, request_hash FROM deployments WHERE requested_by=? AND idempotency_key=?",
    )
    .bind(&actor.id)
    .bind(&stored_idempotency_key)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    {
        if existing_hash != request_hash {
            return Err(ApiError::conflict(
                "idempotency_conflict",
                "幂等键已用于不同部署请求",
                request_id.as_str(),
            ));
        }
        return Ok((
            StatusCode::OK,
            Json(find(state.pool(), &existing_id, request_id.as_str()).await?),
        ));
    }
    let deployment_id = format!("deployment_{}", Ulid::new());
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let insert = sqlx::query("INSERT INTO deployments (id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES (?, ?, ?, 'queued', 'queued', ?, ?, ?, ?)")
        .bind(&deployment_id).bind(&id).bind(&actor.id).bind(&stored_idempotency_key).bind(&request_hash).bind(&payload.snapshot_hash).bind(preview.snapshot.to_string())
        .execute(&mut *transaction).await;
    if let Err(error) = insert {
        if error.to_string().contains("UNIQUE constraint failed") {
            drop(transaction);
            let existing: Option<(String, String)> = sqlx::query_as("SELECT id,request_hash FROM deployments WHERE requested_by=? AND idempotency_key=?")
                .bind(&actor.id).bind(&stored_idempotency_key).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
            if let Some((existing_id, existing_hash)) = existing
                && existing_hash == request_hash
            {
                return Ok((
                    StatusCode::OK,
                    Json(find(state.pool(), &existing_id, request_id.as_str()).await?),
                ));
            }
            return Err(ApiError::conflict(
                "idempotency_conflict",
                "幂等键已经被并发请求使用",
                request_id.as_str(),
            ));
        }
        return Err(ApiError::internal(request_id.as_str()));
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment.create",
        "deployment",
        &deployment_id,
        request_id.as_str(),
        json!({"target_id":id,"snapshot_hash":payload.snapshot_hash}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(find(state.pool(), &deployment_id, request_id.as_str()).await?),
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
            sqlx::query_as::<_, DeploymentResponse>(DEPLOYMENT_SELECT_ALL)
                .bind(fetch_limit)
                .fetch_all(state.pool())
                .await
        }
        ("administrator", Some((created_at, id))) => {
            sqlx::query_as::<_, DeploymentResponse>(DEPLOYMENT_SELECT_ALL_AFTER)
                .bind(created_at)
                .bind(created_at)
                .bind(id)
                .bind(fetch_limit)
                .fetch_all(state.pool())
                .await
        }
        (_, None) => {
            sqlx::query_as::<_, DeploymentResponse>(DEPLOYMENT_SELECT_GRANTED)
                .bind(&actor.id)
                .bind(fetch_limit)
                .fetch_all(state.pool())
                .await
        }
        (_, Some((created_at, id))) => {
            sqlx::query_as::<_, DeploymentResponse>(DEPLOYMENT_SELECT_GRANTED_AFTER)
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
    Ok(Json(DeploymentListResponse {
        items: rows,
        next_cursor,
    }))
}

#[utoipa::path(operation_id = "deployments_show", get, path = "/api/v1/deployments/{id}", params(("id" = String, Path)), responses((status = 200, body = DeploymentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<DeploymentResponse>> {
    let application_id: Option<String> = sqlx::query_scalar("SELECT t.application_id FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE d.id=?")
        .bind(&id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    let application_id = application_id.ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
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
    let application_id: String = sqlx::query_scalar("SELECT t.application_id FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE d.id=?")
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
            let session_active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users u JOIN sessions s ON s.user_id=u.id WHERE u.id=? AND u.status='active' AND s.id=? AND s.revoked_at IS NULL AND s.expires_at>strftime('%Y-%m-%dT%H:%M:%fZ','now'))")
                .bind(&actor_id).bind(&session_id).fetch_one(&pool).await.unwrap_or(false);
            let granted = if administrator { true } else {
                sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM user_application_grants WHERE user_id=? AND application_id=?)")
                    .bind(&actor_id).bind(&application_id).fetch_one(&pool).await.unwrap_or(false)
            };
            if !session_active || !granted {
                yield Ok(Event::default().event("authorization-revoked").data("日志访问授权已经失效"));
                break;
            }
            let rows = match sqlx::query_as::<_, DeploymentLogResponse>("SELECT sequence,stream,content,truncated,created_at FROM deployment_logs WHERE deployment_id=? AND sequence>? ORDER BY sequence LIMIT 200")
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
    let now = chrono::Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let status: String = sqlx::query_scalar("SELECT status FROM deployments WHERE id=?")
        .bind(&id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let active_agent_task: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_tasks WHERE deployment_id=? AND status IN ('delivered','accepted','running','canceling'))")
        .bind(&id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    match status.as_str() {
        "queued" if !active_agent_task => {
            sqlx::query("UPDATE deployments SET status='canceled',phase='canceled',cancel_requested_at=?,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND status='queued'")
                .bind(&now).bind(&now).bind(&now).bind(&id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
            sqlx::query("UPDATE agent_tasks SET status='canceled',finished_at=?,result_json=?,updated_at=? WHERE deployment_id=? AND status='queued'")
                .bind(&now).bind(json!({"error_code":"canceled_before_delivery"}).to_string()).bind(&now).bind(&id)
                .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
        }
        "queued" | "running" => {
            sqlx::query("UPDATE deployments SET status='canceling',phase='canceling',cancel_requested_at=?,updated_at=?,version=version+1 WHERE id=? AND status IN ('queued','running')")
                .bind(&now).bind(&now).bind(&id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
        }
        "canceling" => {}
        _ => {
            return Err(ApiError::conflict(
                "deployment_not_cancelable",
                "部署当前不可取消",
                request_id.as_str(),
            ));
        }
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "deployment.cancel",
        "deployment",
        &id,
        request_id.as_str(),
        json!({}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if status == "running" || active_agent_task {
        runtime::cancel_remote(&state, &id).await?;
    }
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
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
    let original: (String, String, String) = sqlx::query_as("SELECT target_id,snapshot_json,snapshot_hash FROM deployments WHERE id=? AND status IN ('failed','canceled','interrupted')")
        .bind(&id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::conflict("deployment_not_retryable", "部署当前不可重试", request_id.as_str()))?;
    let original_snapshot: Value =
        serde_json::from_str(&original.1).map_err(|_| ApiError::internal(request_id.as_str()))?;
    let preview = if original_snapshot
        .get("execution_mode")
        .and_then(Value::as_str)
        == Some("two_stage")
    {
        preview_from_snapshot(&original_snapshot, &original.2)?
    } else {
        let parameters = original_snapshot
            .get("parameters")
            .ok_or_else(|| ApiError::internal(request_id.as_str()))?;
        build_preview(&state, &actor, &original.0, parameters, request_id.as_str()).await?
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
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,retry_of_id,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES(?,?,?,?,'queued','queued',?,?,?,?)")
        .bind(&new_id).bind(&original.0).bind(&actor.id).bind(&id).bind(&stored_key).bind(&request_hash).bind(&preview.response.snapshot_hash).bind(preview.snapshot.to_string())
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

async fn require_access(
    state: &AppState,
    actor: &AuthUser,
    id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let application_id: Option<String> = sqlx::query_scalar("SELECT t.application_id FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE d.id=?")
        .bind(id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id))?;
    let application_id = application_id.ok_or_else(|| ApiError::not_found(request_id))?;
    grants::require_application_access(state.pool(), actor, &application_id, request_id).await
}

async fn build_preview(
    state: &AppState,
    actor: &AuthUser,
    target_id: &str,
    parameters: &Value,
    request_id: &str,
) -> ApiResult<PreviewData> {
    let row: TargetExecutionRow = sqlx::query_as("SELECT t.id AS target_id,t.application_id,a.name AS application_name,a.status AS application_status,t.node_id,n.name AS node_name,n.status AS node_status,agent.id AS agent_id,n.work_root,n.secrets_root,t.environment,t.execution_mode,t.script_path,t.parameter_schema,t.timeout_seconds,t.verification_config,t.status AS target_status,t.version AS target_version FROM deployment_targets t JOIN applications a ON a.id=t.application_id JOIN nodes n ON n.id=t.node_id LEFT JOIN agents agent ON agent.node_id=n.id AND agent.revoked_at IS NULL AND agent.archived_at IS NULL WHERE t.id=?")
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
    if row.node_status != "online"
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
    execution_spec::validate_parameter_values(&schema, parameters, request_id)?;
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
        version: row.target_version,
    });
    if row.execution_mode == "two_stage" {
        return build_two_stage_preview(state.pool(), row, target_snapshot, parameters, request_id)
            .await;
    }
    let snapshot_hash = execution_spec::snapshot_hash(&target_snapshot);
    let response = DeploymentPreviewResponse {
        target_id: row.target_id,
        application_id: row.application_id,
        application_name: row.application_name,
        node_id: row.node_id,
        node_name: row.node_name,
        environment: row.environment,
        execution_mode: "script".to_owned(),
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

async fn build_two_stage_preview(
    pool: &sqlx::SqlitePool,
    row: TargetExecutionRow,
    target_snapshot: Value,
    parameters: &Value,
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

fn preview_from_snapshot(snapshot: &Value, snapshot_hash: &str) -> ApiResult<PreviewData> {
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

fn validate_idempotency_key(headers: &HeaderMap, request_id: &str) -> ApiResult<String> {
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
async fn find(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<DeploymentResponse> {
    sqlx::query_as(DEPLOYMENT_SELECT_ONE)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))
}

const DEPLOYMENT_SELECT_ONE: &str = "SELECT d.id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version FROM deployments d WHERE d.id=?";
const DEPLOYMENT_SELECT_ALL: &str = "SELECT d.id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version FROM deployments d ORDER BY d.created_at DESC,d.id DESC LIMIT ?";
const DEPLOYMENT_SELECT_ALL_AFTER: &str = "SELECT d.id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version FROM deployments d WHERE d.created_at<? OR (d.created_at=? AND d.id<?) ORDER BY d.created_at DESC,d.id DESC LIMIT ?";
const DEPLOYMENT_SELECT_GRANTED: &str = "SELECT d.id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version FROM deployments d JOIN deployment_targets t ON t.id=d.target_id JOIN user_application_grants g ON g.application_id=t.application_id WHERE g.user_id=? ORDER BY d.created_at DESC,d.id DESC LIMIT ?";
const DEPLOYMENT_SELECT_GRANTED_AFTER: &str = "SELECT d.id,d.target_id,d.requested_by,d.retry_of_id,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.protocol_complete,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at,d.version FROM deployments d JOIN deployment_targets t ON t.id=d.target_id JOIN user_application_grants g ON g.application_id=t.application_id WHERE g.user_id=? AND (d.created_at<? OR (d.created_at=? AND d.id<?)) ORDER BY d.created_at DESC,d.id DESC LIMIT ?";
