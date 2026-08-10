pub mod auth;
pub mod dispatcher;
pub mod store;
pub mod websocket;

use std::path::{Path as FsPath, PathBuf};

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
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
    protocol_version: Option<i64>,
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
    protocol_version: Option<i64>,
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

#[derive(Serialize, ToSchema)]
pub struct AgentReleaseResponse {
    version: String,
    active: bool,
    protocol_minimum: u64,
    protocol_maximum: u64,
}

#[derive(Serialize, ToSchema)]
pub struct AgentReleaseListResponse {
    current_version: Option<String>,
    items: Vec<AgentReleaseResponse>,
}

#[derive(Clone, Debug)]
pub struct AgentInstallation {
    public_base_url: Url,
    release_dir: PathBuf,
    api_version: String,
}

#[derive(Clone, Debug)]
struct AgentRelease {
    version: String,
    download_version: String,
    dir: PathBuf,
    manifest: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum AgentInstallationError {
    #[error("Agent 发布目录不存在或不可读")]
    InvalidReleaseDir,
    #[error("Agent manifest 不是合法 JSON")]
    InvalidJson,
    #[error("Agent manifest 不符合发布 schema")]
    InvalidSchema,
    #[error("Agent manifest 与当前控制协议不兼容")]
    IncompatibleProtocol,
}

impl AgentInstallation {
    pub fn from_dir(
        public_base_url: Url,
        release_dir: PathBuf,
    ) -> Result<Self, AgentInstallationError> {
        if !release_dir.is_dir() {
            return Err(AgentInstallationError::InvalidReleaseDir);
        }
        let installation = Self {
            public_base_url,
            release_dir,
            api_version: env!("CARGO_PKG_VERSION").to_owned(),
        };
        installation.list_releases()?;
        Ok(installation)
    }

