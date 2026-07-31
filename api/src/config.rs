use std::{env, net::SocketAddr, str::FromStr};

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("DEPLOY_GO_BIND_ADDR 格式错误: {0}")]
    InvalidBindAddress(String),
    #[error("DEPLOY_GO_DATABASE_URL 不能为空")]
    EmptyDatabaseUrl,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_value =
            env::var("DEPLOY_GO_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
        let database_url = env::var("DEPLOY_GO_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://deploy-go.db".to_owned());

        Self::from_values(&bind_value, &database_url)
    }

    fn from_values(bind_value: &str, database_url: &str) -> Result<Self, ConfigError> {
        let bind_addr = SocketAddr::from_str(bind_value)
            .map_err(|_| ConfigError::InvalidBindAddress(bind_value.to_owned()))?;
        if database_url.trim().is_empty() {
            return Err(ConfigError::EmptyDatabaseUrl);
        }

        Ok(Self {
            bind_addr,
            database_url: database_url.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, ConfigError};

    #[test]
    fn rejects_invalid_bind_address() {
        let error = Config::from_values("not-an-address", "sqlite::memory:").unwrap_err();
        assert!(matches!(error, ConfigError::InvalidBindAddress(_)));
    }

    #[test]
    fn rejects_empty_database_url() {
        let error = Config::from_values("127.0.0.1:8080", " ").unwrap_err();
        assert!(matches!(error, ConfigError::EmptyDatabaseUrl));
    }
}
