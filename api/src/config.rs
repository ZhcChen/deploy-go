use std::{collections::HashSet, env, fmt, net::SocketAddr, path::PathBuf, str::FromStr};

use thiserror::Error;
use url::Url;

#[derive(Clone, Debug)]
pub struct AgentReleaseConfig {
    pub public_base_url: Url,
    pub release_dir: PathBuf,
}

pub const AGENT_RELEASE_DIR: &str = "/var/lib/deploy-go/agent-releases";
pub const ARTIFACTS_DIR: &str = "/var/lib/deploy-go/artifacts";

#[derive(Clone, Debug)]
pub struct ArtifactConfig {
    pub root: PathBuf,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_files: u32,
    pub max_chunk_bytes: u64,
    pub upload_ttl_seconds: u64,
    pub retention_ttl_seconds: u64,
}

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub allowed_origins: Vec<String>,
    pub cookie_secure: bool,
    pub agent_release: Option<AgentReleaseConfig>,
    pub artifacts: ArtifactConfig,
    pub cross_node_artifacts_enabled: bool,
}

impl fmt::Debug for Config {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Config")
            .field("bind_addr", &self.bind_addr)
            .field("database_url", &self.database_url)
            .field("allowed_origins", &self.allowed_origins)
            .field("cookie_secure", &self.cookie_secure)
            .field("agent_release", &self.agent_release)
            .field("artifacts", &self.artifacts)
            .field(
                "cross_node_artifacts_enabled",
                &self.cross_node_artifacts_enabled,
            )
            .finish()
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("DEPLOY_GO_BIND_ADDR 格式错误: {0}")]
    InvalidBindAddress(String),
    #[error("DEPLOY_GO_DATABASE_URL 不能为空")]
    EmptyDatabaseUrl,
    #[error("DEPLOY_GO_ALLOWED_ORIGIN 与 DEPLOY_GO_ALLOWED_ORIGINS 不能同时设置")]
    ConflictingAllowedOrigins,
    #[error("允许的 Origin 列表不能为空且不能包含空项")]
    EmptyAllowedOrigins,
    #[error("允许的 Origin 必须是不含凭证、路径、查询或 fragment 的 http(s) origin: {0}")]
    InvalidAllowedOrigin(String),
    #[error("DEPLOY_GO_COOKIE_SECURE 必须为 true 或 false")]
    InvalidCookieSecure,
    #[error("DEPLOY_GO_PUBLIC_BASE_URL 必须是不含凭证、查询或 fragment 的 HTTPS origin")]
    InvalidPublicBaseUrl,
    #[error("制品配置 {0} 必须是有效的正整数")]
    InvalidArtifactLimit(&'static str),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_value =
            env::var("DEPLOY_GO_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:30100".to_owned());
        let database_url = env::var("DEPLOY_GO_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://deploy-go.db".to_owned());
        let allowed_origins = Self::allowed_origins_from_values(
            env::var("DEPLOY_GO_ALLOWED_ORIGIN").ok().as_deref(),
            env::var("DEPLOY_GO_ALLOWED_ORIGINS").ok().as_deref(),
        )?;
        let cookie_secure =
            env::var("DEPLOY_GO_COOKIE_SECURE").unwrap_or_else(|_| "true".to_owned());

        let mut config =
            Self::from_values(&bind_value, &database_url, allowed_origins, &cookie_secure)?;
        config.agent_release =
            Self::agent_release_from_values(env::var("DEPLOY_GO_PUBLIC_BASE_URL").ok().as_deref())?;
        config.artifacts = Self::artifact_config_from_values(
            env::var("DEPLOY_GO_ARTIFACTS_ROOT").ok().as_deref(),
            env::var("DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES")
                .ok()
                .as_deref(),
            env::var("DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES")
                .ok()
                .as_deref(),
            env::var("DEPLOY_GO_ARTIFACT_MAX_FILES").ok().as_deref(),
            env::var("DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES")
                .ok()
                .as_deref(),
            env::var("DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS")
                .ok()
                .as_deref(),
            env::var("DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS")
                .ok()
                .as_deref(),
        )?;
        config.cross_node_artifacts_enabled = env::var("DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED")
            .ok()
            .as_deref()
            == Some("true");
        Ok(config)
    }

    fn from_values(
        bind_value: &str,
        database_url: &str,
        allowed_origins: Vec<String>,
        cookie_secure: &str,
    ) -> Result<Self, ConfigError> {
        let bind_addr = SocketAddr::from_str(bind_value)
            .map_err(|_| ConfigError::InvalidBindAddress(bind_value.to_owned()))?;
        if database_url.trim().is_empty() {
            return Err(ConfigError::EmptyDatabaseUrl);
        }
        let cookie_secure = cookie_secure
            .parse::<bool>()
            .map_err(|_| ConfigError::InvalidCookieSecure)?;

        Ok(Self {
            bind_addr,
            database_url: database_url.to_owned(),
            allowed_origins,
            cookie_secure,
            agent_release: None,
            artifacts: Self::artifact_config_from_values(None, None, None, None, None, None, None)
                .expect("默认制品配置必须有效"),
            cross_node_artifacts_enabled: false,
        })
    }

    fn artifact_config_from_values(
        root: Option<&str>,
        max_file_bytes: Option<&str>,
        max_total_bytes: Option<&str>,
        max_files: Option<&str>,
        max_chunk_bytes: Option<&str>,
        upload_ttl_seconds: Option<&str>,
        retention_ttl_seconds: Option<&str>,
    ) -> Result<ArtifactConfig, ConfigError> {
        fn positive<T>(
            value: Option<&str>,
            default: &str,
            key: &'static str,
        ) -> Result<T, ConfigError>
        where
            T: FromStr + PartialEq + Default,
            T::Err: std::fmt::Debug,
        {
            let parsed = value
                .unwrap_or(default)
                .parse::<T>()
                .map_err(|_| ConfigError::InvalidArtifactLimit(key))?;
            if parsed == T::default() {
                return Err(ConfigError::InvalidArtifactLimit(key));
            }
            Ok(parsed)
        }
        let root = root.unwrap_or(ARTIFACTS_DIR).trim();
        if root.is_empty() || !PathBuf::from(root).is_absolute() {
            return Err(ConfigError::InvalidArtifactLimit(
                "DEPLOY_GO_ARTIFACTS_ROOT",
            ));
        }
        let config = ArtifactConfig {
            root: PathBuf::from(root),
            max_file_bytes: positive(
                max_file_bytes,
                "536870912",
                "DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES",
            )?,
            max_total_bytes: positive(
                max_total_bytes,
                "2147483648",
                "DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES",
            )?,
            max_files: positive(max_files, "256", "DEPLOY_GO_ARTIFACT_MAX_FILES")?,
            max_chunk_bytes: positive(
                max_chunk_bytes,
                "8388608",
                "DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES",
            )?,
            upload_ttl_seconds: positive(
                upload_ttl_seconds,
                "1800",
                "DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS",
            )?,
            retention_ttl_seconds: positive(
                retention_ttl_seconds,
                "86400",
                "DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS",
            )?,
        };
        if config.max_file_bytes > 512 * 1024 * 1024
            || config.max_total_bytes > 2 * 1024 * 1024 * 1024
            || config.max_chunk_bytes > 8 * 1024 * 1024
            || config.max_file_bytes > config.max_total_bytes
            || config.max_chunk_bytes > config.max_total_bytes
            || config.max_files > 256
        {
            return Err(ConfigError::InvalidArtifactLimit("artifact limits"));
        }
        Ok(config)
    }

    fn agent_release_from_values(
        public_base_url: Option<&str>,
    ) -> Result<Option<AgentReleaseConfig>, ConfigError> {
        let Some(public_base_url) = public_base_url else {
            return Ok(None);
        };
        let public_base_url =
            Url::parse(public_base_url).map_err(|_| ConfigError::InvalidPublicBaseUrl)?;
        if public_base_url.scheme() != "https"
            || public_base_url.host_str().is_none()
            || public_base_url.username() != ""
            || public_base_url.password().is_some()
            || public_base_url.query().is_some()
            || public_base_url.fragment().is_some()
            || public_base_url.path() != "/"
        {
            return Err(ConfigError::InvalidPublicBaseUrl);
        }
        Ok(Some(AgentReleaseConfig {
            public_base_url,
            release_dir: PathBuf::from(AGENT_RELEASE_DIR),
        }))
    }

    fn allowed_origins_from_values(
        allowed_origin: Option<&str>,
        allowed_origins: Option<&str>,
    ) -> Result<Vec<String>, ConfigError> {
        if allowed_origin.is_some() && allowed_origins.is_some() {
            return Err(ConfigError::ConflictingAllowedOrigins);
        }
        let values = match allowed_origins {
            Some(value) => value.split(',').collect::<Vec<_>>(),
            None => vec![allowed_origin.unwrap_or("http://localhost")],
        };
        if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
            return Err(ConfigError::EmptyAllowedOrigins);
        }
        let mut seen = HashSet::new();
        let mut normalized = Vec::new();
        for value in values {
            let value = value.trim();
            let url = Url::parse(value)
                .map_err(|_| ConfigError::InvalidAllowedOrigin(value.to_owned()))?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || url.host_str().is_some_and(|host| host.contains('*'))
                || url.username() != ""
                || url.password().is_some()
                || url.path() != "/"
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(ConfigError::InvalidAllowedOrigin(value.to_owned()));
            }
            let origin = url.origin().ascii_serialization();
            if seen.insert(origin.clone()) {
                normalized.push(origin);
            }
        }
        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::{AGENT_RELEASE_DIR, Config, ConfigError};
    use std::path::PathBuf;

    #[test]
    fn rejects_invalid_bind_address() {
        let error = Config::from_values(
            "not-an-address",
            "sqlite::memory:",
            vec!["http://localhost".to_owned()],
            "true",
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::InvalidBindAddress(_)));
    }

    #[test]
    fn rejects_empty_database_url() {
        let error = Config::from_values(
            "127.0.0.1:30100",
            " ",
            vec!["http://localhost".to_owned()],
            "true",
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::EmptyDatabaseUrl));
    }

    #[test]
    fn rejects_invalid_cookie_secure() {
        let error = Config::from_values(
            "127.0.0.1:30100",
            "sqlite::memory:",
            vec!["http://localhost".to_owned()],
            "yes",
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::InvalidCookieSecure));
    }

    #[test]
    fn parses_normalizes_and_deduplicates_multiple_allowed_origins() {
        let origins = Config::allowed_origins_from_values(
            None,
            Some(" https://ADMIN.example.test:443, http://127.0.0.1:30101,https://admin.example.test "),
        )
        .unwrap();
        assert_eq!(
            origins,
            vec![
                "https://admin.example.test".to_owned(),
                "http://127.0.0.1:30101".to_owned()
            ]
        );
    }

    #[test]
    fn rejects_conflicting_or_invalid_allowed_origin_configuration() {
        assert!(matches!(
            Config::allowed_origins_from_values(
                Some("https://admin.example.test"),
                Some("https://backup.example.test")
            ),
            Err(ConfigError::ConflictingAllowedOrigins)
        ));
        for value in [
            "",
            "https://admin.example.test,",
            "*",
            "https://*.example.test",
            "https://admin.example.test/path",
        ] {
            assert!(
                Config::allowed_origins_from_values(None, Some(value)).is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn agent_release_config_uses_fixed_https_release_dir() {
        assert!(Config::agent_release_from_values(None).unwrap().is_none());
        assert!(matches!(
            Config::agent_release_from_values(Some("http://deploy.example")),
            Err(ConfigError::InvalidPublicBaseUrl)
        ));
        let release = Config::agent_release_from_values(Some("https://deploy.example"))
            .unwrap()
            .unwrap();
        assert_eq!(release.release_dir, PathBuf::from(AGENT_RELEASE_DIR));
    }

    #[test]
    fn artifact_limits_are_bounded_and_require_an_absolute_root() {
        let defaults =
            Config::artifact_config_from_values(None, None, None, None, None, None, None).unwrap();
        assert_eq!(defaults.max_file_bytes, 512 * 1024 * 1024);
        assert_eq!(defaults.max_total_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(defaults.max_files, 256);
        assert_eq!(defaults.max_chunk_bytes, 8 * 1024 * 1024);
        assert!(
            Config::artifact_config_from_values(
                Some("relative"),
                None,
                None,
                None,
                None,
                None,
                None
            )
            .is_err()
        );
        assert!(
            Config::artifact_config_from_values(
                None,
                Some("4096"),
                Some("1024"),
                None,
                None,
                None,
                None
            )
            .is_err()
        );
        assert!(
            Config::artifact_config_from_values(None, None, None, Some("257"), None, None, None)
                .is_err()
        );
        for values in [
            (Some("536870913"), None, None),
            (None, Some("2147483649"), None),
            (None, None, Some("8388609")),
        ] {
            assert!(
                Config::artifact_config_from_values(
                    None, values.0, values.1, None, values.2, None, None
                )
                .is_err()
            );
        }
    }
}