    fn validate_manifest(
        &self,
        manifest: &[u8],
    ) -> Result<serde_json::Value, AgentInstallationError> {
        let manifest: serde_json::Value =
            serde_json::from_slice(manifest).map_err(|_| AgentInstallationError::InvalidJson)?;
        let schema_source = match manifest["schema_version"].as_u64() {
            Some(1) => include_str!("../../../agent/release/manifest-v1.schema.json"),
            Some(2) => include_str!("../../../agent/release/manifest-v2.schema.json"),
            _ => include_str!("../../../agent/release/manifest.schema.json"),
        };
        let schema: serde_json::Value =
            serde_json::from_str(schema_source).expect("Agent release schema must be valid JSON");
        let validator = jsonschema::validator_for(&schema)
            .map_err(|_| AgentInstallationError::InvalidSchema)?;
        if !validator.is_valid(&manifest) {
            return Err(AgentInstallationError::InvalidSchema);
        }
        if matches!(manifest["schema_version"].as_u64(), Some(2 | 3)) {
            if manifest["agent_version"] != manifest["executor_version"] {
                return Err(AgentInstallationError::InvalidSchema);
            }
            let mut components = manifest["artifacts"]
                .as_array()
                .ok_or(AgentInstallationError::InvalidSchema)?
                .iter()
                .map(|artifact| {
                    format!(
                        "{}/{}",
                        artifact["component"].as_str().unwrap_or_default(),
                        artifact["architecture"].as_str().unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>();
            components.sort();
            if components
                != [
                    "agent/aarch64",
                    "agent/x86_64",
                    "executor/aarch64",
                    "executor/x86_64",
                ]
            {
                return Err(AgentInstallationError::InvalidSchema);
            }
        }
        let minimum = manifest["protocol"]["minimum"].as_u64().unwrap_or(u64::MAX);
        let maximum = manifest["protocol"]["maximum"].as_u64().unwrap_or_default();
        let supported_minimum = u64::from(deploy_go_agent_protocol::MIN_SUPPORTED_PROTOCOL_VERSION);
        let supported_maximum = u64::from(deploy_go_agent_protocol::PROTOCOL_VERSION);
        if maximum < supported_minimum || minimum > supported_maximum {
            return Err(AgentInstallationError::IncompatibleProtocol);
        }
        let _ = manifest["agent_version"]
            .as_str()
            .ok_or(AgentInstallationError::InvalidSchema)?;
        Ok(manifest)
    }

    fn read_release(&self, dir: &FsPath) -> Result<Option<AgentRelease>, AgentInstallationError> {
        let manifest_path = dir.join("deploy-go-agent-manifest.json");
        let Ok(manifest_bytes) = std::fs::read(&manifest_path) else {
            return Ok(None);
        };
        let manifest = self.validate_manifest(&manifest_bytes)?;
        let version = manifest["agent_version"]
            .as_str()
            .ok_or(AgentInstallationError::InvalidSchema)?
            .to_owned();
        let download_version = version.replace('.', "_");
        let dir_name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if dir_name != version && dir_name != download_version {
            return Ok(None);
        }
        Ok(Some(AgentRelease {
            version,
            download_version,
            dir: dir.to_path_buf(),
            manifest,
        }))
    }

    fn list_releases(&self) -> Result<Vec<AgentRelease>, AgentInstallationError> {
        let mut releases = Vec::new();
        for entry in std::fs::read_dir(&self.release_dir)
            .map_err(|_| AgentInstallationError::InvalidReleaseDir)?
        {
            let path = entry
                .map_err(|_| AgentInstallationError::InvalidReleaseDir)?
                .path();
            if !path.is_dir() {
                continue;
            }
            if let Some(release) = self.read_release(&path)? {
                releases.push(release);
            }
        }
        releases.sort_by(|left, right| left.version.cmp(&right.version));
        Ok(releases)
    }

    fn find_release(&self, version: &str) -> Result<Option<AgentRelease>, AgentInstallationError> {
        let normalized = version.replace('_', ".");
        for release in self.list_releases()? {
            if release.version == normalized {
                return Ok(Some(release));
            }
        }
        Ok(None)
    }

    fn current(&self) -> Result<Option<AgentRelease>, AgentInstallationError> {
        self.find_release(&self.api_version)
    }

    fn current_or_unavailable(&self, request_id: &str) -> ApiResult<AgentRelease> {
        self.current()
            .map_err(|_| ApiError::internal(request_id))?
            .filter(|release| release.manifest["schema_version"].as_u64() == Some(3))
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "agent_installation_unavailable",
                    "当前 API 版本对应的 Agent 发布物尚未同步",
                    request_id,
                )
            })
    }

    fn command(
        &self,
        release: &AgentRelease,
        agent_id: &str,
        node_id: &str,
        capability_public_key: &str,
        release_public_key: &str,
        enrollment_token: &str,
        rebind: bool,
    ) -> String {
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
            "sudo env 'DEPLOY_GO_AGENT_ID={agent_id}' 'DEPLOY_GO_NODE_ID={node_id}' 'DEPLOY_GO_TERMINAL_CAPABILITY_PUBLIC_KEY={capability_public_key}' 'DEPLOY_GO_RELEASE_AUTHORIZATION_PUBLIC_KEY={release_public_key}' 'DEPLOY_GO_AGENT_API_BASE_URL={api_base}' 'DEPLOY_GO_AGENT_CONTROL_URL={control_url}' 'DEPLOY_GO_AGENT_MANIFEST_URL={manifest_url}' 'DEPLOY_GO_AGENT_ENROLLMENT_TOKEN={enrollment_token}'{rebind} bash -c \"curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 '{api_base}/api/v1/agent/install' | bash\"",
            manifest_url = self.release_url(release, "manifest.json"),
            enrollment_token = enrollment_token,
        )
    }

    fn release_url(&self, release: &AgentRelease, suffix: &str) -> String {
        format!(
            "{}/api/v1/agent/download/{}/{}",
            self.public_base_url.as_str().trim_end_matches('/'),
            release.download_version,
            suffix
        )
    }

