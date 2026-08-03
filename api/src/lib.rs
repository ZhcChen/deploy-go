pub mod agents;
pub mod applications;
pub mod audit;
pub mod auth;
pub mod config;
pub mod crypto;
pub mod db;
pub mod deployment_targets;
pub mod deployments;
pub mod error;
pub mod execution_spec;
pub mod executor;
pub mod grants;
pub mod http;
pub mod nodes;
mod pagination;
pub mod settings;
pub mod ssh_credentials;
pub mod users;

use axum::{
    Json, Router,
    extract::{Extension, State},
    http::{HeaderName, HeaderValue, Request},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::Arc;
use ulid::Ulid;
use utoipa::{OpenApi, ToSchema};

use crate::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    setup_token: Option<Arc<str>>,
    allowed_origin: Arc<str>,
    cookie_secure: bool,
    master_key_ring: Option<Arc<crypto::MasterKeyRing>>,
    node_probe: Arc<dyn executor::ssh::NodeProbe>,
    deployment_executor: Arc<dyn executor::deployment::DeploymentExecutor>,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            setup_token: None,
            allowed_origin: Arc::from("http://localhost"),
            cookie_secure: true,
            master_key_ring: None,
            node_probe: Arc::new(executor::ssh::OpenSshProbe::default()),
            deployment_executor: Arc::new(
                executor::deployment::OpenSshDeploymentExecutor::default(),
            ),
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn with_setup_token(mut self, setup_token: impl Into<Arc<str>>) -> Self {
        self.setup_token = Some(setup_token.into());
        self
    }

    pub fn with_allowed_origin(mut self, origin: impl Into<Arc<str>>) -> Self {
        self.allowed_origin = origin.into();
        self
    }

    pub fn with_cookie_secure(mut self, secure: bool) -> Self {
        self.cookie_secure = secure;
        self
    }

    pub fn with_master_key_ring(mut self, key_ring: crypto::MasterKeyRing) -> Self {
        self.master_key_ring = Some(Arc::new(key_ring));
        self
    }

    pub fn with_node_probe(mut self, probe: impl executor::ssh::NodeProbe + 'static) -> Self {
        self.node_probe = Arc::new(probe);
        self
    }

    pub fn with_deployment_executor(
        mut self,
        executor: impl executor::deployment::DeploymentExecutor + 'static,
    ) -> Self {
        self.deployment_executor = Arc::new(executor);
        self
    }

    pub(crate) fn setup_token(&self) -> Option<&str> {
        self.setup_token.as_deref()
    }

    pub(crate) fn allowed_origin(&self) -> &str {
        &self.allowed_origin
    }

    pub(crate) fn cookie_secure(&self) -> bool {
        self.cookie_secure
    }

    pub(crate) fn master_key_ring(&self) -> Option<&crypto::MasterKeyRing> {
        self.master_key_ring.as_deref()
    }

    pub(crate) fn node_probe(&self) -> &dyn executor::ssh::NodeProbe {
        self.node_probe.as_ref()
    }

    pub(crate) fn deployment_executor(&self) -> &dyn executor::deployment::DeploymentExecutor {
        self.deployment_executor.as_ref()
    }
}

#[derive(Clone, Debug)]
pub struct RequestId(String);

impl RequestId {
    fn generate() -> Self {
        Self(format!("req_{}", Ulid::new()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Serialize, ToSchema)]
struct StatusResponse {
    status: &'static str,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        healthz,
        readyz,
        auth::setup_status,
        auth::setup,
        auth::login,
        auth::logout,
        auth::me,
        auth::profile,
        auth::update_profile,
        auth::preferences,
        auth::update_preferences,
        auth::refresh_csrf,
        audit::list,
        users::list,
        users::show,
        users::create,
        users::update_status,
        users::reset_password,
        grants::list,
        grants::grant,
        grants::revoke,
        settings::show,
        settings::update,
        ssh_credentials::list,
        ssh_credentials::show,
        ssh_credentials::create,
        ssh_credentials::rename,
        ssh_credentials::delete_credential,
        nodes::list,
        nodes::show,
        nodes::create,
        nodes::update,
        nodes::update_status,
        nodes::bind_credential,
        nodes::unbind_credential,
        nodes::scan_host_key,
        nodes::confirm_host_key,
        nodes::run_check,
        applications::list,
        applications::show,
        applications::create,
        applications::update,
        applications::update_status,
        deployment_targets::list,
        deployment_targets::show,
        deployment_targets::create,
        deployment_targets::update,
        deployment_targets::update_status,
        deployments::preview,
        deployments::confirm,
        deployments::list,
        deployments::show,
        deployments::logs,
        deployments::cancel,
        deployments::retry,
        agents::create,
        agents::auth::enroll,
        agents::auth::refresh
    ),
    components(schemas(
        StatusResponse,
        crate::error::ErrorResponse,
        audit::AuditLogResponse,
        audit::AuditLogListResponse,
        auth::UserIdentity,
        auth::SetupStatusResponse,
        auth::UserPreferencesResponse,
        auth::CsrfTokenResponse,
        users::UserResponse,
        users::UserListResponse,
        grants::ApplicationGrantResponse,
        grants::ApplicationGrantListResponse,
        settings::RuntimeSettings,
        ssh_credentials::SshCredentialResponse,
        ssh_credentials::SshCredentialListResponse,
        nodes::NodeResponse,
        nodes::NodeListResponse,
        nodes::HostKeyScanResponse,
        nodes::NodeCheckResponse,
        applications::ApplicationResponse,
        applications::ApplicationListResponse,
        deployment_targets::DeploymentTargetResponse,
        deployment_targets::DeploymentTargetListResponse,
        deployment_targets::SecretFileReference,
        deployments::DeploymentResponse,
        deployments::DeploymentListResponse,
        deployments::DeploymentPreviewResponse,
        deployments::DeploymentLogResponse,
        agents::AgentResponse,
        agents::AgentEnrollmentResponse,
        agents::auth::TokenPairResponse,
        agents::auth::RefreshTokenPairResponse
    ))
)]
struct ApiDoc;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/v1/openapi.json", get(openapi))
        .nest("/api/v1", auth::router())
        .nest("/api/v1", audit::router())
        .nest("/api/v1", users::router())
        .nest("/api/v1", grants::router())
        .nest("/api/v1", settings::router())
        .nest("/api/v1", ssh_credentials::router())
        .nest("/api/v1", nodes::router())
        .nest("/api/v1", applications::router())
        .nest("/api/v1", deployment_targets::router())
        .nest("/api/v1", deployments::router())
        .nest("/api/v1", agents::router())
        .with_state(state)
        .layer(middleware::from_fn(request_id))
}

