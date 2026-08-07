pub mod registry;
pub mod store;
pub mod websocket;

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
};
use deploy_go_agent_protocol::AgentCapability;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};

#[derive(sqlx::FromRow)]
struct CapabilityFacts {
    node_id: String,
    node_status: String,
    privileged_execution: bool,
    agent_id: Option<String>,
    protocol_version: Option<i64>,
    capabilities_json: Option<String>,
    revoked_at: Option<String>,
    archived_at: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct TerminalCapabilityResponse {
    node_id: String,
    privileged_execution: bool,
    available: bool,
    unavailable_code: Option<String>,
    agent_id: Option<String>,
    agent_online: bool,
    identity_valid: bool,
    protocol_version: Option<i64>,
    pty_terminal: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdatePrivilegedExecutionRequest {
    enabled: bool,
}

#[derive(Serialize, ToSchema)]
pub struct PrivilegedExecutionResponse {
    node_id: String,
    enabled: bool,
}

#[derive(Serialize, ToSchema)]
pub struct TerminalSessionResponse {
    id: String,
    node_id: String,
    agent_id: String,
    actor_id: String,
    status: String,
    started_at: String,
    opened_at: Option<String>,
    close_requested_at: Option<String>,
    finished_at: Option<String>,
    exit_reason: Option<String>,
    exit_code: Option<i64>,
    input_bytes: i64,
    output_bytes: i64,
}

impl From<store::TerminalSessionRecord> for TerminalSessionResponse {
    fn from(value: store::TerminalSessionRecord) -> Self {
        Self {
            id: value.id,
            node_id: value.node_id,
            agent_id: value.agent_id,
            actor_id: value.actor_id,
            status: value.status,
            started_at: value.started_at,
            opened_at: value.opened_at,
            close_requested_at: value.close_requested_at,
            finished_at: value.finished_at,
            exit_reason: value.exit_reason,
            exit_code: value.exit_code,
            input_bytes: value.input_bytes,
            output_bytes: value.output_bytes,
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/nodes/{node_id}/terminal-capability", get(capability))
        .route(
            "/nodes/{node_id}/privileged-execution",
            put(update_privileged_execution),
        )
        .route("/nodes/{node_id}/terminal-sessions", post(create_session))
        .route("/terminal-sessions/{session_id}/close", post(close_session))
        .merge(websocket::router())
}

#[utoipa::path(operation_id = "terminals_capability", get, path = "/api/v1/nodes/{node_id}/terminal-capability", params(("node_id" = String, Path)), responses((status = 200, body = TerminalCapabilityResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn capability(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<TerminalCapabilityResponse>> {
    actor.require_administrator(request_id.as_str())?;
    Ok(Json(
        capability_for_node(&state, &node_id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "terminals_update_privileged_execution", put, path = "/api/v1/nodes/{node_id}/privileged-execution", params(("node_id" = String, Path), ("X-CSRF-Token" = String, Header)), request_body = UpdatePrivilegedExecutionRequest, responses((status = 200, body = PrivilegedExecutionResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_privileged_execution(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<UpdatePrivilegedExecutionRequest>,
) -> ApiResult<Json<PrivilegedExecutionResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let found = if payload.enabled {
        store::set_privileged_execution(state.pool(), &node_id, true).await
    } else {
        store::disable_privileged_execution(state.pool(), &node_id, "privileged_execution_disabled")
            .await
    }
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if !found {
        return Err(ApiError::not_found(request_id.as_str()));
    }
    if !payload.enabled {
        state
            .terminal_connections()
            .authorization_revoked_for_node(&state, &node_id, "privileged_execution_disabled")
            .await;
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "node.privileged_execution.update",
        "node",
        &node_id,
        request_id.as_str(),
        json!({"enabled":payload.enabled}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(PrivilegedExecutionResponse {
        node_id,
        enabled: payload.enabled,
    }))
}

#[utoipa::path(operation_id = "terminals_create_session", post, path = "/api/v1/nodes/{node_id}/terminal-sessions", params(("node_id" = String, Path), ("X-CSRF-Token" = String, Header)), responses((status = 201, body = TerminalSessionResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn create_session(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<(StatusCode, Json<TerminalSessionResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let capability = capability_for_node(&state, &node_id, request_id.as_str()).await?;
    if let Some(code) = capability.unavailable_code.as_deref() {
        return Err(gate_error(code, request_id.as_str()));
    }
    let agent_id = capability
        .agent_id
        .ok_or_else(|| gate_error("terminal_agent_identity_invalid", request_id.as_str()))?;
    let session_id = format!("term_{}", Ulid::new());
    let session = match store::create_session(
        state.pool(),
        &session_id,
        &node_id,
        &agent_id,
        &actor.id,
        request_id.as_str(),
    )
    .await
    {
        Ok(session) => session,
        Err(store::CreateSessionError::ActiveSessionConflict) => {
            return Err(ApiError::conflict(
                "terminal_session_active",
                "节点已有活动终端会话",
                request_id.as_str(),
            ));
        }
        Err(store::CreateSessionError::GateRejected) => {
            let current = capability_for_node(&state, &node_id, request_id.as_str()).await?;
            return Err(gate_error(
                current
                    .unavailable_code
                    .as_deref()
                    .unwrap_or("terminal_unavailable"),
                request_id.as_str(),
            ));
        }
        Err(store::CreateSessionError::Database(_)) => {
            return Err(ApiError::internal(request_id.as_str()));
        }
    };
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "terminal.session.create",
        "terminal_session",
        &session_id,
        request_id.as_str(),
        json!({"node_id":node_id,"agent_id":agent_id,"status":"opening"}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((StatusCode::CREATED, Json(session.into())))
}

#[utoipa::path(operation_id = "terminals_close_session", post, path = "/api/v1/terminal-sessions/{session_id}/close", params(("session_id" = String, Path), ("X-CSRF-Token" = String, Header)), responses((status = 200, body = TerminalSessionResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn close_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Json<TerminalSessionResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let attached = state
        .terminal_connections()
        .request_administrator_close(&state, &session_id)
        .await?;
    let session = if attached {
        store::find_session(state.pool(), &session_id).await
    } else {
        store::close_session(state.pool(), &session_id, "administrator_closed").await
    }
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(&mut transaction, Some(&actor.id), "terminal.session.close", "terminal_session", &session_id, request_id.as_str(), json!({"node_id":session.node_id,"agent_id":session.agent_id,"exit_reason":session.exit_reason,"exit_code":session.exit_code,"input_bytes":session.input_bytes,"output_bytes":session.output_bytes}))
        .await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(session.into()))
}

async fn capability_for_node(
    state: &AppState,
    node_id: &str,
    request_id: &str,
) -> ApiResult<TerminalCapabilityResponse> {
    let facts: CapabilityFacts = sqlx::query_as("SELECT n.id AS node_id,n.status AS node_status,n.privileged_execution,a.id AS agent_id,a.protocol_version,a.capabilities_json,a.revoked_at,a.archived_at FROM nodes n LEFT JOIN agents a ON a.node_id=n.id WHERE n.id=?")
        .bind(node_id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))?;
    let capabilities = facts
        .capabilities_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Vec<AgentCapability>>(value).ok())
        .unwrap_or_default();
    let pty_terminal = capabilities.contains(&AgentCapability::PtyTerminal);
    let identity_valid =
        facts.agent_id.is_some() && facts.revoked_at.is_none() && facts.archived_at.is_none();
    let agent_online = facts.node_status == "online";
    let unavailable_code = if !facts.privileged_execution {
        Some("terminal_privileged_execution_disabled")
    } else if !identity_valid {
        Some("terminal_agent_identity_invalid")
    } else if !agent_online {
        Some("terminal_agent_offline")
    } else if facts.protocol_version.unwrap_or_default() < 5 {
        Some("terminal_protocol_unsupported")
    } else if !pty_terminal {
        Some("terminal_executor_unavailable")
    } else {
        None
    };
    Ok(TerminalCapabilityResponse {
        node_id: facts.node_id,
        privileged_execution: facts.privileged_execution,
        available: unavailable_code.is_none(),
        unavailable_code: unavailable_code.map(str::to_owned),
        agent_id: facts.agent_id,
        agent_online,
        identity_valid,
        protocol_version: facts.protocol_version,
        pty_terminal,
    })
}

fn gate_error(code: &str, request_id: &str) -> ApiError {
    let message = match code {
        "terminal_privileged_execution_disabled" => "节点尚未启用特权执行",
        "terminal_agent_identity_invalid" => "节点 Agent 身份无效或已撤销",
        "terminal_agent_offline" => "节点 Agent 当前离线",
        "terminal_protocol_unsupported" => "节点 Agent 协议版本不支持终端",
        "terminal_executor_unavailable" => "节点 Agent 未上报可用的终端 executor",
        _ => "节点终端当前不可用",
    };
    ApiError::conflict(code, message, request_id)
}

pub async fn interrupt_active_sessions(pool: &sqlx::SqlitePool) -> sqlx::Result<u64> {
    let sessions = store::active_sessions(pool).await?;
    let affected = store::interrupt_active_sessions(pool).await?;
    if sessions.is_empty() {
        return Ok(affected);
    }
    let mut transaction = pool.begin().await?;
    for session in sessions {
        let session = store::find_session(pool, &session.id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;
        audit::record(
            &mut transaction,
            Some(&session.actor_id),
            "terminal.session.finished",
            "terminal_session",
            &session.id,
            &session.request_id,
            json!({
                "node_id":session.node_id,
                "agent_id":session.agent_id,
                "started_at":session.started_at,
                "opened_at":session.opened_at,
                "finished_at":session.finished_at,
                "exit_reason":session.exit_reason,
                "exit_code":session.exit_code,
                "input_bytes":session.input_bytes,
                "output_bytes":session.output_bytes,
            }),
        )
        .await?;
    }
    transaction.commit().await?;
    Ok(affected)
}