    fn api_manifest(&self, release: &AgentRelease) -> serde_json::Value {
        let mut manifest = release.manifest.clone();
        if manifest["schema_version"].as_u64() == Some(1) {
            manifest["systemd_unit"]["url"] = self.release_url(release, "systemd-unit").into();
            for artifact in manifest["artifacts"]
                .as_array_mut()
                .expect("Agent manifest 已通过 schema 校验")
            {
                let architecture = artifact["architecture"]
                    .as_str()
                    .expect("Agent manifest 已通过 schema 校验");
                artifact["url"] = self
                    .release_url(release, &format!("agent/{architecture}"))
                    .into();
            }
        } else {
            manifest["systemd_units"]["agent"]["url"] =
                self.release_url(release, "systemd-unit/agent").into();
            if manifest["schema_version"].as_u64() == Some(3) {
                manifest["systemd_units"]["runner"]["url"] =
                    self.release_url(release, "systemd-unit/runner").into();
            }
            manifest["systemd_units"]["executor"]["url"] =
                self.release_url(release, "systemd-unit/executor").into();
            manifest["executor_config"]["url"] =
                self.release_url(release, "executor-config").into();
            for artifact in manifest["artifacts"]
                .as_array_mut()
                .expect("Agent manifest 已通过 schema 校验")
            {
                let component = artifact["component"]
                    .as_str()
                    .expect("Agent manifest 已通过 schema 校验");
                let architecture = artifact["architecture"]
                    .as_str()
                    .expect("Agent manifest 已通过 schema 校验");
                artifact["url"] = self
                    .release_url(release, &format!("{component}/{architecture}"))
                    .into();
            }
        }
        manifest
    }

    async fn serve_file(
        &self,
        path: &FsPath,
        content_type: &'static str,
        filename: &str,
        request_id: &str,
    ) -> ApiResult<Response> {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|_| ApiError::not_found(request_id))?;
        let mut response = Response::new(Body::from(bytes));
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
        response.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
                .expect("静态 ASCII 文件名可以转为 header"),
        );
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
        response.headers_mut().insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
        Ok(response)
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
        .route("/agent/releases", get(list_releases))
        .route("/agent/releases/{version}", delete(delete_release))
        .route(
            "/agent/download/{version}/manifest.json",
            get(download_manifest),
        )
        .route(
            "/agent/download/{version}/agent/{arch}",
            get(download_agent),
        )
        .route(
            "/agent/download/{version}/executor/{arch}",
            get(download_executor),
        )
        .route("/agent/download/{version}/systemd-unit", get(download_unit))
        .route(
            "/agent/download/{version}/systemd-unit/{component}",
            get(download_component_unit),
        )
        .route(
            "/agent/download/{version}/executor-config",
            get(download_executor_config),
        )
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
        protocol_version: row.protocol_version,
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
        "SELECT a.id,a.node_id,n.name,a.environment,n.status AS node_status,a.protocol_version,a.registered_at,a.last_seen_at,a.agent_version,a.hostname,a.architecture,a.revoked_at,a.created_at FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.id=? AND a.archived_at IS NULL",
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
        "SELECT a.id,a.node_id,n.name,a.environment,n.status AS node_status,a.protocol_version,a.registered_at,a.last_seen_at,a.agent_version,a.hostname,a.architecture,a.revoked_at,a.created_at FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.archived_at IS NULL AND (a.created_at>? OR (a.created_at=? AND a.id>?)) ORDER BY a.created_at,a.id LIMIT ?",
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
    let agent: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT node_id,revoked_at FROM agents WHERE id=? AND archived_at IS NULL")
            .bind(&agent_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some((node_id, revoked_at)) = agent else {
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
    let release = installation.current_or_unavailable(request_id.as_str())?;
    let capability_public_key = state
        .terminal_signer()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "terminal_capability_unavailable",
                "终端授权签名器未配置",
                request_id.as_str(),
            )
        })?
        .public_key_base64();
    let release_public_key = state
        .release_signer()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "release_authorization_unavailable",
                "特权发布授权签名器未配置",
                request_id.as_str(),
            )
        })?
        .public_key_base64();
    Ok(Json(AgentInstallCommandResponse {
        install_command: installation.command(
            &release,
            &agent_id,
            &node_id,
            &capability_public_key,
            &release_public_key,
            &enrollment.token,
            revoked_at.is_some(),
        ),
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

async fn download_manifest(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(version): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let installation = state.agent_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_installation_unavailable",
            "Agent 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
    let release = installation
        .find_release(&version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    Ok(Json(installation.api_manifest(&release)))
}

async fn download_agent(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path((version, arch)): Path<(String, String)>,
) -> ApiResult<Response> {
    download_release_binary(
        &state,
        &version,
        &arch,
        "deploy-go-agent",
        request_id.as_str(),
    )
    .await
}

async fn download_executor(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path((version, arch)): Path<(String, String)>,
) -> ApiResult<Response> {
    download_release_binary(
        &state,
        &version,
        &arch,
        "deploy-go-agent-executor",
        request_id.as_str(),
    )
    .await
}

async fn download_release_binary(
    state: &AppState,
    version: &str,
    arch: &str,
    component: &str,
    request_id: &str,
) -> ApiResult<Response> {
    let installation = state.agent_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_installation_unavailable",
            "Agent 发布配置尚未就绪",
            request_id,
        )
    })?;
    let release = installation
        .find_release(version)
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))?;
    let filename = match arch {
        "x86_64" | "aarch64" => format!("{component}-linux-{arch}"),
        _ => return Err(ApiError::not_found(request_id)),
    };
    installation
        .serve_file(
            &release.dir.join(&filename),
            "application/octet-stream",
            &filename,
            request_id,
        )
        .await
}

