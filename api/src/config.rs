use std::{collections::HashSet, env, fmt, net::SocketAddr, path::PathBuf, str::FromStr};

use thiserror::Error;
use url::Url;

#[derive(Clone, Debug)]
pub struct AgentReleaseConfig {
    pub public_base_url: Url,
    pub release_dir: PathBuf,
}

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub allowed_origins: Vec<String>,
    pub cookie_secure: bool,
    pub agent_release: Option<AgentReleaseConfig>,
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
    #[error("Agent 发布配置必须同时设置公开基址和本地发布目录")]
    IncompleteAgentRelease,
    #[error("DEPLOY_GO_PUBLIC_BASE_URL 必须是不含凭证、查询或 fragment 的 HTTPS origin")]
    InvalidPublicBaseUrl,
    #[error("DEPLOY_GO_AGENT_RELEASE_DIR 必须是绝对路径")]
    InvalidAgentReleaseDir,
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
        config.agent_release = Self::agent_release_from_values(
            env::var("DEPLOY_GO_PUBLIC_BASE_URL").ok().as_deref(),
            env::var_os("DEPLOY_GO_AGENT_RELEASE_DIR")
                .as_deref()
                .and_then(|value| value.to_str()),
        )?;
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
        })
    }

    fn agent_release_from_values(
        public_base_url: Option<&str>,
        release_dir: Option<&str>,
    ) -> Result<Option<AgentReleaseConfig>, ConfigError> {
        let (Some(public_base_url), Some(release_dir)) = (public_base_url, release_dir) else {
            if public_base_url.is_some() || release_dir.is_some() {
                return Err(ConfigError::IncompleteAgentRelease);
            }
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
        let release_dir = PathBuf::from(release_dir);
        if !release_dir.is_absolute() {
            return Err(ConfigError::InvalidAgentReleaseDir);
        }
        Ok(Some(AgentReleaseConfig {
            public_base_url,
            release_dir,
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
    use super::{Config, ConfigError};

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
    fn agent_release_config_requires_complete_https_values() {
        assert!(matches!(
            Config::agent_release_from_values(Some("https://deploy.example"), None),
            Err(ConfigError::IncompleteAgentRelease)
        ));
        assert!(matches!(
            Config::agent_release_from_values(
                Some("http://deploy.example"),
                Some("/etc/deploy-go/release")
            ),
            Err(ConfigError::InvalidPublicBaseUrl)
        ));
        assert!(
            Config::agent_release_from_values(
                Some("https://deploy.example"),
                Some("/etc/deploy-go/release")
            )
            .unwrap()
            .is_some()
        );
        assert!(matches!(
            Config::agent_release_from_values(
                Some("https://deploy.example"),
                Some("relative/release")
            ),
            Err(ConfigError::InvalidAgentReleaseDir)
        ));
    }
}
