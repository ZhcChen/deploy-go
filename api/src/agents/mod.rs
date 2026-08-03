pub mod auth;
pub mod dispatcher;
pub mod store;
pub mod websocket;

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use url::Url;
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
    install_command: String,
}

#[derive(Serialize, ToSchema)]
pub struct AgentInstallCommandResponse {
    agent_id: String,
    enrollment_token: String,
    enrollment_expires_at: String,
    install_command: String,
}

#[derive(Clone, Debug)]
pub struct AgentInstallation {
    public_base_url: Url,
    manifest_url: Url,
}

#[derive(Debug, Error)]
pub enum AgentInstallationError {
    #[error("Agent manifest 不是合法 JSON")]
    InvalidJson,
    #[error("Agent manifest 不符合发布 schema")]
    InvalidSchema,
    #[error("Agent manifest 与当前控制协议不兼容")]
    IncompatibleProtocol,
}

impl AgentInstallation {
    pub fn from_manifest(
        public_base_url: Url,
        manifest_url: Url,
        manifest: &[u8],
    ) -> Result<Self, AgentInstallationError> {
        let manifest: serde_json::Value =
            serde_json::from_slice(manifest).map_err(|_| AgentInstallationError::InvalidJson)?;
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../agent/release/manifest.schema.json"))
                .expect("Agent release schema must be valid JSON");
        let validator = jsonschema::validator_for(&schema)
            .map_err(|_| AgentInstallationError::InvalidSchema)?;
        if !validator.is_valid(&manifest) {
            return Err(AgentInstallationError::InvalidSchema);
        }
        let minimum = manifest["protocol"]["minimum"].as_u64().unwrap_or(u64::MAX);
        let maximum = manifest["protocol"]["maximum"].as_u64().unwrap_or_default();
        let protocol = u64::from(deploy_go_agent_protocol::PROTOCOL_VERSION);
        if !(minimum..=maximum).contains(&protocol) {
            return Err(AgentInstallationError::IncompatibleProtocol);
        }
        Ok(Self {
            public_base_url,
            manifest_url,
        })
    }

    fn command(&self, agent_id: &str, enrollment_token: &str, rebind: bool) -> String {
        let api_base = self.public_base_url.as_str().trim_end_matches('/');
        let mut control_url = self.public_base_url.clone();
        control_url
            .set_scheme("wss")
            .expect("HTTPS URL always accepts the WSS scheme");
        control_url.set_path("/api/v1/agent/control");
        let rebind = if rebind {
            " 'DEPLOY_GO_AGENT_REBIND=1'"
        } else {
            ""
        };
        format!(
            "printf '%s\\n' '{enrollment_token}' | sudo env 'DEPLOY_GO_AGENT_ID={agent_id}' 'DEPLOY_GO_AGENT_API_BASE_URL={api_base}' 'DEPLOY_GO_AGENT_CONTROL_URL={control_url}' 'DEPLOY_GO_AGENT_MANIFEST_URL={}'{rebind} bash -c \"IFS= read -r DEPLOY_GO_AGENT_ENROLLMENT_TOKEN; export DEPLOY_GO_AGENT_ENROLLMENT_TOKEN; curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 '{api_base}/api/v1/agent/install' | bash\"",
            self.manifest_url,
        )
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agents", post(create))
        .route("/agents/{agent_id}/revoke", post(revoke))
        .route(
            "/agents/{agent_id}/install-command",
            post(create_install_command),
        )
        .route("/agent/install", get(installer))
        .merge(auth::router())
        .merge(websocket::router())
}

#[utoipa::path(operation_id = "agents_create_install_command", post, path = "/api/v1/agents/{agent_id}/install-command", params(("agent_id" = String, Path)), responses((status = 200, body = AgentInstallCommandResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 503, body = crate::error::ErrorResponse)))]
pub(crate) async fn create_install_command(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(agent_id): Path<String>,
    actor: AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<AgentInstallCommandResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let installation = state.agent_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_installation_unavailable",
            "Agent 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let revoked_at: Option<Option<String>> =
        sqlx::query_scalar("SELECT revoked_at FROM agents WHERE id=? AND archived_at IS NULL")
            .bind(&agent_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some(revoked_at) = revoked_at else {
        return Err(ApiError::not_found(request_id.as_str()));
    };
    let enrollment = auth::issue_enrollment(&mut transaction, &agent_id, Some(actor.id.as_str()))
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "agent.install_command.create",
        "agent",
        &agent_id,
        request_id.as_str(),
        json!({"rebind":revoked_at.is_some()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(AgentInstallCommandResponse {
        install_command: installation.command(&agent_id, &enrollment.token, revoked_at.is_some()),
        agent_id,
        enrollment_token: enrollment.token,
        enrollment_expires_at: enrollment.expires_at,
    }))
}

pub(crate) async fn installer() -> Response {
    let mut response = Response::new(Body::from(include_str!(
        "../../../agent/install/install.sh"
    )));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/x-shellscript; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
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
    let installation = state.agent_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_installation_unavailable",
            "Agent 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
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
    let install_command = installation.command(&agent_id, &enrollment.token, false);
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
            install_command,
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

#[cfg(test)]
mod tests {
    use super::{AgentInstallation, AgentInstallationError};

    #[test]
    fn rejects_manifest_outside_current_protocol_range() {
        let mut manifest: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../agent/tests/fixtures/release-manifest.json"
        ))
        .unwrap();
        manifest["protocol"] = serde_json::json!({"minimum": 2, "maximum": 3});

        let error = AgentInstallation::from_manifest(
            "https://deploy.example.test".parse().unwrap(),
            "https://release.example.test/manifest.json"
                .parse()
                .unwrap(),
            &serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            AgentInstallationError::IncompatibleProtocol
        ));
    }
}
