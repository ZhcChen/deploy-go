use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ulid::Ulid;
use utoipa::{IntoParams, ToSchema};

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
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeListQuery {
    pub limit: Option<u32>,
    pub after: Option<String>,
    pub archived: Option<bool>,
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
        .route("/nodes/{id}/telemetry", get(telemetry))
        .route("/nodes/{id}/checks", post(run_check))
        .route("/nodes/{id}/archive", post(archive))
        .route("/nodes/{id}/unarchive", post(unarchive))
}

#[utoipa::path(operation_id = "nodes_telemetry", get, path = "/api/v1/nodes/{id}/telemetry", params(("id" = String, Path)), responses((status = 200, body = crate::node_telemetry::TelemetryResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn telemetry(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<crate::node_telemetry::TelemetryResponse>> {
    ensure_visible(&state, &id, request_id.as_str(), &actor).await?;
    Ok(Json(
        crate::node_telemetry::query(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "nodes_list", get, path = "/api/v1/nodes", params(NodeListQuery), responses((status = 200, body = NodeListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<NodeListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<NodeListResponse>> {
    let limit = pagination::limit(
        &pagination::ListQuery {
            limit: query.limit,
            after: query.after.clone(),
        },
        request_id.as_str(),
    )?;
    let (created_at, id) = pagination::decode_after(
        &pagination::ListQuery {
            limit: query.limit,
            after: query.after,
        },
        request_id.as_str(),
    )?
    .unwrap_or_else(|| ("0000".to_owned(), "".to_owned()));
    let archived = query.archived.unwrap_or(false);
    let archive_clause = if archived {
        "archived_at IS NOT NULL"
    } else {
        "archived_at IS NULL"
    };
    let nodes = if actor.identity == "administrator" {
        sqlx::query_as::<_, NodeResponse>(&format!("SELECT id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status, trusted_host_fingerprint, checked_at, archived_at, created_at, updated_at, version FROM nodes WHERE {archive_clause} AND (created_at>? OR (created_at=? AND id>?)) ORDER BY created_at, id LIMIT ?"))
            .bind(&created_at).bind(&created_at).bind(&id).bind((limit + 1) as i64).fetch_all(state.pool()).await
    } else {
        sqlx::query_as::<_, NodeResponse>(&format!("SELECT DISTINCT n.id, n.name, n.host, n.port, n.username, n.ssh_credential_id, n.work_root, n.secrets_root, n.status, n.trusted_host_fingerprint, n.checked_at, n.archived_at, n.created_at, n.updated_at, n.version FROM nodes n JOIN deployment_targets t ON t.node_id=n.id JOIN user_application_grants g ON g.application_id=t.application_id WHERE g.user_id=? AND n.{archive_clause} AND (n.created_at>? OR (n.created_at=? AND n.id>?)) ORDER BY n.created_at, n.id LIMIT ?"))
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
    ensure_visible(&state, &id, request_id.as_str(), &actor).await?;
    Ok(Json(
        find_node(state.pool(), &id, request_id.as_str()).await?,
    ))
}

async fn ensure_visible(
    state: &AppState,
    id: &str,
    request_id: &str,
    actor: &AuthUser,
) -> ApiResult<()> {
    if actor.identity != "administrator" {
        let visible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM deployment_targets t JOIN user_application_grants g ON g.application_id=t.application_id WHERE t.node_id=? AND g.user_id=?)")
            .bind(id).bind(&actor.id).fetch_one(state.pool()).await.map_err(|_| ApiError::internal(request_id))?;
        if !visible {
            return Err(ApiError::not_found(request_id));
        }
    }
    Ok(())
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
    let (status, archived_at): (String, Option<String>) =
        sqlx::query_as("SELECT status, archived_at FROM nodes WHERE id=?")
            .bind(&id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?
            .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    if status != "online" {
        return Err(ApiError::conflict(
            "agent_not_available",
            "节点 Agent 当前离线",
            request_id.as_str(),
        ));
    }
    if archived_at.is_some() {
        return Err(ApiError::conflict(
            "node_archived",
            "节点已归档",
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
    sqlx::query_as("SELECT id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status, trusted_host_fingerprint, checked_at, archived_at, created_at, updated_at, version FROM nodes WHERE id=?")
        .bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}

#[utoipa::path(operation_id = "nodes_archive", post, path = "/api/v1/nodes/{id}/archive", params(("id" = String, Path)), responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn archive(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<StatusCode> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let current: Option<Option<String>> =
        sqlx::query_scalar("SELECT archived_at FROM nodes WHERE id=?")
            .bind(&id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some(current) = current else {
        return Err(ApiError::not_found(request_id.as_str()));
    };
    if current.is_some() {
        return Err(ApiError::conflict(
            "node_already_archived",
            "节点已归档",
            request_id.as_str(),
        ));
    }
    let running: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM deployments d JOIN deployment_targets t ON t.id=d.target_id WHERE t.node_id=? AND d.status IN ('queued','running','canceling'))",
    )
    .bind(&id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if running {
        return Err(ApiError::conflict(
            "node_has_active_deployments",
            "节点存在进行中的部署，无法归档",
            request_id.as_str(),
        ));
    }
    sqlx::query("UPDATE nodes SET archived_at=?,updated_at=?,version=version+1 WHERE id=?")
        .bind(&now)
        .bind(&now)
        .bind(&id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "node.archive",
        "node",
        &id,
        request_id.as_str(),
        json!({"archived_at": now}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(operation_id = "nodes_unarchive", post, path = "/api/v1/nodes/{id}/unarchive", params(("id" = String, Path)), responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn unarchive(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<StatusCode> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let current: Option<Option<String>> =
        sqlx::query_scalar("SELECT archived_at FROM nodes WHERE id=?")
            .bind(&id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some(current) = current else {
        return Err(ApiError::not_found(request_id.as_str()));
    };
    if current.is_none() {
        return Err(ApiError::conflict(
            "node_not_archived",
            "节点未归档",
            request_id.as_str(),
        ));
    }
    sqlx::query("UPDATE nodes SET archived_at=NULL,updated_at=?,version=version+1 WHERE id=?")
        .bind(&now)
        .bind(&id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "node.unarchive",
        "node",
        &id,
        request_id.as_str(),
        json!({"archived_at": null}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn find_check(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<NodeCheckResponse> {
    sqlx::query_as("SELECT id, status, failure_code, failure_message, os_name, architecture, disk_available_bytes, created_at, finished_at FROM node_checks WHERE id=?").bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}
