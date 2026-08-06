use std::{env, path::PathBuf, time::Duration};

use thiserror::Error;
use url::Url;

const DEFAULT_HEARTBEAT_SECONDS: u64 = 30;
const DEFAULT_STAGING_SIZE_LIMIT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const DEFAULT_STAGING_MAX_FILES: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub control_url: Url,
    pub refresh_url: Url,
    pub data_dir: PathBuf,
    pub credential_file: PathBuf,
    pub secrets_root: PathBuf,
    pub heartbeat_interval: Duration,
    pub staging_size_limit_bytes: u64,
    pub staging_max_files: usize,
    pub artifact_transfer_enabled: bool,
    pub env_sync_enabled: bool,
}

#[derive(Debug, Error, PartialEq)]
pub enum ConfigError {
    #[error("DEPLOY_GO_AGENT_CONTROL_URL 缺失")]
    MissingControlUrl,
    #[error("DEPLOY_GO_AGENT_CONTROL_URL 必须是合法的 wss URL")]
    InvalidControlUrl,
    #[error("DEPLOY_GO_AGENT_DATA_DIR 必须是绝对路径")]
    InvalidDataDir,
    #[error("DEPLOY_GO_AGENT_HEARTBEAT_SECONDS 必须在 5 到 300 之间")]
    InvalidHeartbeatInterval,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let control_url =
            env::var("DEPLOY_GO_AGENT_CONTROL_URL").map_err(|_| ConfigError::MissingControlUrl)?;
        let data_dir = env::var_os("DEPLOY_GO_AGENT_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/lib/deploy-go-agent"));
        let heartbeat_seconds = env::var("DEPLOY_GO_AGENT_HEARTBEAT_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_HEARTBEAT_SECONDS);
        let staging_size_limit_bytes = env::var("DEPLOY_GO_AGENT_STAGING_SIZE_LIMIT_BYTES")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_STAGING_SIZE_LIMIT_BYTES);
        let staging_max_files = env::var("DEPLOY_GO_AGENT_STAGING_MAX_FILES")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_STAGING_MAX_FILES);
        let mut config = Self::parse_full(
            &control_url,
            data_dir,
            heartbeat_seconds,
            staging_size_limit_bytes,
            staging_max_files,
        )?;
        config.artifact_transfer_enabled = env::var("DEPLOY_GO_AGENT_ARTIFACT_TRANSFER_ENABLED")
            .ok()
            .as_deref()
            == Some("true");
        config.env_sync_enabled =
            env::var("DEPLOY_GO_AGENT_ENV_SYNC_ENABLED").ok().as_deref() == Some("true");
        Ok(config)
    }

    pub fn parse(
        control_url: &str,
        data_dir: PathBuf,
        heartbeat_seconds: u64,
    ) -> Result<Self, ConfigError> {
        Self::parse_full(
            control_url,
            data_dir,
            heartbeat_seconds,
            DEFAULT_STAGING_SIZE_LIMIT_BYTES,
            DEFAULT_STAGING_MAX_FILES,
        )
    }

    pub fn parse_full(
        control_url: &str,
        data_dir: PathBuf,
        heartbeat_seconds: u64,
        staging_size_limit_bytes: u64,
        staging_max_files: usize,
    ) -> Result<Self, ConfigError> {
        let control_url = Url::parse(control_url).map_err(|_| ConfigError::InvalidControlUrl)?;
        if control_url.scheme() != "wss"
            || control_url.host_str().is_none()
            || control_url.username() != ""
            || control_url.password().is_some()
            || control_url.query().is_some()
            || control_url.fragment().is_some()
        {
            return Err(ConfigError::InvalidControlUrl);
        }
        if !data_dir.is_absolute() {
            return Err(ConfigError::InvalidDataDir);
        }
        if !(5..=300).contains(&heartbeat_seconds) {
            return Err(ConfigError::InvalidHeartbeatInterval);
        }
        let credential_file = data_dir.join("credentials.json");
        let secrets_root = data_dir.join("secrets");
        let mut refresh_url = control_url.clone();
        refresh_url
            .set_scheme("https")
            .map_err(|_| ConfigError::InvalidControlUrl)?;
        refresh_url.set_path("/api/v1/agent/refresh");
        Ok(Self {
            control_url,
            refresh_url,
            data_dir,
            credential_file,
            secrets_root,
            heartbeat_interval: Duration::from_secs(heartbeat_seconds),
            staging_size_limit_bytes,
            staging_max_files,
            artifact_transfer_enabled: false,
            env_sync_enabled: false,
        })
    }
}
