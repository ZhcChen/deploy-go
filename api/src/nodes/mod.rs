use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    pagination,
};

#[derive(Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct NodeResponse {
    pub id: String,
    pub name: String,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub ssh_credential_id: Option<String>,
    pub work_root: Option<String>,
    pub secrets_root: Option<String>,
    pub status: String,
    pub trusted_host_fingerprint: Option<String>,
    pub checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct NodeListResponse {
    items: Vec<NodeResponse>,
    next_cursor: Option<String>,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct NodeCheckResponse {
    id: String,
    status: String,
    failure_code: Option<String>,
    failure_message: Option<String>,
    os_name: Option<String>,
    architecture: Option<String>,
    disk_available_bytes: Option<i64>,
    created_at: String,
    finished_at: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/nodes", get(list))
        .route("/nodes/{id}", get(show))
        .route("/nodes/{id}/checks", post(run_check))
}

#[utoipa::path(operation_id = "nodes_list", get, path = "/api/v1/nodes", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = NodeListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<pagination::ListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<NodeListResponse>> {
    let limit = pagination::limit(&query, request_id.as_str())?;
    let (created_at, id) = pagination::decode_after(&query, request_id.as_str())?
        .unwrap_or_else(|| ("0000".to_owned(), "".to_owned()));
    let nodes = if actor.identity == "administrator" {
        sqlx::query_as::<_, NodeResponse>("SELECT id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status, trusted_host_fingerprint, checked_at, created_at, updated_at, version FROM nodes WHERE (created_at>? OR (created_at=? AND id>?)) ORDER BY created_at, id LIMIT ?")
            .bind(&created_at).bind(&created_at).bind(&id).bind((limit + 1) as i64).fetch_all(state.pool()).await
    } else {
        sqlx::query_as::<_, NodeResponse>("SELECT DISTINCT n.id, n.name, n.host, n.port, n.username, n.ssh_credential_id, n.work_root, n.secrets_root, n.status, n.trusted_host_fingerprint, n.checked_at, n.created_at, n.updated_at, n.version FROM nodes n JOIN deployment_targets t ON t.node_id=n.id JOIN user_application_grants g ON g.application_id=t.application_id WHERE g.user_id=? AND (n.created_at>? OR (n.created_at=? AND n.id>?)) ORDER BY n.created_at, n.id LIMIT ?")
            .bind(&actor.id).bind(&created_at).bind(&created_at).bind(&id).bind((limit + 1) as i64).fetch_all(state.pool()).await
    }.map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (items, next_cursor) =
        pagination::finish(nodes, limit, |item| (&item.created_at, &item.id));
    Ok(Json(NodeListResponse { items, next_cursor }))
}

#[utoipa::path(operation_id = "nodes_show", get, path = "/api/v1/nodes/{id}", params(("id" = String, Path)), responses((status = 200, body = NodeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<NodeResponse>> {
    if actor.identity != "administrator" {
        let visible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM deployment_targets t JOIN user_application_grants g ON g.application_id=t.application_id WHERE t.node_id=? AND g.user_id=?)")
            .bind(&id).bind(&actor.id).fetch_one(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
        if !visible {
            return Err(ApiError::not_found(request_id.as_str()));
        }
    }
    Ok(Json(
        find_node(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "nodes_run_check", post, path = "/api/v1/nodes/{id}/checks", params(("id" = String, Path)), responses((status = 201, body = NodeCheckResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn run_check(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<(StatusCode, Json<NodeCheckResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM nodes WHERE id=?")
        .bind(&id)
        .fetch_optional(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let status = status.ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    if status != "online" {
        return Err(ApiError::conflict(
            "agent_not_available",
            "节点 Agent 当前离线",
            request_id.as_str(),
        ));
    }
    let check_id = format!("check_{}", Ulid::new());
    let started_at = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM node_checks WHERE node_id=? AND status='running')",
    )
    .bind(&id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if existing {
        return Err(ApiError::conflict(
            "node_check_in_progress",
            "节点检查正在进行",
            request_id.as_str(),
        ));
    }
    sqlx::query(
        "INSERT INTO node_checks (id, node_id, status, started_at) VALUES (?, ?, 'running', ?)",
    )
    .bind(&check_id)
    .bind(&id)
    .bind(&started_at)
    .execute(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if let Err(error) =
        crate::agents::dispatcher::enqueue_node_inspect(&state, &id, &check_id).await
    {
        sqlx::query("UPDATE node_checks SET status='failed',failure_code='agent_not_available',failure_message='节点 Agent 当前不可检查',finished_at=? WHERE id=?")
            .bind(Utc::now().to_rfc3339()).bind(&check_id).execute(state.pool()).await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        return Err(error);
    }
    let mut audit_transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut audit_transaction,
        Some(&actor.id),
        "node.check",
        "node",
        &id,
        request_id.as_str(),
        json!({"check_id":check_id,"status":"running"}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit_transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(find_check(state.pool(), &check_id, request_id.as_str()).await?),
    ))
}

async fn find_node(pool: &sqlx::SqlitePool, id: &str, request_id: &str) -> ApiResult<NodeResponse> {
    sqlx::query_as("SELECT id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status, trusted_host_fingerprint, checked_at, created_at, updated_at, version FROM nodes WHERE id=?")
        .bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}

async fn find_check(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<NodeCheckResponse> {
    sqlx::query_as("SELECT id, status, failure_code, failure_message, os_name, architecture, disk_available_bytes, created_at, finished_at FROM node_checks WHERE id=?").bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}