async fn download_unit(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(version): Path<String>,
) -> ApiResult<Response> {
    let installation = state.agent_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_installation_unavailable",
            "Agent 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
    let release = installation
        .find_release(&version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    installation
        .serve_file(
            &release.dir.join("deploy-go-agent.service"),
            "text/plain; charset=utf-8",
            "deploy-go-agent.service",
            request_id.as_str(),
        )
        .await
}

async fn download_component_unit(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path((version, component)): Path<(String, String)>,
) -> ApiResult<Response> {
    let installation = state.agent_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_installation_unavailable",
            "Agent 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
    let release = installation
        .find_release(&version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let filename = match component.as_str() {
        "agent" => "deploy-go-agent.service",
        "runner" => "deploy-go-agent-runner.service",
        "executor" => "deploy-go-agent-executor.service",
        _ => return Err(ApiError::not_found(request_id.as_str())),
    };
    installation
        .serve_file(
            &release.dir.join(filename),
            "text/plain; charset=utf-8",
            filename,
            request_id.as_str(),
        )
        .await
}

async fn download_executor_config(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(version): Path<String>,
) -> ApiResult<Response> {
    let installation = state.agent_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_installation_unavailable",
            "Agent 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
    let release = installation
        .find_release(&version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    installation
        .serve_file(
            &release.dir.join("executor.json.in"),
            "application/json; charset=utf-8",
            "executor.json.in",
            request_id.as_str(),
        )
        .await
}

#[utoipa::path(operation_id = "agent_releases_list", get, path = "/api/v1/agent/releases", responses((status = 200, body = AgentReleaseListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 503, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_releases(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<AgentReleaseListResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let installation = state.agent_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_installation_unavailable",
            "Agent 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
    let releases = installation
        .list_releases()
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let current = installation
        .current()
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(AgentReleaseListResponse {
        current_version: current.as_ref().map(|release| release.version.clone()),
        items: releases
            .into_iter()
            .map(|release| AgentReleaseResponse {
                active: current
                    .as_ref()
                    .is_some_and(|current| current.version == release.version),
                version: release.version,
                protocol_minimum: release.manifest["protocol"]["minimum"]
                    .as_u64()
                    .unwrap_or_default(),
                protocol_maximum: release.manifest["protocol"]["maximum"]
                    .as_u64()
                    .unwrap_or_default(),
            })
            .collect(),
    }))
}

#[utoipa::path(operation_id = "agent_releases_delete", delete, path = "/api/v1/agent/releases/{version}", params(("version" = String, Path)), responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 503, body = crate::error::ErrorResponse)))]
pub(crate) async fn delete_release(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(version): Path<String>,
    actor: AuthUser,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
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
    let release = installation
        .find_release(&version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    if release.version == installation.api_version {
        return Err(ApiError::conflict(
            "agent_release_current",
            "当前 API 版本对应的 Agent 发布物不能清理",
            request_id.as_str(),
        ));
    }
    tokio::fs::remove_dir_all(&release.dir)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "agent_release.delete",
        "agent_release",
        &release.version,
        request_id.as_str(),
        json!({"version": release.version}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(StatusCode::NO_CONTENT)
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
        crate::terminals::store::close_sessions_for_agent_in(
            &mut transaction,
            &agent_id,
            "agent_identity_revoked",
        )
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
    state
        .terminal_connections()
        .authorization_revoked_for_agent(&state, &agent_id, "agent_identity_revoked")
        .await;
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
    let release = installation.current_or_unavailable(request_id.as_str())?;
    let capability_public_key = state
        .terminal_signer()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "terminal_capability_unavailable",
                "终端授权签名器未配置",
                request_id.as_str(),
            )
        })?
        .public_key_base64();
    let release_public_key = state
        .release_signer()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "release_authorization_unavailable",
                "特权发布授权签名器未配置",
                request_id.as_str(),
            )
        })?
        .public_key_base64();
    let install_command = installation.command(
        &release,
        &agent_id,
        &node_id,
        &capability_public_key,
        &release_public_key,
        &enrollment.token,
        false,
    );
    Ok((
        StatusCode::CREATED,
        Json(AgentEnrollmentResponse {
            agent: AgentResponse {
                id: agent_id,
                node_id,
                name: node_name,
                environment: payload.environment,
                status: "offline".to_owned(),
                protocol_version: None,
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
    fn accepts_legacy_v1_release_for_existing_nodes() {
        let manifest = serde_json::json!({
            "schema_version": 1,
            "agent_version": "0.0.9",
            "protocol": {"minimum": 1, "maximum": 5},
            "systemd_unit": {
                "url": "https://release.example.test/deploy-go-agent.service",
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "artifacts": [
                {
                    "os": "linux",
                    "architecture": "x86_64",
                    "url": "https://release.example.test/deploy-go-agent-linux-x86_64",
                    "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                },
                {
                    "os": "linux",
                    "architecture": "aarch64",
                    "url": "https://release.example.test/deploy-go-agent-linux-aarch64",
                    "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                }
            ]
        });
        let release_dir = std::env::temp_dir().join(format!(
            "deploy-go-agent-legacy-release-test-{}",
            std::process::id()
        ));
        let version_dir = release_dir.join("0.0.9");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(
            version_dir.join("deploy-go-agent-manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let installation = AgentInstallation::from_dir(
            "https://deploy.example.test".parse().unwrap(),
            release_dir.clone(),
        )
        .unwrap();
        assert_eq!(installation.list_releases().unwrap().len(), 1);
        std::fs::remove_dir_all(release_dir).unwrap();
    }

    #[test]
    fn rejects_manifest_outside_current_protocol_range() {
        let mut manifest: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../agent/tests/fixtures/release/0.1.0/deploy-go-agent-manifest.json"
        ))
        .unwrap();
        let unsupported = deploy_go_agent_protocol::PROTOCOL_VERSION + 1;
        manifest["protocol"] =
            serde_json::json!({"minimum": unsupported, "maximum": unsupported + 1});
        let release_dir = std::env::temp_dir().join(format!(
            "deploy-go-agent-release-test-{}",
            std::process::id()
        ));
        let version_dir = release_dir.join("0.1.0");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(
            version_dir.join("deploy-go-agent-manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let error = AgentInstallation::from_dir(
            "https://deploy.example.test".parse().unwrap(),
            release_dir.clone(),
        )
        .unwrap_err();
        std::fs::remove_dir_all(release_dir).unwrap();

        assert!(matches!(
            error,
            AgentInstallationError::IncompatibleProtocol
        ));
    }
}
