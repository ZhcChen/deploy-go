pub mod auth;
pub mod store;
pub mod websocket;

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateAgentRequest {
    name: String,
}

#[derive(Serialize, ToSchema)]
pub struct AgentResponse {
    id: String,
    node_id: String,
    name: String,
    status: &'static str,
    registered_at: Option<String>,
    last_seen_at: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AgentEnrollmentResponse {
    agent: AgentResponse,
    enrollment_token: String,
    enrollment_expires_at: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agents", post(create))
        .route("/agents/{agent_id}/revoke", post(revoke))
        .merge(auth::router())
        .merge(websocket::router())
}

#[utoipa::path(operation_id = "agents_revoke", post, path = "/api/v1/agents/{agent_id}/revoke", params(("agent_id" = String, Path)), responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn revoke(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(agent_id): Path<String>,
    actor: AuthUser,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let agent: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT node_id,revoked_at FROM agents WHERE id=?")
            .bind(&agent_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some((node_id, revoked_at)) = agent else {
        return Err(ApiError::not_found(request_id.as_str()));
    };
    if revoked_at.is_none() {
        sqlx::query("UPDATE agents SET revoked_at=?,updated_at=?,version=version+1 WHERE id=?")
            .bind(&now)
            .bind(&now)
            .bind(&agent_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sqlx::query("UPDATE agent_enrollment_tokens SET revoked_at=COALESCE(revoked_at,?) WHERE agent_id=? AND consumed_at IS NULL")
            .bind(&now)
            .bind(&agent_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sqlx::query("UPDATE agent_credential_families SET revoked_at=COALESCE(revoked_at,?),revoke_reason=COALESCE(revoke_reason,'administrator_revoked') WHERE agent_id=?")
            .bind(&now)
            .bind(&agent_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sqlx::query("UPDATE agent_refresh_credentials SET revoked_at=COALESCE(revoked_at,?) WHERE family_id IN (SELECT id FROM agent_credential_families WHERE agent_id=?)")
            .bind(&now)
            .bind(&agent_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sqlx::query(
            "UPDATE agent_access_sessions SET revoked_at=COALESCE(revoked_at,?) WHERE agent_id=?",
        )
        .bind(&now)
        .bind(&agent_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sqlx::query("UPDATE nodes SET status='offline',updated_at=?,version=version+1 WHERE id=? AND status!='disabled'")
            .bind(&now)
            .bind(node_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        audit::record(
            &mut transaction,
            Some(&actor.id),
            "agent.revoke",
            "agent",
            &agent_id,
            request_id.as_str(),
            json!({}),
        )
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    }
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    state.agent_connections().disconnect(&agent_id);
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(operation_id = "agents_create", post, path = "/api/v1/agents", request_body = CreateAgentRequest, responses((status = 201, body = AgentEnrollmentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
    headers: HeaderMap,
    crate::http::ApiJson(payload): crate::http::ApiJson<CreateAgentRequest>,
) -> ApiResult<impl IntoResponse> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (agent_id, node_id) = store::create_with_node_in(&mut transaction, &payload.name)
        .await
        .map_err(|error| map_create_error(error, request_id.as_str()))?;
    let enrollment = auth::issue_enrollment(&mut transaction, &agent_id, Some(actor.id.as_str()))
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "agent.create",
        "agent",
        &agent_id,
        request_id.as_str(),
        json!({"node_id":node_id,"name":payload.name.trim()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(AgentEnrollmentResponse {
            agent: AgentResponse {
                id: agent_id,
                node_id,
                name: payload.name.trim().to_owned(),
                status: "offline",
                registered_at: None,
                last_seen_at: None,
            },
            enrollment_token: enrollment.token,
            enrollment_expires_at: enrollment.expires_at,
        }),
    ))
}

fn map_create_error(error: store::CreateAgentError, request_id: &str) -> ApiError {
    match error {
        store::CreateAgentError::InvalidName => {
            ApiError::validation("Agent 名称格式不正确", request_id)
        }
        store::CreateAgentError::NameConflict => {
            ApiError::conflict("agent_name_conflict", "Agent 名称已存在", request_id)
        }
        store::CreateAgentError::NodeNotFound
        | store::CreateAgentError::NodeAlreadyBound
        | store::CreateAgentError::Database(_) => ApiError::internal(request_id),
    }
}
