pub mod auth;
pub mod dispatcher;
pub mod store;
pub mod websocket;

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, Path, Query, State},
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
    pagination,
};

pub const ENVIRONMENTS: [&str; 4] = ["dev", "test", "staging", "prod"];

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateAgentRequest {
    name: String,
    node_id: Option<String>,
    environment: String,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct AgentResponse {
    id: String,
    node_id: String,
    name: String,
    environment: String,
    status: String,
    registered_at: Option<String>,
    last_seen_at: Option<String>,
    agent_version: Option<String>,
    hostname: Option<String>,
    architecture: Option<String>,
    revoked_at: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct AgentListRow {
    id: String,
    node_id: String,
    name: String,
    environment: String,
    node_status: String,
    registered_at: Option<String>,
    last_seen_at: Option<String>,
    agent_version: Option<String>,
    hostname: Option<String>,
    architecture: Option<String>,
    revoked_at: Option<String>,
    created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct AgentListResponse {
    items: Vec<AgentResponse>,
    next_cursor: Option<String>,
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

    fn command(&self, agent_id: &str, rebind: bool) -> String {
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
            "IFS= read -r -s -p 'Enrollment token: ' DEPLOY_GO_AGENT_ENROLLMENT_TOKEN; printf '\\n'; printf '%s\\n' \"$DEPLOY_GO_AGENT_ENROLLMENT_TOKEN\" | sudo env 'DEPLOY_GO_AGENT_ID={agent_id}' 'DEPLOY_GO_AGENT_API_BASE_URL={api_base}' 'DEPLOY_GO_AGENT_CONTROL_URL={control_url}' 'DEPLOY_GO_AGENT_MANIFEST_URL={}'{rebind} bash -c \"IFS= read -r DEPLOY_GO_AGENT_ENROLLMENT_TOKEN; export DEPLOY_GO_AGENT_ENROLLMENT_TOKEN; curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 '{api_base}/api/v1/agent/install' | bash\"; unset DEPLOY_GO_AGENT_ENROLLMENT_TOKEN",
            self.manifest_url,
        )
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agents", get(list).post(create))
        .route("/agents/{agent_id}", get(show))
        .route("/agents/{agent_id}/revoke", post(revoke))
        .route(
            "/agents/{agent_id}/install-command",
            post(create_install_command),
        )
        .route("/agent/install", get(installer))
        .merge(auth::router())
        .merge(websocket::router())
}

fn agent_response(row: AgentListRow) -> AgentResponse {
    AgentResponse {
        id: row.id,
        node_id: row.node_id,
        name: row.name,
        environment: row.environment,
        status: if row.node_status == "online" && row.revoked_at.is_none() {
            "online".to_owned()
        } else {
            "offline".to_owned()
        },
        registered_at: row.registered_at,
        last_seen_at: row.last_seen_at,
        agent_version: row.agent_version,
        hostname: row.hostname,
        architecture: row.architecture,
        revoked_at: row.revoked_at,
        created_at: row.created_at,
    }
}

#[utoipa::path(operation_id = "agents_show", get, path = "/api/v1/agents/{agent_id}", params(("agent_id" = String, Path)), responses((status = 200, body = AgentResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(agent_id): Path<String>,
    actor: AuthUser,
) -> ApiResult<Json<AgentResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let row = sqlx::query_as::<_, AgentListRow>(
        "SELECT a.id,a.node_id,n.name,a.environment,n.status AS node_status,a.registered_at,a.last_seen_at,a.agent_version,a.hostname,a.architecture,a.revoked_at,a.created_at FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.id=? AND a.archived_at IS NULL",
    )
    .bind(agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    Ok(Json(agent_response(row)))
}

#[utoipa::path(operation_id = "agents_list", get, path = "/api/v1/agents", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = AgentListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<pagination::ListQuery>,
    actor: AuthUser,
) -> ApiResult<Json<AgentListResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let limit = pagination::limit(&query, request_id.as_str())?;
    let (created_at, id) = pagination::decode_after(&query, request_id.as_str())?
        .unwrap_or_else(|| ("0000".to_owned(), "".to_owned()));
    let rows = sqlx::query_as::<_, AgentListRow>(
        "SELECT a.id,a.node_id,n.name,a.environment,n.status AS node_status,a.registered_at,a.last_seen_at,a.agent_version,a.hostname,a.architecture,a.revoked_at,a.created_at FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.archived_at IS NULL AND (a.created_at>? OR (a.created_at=? AND a.id>?)) ORDER BY a.created_at,a.id LIMIT ?",
    )
    .bind(&created_at)
    .bind(&created_at)
    .bind(&id)
    .bind((limit + 1) as i64)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (rows, next_cursor) = pagination::finish(rows, limit, |row| (&row.created_at, &row.id));
    Ok(Json(AgentListResponse {
        items: rows.into_iter().map(agent_response).collect(),
        next_cursor,
    }))
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
        install_command: installation.command(&agent_id, revoked_at.is_some()),
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
    if !ENVIRONMENTS.contains(&payload.environment.as_str()) {
        return Err(ApiError::validation(
            "环境必须是 dev、test、staging、prod 之一",
            request_id.as_str(),
        ));
    }
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
    let (agent_id, node_id) = if let Some(node_id) = payload.node_id.as_deref() {
        store::bind_existing_node_in(&mut transaction, node_id, &payload.environment)
            .await
            .map_err(|error| map_create_error(error, request_id.as_str()))?
    } else {
        store::create_with_node_in(&mut transaction, &payload.name, &payload.environment)
            .await
            .map_err(|error| map_create_error(error, request_id.as_str()))?
    };
    let node_name: String = sqlx::query_scalar("SELECT name FROM nodes WHERE id=?")
        .bind(&node_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
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
        json!({"node_id":node_id,"name":node_name,"environment":payload.environment,"existing_node":payload.node_id.is_some()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let install_command = installation.command(&agent_id, false);
    Ok((
        StatusCode::CREATED,
        Json(AgentEnrollmentResponse {
            agent: AgentResponse {
                id: agent_id,
                node_id,
                name: node_name,
                environment: payload.environment,
                status: "offline".to_owned(),
                registered_at: None,
                last_seen_at: None,
                agent_version: None,
                hostname: None,
                architecture: None,
                revoked_at: None,
                created_at: chrono::Utc::now().to_rfc3339(),
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
        store::CreateAgentError::NodeNotFound => ApiError::not_found(request_id),
        store::CreateAgentError::NodeAlreadyBound => {
            ApiError::conflict("node_agent_conflict", "节点已经关联 Agent", request_id)
        }
        store::CreateAgentError::Database(_) => ApiError::internal(request_id),
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
