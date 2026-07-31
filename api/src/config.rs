use std::{env, fmt, net::SocketAddr, str::FromStr};

use thiserror::Error;

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub setup_token: Option<String>,
    pub allowed_origin: String,
    pub cookie_secure: bool,
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
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_value =
            env::var("DEPLOY_GO_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
        let database_url = env::var("DEPLOY_GO_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://deploy-go.db".to_owned());
        let setup_token = env::var("DEPLOY_GO_SETUP_TOKEN")
            .ok()
            .filter(|value| !value.is_empty());
        let allowed_origin =
            env::var("DEPLOY_GO_ALLOWED_ORIGIN").unwrap_or_else(|_| "http://localhost".to_owned());
        let cookie_secure =
            env::var("DEPLOY_GO_COOKIE_SECURE").unwrap_or_else(|_| "true".to_owned());

        Self::from_values(
            &bind_value,
            &database_url,
            setup_token,
            &allowed_origin,
            &cookie_secure,
        )
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
        })
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
        let error = Config::from_values("127.0.0.1:8080", " ", None, "http://localhost", "true")
            .unwrap_err();
        assert!(matches!(error, ConfigError::EmptyDatabaseUrl));
    }

    #[test]
    fn rejects_invalid_cookie_secure() {
        let error = Config::from_values(
            "127.0.0.1:8080",
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
            "127.0.0.1:8080",
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
}
