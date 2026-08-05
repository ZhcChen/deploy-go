use std::{env, fmt, net::SocketAddr, path::PathBuf, str::FromStr};

use thiserror::Error;
use url::Url;

#[derive(Clone, Debug)]
pub struct AgentReleaseConfig {
    pub public_base_url: Url,
    pub manifest_url: Url,
    pub manifest_path: PathBuf,
}

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub setup_token: Option<String>,
    pub allowed_origin: String,
    pub cookie_secure: bool,
    pub agent_release: Option<AgentReleaseConfig>,
}

impl fmt::Debug for Config {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Config")
            .field("bind_addr", &self.bind_addr)
            .field("database_url", &self.database_url)
            .field("setup_token_configured", &self.setup_token.is_some())
            .field("allowed_origin", &self.allowed_origin)
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
    #[error("DEPLOY_GO_ALLOWED_ORIGIN 不能为空")]
    EmptyAllowedOrigin,
    #[error("DEPLOY_GO_COOKIE_SECURE 必须为 true 或 false")]
    InvalidCookieSecure,
    #[error("Agent 发布配置必须同时设置公开基址、manifest URL 和本地 manifest 路径")]
    IncompleteAgentRelease,
    #[error("DEPLOY_GO_PUBLIC_BASE_URL 必须是不含凭证、查询或 fragment 的 HTTPS origin")]
    InvalidPublicBaseUrl,
    #[error("DEPLOY_GO_AGENT_MANIFEST_URL 必须是 HTTPS URL")]
    InvalidAgentManifestUrl,
    #[error("DEPLOY_GO_AGENT_MANIFEST_PATH 必须是绝对路径")]
    InvalidAgentManifestPath,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_value =
            env::var("DEPLOY_GO_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:30100".to_owned());
        let database_url = env::var("DEPLOY_GO_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://deploy-go.db".to_owned());
        let setup_token = env::var("DEPLOY_GO_SETUP_TOKEN")
            .ok()
            .filter(|value| !value.is_empty());
        let allowed_origin =
            env::var("DEPLOY_GO_ALLOWED_ORIGIN").unwrap_or_else(|_| "http://localhost".to_owned());
        let cookie_secure =
            env::var("DEPLOY_GO_COOKIE_SECURE").unwrap_or_else(|_| "true".to_owned());

        let mut config = Self::from_values(
            &bind_value,
            &database_url,
            setup_token,
            &allowed_origin,
            &cookie_secure,
        )?;
        config.agent_release = Self::agent_release_from_values(
            env::var("DEPLOY_GO_PUBLIC_BASE_URL").ok().as_deref(),
            env::var("DEPLOY_GO_AGENT_MANIFEST_URL").ok().as_deref(),
            env::var_os("DEPLOY_GO_AGENT_MANIFEST_PATH")
                .as_deref()
                .and_then(|value| value.to_str()),
        )?;
        Ok(config)
    }

    fn from_values(
        bind_value: &str,
        database_url: &str,
        setup_token: Option<String>,
        allowed_origin: &str,
        cookie_secure: &str,
    ) -> Result<Self, ConfigError> {
        let bind_addr = SocketAddr::from_str(bind_value)
            .map_err(|_| ConfigError::InvalidBindAddress(bind_value.to_owned()))?;
        if database_url.trim().is_empty() {
            return Err(ConfigError::EmptyDatabaseUrl);
        }
        if allowed_origin.trim().is_empty() {
            return Err(ConfigError::EmptyAllowedOrigin);
        }
        let cookie_secure = cookie_secure
            .parse::<bool>()
            .map_err(|_| ConfigError::InvalidCookieSecure)?;

        Ok(Self {
            bind_addr,
            database_url: database_url.to_owned(),
            setup_token,
            allowed_origin: allowed_origin.to_owned(),
            cookie_secure,
            agent_release: None,
        })
    }

    fn agent_release_from_values(
        public_base_url: Option<&str>,
        manifest_url: Option<&str>,
        manifest_path: Option<&str>,
    ) -> Result<Option<AgentReleaseConfig>, ConfigError> {
        let (Some(public_base_url), Some(manifest_url), Some(manifest_path)) =
            (public_base_url, manifest_url, manifest_path)
        else {
            if public_base_url.is_some() || manifest_url.is_some() || manifest_path.is_some() {
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
        let manifest_url =
            Url::parse(manifest_url).map_err(|_| ConfigError::InvalidAgentManifestUrl)?;
        if manifest_url.scheme() != "https"
            || manifest_url.host_str().is_none()
            || manifest_url.username() != ""
            || manifest_url.password().is_some()
        {
            return Err(ConfigError::InvalidAgentManifestUrl);
        }
        let manifest_path = PathBuf::from(manifest_path);
        if !manifest_path.is_absolute() {
            return Err(ConfigError::InvalidAgentManifestPath);
        }
        Ok(Some(AgentReleaseConfig {
            public_base_url,
            manifest_url,
            manifest_path,
        }))
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
            None,
            "http://localhost",
            "true",
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::InvalidBindAddress(_)));
    }

    #[test]
    fn rejects_empty_database_url() {
        let error = Config::from_values("127.0.0.1:30100", " ", None, "http://localhost", "true")
            .unwrap_err();
        assert!(matches!(error, ConfigError::EmptyDatabaseUrl));
    }

    #[test]
    fn rejects_invalid_cookie_secure() {
        let error = Config::from_values(
            "127.0.0.1:30100",
            "sqlite::memory:",
            None,
            "http://localhost",
            "yes",
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::InvalidCookieSecure));
    }

    #[test]
    fn debug_output_redacts_setup_token() {
        let config = Config::from_values(
            "127.0.0.1:30100",
            "sqlite::memory:",
            Some("secret-setup-token".to_owned()),
            "http://localhost",
            "true",
        )
        .unwrap();
        let output = format!("{config:?}");
        assert!(output.contains("setup_token_configured: true"));
        assert!(!output.contains("secret-setup-token"));
    }

    #[test]
    fn agent_release_config_requires_complete_https_values() {
        assert!(matches!(
            Config::agent_release_from_values(Some("https://deploy.example"), None, None),
            Err(ConfigError::IncompleteAgentRelease)
        ));
        assert!(matches!(
            Config::agent_release_from_values(
                Some("http://deploy.example"),
                Some("https://release.example/manifest.json"),
                Some("/etc/deploy-go/agent-manifest.json")
            ),
            Err(ConfigError::InvalidPublicBaseUrl)
        ));
        assert!(
            Config::agent_release_from_values(
                Some("https://deploy.example"),
                Some("https://release.example/manifest.json"),
                Some("/etc/deploy-go/agent-manifest.json")
            )
            .unwrap()
            .is_some()
        );
    }
}
