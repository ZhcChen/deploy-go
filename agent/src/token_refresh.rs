use std::{fmt, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use ulid::Ulid;
use url::Url;

use crate::credential_store::{
    AgentCredentials, CredentialError, CredentialStore, PendingRotation,
};

#[derive(Clone)]
pub struct PreparedAccess {
    pub access_token: String,
    pub access_expires_at: String,
    pub rotation_id: Option<String>,
}

impl fmt::Debug for PreparedAccess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedAccess")
            .field("access_token", &"[REDACTED]")
            .field("access_expires_at", &self.access_expires_at)
            .field("rotation_id", &self.rotation_id)
            .finish()
    }
}

#[derive(Debug, Error)]
pub enum TokenRefreshError {
    #[error("Agent 凭证文件无效")]
    Credential(#[from] CredentialError),
    #[error("Agent token 刷新请求失败")]
    Transport,
    #[error("Agent token 刷新被拒绝")]
    Rejected,
    #[error("Agent token 刷新响应无效")]
    InvalidResponse,
    #[error("Agent token 轮换状态冲突")]
    StateConflict,
}

#[async_trait]
pub trait AccessProvider: Send + Sync {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError>;
    async fn commit(&self, rotation_id: &str) -> Result<(), TokenRefreshError>;
}

#[async_trait]
pub trait TokenRefresher: Send + Sync {
    async fn refresh(
        &self,
        refresh_token: &str,
        rotation_id: &str,
    ) -> Result<TokenPair, TokenRefreshError>;
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenPair {
    pub agent_id: String,
    pub rotation_id: String,
    pub access_token: String,
    pub access_expires_at: String,
    pub refresh_token: String,
    pub refresh_expires_at: String,
}

impl fmt::Debug for TokenPair {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenPair")
            .field("agent_id", &self.agent_id)
            .field("rotation_id", &self.rotation_id)
            .field("access_token", &"[REDACTED]")
            .field("access_expires_at", &self.access_expires_at)
            .field("refresh_token", &"[REDACTED]")
            .field("refresh_expires_at", &self.refresh_expires_at)
            .finish()
    }
}

#[derive(Clone)]
pub struct HttpTokenRefresher {
    client: reqwest::Client,
    endpoint: Url,
}

#[derive(Serialize)]
struct RefreshBody<'a> {
    refresh_token: &'a str,
    rotation_id: &'a str,
}

impl HttpTokenRefresher {
    pub fn new(endpoint: Url) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("固定 HTTP client 配置有效"),
            endpoint,
        }
    }
}

#[async_trait]
impl TokenRefresher for HttpTokenRefresher {
    async fn refresh(
        &self,
        refresh_token: &str,
        rotation_id: &str,
    ) -> Result<TokenPair, TokenRefreshError> {
        let response = self
            .client
            .post(self.endpoint.clone())
            .json(&RefreshBody {
                refresh_token,
                rotation_id,
            })
            .send()
            .await
            .map_err(|_| TokenRefreshError::Transport)?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(TokenRefreshError::Rejected);
        }
        if !response.status().is_success() {
            return Err(TokenRefreshError::Transport);
        }
        response
            .json()
            .await
            .map_err(|_| TokenRefreshError::InvalidResponse)
    }
}

pub struct CredentialAccessProvider {
    store: CredentialStore,
    refresher: Arc<dyn TokenRefresher>,
    state_lock: Mutex<()>,
}

impl CredentialAccessProvider {
    pub fn new(store: CredentialStore, refresher: Arc<dyn TokenRefresher>) -> Self {
        Self {
            store,
            refresher,
            state_lock: Mutex::new(()),
        }
    }
}

