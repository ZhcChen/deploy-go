use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use deploy_go_agent::{
    config::Config,
    connection::{ConnectionClient, ConnectionError, MessageHandler, TokioWebSocketConnector},
    credential_store::CredentialStore,
    system_info,
    token_refresh::{CredentialAccessProvider, HttpTokenRefresher},
};
use deploy_go_agent_protocol::{
    Envelope, Hello, MIN_SUPPORTED_PROTOCOL_VERSION, Message, PROTOCOL_VERSION,
};
use tracing_subscriber::EnvFilter;

struct ControlHandler;

#[async_trait]
impl MessageHandler for ControlHandler {
    async fn handle(&self, envelope: Envelope) -> Result<(), ConnectionError> {
        match envelope.message {
            Message::HeartbeatAck(_) | Message::ProtocolError(_) => Ok(()),
            _ => Err(ConnectionError::InvalidMessage),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("runner") {
        return deploy_go_agent::runner::run_from_args()
            .await
            .context("执行 durable runner 失败");
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env().context("加载 Agent 配置失败")?;
    let credential_store = CredentialStore::new(config.credential_file.clone());
    let credentials = credential_store.load().context("加载 Agent 凭证失败")?;
    let system = system_info::collect();
    tracing::info!(
        agent_id = %credentials.agent_id,
        os = %system.os,
        architecture = %system.architecture,
        control_url = %config.control_url,
        "Deploy Go Agent initialized"
    );
    let access_provider = Arc::new(CredentialAccessProvider::new(
        credential_store,
        Arc::new(HttpTokenRefresher::new(config.refresh_url)),
    ));
    let client = ConnectionClient::with_access_provider(
        Arc::new(TokioWebSocketConnector),
        Arc::new(ControlHandler),
        config.control_url,
        access_provider,
        Hello {
            agent_id: credentials.agent_id,
            agent_version: env!("CARGO_PKG_VERSION").to_owned(),
            min_protocol_version: MIN_SUPPORTED_PROTOCOL_VERSION,
            max_protocol_version: PROTOCOL_VERSION,
            os: system.os,
            architecture: system.architecture,
        },
    );
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let signal = tokio::spawn(async move {
        if shutdown_signal().await.is_ok() {
            let _ = shutdown_tx.send(true);
        }
    });
    client.run(shutdown_rx).await;
    signal.abort();
    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() -> std::io::Result<()> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() -> std::io::Result<()> {
    tokio::signal::ctrl_c().await
}