#[utoipa::path(operation_id = "system_healthz", get, path = "/healthz", responses((status = 200, body = StatusResponse)))]
async fn healthz() -> Json<StatusResponse> {
    Json(StatusResponse { status: "ok" })
}

#[utoipa::path(operation_id = "system_readyz",
    get,
    path = "/readyz",
    responses(
        (status = 200, body = StatusResponse),
        (status = 503, body = crate::error::ErrorResponse)
    )
)]
async fn readyz(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> ApiResult<Json<StatusResponse>> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(state.pool())
        .await
        .map_err(|error| {
            tracing::warn!(%error, request_id = request_id.as_str(), "readiness database check failed");
            ApiError::service_not_ready(request_id.as_str())
        })?;

    Ok(Json(StatusResponse { status: "ready" }))
}

async fn openapi() -> Json<serde_json::Value> {
    Json(openapi_document())
}

pub fn openapi_document() -> serde_json::Value {
    let mut document = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI 可以序列化");
    enrich_openapi_security_contract(&mut document);
    document
}

fn enrich_openapi_security_contract(document: &mut serde_json::Value) {
    document["components"]["securitySchemes"]["cookieAuth"] = serde_json::json!({
        "type": "apiKey",
        "in": "cookie",
        "name": "deploy_go_session"
    });

    let Some(paths) = document["paths"].as_object_mut() else {
        return;
    };
    for (path, path_item) in paths {
        let Some(operations) = path_item.as_object_mut() else {
            continue;
        };
        for (method, operation) in operations {
            let is_public = matches!(
                path.as_str(),
                "/healthz"
                    | "/readyz"
                    | "/api/v1/setup"
                    | "/api/v1/auth/login"
                    | "/api/v1/agent/enroll"
                    | "/api/v1/agent/refresh"
            );
            if !is_public {
                operation["security"] = serde_json::json!([{ "cookieAuth": [] }]);
            }

            let is_csrf_protected = !matches!(method.as_str(), "get" | "head" | "options")
                && !matches!(
                    path.as_str(),
                    "/api/v1/setup"
                        | "/api/v1/auth/login"
                        | "/api/v1/auth/csrf"
                        | "/api/v1/agent/enroll"
                        | "/api/v1/agent/refresh"
                );
            if is_csrf_protected {
                let parameters = operation
                    .as_object_mut()
                    .expect("operation 是对象")
                    .entry("parameters")
                    .or_insert_with(|| serde_json::json!([]));
                let parameters = parameters.as_array_mut().expect("parameters 是数组");
                if !parameters
                    .iter()
                    .any(|parameter| parameter["name"] == "X-CSRF-Token")
                {
                    parameters.push(serde_json::json!({
                        "name": "X-CSRF-Token",
                        "in": "header",
                        "required": true,
                        "schema": { "type": "string" }
                    }));
                }
            }

            if operation.get("requestBody").is_some() {
                operation["responses"]["422"] = error_response("请求 JSON 不符合约束");
            }
            if path == "/api/v1/setup" && method == "post" {
                operation["responses"]["403"] = error_response("请求来源不允许");
            }
        }
    }
}

fn error_response(description: &str) -> serde_json::Value {
    serde_json::json!({
        "description": description,
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorResponse" }
            }
        }
    })
}

async fn request_id(mut request: Request<axum::body::Body>, next: Next) -> Response {
    static HEADER: HeaderName = HeaderName::from_static("x-request-id");

    let request_id = request
        .headers()
        .get(&HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| valid_request_id(value))
        .map(|value| RequestId(value.to_owned()))
        .unwrap_or_else(RequestId::generate);

    request.extensions_mut().insert(request_id.clone());
    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(request_id.as_str()) {
        response.headers_mut().insert(HEADER.clone(), value);
    }
    response
}

fn valid_request_id(value: &str) -> bool {
    (8..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::valid_request_id;

    #[test]
    fn request_id_validation_rejects_control_and_short_values() {
        assert!(valid_request_id("request-123"));
        assert!(!valid_request_id("short"));
        assert!(!valid_request_id("request\n123"));
    }
}
