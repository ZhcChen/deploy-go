use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::Utc;
use deploy_go_agent_protocol::{
    AuthRefresh, Envelope, Heartbeat, Hello, MIN_SUPPORTED_PROTOCOL_VERSION, Message,
    PROTOCOL_VERSION,
};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use thiserror::Error;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{
        Message as WebSocketMessage,
        client::IntoClientRequest,
        http::{HeaderValue, header::AUTHORIZATION},
        protocol::WebSocketConfig,
    },
};
use ulid::Ulid;
use url::Url;

use crate::token_refresh::{AccessProvider, PreparedAccess, TokenRefreshError};

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("WebSocket 请求无效")]
    InvalidRequest,
    #[error("WebSocket 连接失败")]
    Transport,
    #[error("控制协议消息无效")]
    InvalidMessage,
    #[error("控制协议版本不兼容")]
    IncompatibleProtocol,
    #[error("主控未按顺序返回 hello_ack")]
    MissingHelloAck,
    #[error("主控关闭连接")]
    Closed,
    #[error("Agent token 刷新失败")]
    TokenRefresh,
    #[error("主控未确认 Agent token 轮换")]
    MissingAuthConfirmation,
}

#[async_trait]
pub trait ControlSession: Send {
    async fn send(&mut self, envelope: &Envelope) -> Result<(), ConnectionError>;
    async fn receive(&mut self) -> Result<Option<Envelope>, ConnectionError>;
}

#[async_trait]
pub trait ControlConnector: Send + Sync {
    async fn connect(
        &self,
        url: &Url,
        access_token: &str,
    ) -> Result<Box<dyn ControlSession>, ConnectionError>;
}

#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle(
        &self,
        envelope: Envelope,
        outbound: mpsc::Sender<Message>,
    ) -> Result<(), ConnectionError>;

    fn active_task_ids(&self) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct TokioWebSocketConnector;

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct TokioSession(Socket);

#[async_trait]
impl ControlConnector for TokioWebSocketConnector {
    async fn connect(
        &self,
        url: &Url,
        access_token: &str,
    ) -> Result<Box<dyn ControlSession>, ConnectionError> {
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|_| ConnectionError::InvalidRequest)?;
        let authorization = HeaderValue::from_str(&format!("Bearer {access_token}"))
            .map_err(|_| ConnectionError::InvalidRequest)?;
        request.headers_mut().insert(AUTHORIZATION, authorization);
        let config = WebSocketConfig::default()
            .max_message_size(Some(1024 * 1024))
            .max_frame_size(Some(1024 * 1024));
        let (socket, _) = connect_async_with_config(request, Some(config), false)
            .await
            .map_err(|_| ConnectionError::Transport)?;
        Ok(Box::new(TokioSession(socket)))
    }
}

#[async_trait]
impl ControlSession for TokioSession {
    async fn send(&mut self, envelope: &Envelope) -> Result<(), ConnectionError> {
        let text = serde_json::to_string(envelope).map_err(|_| ConnectionError::InvalidMessage)?;
        self.0
            .send(WebSocketMessage::Text(text.into()))
            .await
            .map_err(|_| ConnectionError::Transport)
    }

    async fn receive(&mut self) -> Result<Option<Envelope>, ConnectionError> {
        loop {
            let Some(message) = self.0.next().await else {
                return Ok(None);
            };
            match message.map_err(|_| ConnectionError::Transport)? {
                WebSocketMessage::Text(text) => {
                    let envelope = serde_json::from_str::<Envelope>(&text)
                        .map_err(|_| ConnectionError::InvalidMessage)?;
                    envelope
                        .validate_version()
                        .map_err(|_| ConnectionError::IncompatibleProtocol)?;
                    return Ok(Some(envelope));
                }
                WebSocketMessage::Ping(bytes) => self
                    .0
                    .send(WebSocketMessage::Pong(bytes))
                    .await
                    .map_err(|_| ConnectionError::Transport)?,
                WebSocketMessage::Close(_) => return Ok(None),
                WebSocketMessage::Binary(_) => return Err(ConnectionError::InvalidMessage),
                WebSocketMessage::Pong(_) | WebSocketMessage::Frame(_) => {}
            }
        }
    }
}

