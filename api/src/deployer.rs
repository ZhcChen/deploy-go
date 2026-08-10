use std::path::{Path as FsPath, PathBuf};

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, Path, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
    routing::get,
};
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::{
    AppState, RequestId,
    error::{ApiError, ApiResult},
};

pub const DEPLOYER_RELEASE_DIR: &str = "/var/lib/deploy-go/deployer-releases";

#[derive(Clone, Debug)]
pub struct DeployerInstallation {
    public_base_url: Url,
    release_dir: PathBuf,
    api_version: String,
}

#[derive(Clone, Debug)]
struct DeployerRelease {
    version: String,
    download_version: String,
    dir: PathBuf,
    manifest: Value,
}

#[derive(Debug, Error)]
pub enum DeployerInstallationError {
    #[error("deployer 发布目录不存在或不可读")]
    InvalidReleaseDir,
    #[error("deployer manifest 不是合法 JSON")]
    InvalidJson,
    #[error("deployer manifest 不符合发布 schema")]
    InvalidSchema,
}

impl DeployerInstallation {
    pub fn from_dir(
        public_base_url: Url,
        release_dir: PathBuf,
    ) -> Result<Self, DeployerInstallationError> {
        if !release_dir.is_dir() {
            return Err(DeployerInstallationError::InvalidReleaseDir);
        }
        let installation = Self {
            public_base_url,
            release_dir,
            api_version: env!("CARGO_PKG_VERSION").to_owned(),
        };
        installation.list_releases()?;
        installation.current()?;
        Ok(installation)
    }

    fn validate_manifest(&self, manifest: &[u8]) -> Result<Value, DeployerInstallationError> {
        let manifest: Value =
            serde_json::from_slice(manifest).map_err(|_| DeployerInstallationError::InvalidJson)?;
        let schema: Value = serde_json::from_str(include_str!(
            "../../deploy-go-deployer/release/manifest.schema.json"
        ))
        .expect("deployer release schema must be valid JSON");
        let validator = jsonschema::validator_for(&schema)
            .map_err(|_| DeployerInstallationError::InvalidSchema)?;
        if !validator.is_valid(&manifest) {
            return Err(DeployerInstallationError::InvalidSchema);
        }
        let _ = manifest["deployer_version"]
            .as_str()
            .ok_or(DeployerInstallationError::InvalidSchema)?;
        Ok(manifest)
    }

    fn read_release(
        &self,
        dir: &FsPath,
    ) -> Result<Option<DeployerRelease>, DeployerInstallationError> {
        let manifest_path = dir.join("deploy-go-deployer-manifest.json");
        let Ok(manifest_bytes) = std::fs::read(&manifest_path) else {
            return Ok(None);
        };
        let manifest = self.validate_manifest(&manifest_bytes)?;
        let version = manifest["deployer_version"]
            .as_str()
            .ok_or(DeployerInstallationError::InvalidSchema)?
            .to_owned();
        let download_version = version.replace('.', "_");
        let dir_name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if dir_name != version && dir_name != download_version {
            return Ok(None);
        }
        for architecture in ["x86_64", "aarch64"] {
            let path = dir.join(format!("deploy-go-deployer-linux-{architecture}"));
            if !path.is_file() {
                return Err(DeployerInstallationError::InvalidSchema);
            }
        }
        Ok(Some(DeployerRelease {
            version,
            download_version,
            dir: dir.to_path_buf(),
            manifest,
        }))
    }

    fn list_releases(&self) -> Result<Vec<DeployerRelease>, DeployerInstallationError> {
        let mut releases = Vec::new();
        for entry in std::fs::read_dir(&self.release_dir)
            .map_err(|_| DeployerInstallationError::InvalidReleaseDir)?
        {
            let path = entry
                .map_err(|_| DeployerInstallationError::InvalidReleaseDir)?
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

    fn find_release(
        &self,
        version: &str,
    ) -> Result<Option<DeployerRelease>, DeployerInstallationError> {
        let normalized = version.replace('_', ".");
        for release in self.list_releases()? {
            if release.version == normalized {
                return Ok(Some(release));
            }
        }
        Ok(None)
    }

    fn current(&self) -> Result<Option<DeployerRelease>, DeployerInstallationError> {
        self.find_release(&self.api_version)
    }

    fn release_url(&self, release: &DeployerRelease, suffix: &str) -> String {
        format!(
            "{}/api/v1/deployer/download/{}/{}",
            self.public_base_url.as_str().trim_end_matches('/'),
            release.download_version,
            suffix
        )
    }

    fn api_manifest(&self, release: &DeployerRelease) -> Value {
        let mut manifest = release.manifest.clone();
        for artifact in manifest["artifacts"]
            .as_array_mut()
            .expect("deployer manifest 已通过 schema 校验")
        {
            let architecture = artifact["architecture"]
                .as_str()
                .expect("deployer manifest 已通过 schema 校验");
            artifact["url"] = self
                .release_url(release, &format!("deployer/{architecture}"))
                .into();
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
        .route(
            "/deployer/download/{version}/manifest.json",
            get(download_manifest),
        )
        .route(
            "/deployer/download/{version}/deployer/{arch}",
            get(download_deployer),
        )
}

async fn download_manifest(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(version): Path<String>,
) -> ApiResult<Json<Value>> {
    let installation = state.deployer_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "deployer_installation_unavailable",
            "deployer 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
    let release = installation
        .find_release(&version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    Ok(Json(installation.api_manifest(&release)))
}

async fn download_deployer(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path((version, arch)): Path<(String, String)>,
) -> ApiResult<Response> {
    let installation = state.deployer_installation().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "deployer_installation_unavailable",
            "deployer 发布配置尚未就绪",
            request_id.as_str(),
        )
    })?;
    let release = installation
        .find_release(&version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let filename = match arch.as_str() {
        "x86_64" | "aarch64" => format!("deploy-go-deployer-linux-{arch}"),
        _ => return Err(ApiError::not_found(request_id.as_str())),
    };
    installation
        .serve_file(
            &release.dir.join(&filename),
            "application/octet-stream",
            &filename,
            request_id.as_str(),
        )
        .await
}
