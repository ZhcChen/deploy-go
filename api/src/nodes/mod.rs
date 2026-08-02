use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    crypto::EncryptedSecret,
    error::{ApiError, ApiResult},
    executor::ssh::{CapabilityReport, NodeProbeInput, ScannedHostKey, validate_connection},
};

#[derive(Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct NodeResponse {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub ssh_credential_id: Option<String>,
    pub work_root: String,
    pub secrets_root: String,
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

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveNodeRequest {
    name: String,
    host: String,
    port: u16,
    username: String,
    ssh_credential_id: Option<String>,
    work_root: String,
    secrets_root: String,
    version: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeStatusRequest {
    status: String,
    version: i64,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BindCredentialRequest {
    credential_id: String,
    version: i64,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionRequest {
    version: i64,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfirmHostKeyRequest {
    check_id: String,
    snapshot_hash: String,
    version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct HostKeyScanResponse {
    check_id: String,
    fingerprint: String,
    snapshot_hash: String,
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

#[derive(sqlx::FromRow)]
struct NodeRuntime {
    id: String,
    host: String,
    port: i64,
    username: String,
    work_root: String,
    status: String,
    ssh_credential_id: Option<String>,
    trusted_host_key: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/nodes", get(list).post(create))
        .route("/nodes/{id}", get(show).patch(update))
        .route("/nodes/{id}/status", put(update_status))
        .route(
            "/nodes/{id}/ssh-credential",
            put(bind_credential).delete(unbind_credential),
        )
        .route("/nodes/{id}/host-key/scan", post(scan_host_key))
        .route("/nodes/{id}/host-key/confirm", post(confirm_host_key))
        .route("/nodes/{id}/checks", post(run_check))
}

#[utoipa::path(operation_id = "nodes_list", get, path = "/api/v1/nodes", responses((status = 200, body = NodeListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<NodeListResponse>> {
    let nodes = if actor.identity == "administrator" {
        sqlx::query_as::<_, NodeResponse>("SELECT id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status, trusted_host_fingerprint, checked_at, created_at, updated_at, version FROM nodes ORDER BY created_at, id LIMIT 200")
            .fetch_all(state.pool()).await
    } else {
        sqlx::query_as::<_, NodeResponse>("SELECT DISTINCT n.id, n.name, n.host, n.port, n.username, n.ssh_credential_id, n.work_root, n.secrets_root, n.status, n.trusted_host_fingerprint, n.checked_at, n.created_at, n.updated_at, n.version FROM nodes n JOIN deployment_targets t ON t.node_id=n.id JOIN user_application_grants g ON g.application_id=t.application_id WHERE g.user_id=? ORDER BY n.created_at, n.id LIMIT 200")
            .bind(&actor.id).fetch_all(state.pool()).await
    }.map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(NodeListResponse {
        items: nodes,
        next_cursor: None,
    }))
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

#[utoipa::path(operation_id = "nodes_create", post, path = "/api/v1/nodes", request_body = SaveNodeRequest, responses((status = 201, body = NodeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveNodeRequest>,
) -> ApiResult<(StatusCode, Json<NodeResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    validate_node(&payload, request_id.as_str())?;
    if let Some(credential_id) = &payload.ssh_credential_id {
        ensure_credential(state.pool(), credential_id, request_id.as_str()).await?;
    }
    let id = format!("node_{}", Ulid::new());
    let status = if payload.ssh_credential_id.is_some() {
        "unchecked"
    } else {
        "missing_credential"
    };
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO nodes (id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(payload.name.trim()).bind(&payload.host).bind(payload.port as i64)
        .bind(&payload.username).bind(&payload.ssh_credential_id).bind(&payload.work_root).bind(&payload.secrets_root).bind(status)
        .execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "node.create",
        "node",
        &id,
        request_id.as_str(),
        json!({"name":payload.name.trim(),"host":payload.host,"port":payload.port,"credential_id":payload.ssh_credential_id}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(find_node(state.pool(), &id, request_id.as_str()).await?),
    ))
}

#[utoipa::path(operation_id = "nodes_update", patch, path = "/api/v1/nodes/{id}", params(("id" = String, Path)), request_body = SaveNodeRequest, responses((status = 200, body = NodeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveNodeRequest>,
) -> ApiResult<Json<NodeResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    validate_node(&payload, request_id.as_str())?;
    let version = payload
        .version
        .ok_or_else(|| ApiError::validation("编辑节点必须提供 version", request_id.as_str()))?;
    let current = find_node(state.pool(), &id, request_id.as_str()).await?;
    if let Some(credential_id) = &payload.ssh_credential_id {
        ensure_credential(state.pool(), credential_id, request_id.as_str()).await?;
    }
    let connection_changed = current.host != payload.host
        || current.port != payload.port as i64
        || current.username != payload.username
        || current.work_root != payload.work_root
        || current.ssh_credential_id != payload.ssh_credential_id;
    let status = if current.status == "disabled" {
        "disabled"
    } else if payload.ssh_credential_id.is_none() {
        "missing_credential"
    } else if connection_changed {
        "unchecked"
    } else {
        current.status.as_str()
    };
    let clear_trust = current.host != payload.host || current.port != payload.port as i64;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE nodes SET name=?, host=?, port=?, username=?, ssh_credential_id=?, work_root=?, secrets_root=?, status=?, trusted_host_key=CASE WHEN ? THEN NULL ELSE trusted_host_key END, trusted_host_fingerprint=CASE WHEN ? THEN NULL ELSE trusted_host_fingerprint END, checked_at=CASE WHEN ? THEN NULL ELSE checked_at END, updated_at=?, version=version+1 WHERE id=? AND version=? AND status != 'checking'")
        .bind(payload.name.trim()).bind(&payload.host).bind(payload.port as i64).bind(&payload.username).bind(&payload.ssh_credential_id)
        .bind(&payload.work_root).bind(&payload.secrets_root).bind(status).bind(clear_trust).bind(clear_trust).bind(connection_changed)
        .bind(Utc::now().to_rfc3339()).bind(&id).bind(version).execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    require_updated(result.rows_affected(), request_id.as_str())?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "node.update",
        "node",
        &id,
        request_id.as_str(),
        json!({"name":payload.name.trim()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(
        find_node(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "nodes_update_status", put, path = "/api/v1/nodes/{id}/status", params(("id" = String, Path)), request_body = NodeStatusRequest, responses((status = 200, body = NodeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<NodeStatusRequest>,
) -> ApiResult<Json<NodeResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    if !matches!(payload.status.as_str(), "disabled" | "unchecked") {
        return Err(ApiError::validation(
            "节点状态操作无效",
            request_id.as_str(),
        ));
    }
    let node = find_node(state.pool(), &id, request_id.as_str()).await?;
    if payload.status == "unchecked" && node.ssh_credential_id.is_none() {
        return Err(ApiError::conflict(
            "credential_required",
            "节点缺少 SSH 密钥",
            request_id.as_str(),
        ));
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE nodes SET status=?, checked_at=NULL, updated_at=?, version=version+1 WHERE id=? AND version=? AND status != 'checking'")
        .bind(&payload.status).bind(Utc::now().to_rfc3339()).bind(&id).bind(payload.version).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    require_updated(result.rows_affected(), request_id.as_str())?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "node.status.update",
        "node",
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
        find_node(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "nodes_bind_credential", put, path = "/api/v1/nodes/{id}/ssh-credential", params(("id" = String, Path)), request_body = BindCredentialRequest, responses((status = 200, body = NodeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn bind_credential(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<BindCredentialRequest>,
) -> ApiResult<Json<NodeResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    find_node(state.pool(), &id, request_id.as_str()).await?;
    ensure_credential(state.pool(), &payload.credential_id, request_id.as_str()).await?;
    mutate_credential(
        &state,
        &actor,
        &id,
        Some(&payload.credential_id),
        payload.version,
        request_id.as_str(),
    )
    .await?;
    Ok(Json(
        find_node(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "nodes_unbind_credential", delete, path = "/api/v1/nodes/{id}/ssh-credential", params(("id" = String, Path)), request_body = VersionRequest, responses((status = 200, body = NodeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn unbind_credential(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<VersionRequest>,
) -> ApiResult<Json<NodeResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    find_node(state.pool(), &id, request_id.as_str()).await?;
    mutate_credential(
        &state,
        &actor,
        &id,
        None,
        payload.version,
        request_id.as_str(),
    )
    .await?;
    Ok(Json(
        find_node(state.pool(), &id, request_id.as_str()).await?,
    ))
}

async fn mutate_credential(
    state: &AppState,
    actor: &AuthUser,
    id: &str,
    credential_id: Option<&str>,
    version: i64,
    request_id: &str,
) -> ApiResult<()> {
    let current_status: String = sqlx::query_scalar("SELECT status FROM nodes WHERE id=?")
        .bind(id)
        .fetch_one(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let status = if current_status == "disabled" && credential_id.is_some() {
        "disabled"
    } else if credential_id.is_some() {
        "unchecked"
    } else {
        "missing_credential"
    };
    let action = if credential_id.is_some() {
        "node.credential.bind"
    } else {
        "node.credential.unbind"
    };
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let result = sqlx::query("UPDATE nodes SET ssh_credential_id=?, status=?, checked_at=NULL, updated_at=?, version=version+1 WHERE id=? AND version=? AND status != 'checking'")
        .bind(credential_id).bind(status).bind(Utc::now().to_rfc3339()).bind(id).bind(version).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
    require_updated(result.rows_affected(), request_id)?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        action,
        "node",
        id,
        request_id,
        json!({"credential_id":credential_id}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(())
}

#[utoipa::path(operation_id = "nodes_scan_host_key", post, path = "/api/v1/nodes/{id}/host-key/scan", params(("id" = String, Path)), responses((status = 201, body = HostKeyScanResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 502, body = crate::error::ErrorResponse)))]
pub(crate) async fn scan_host_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<(StatusCode, Json<HostKeyScanResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let node = runtime_node(state.pool(), &id, request_id.as_str()).await?;
    if node.status == "disabled" {
        return Err(ApiError::conflict(
            "node_disabled",
            "节点已停用",
            request_id.as_str(),
        ));
    }
    let input = probe_input(&node)?;
    let scanned = state
        .node_probe()
        .scan_host_key(&input)
        .await
        .map_err(|error| {
            ApiError::new(
                StatusCode::BAD_GATEWAY,
                error.code,
                error.message,
                request_id.as_str(),
            )
        })?;
    let snapshot_hash = host_snapshot(&input, &scanned);
    let check_id = format!("check_{}", Ulid::new());
    sqlx::query("INSERT INTO node_checks (id, node_id, status, capabilities_json, host_fingerprint) VALUES (?, ?, 'pending', ?, ?)")
        .bind(&check_id).bind(&id).bind(json!({"host_key":scanned.host_key,"snapshot_hash":snapshot_hash}).to_string()).bind(&scanned.fingerprint)
        .execute(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(HostKeyScanResponse {
            check_id,
            fingerprint: scanned.fingerprint,
            snapshot_hash,
        }),
    ))
}

#[utoipa::path(operation_id = "nodes_confirm_host_key", post, path = "/api/v1/nodes/{id}/host-key/confirm", params(("id" = String, Path)), request_body = ConfirmHostKeyRequest, responses((status = 200, body = NodeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn confirm_host_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<ConfirmHostKeyRequest>,
) -> ApiResult<Json<NodeResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let pending: Option<(String, String)> = sqlx::query_as("SELECT capabilities_json, host_fingerprint FROM node_checks WHERE id=? AND node_id=? AND status='pending'")
        .bind(&payload.check_id).bind(&id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (capabilities, fingerprint) =
        pending.ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let capabilities: Value =
        serde_json::from_str(&capabilities).map_err(|_| ApiError::internal(request_id.as_str()))?;
    let expected = capabilities["snapshot_hash"]
        .as_str()
        .ok_or_else(|| ApiError::internal(request_id.as_str()))?;
    if expected != payload.snapshot_hash {
        return Err(ApiError::conflict(
            "host_key_snapshot_changed",
            "host key 确认摘要不匹配",
            request_id.as_str(),
        ));
    }
    let host_key = capabilities["host_key"]
        .as_str()
        .ok_or_else(|| ApiError::internal(request_id.as_str()))?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE nodes SET trusted_host_key=?, trusted_host_fingerprint=?, status=CASE WHEN status='disabled' THEN 'disabled' ELSE 'unchecked' END, checked_at=NULL, updated_at=?, version=version+1 WHERE id=? AND version=? AND ssh_credential_id IS NOT NULL AND status != 'checking'")
        .bind(host_key).bind(&fingerprint).bind(Utc::now().to_rfc3339()).bind(&id).bind(payload.version).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    require_updated(result.rows_affected(), request_id.as_str())?;
    sqlx::query("UPDATE node_checks SET status='succeeded', finished_at=? WHERE id=?")
        .bind(Utc::now().to_rfc3339())
        .bind(&payload.check_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "node.host_key.confirm",
        "node",
        &id,
        request_id.as_str(),
        json!({"fingerprint":fingerprint}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
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
    let node = runtime_node(state.pool(), &id, request_id.as_str()).await?;
    if node.status == "disabled" {
        return Err(ApiError::conflict(
            "node_disabled",
            "节点已停用",
            request_id.as_str(),
        ));
    }
    let credential_id = node.ssh_credential_id.as_deref().ok_or_else(|| {
        ApiError::conflict(
            "credential_required",
            "节点缺少 SSH 密钥",
            request_id.as_str(),
        )
    })?;
    let trusted = node.trusted_host_key.as_deref().ok_or_else(|| {
        ApiError::conflict(
            "host_key_confirmation_required",
            "必须先确认节点 host key",
            request_id.as_str(),
        )
    })?;
    let credential: (Vec<u8>, Vec<u8>, i64, String) = sqlx::query_as("SELECT encrypted_private_key, nonce, key_version, algorithm FROM ssh_credentials WHERE id=?")
        .bind(credential_id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?.ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::internal(request_id.as_str()))?;
    let private_key = ring
        .decrypt(
            credential_id,
            &credential.3,
            &EncryptedSecret {
                ciphertext: credential.0,
                nonce: credential.1,
                key_version: credential.2,
            },
        )
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let check_id = format!("check_{}", Ulid::new());
    let started_at = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let claimed = sqlx::query("UPDATE nodes SET status='checking', updated_at=? WHERE id=? AND status NOT IN ('checking','disabled','missing_credential')")
        .bind(&started_at).bind(&id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    if claimed.rows_affected() == 0 {
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
    let result = state
        .node_probe()
        .check(&probe_input(&node)?, private_key.as_slice(), trusted)
        .await;
    finish_check(&state, &actor, &id, &check_id, result, request_id.as_str()).await?;
    Ok((
        StatusCode::CREATED,
        Json(find_check(state.pool(), &check_id, request_id.as_str()).await?),
    ))
}

async fn finish_check(
    state: &AppState,
    actor: &AuthUser,
    node_id: &str,
    check_id: &str,
    result: Result<CapabilityReport, crate::executor::ssh::ProbeError>,
    request_id: &str,
) -> ApiResult<()> {
    let finished = Utc::now().to_rfc3339();
    let audit_summary = match &result {
        Ok(_) => json!({"check_id":check_id,"status":"succeeded"}),
        Err(error) => json!({"check_id":check_id,"status":"failed","failure_code":error.code}),
    };
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    match result {
        Ok(report) => {
            let disk_available_bytes =
                i64::try_from(report.disk_available_bytes).unwrap_or(i64::MAX);
            sqlx::query("UPDATE node_checks SET status='succeeded', os_name=?, architecture=?, disk_available_bytes=?, capabilities_json=?, finished_at=? WHERE id=?")
                .bind(&report.os_name).bind(&report.architecture).bind(disk_available_bytes).bind(json!({}).to_string()).bind(&finished).bind(check_id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
            sqlx::query("UPDATE nodes SET status='online', checked_at=?, updated_at=?, version=version+1 WHERE id=? AND status='checking'").bind(&finished).bind(&finished).bind(node_id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
        }
        Err(error) => {
            sqlx::query("UPDATE node_checks SET status='failed', failure_code=?, failure_message=?, finished_at=? WHERE id=?").bind(error.code).bind(error.message).bind(&finished).bind(check_id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
            sqlx::query("UPDATE nodes SET status='offline', checked_at=?, updated_at=?, version=version+1 WHERE id=? AND status='checking'").bind(&finished).bind(&finished).bind(node_id).execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id))?;
        }
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "node.check",
        "node",
        node_id,
        request_id,
        audit_summary,
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(())
}

fn validate_node(payload: &SaveNodeRequest, request_id: &str) -> ApiResult<()> {
    let input = NodeProbeInput {
        id: String::new(),
        host: payload.host.clone(),
        port: payload.port,
        username: payload.username.clone(),
        work_root: payload.work_root.clone(),
    };
    if payload.name.trim().is_empty()
        || payload.name.chars().count() > 64
        || payload.name.chars().any(char::is_control)
        || payload.secrets_root.is_empty()
        || !payload.secrets_root.starts_with('/')
        || payload.secrets_root.chars().any(char::is_control)
        || validate_connection(&input).is_err()
    {
        return Err(ApiError::validation("节点配置格式不正确", request_id));
    }
    Ok(())
}

fn probe_input(node: &NodeRuntime) -> ApiResult<NodeProbeInput> {
    Ok(NodeProbeInput {
        id: node.id.clone(),
        host: node.host.clone(),
        port: u16::try_from(node.port).map_err(|_| ApiError::internal("req_unknown"))?,
        username: node.username.clone(),
        work_root: node.work_root.clone(),
    })
}

fn host_snapshot(node: &NodeProbeInput, scanned: &ScannedHostKey) -> String {
    let value = json!({"node_id":node.id,"host":node.host,"port":node.port,"host_key":scanned.host_key,"fingerprint":scanned.fingerprint});
    URL_SAFE_NO_PAD.encode(Sha256::digest(value.to_string().as_bytes()))
}

async fn find_node(pool: &sqlx::SqlitePool, id: &str, request_id: &str) -> ApiResult<NodeResponse> {
    sqlx::query_as("SELECT id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status, trusted_host_fingerprint, checked_at, created_at, updated_at, version FROM nodes WHERE id=?")
        .bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}
async fn runtime_node(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<NodeRuntime> {
    sqlx::query_as("SELECT id, host, port, username, work_root, status, ssh_credential_id, trusted_host_key FROM nodes WHERE id=?").bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}
async fn find_check(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<NodeCheckResponse> {
    sqlx::query_as("SELECT id, status, failure_code, failure_message, os_name, architecture, disk_available_bytes, created_at, finished_at FROM node_checks WHERE id=?").bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}
async fn ensure_credential(pool: &sqlx::SqlitePool, id: &str, request_id: &str) -> ApiResult<()> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ssh_credentials WHERE id=?)")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    if exists {
        Ok(())
    } else {
        Err(ApiError::not_found(request_id))
    }
}
fn require_updated(rows: u64, request_id: &str) -> ApiResult<()> {
    if rows == 0 {
        Err(ApiError::conflict(
            "resource_version_conflict",
            "节点已经被其他请求修改",
            request_id,
        ))
    } else {
        Ok(())
    }
}
fn map_unique(error: sqlx::Error, request_id: &str) -> ApiError {
    if error.to_string().contains("UNIQUE constraint failed") {
        ApiError::conflict("node_name_exists", "节点名称已经存在", request_id)
    } else {
        ApiError::internal(request_id)
    }
}
