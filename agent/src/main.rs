use std::sync::Arc;

use anyhow::Context;
use deploy_go_agent::{
    artifact_transfer::ArtifactTransferClient,
    config::Config,
    connection::{ConnectionClient, TokioWebSocketConnector},
    credential_store::CredentialStore,
    executor::Executor,
    system_info,
    task_handler::TaskHandler,
    terminal::TerminalBridge,
    token_refresh::{CredentialAccessProvider, HttpTokenRefresher},
};
use deploy_go_agent_protocol::{
    AgentCapability, Hello, MIN_SUPPORTED_PROTOCOL_VERSION, PROTOCOL_VERSION,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Some(command) = deploy_go_agent::diagnostics::Command::from_args() {
        std::process::exit(deploy_go_agent::diagnostics::run(command).await);
    }
    if std::env::args().nth(1).as_deref() == Some("runner-service") {
        return deploy_go_agent::runner_service::serve_from_env()
            .await
            .context("运行 durable runner service 失败");
    }
    if std::env::args().nth(1).as_deref() == Some("executor-probe") {
        let client = deploy_go_agent::executor_client::ExecutorClient::new(
            deploy_go_agent::executor_client::DEFAULT_EXECUTOR_SOCKET_PATH.into(),
        );
        if !client.probe().await {
            anyhow::bail!("root executor unavailable or incompatible");
        }
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("runner-probe") {
        let client = deploy_go_agent::runner_service::RunnerServiceClient::new(
            deploy_go_agent::runner_service::DEFAULT_RUNNER_SOCKET_PATH.into(),
        );
        if !client.probe().await {
            anyhow::bail!("runner broker unavailable or incompatible");
        }
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("runner") {
        return deploy_go_agent::runner::run_from_args()
            .await
            .context("执行 durable runner 失败");
    }
    if std::env::args().nth(1).as_deref() == Some("runner-stdin") {
        return deploy_go_agent::runner::run_from_stdin_args()
            .await
            .context("执行 durable runner 失败");
    }
    if std::env::args().nth(1).as_deref() == Some("runner-cancel") {
        return deploy_go_agent::runner_service::run_cancel_from_args()
            .await
            .context("取消 durable runner 失败");
    }
    #[cfg(target_os = "linux")]
    if unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0 {
        anyhow::bail!("无法禁用 Agent 进程转储");
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
        Arc::new(HttpTokenRefresher::new(config.refresh_url.clone())),
    ));
    let mut artifact_api_base = config.refresh_url.clone();
    artifact_api_base.set_path("/");
    artifact_api_base.set_query(None);
    let mut task_handler = TaskHandler::new(
        Executor::new(config.data_dir.join("tasks"))?
            .with_runner_service(deploy_go_agent::runner_service::DEFAULT_RUNNER_SOCKET_PATH.into())
            .with_staging_limits(config.staging_size_limit_bytes, config.staging_max_files),
    )
    .with_artifact_transfer(ArtifactTransferClient::new(
        artifact_api_base.clone(),
        access_provider.clone(),
        config.artifact_transfer_enabled,
    ));
    if config.env_sync_enabled {
        std::fs::create_dir_all(&config.secrets_root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &config.secrets_root,
                std::fs::Permissions::from_mode(0o2700),
            )?;
        }
        task_handler = task_handler.with_env_sync(
            deploy_go_agent::env_sync::EnvSecretClient::new(
                artifact_api_base,
                access_provider.clone(),
                true,
            ),
            deploy_go_agent::env_sync::EnvFileStore::new(config.secrets_root.clone())?,
        );
    }
    let terminal = Arc::new(TerminalBridge::new(
        deploy_go_agent::executor_client::DEFAULT_EXECUTOR_SOCKET_PATH.into(),
    ));
    let capabilities = if terminal.probe().await {
        vec![AgentCapability::PtyTerminal]
    } else {
        tracing::info!("root executor unavailable or incompatible; terminal capability disabled");
        vec![]
    };
    let client = ConnectionClient::with_access_provider(
        Arc::new(TokioWebSocketConnector),
        Arc::new(task_handler),
        config.control_url,
        access_provider,
        Hello {
            agent_id: credentials.agent_id,
            agent_version: env!("CARGO_PKG_VERSION").to_owned(),
            min_protocol_version: MIN_SUPPORTED_PROTOCOL_VERSION,
            max_protocol_version: PROTOCOL_VERSION,
            os: system.os,
            architecture: system.architecture,
            capabilities,
        },
    )
    .with_terminal_bridge(terminal);
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
