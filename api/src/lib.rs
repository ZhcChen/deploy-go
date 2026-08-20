pub mod agents;
pub mod application_configs;
pub mod application_envs;
pub mod application_sources;
pub mod application_templates;
pub mod applications;
pub mod artifacts;
pub mod audit;
pub mod auth;
pub mod config;
pub mod configuration_centers;
pub mod crypto;
pub mod db;
pub mod deployer;
pub mod deployment_targets;
pub mod deployments;
pub mod error;
pub mod execution_spec;
pub mod external;
pub mod external_keys;
pub mod git_credentials;
pub mod grants;
pub mod http;
pub mod node_telemetry;
pub mod nodes;
mod pagination;
pub mod release_authorization;
pub mod runtime_logs;
pub mod settings;
pub mod ssh_credentials;
pub mod terminal_capability;
pub mod terminals;
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
    allowed_origins: Arc<[String]>,
    cookie_secure: bool,
    master_key_ring: Option<Arc<crypto::MasterKeyRing>>,
    agent_connections: Arc<agents::websocket::ConnectionRegistry>,
    terminal_connections: Arc<terminals::registry::TerminalRegistry>,
    terminal_signer: Option<Arc<deploy_go_terminal_capability::CapabilitySigner>>,
    release_signer: Option<Arc<deploy_go_release_authorization::ReleaseSigner>>,
    agent_installation: Option<Arc<agents::AgentInstallation>>,
    deployer_installation: Option<Arc<deployer::DeployerInstallation>>,
    artifact_store: Option<Arc<artifacts::ArtifactStore>>,
    cross_node_artifacts_enabled: bool,
    runtime_logs: runtime_logs::RuntimeLogStore,
    telemetry_budget: Arc<node_telemetry::TelemetryBudget>,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        let (runtime_logs, _layer) = runtime_logs::RuntimeLogStore::start();
        Self::with_runtime_logs(pool, runtime_logs)
    }

    pub fn with_runtime_logs(
        pool: SqlitePool,
        runtime_logs: runtime_logs::RuntimeLogStore,
    ) -> Self {
        Self {
            pool,
            allowed_origins: Arc::from(["http://localhost".to_owned()]),
            cookie_secure: true,
            master_key_ring: None,
            agent_connections: Arc::new(agents::websocket::ConnectionRegistry::default()),
            terminal_connections: Arc::new(terminals::registry::TerminalRegistry::default()),
            terminal_signer: None,
            release_signer: None,
            agent_installation: None,
            deployer_installation: None,
            artifact_store: None,
            cross_node_artifacts_enabled: false,
            runtime_logs,
            telemetry_budget: Arc::new(node_telemetry::TelemetryBudget::default()),
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn with_allowed_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins.into();
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

    pub fn with_terminal_signer(
        mut self,
        signer: deploy_go_terminal_capability::CapabilitySigner,
    ) -> Self {
        self.terminal_signer = Some(Arc::new(signer));
        self
    }

    pub fn with_release_signer(
        mut self,
        signer: deploy_go_release_authorization::ReleaseSigner,
    ) -> Self {
        self.release_signer = Some(Arc::new(signer));
        self
    }

    pub fn with_agent_installation(mut self, installation: agents::AgentInstallation) -> Self {
        self.agent_installation = Some(Arc::new(installation));
        self
    }

    pub fn with_deployer_installation(
        mut self,
        installation: deployer::DeployerInstallation,
    ) -> Self {
        self.deployer_installation = Some(Arc::new(installation));
        self
    }

    pub fn with_artifact_store(mut self, store: artifacts::ArtifactStore) -> Self {
        self.artifact_store = Some(Arc::new(store));
        self
    }

    pub fn with_cross_node_artifacts_enabled(mut self, enabled: bool) -> Self {
        self.cross_node_artifacts_enabled = enabled;
        self
    }

    pub(crate) fn allows_origin(&self, origin: &str) -> bool {
        self.allowed_origins.iter().any(|allowed| allowed == origin)
    }

    pub(crate) fn cookie_secure(&self) -> bool {
        self.cookie_secure
    }

    pub(crate) fn master_key_ring(&self) -> Option<&crypto::MasterKeyRing> {
        self.master_key_ring.as_deref()
    }

    pub(crate) fn agent_connections(&self) -> &agents::websocket::ConnectionRegistry {
        self.agent_connections.as_ref()
    }

    pub(crate) fn terminal_connections(&self) -> &terminals::registry::TerminalRegistry {
        self.terminal_connections.as_ref()
    }

    pub(crate) fn terminal_signer(
        &self,
    ) -> Option<&deploy_go_terminal_capability::CapabilitySigner> {
        self.terminal_signer.as_deref()
    }

    pub(crate) fn release_signer(&self) -> Option<&deploy_go_release_authorization::ReleaseSigner> {
        self.release_signer.as_deref()
    }

    pub(crate) fn agent_installation(&self) -> Option<&agents::AgentInstallation> {
        self.agent_installation.as_deref()
    }

    pub(crate) fn deployer_installation(&self) -> Option<&deployer::DeployerInstallation> {
        self.deployer_installation.as_deref()
    }

    pub(crate) fn artifact_store(&self) -> Option<&artifacts::ArtifactStore> {
        self.artifact_store.as_deref()
    }

    pub(crate) fn cross_node_artifacts_enabled(&self) -> bool {
        self.cross_node_artifacts_enabled
    }

    pub(crate) fn runtime_logs(&self) -> &runtime_logs::RuntimeLogStore {
        &self.runtime_logs
    }

    pub(crate) fn telemetry_budget(&self) -> &node_telemetry::TelemetryBudget {
        self.telemetry_budget.as_ref()
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
        configuration_centers::show_platform,
        configuration_centers::save_platform,
        configuration_centers::delete_platform,
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
        ssh_credentials::delete_credential,
        git_credentials::list,
        git_credentials::create,
        git_credentials::show,
        git_credentials::update_status,
        nodes::list,
        nodes::show,
        nodes::telemetry,
        nodes::run_check,
        nodes::archive,
        nodes::unarchive,
        terminals::capability,
        terminals::create_session,
        terminals::close_session,
        terminals::websocket::upgrade,
        applications::list,
        applications::show,
        applications::create,
        applications::update,
        applications::update_status,
        application_templates::list,
        application_templates::show,
        application_templates::file,
        application_configs::list,
        application_configs::show,
        application_configs::versions,
        application_configs::update,
        application_configs::restore,
        application_configs::reauthenticate,
        application_configs::controlled_patch,
        application_configs::validate_file,
        application_configs::validate_all,
        application_configs::diff,
        application_configs::generate_secret,
        application_configs::initialize,
        application_configs::delete_workspace,
        application_envs::list,
        application_envs::reauthenticate,
        application_envs::reveal,
        application_envs::update,
        application_envs::delete_env,
        application_envs::retry_sync,
        application_envs::register,
        application_envs::register_admin,
        application_envs::fetch_secret_lease,
        application_sources::show,
        application_sources::save,
        application_sources::refresh,
        application_sources::show_discovery,
        application_sources::set_branch,
        deployment_targets::list,
        deployment_targets::show,
        deployment_targets::create,
        deployment_targets::update,
        deployment_targets::update_status,
        deployments::application_preview,
        deployments::application_confirm,
        deployments::preview,
        deployments::confirm,
        deployments::list,
        deployments::show,
        deployments::events,
        deployments::logs,
        deployments::cancel,
        deployments::retry,
        deployments::release,
        external_keys::list,
        external_keys::create,
        external_keys::show,
        external_keys::revoke,
        external_keys::update_applications,
        agents::create,
        agents::list,
        agents::show,
        agents::create_install_command,
        agents::revoke,
        agents::list_releases,
        agents::delete_release,
        agents::auth::enroll,
        agents::auth::refresh,
        artifacts::http::download_artifact,
        artifacts::http::initiate_upload,
        artifacts::http::upload_status,
        artifacts::http::upload_chunk,
        artifacts::http::finalize_upload,
        runtime_logs::stream,
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
        git_credentials::GitCredentialResponse,
        git_credentials::GitCredentialListResponse,
        nodes::NodeResponse,
        nodes::NodeListResponse,
        nodes::NodeCheckResponse,
        node_telemetry::TelemetryResponse,
        node_telemetry::LatestTelemetry,
        node_telemetry::MetricValue,
        node_telemetry::HistoryPoint,
        terminals::TerminalCapabilityResponse,
        terminals::TerminalSessionResponse,
        applications::ApplicationResponse,
        applications::ApplicationListResponse,
        application_templates::ApplicationTemplateFileResponse,
        application_templates::ApplicationTemplateResponse,
        application_templates::ApplicationTemplateListResponse,
        application_envs::ApplicationEnvFileResponse,
        application_envs::ApplicationEnvFileListResponse,
        application_envs::EnvRevealGrantResponse,
        application_envs::ApplicationEnvPlaintextResponse,
        application_envs::RegisterApplicationEnvsResponse,
        application_envs::ApplicationEnvRegistrationResponse,
        application_envs::RetryApplicationEnvSyncResponse,
        application_configs::ApplicationConfigFileResponse,
        application_configs::ApplicationConfigFileListResponse,
        application_configs::ApplicationConfigVersionResponse,
        application_configs::ApplicationConfigVersionListResponse,
        application_configs::UpdateApplicationConfigRequest,
        application_configs::RestoreApplicationConfigRequest,
        application_configs::InitializeApplicationConfigsRequest,
        application_configs::InitializeApplicationConfigsResponse,
        application_configs::DeleteApplicationConfigWorkspaceRequest,
        application_configs::ConfigGrantAction,
        application_configs::ConfigReauthenticateRequest,
        application_configs::ConfigRevealGrantResponse,
        application_configs::ControlledPatchRequest,
        application_configs::ValidateApplicationConfigRequest,
        application_configs::ConfigDiagnostic,
        application_configs::ApplicationConfigValidationResponse,
        application_configs::GenerateSecretRequest,
        application_configs::GenerateSecretResponse,
        application_configs::ConfigDiffQuery,
        application_configs::ApplicationConfigDiffResponse,
        application_sources::ApplicationSourceResponse,
        application_sources::GitRefResponse,
        application_sources::GitRefDiscoveryResponse,
        deployment_targets::DeploymentTargetResponse,
        deployment_targets::DeploymentTargetListResponse,
        deployment_targets::SecretFileReference,
        deployment_targets::ImageDeploySpec,
        deployment_targets::ImageTemplate,
        deployments::DeploymentResponse,
        deployments::DeploymentTargetRunResponse,
        deployments::DeploymentListResponse,
        deployments::DeploymentPreviewResponse,
        deployments::ApplicationDeploymentPreviewResponse,
        deployments::DeploymentTargetPreviewResponse,
        deployments::DeploymentLogResponse,
        deployments::DeploymentEventResponse,
        deployments::DeploymentEventListResponse,
        external_keys::ExternalApiKeySummary,
        external_keys::ExternalApiKeyListResponse,
        external_keys::ExternalApiKeyCreatedResponse,
        external_keys::CreateExternalApiKeyRequest,
        external_keys::UpdateExternalApiKeyApplicationsRequest,
        agents::AgentResponse,
        agents::AgentListResponse,
        agents::AgentEnrollmentResponse,
        agents::AgentInstallCommandResponse,
        agents::AgentReleaseResponse,
        agents::AgentReleaseListResponse,
        agents::auth::TokenPairResponse,
        agents::auth::RefreshTokenPairResponse,
        runtime_logs::RuntimeLogResponse,
        configuration_centers::PlatformConfigurationCenterResponse,
        configuration_centers::SavePlatformConfigurationCenterRequest,
        configuration_centers::DeletePlatformConfigurationCenterRequest,
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
        .nest("/api/v1", configuration_centers::router())
        .nest("/api/v1", users::router())
        .nest("/api/v1", grants::router())
        .nest("/api/v1", settings::router())
        .nest("/api/v1", ssh_credentials::router())
        .nest("/api/v1", git_credentials::router())
        .nest("/api/v1", nodes::router())
        .nest("/api/v1", terminals::router())
        .nest("/api/v1", applications::router())
        .nest("/api/v1", application_templates::router())
        .nest("/api/v1", application_configs::router())
        .nest("/api/v1", application_envs::router())
        .nest("/api/v1", application_sources::router())
        .nest("/api/v1", deployment_targets::router())
        .nest("/external/v1", external::router())
        .nest("/api/v1", deployments::router())
        .nest("/api/v1", external_keys::router())
        .nest("/api/v1", deployer::router())
        .nest("/api/v1", artifacts::router())
        .nest("/api/v1", agents::router())
        .nest("/api/v1", runtime_logs::router())
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
    document["components"]["securitySchemes"]["agentBearerAuth"] = serde_json::json!({
        "type": "http",
        "scheme": "bearer"
    });

    let Some(paths) = document["paths"].as_object_mut() else {
        return;
    };
    for (path, path_item) in paths {
        let Some(operations) = path_item.as_object_mut() else {
            continue;
        };
        for (method, operation) in operations {
            let is_agent_bearer = path.starts_with("/api/v1/agent/artifact-leases/")
                || path.starts_with("/api/v1/agent/env-registration-leases/")
                || path.starts_with("/api/v1/agent/application-env-leases/");
            let is_public = matches!(
                path.as_str(),
                "/healthz"
                    | "/readyz"
                    | "/api/v1/setup"
                    | "/api/v1/auth/login"
                    | "/api/v1/agent/enroll"
                    | "/api/v1/agent/refresh"
            );
            if is_agent_bearer {
                operation["security"] = serde_json::json!([{ "agentBearerAuth": [] }]);
            } else if !is_public {
                operation["security"] = serde_json::json!([{ "cookieAuth": [] }]);
            }

            let is_csrf_protected = !matches!(method.as_str(), "get" | "head" | "options")
                && !is_agent_bearer
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

    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let started = std::time::Instant::now();
    let request_id = request
        .headers()
        .get(&HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| valid_request_id(value))
        .map(|value| RequestId(value.to_owned()))
        .unwrap_or_else(RequestId::generate);

    request.extensions_mut().insert(request_id.clone());
    let mut response = next.run(request).await;
    if path.contains("/env-")
        || path.contains("/application-env-")
        || path.contains("/config-files")
        || path.contains("/application-config-")
        || path.contains("/config-reveal-grants")
    {
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        );
        response.headers_mut().insert(
            axum::http::header::PRAGMA,
            HeaderValue::from_static("no-cache"),
        );
    }
    let elapsed_ms = started.elapsed().as_millis() as u64;
    if response.status().is_server_error() {
        tracing::error!(
            request_id = request_id.as_str(),
            method = %method,
            path = %path,
            status = response.status().as_u16(),
            elapsed_ms,
            "request failed"
        );
    } else if response.status().is_client_error() {
        tracing::warn!(
            request_id = request_id.as_str(),
            method = %method,
            path = %path,
            status = response.status().as_u16(),
            elapsed_ms,
            "request rejected"
        );
    } else {
        tracing::info!(request_id = request_id.as_str(), method = %method, path = %path, status = response.status().as_u16(), elapsed_ms, "request completed");
    }
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