#[derive(Clone)]
pub struct ConnectionClient {
    connector: Arc<dyn ControlConnector>,
    handler: Arc<dyn MessageHandler>,
    url: Url,
    access_provider: Arc<dyn AccessProvider>,
    hello: Hello,
    backoff: Backoff,
}

impl ConnectionClient {
    pub fn new(
        connector: Arc<dyn ControlConnector>,
        handler: Arc<dyn MessageHandler>,
        url: Url,
        access_token: impl Into<Arc<str>>,
        hello: Hello,
    ) -> Self {
        let access_token = access_token.into();
        Self {
            connector,
            handler,
            url,
            access_provider: Arc::new(StaticAccessProvider { access_token }),
            hello,
            backoff: Backoff::default(),
        }
    }

    pub fn with_access_provider(
        connector: Arc<dyn ControlConnector>,
        handler: Arc<dyn MessageHandler>,
        url: Url,
        access_provider: Arc<dyn AccessProvider>,
        hello: Hello,
    ) -> Self {
        Self {
            connector,
            handler,
            url,
            access_provider,
            hello,
            backoff: Backoff::default(),
        }
    }

    pub fn with_backoff(mut self, backoff: Backoff) -> Self {
        self.backoff = backoff;
        self
    }

    pub async fn run(&self, mut shutdown: watch::Receiver<bool>) {
        let mut attempts = 0_u32;
        while !*shutdown.borrow() {
            match self.run_once(&mut shutdown).await {
                Ok(()) if *shutdown.borrow() => break,
                Ok(()) => {
                    attempts = 0;
                    let delay = self.backoff.delay(attempts);
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        _ = shutdown.changed() => {}
                    }
                }
                Err(_) => {
                    let delay = self.backoff.delay(attempts);
                    attempts = attempts.saturating_add(1);
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        _ = shutdown.changed() => {}
                    }
                }
            }
        }
    }

    pub async fn run_once(
        &self,
        shutdown: &mut watch::Receiver<bool>,
    ) -> Result<(), ConnectionError> {
        let mut access = self
            .access_provider
            .prepare()
            .await
            .map_err(|_| ConnectionError::TokenRefresh)?;
        let mut session = self
            .connector
            .connect(&self.url, &access.access_token)
            .await?;
        session
            .send(&envelope(Message::Hello(self.hello.clone())))
            .await?;
        let hello_ack = session.receive().await?.ok_or(ConnectionError::Closed)?;
        let Message::HelloAck(hello_ack) = hello_ack.message else {
            return Err(ConnectionError::MissingHelloAck);
        };
        let negotiated_version = hello_ack.protocol_version;
        if !(MIN_SUPPORTED_PROTOCOL_VERSION..=PROTOCOL_VERSION)
            .contains(&hello_ack.protocol_version)
            || !(5..=300).contains(&hello_ack.heartbeat_interval_seconds)
        {
            return Err(ConnectionError::IncompatibleProtocol);
        }
        let mut heartbeat = tokio::time::interval(Duration::from_secs(u64::from(
            hello_ack.heartbeat_interval_seconds,
        )));
        heartbeat.tick().await;
        let mut pending_rotation = access.rotation_id.clone();
        let mut confirmation_deadline = None;
        if let Some(rotation_id) = &pending_rotation {
            session
                .send(&envelope_version(
                    negotiated_version,
                    Message::AuthRefresh(AuthRefresh {
                        access_token: access.access_token.clone(),
                        rotation_id: rotation_id.clone(),
                    }),
                ))
                .await?;
            confirmation_deadline = Some(tokio::time::Instant::now() + Duration::from_secs(10));
        }
        let mut refresh_check = tokio::time::interval(Duration::from_secs(30));
        refresh_check.tick().await;
        let mut confirmation_check = tokio::time::interval(Duration::from_secs(1));
        confirmation_check.tick().await;
        let (outbound_tx, mut outbound_rx) = mpsc::channel(64);
        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    session.send(&envelope_version(negotiated_version, Message::Heartbeat(Heartbeat {
                        connection_generation: hello_ack.connection_generation,
                        active_task_ids: self.handler.active_task_ids(),
                    }))).await?;
                }
                message = session.receive() => {
                    let Some(message) = message? else {
                        return Ok(());
                    };
                    if let Message::AuthRefreshed(confirmation) = &message.message {
                        if pending_rotation.as_deref() != Some(confirmation.rotation_id.as_str()) {
                            return Err(ConnectionError::MissingAuthConfirmation);
                        }
                        self.access_provider.commit(&confirmation.rotation_id).await.map_err(|_| ConnectionError::TokenRefresh)?;
                        access.access_expires_at = confirmation.access_expires_at.clone();
                        access.rotation_id = None;
                        pending_rotation = None;
                        confirmation_deadline = None;
                    } else {
                        self.handler.handle(message, outbound_tx.clone()).await?;
                    }
                }
                Some(message) = outbound_rx.recv() => {
                    session.send(&envelope_version(negotiated_version, message)).await?;
                }
                _ = refresh_check.tick(), if pending_rotation.is_none() && should_refresh(&access.access_expires_at) => {
                    match self.access_provider.prepare().await {
                        Ok(next) => {
                            let rotation_id = next.rotation_id.clone().ok_or(ConnectionError::MissingAuthConfirmation)?;
                            session.send(&envelope_version(negotiated_version, Message::AuthRefresh(AuthRefresh {
                                access_token: next.access_token.clone(),
                                rotation_id: rotation_id.clone(),
                            }))).await?;
                            access = next;
                            pending_rotation = Some(rotation_id);
                            confirmation_deadline = Some(tokio::time::Instant::now() + Duration::from_secs(10));
                        }
                        Err(_) if access_expired(&access.access_expires_at) => return Err(ConnectionError::TokenRefresh),
                        Err(_) => {}
                    }
                }
                _ = confirmation_check.tick(), if confirmation_deadline.is_some() => {
                    if confirmation_deadline.is_some_and(|deadline| tokio::time::Instant::now() >= deadline) {
                        return Err(ConnectionError::MissingAuthConfirmation);
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }
}