#[async_trait]
impl AccessProvider for CredentialAccessProvider {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        let _guard = self.state_lock.lock().await;
        let mut credentials = self.store.load()?;
        if credentials.pending_rotation.is_none() {
            credentials.pending_rotation = Some(PendingRotation {
                rotation_id: format!("rotation_{}", Ulid::new()),
                next_refresh_token: None,
                access_token: None,
                access_expires_at: None,
            });
            self.store.store(&credentials)?;
        }
        if let Some(access) = cached_pending_access(&credentials)? {
            return Ok(access);
        }
        let rotation_id = credentials
            .pending_rotation
            .as_ref()
            .ok_or(TokenRefreshError::StateConflict)?
            .rotation_id
            .clone();
        let pair = self
            .refresher
            .refresh(&credentials.refresh_token, &rotation_id)
            .await?;
        validate_pair(
            &credentials,
            credentials
                .pending_rotation
                .as_ref()
                .ok_or(TokenRefreshError::StateConflict)?,
            &pair,
        )?;
        let mut updated = credentials.clone();
        let pending = updated
            .pending_rotation
            .as_mut()
            .ok_or(TokenRefreshError::StateConflict)?;
        if let Some(expected) = &pending.next_refresh_token {
            if expected != &pair.refresh_token {
                return Err(TokenRefreshError::StateConflict);
            }
        }
        pending.next_refresh_token = Some(pair.refresh_token.clone());
        pending.access_token = Some(pair.access_token.clone());
        pending.access_expires_at = Some(pair.access_expires_at.clone());
        self.store.store(&updated)?;
        Ok(PreparedAccess {
            access_token: pair.access_token,
            access_expires_at: pair.access_expires_at,
            rotation_id: Some(rotation_id),
        })
    }

    async fn commit(&self, rotation_id: &str) -> Result<(), TokenRefreshError> {
        let _guard = self.state_lock.lock().await;
        let credentials = self.store.load()?;
        let pending = credentials
            .pending_rotation
            .as_ref()
            .filter(|pending| pending.rotation_id == rotation_id)
            .ok_or(TokenRefreshError::StateConflict)?;
        let next_refresh_token = pending
            .next_refresh_token
            .clone()
            .ok_or(TokenRefreshError::StateConflict)?;
        self.store.store(&AgentCredentials {
            agent_id: credentials.agent_id,
            refresh_token: next_refresh_token,
            pending_rotation: None,
        })?;
        Ok(())
    }
}

fn cached_pending_access(
    credentials: &AgentCredentials,
) -> Result<Option<PreparedAccess>, TokenRefreshError> {
    let Some(pending) = credentials.pending_rotation.as_ref() else {
        return Ok(None);
    };
    let (Some(access_token), Some(access_expires_at)) =
        (pending.access_token.as_ref(), pending.access_expires_at.as_ref())
    else {
        return Ok(None);
    };
    let expires_at = DateTime::parse_from_rfc3339(access_expires_at)
        .map_err(|_| TokenRefreshError::StateConflict)?;
    if expires_at <= Utc::now() {
        return Ok(None);
    }
    Ok(Some(PreparedAccess {
        access_token: access_token.clone(),
        access_expires_at: access_expires_at.clone(),
        rotation_id: Some(pending.rotation_id.clone()),
    }))
}

fn validate_pair(
    credentials: &AgentCredentials,
    pending: &PendingRotation,
    pair: &TokenPair,
) -> Result<(), TokenRefreshError> {
    let access_expiry = DateTime::parse_from_rfc3339(&pair.access_expires_at)
        .map_err(|_| TokenRefreshError::InvalidResponse)?;
    let refresh_expiry = DateTime::parse_from_rfc3339(&pair.refresh_expires_at)
        .map_err(|_| TokenRefreshError::InvalidResponse)?;
    if pair.agent_id != credentials.agent_id
        || pair.rotation_id != pending.rotation_id
        || pair.access_token.len() < 32
        || pair.refresh_token.len() < 32
        || pair.access_token.chars().any(char::is_whitespace)
        || pair.refresh_token.chars().any(char::is_whitespace)
        || access_expiry <= Utc::now()
        || refresh_expiry <= Utc::now()
    {
        return Err(TokenRefreshError::InvalidResponse);
    }
    Ok(())
}