struct StaticAccessProvider {
    access_token: Arc<str>,
}

#[async_trait]
impl AccessProvider for StaticAccessProvider {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        Ok(PreparedAccess {
            access_token: self.access_token.to_string(),
            access_expires_at: "9999-12-31T23:59:59Z".to_owned(),
            rotation_id: None,
        })
    }

    async fn commit(&self, _rotation_id: &str) -> Result<(), TokenRefreshError> {
        Err(TokenRefreshError::StateConflict)
    }
}

fn should_refresh(expires_at: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(expires_at)
        .map(|expires_at| expires_at <= chrono::Utc::now() + chrono::Duration::minutes(5))
        .unwrap_or(true)
}

fn access_expired(expires_at: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(expires_at)
        .map(|expires_at| expires_at <= chrono::Utc::now())
        .unwrap_or(true)
}

#[derive(Clone, Debug)]
pub struct Backoff {
    base: Duration,
    maximum: Duration,
    jitter_ratio: f64,
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(60), 0.2)
    }
}

impl Backoff {
    pub fn new(base: Duration, maximum: Duration, jitter_ratio: f64) -> Self {
        Self {
            base,
            maximum,
            jitter_ratio: jitter_ratio.clamp(0.0, 0.5),
        }
    }

    pub fn delay(&self, attempt: u32) -> Duration {
        let multiplier = 2_u32.saturating_pow(attempt.min(16));
        let nominal = self.base.saturating_mul(multiplier).min(self.maximum);
        let jitter = rand::rng().random_range(-self.jitter_ratio..=self.jitter_ratio);
        nominal.mul_f64(1.0 + jitter)
    }
}

pub fn envelope(message: Message) -> Envelope {
    envelope_version(PROTOCOL_VERSION, message)
}

pub fn envelope_version(protocol_version: u16, message: Message) -> Envelope {
    Envelope {
        protocol_version,
        message_id: format!("msg_{}", Ulid::new()),
        sent_at: Utc::now().to_rfc3339(),
        message,
    }
}
