use std::sync::Arc;

use anyhow::Context;
use deploy_go_agent::{
    artifact_transfer::ArtifactTransferClient,
    config::Config,
    connection::{ConnectionClient, TokioWebSocketConnector},
    credential_store::CredentialStore,
    executor::Executor,
    storage_cleanup::StorageCleanup,
    system_info,
    task_handler::TaskHandler,
    telemetry::LinuxTelemetryFactory,
    terminal::TerminalBridge,
    token_refresh::{CredentialAccessProvider, HttpTokenRefresher},
};
use deploy_go_agent_executor::protocol::ExecutorCapability;
use deploy_go_agent_protocol::{
    AgentCapability, Hello, MIN_SUPPORTED_PROTOCOL_VERSION, PROTOCOL_VERSION,
};
use tracing_subscriber::EnvFilter;

const MAX_AGENT_WORKER_THREADS: usize = 4;
const MAX_AGENT_BLOCKING_THREADS: usize = 16;

fn main() -> anyhow::Result<()> {
    build_runtime()?.block_on(agent_main())
}

fn build_runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    if runtime_is_single_threaded(
        deploy_go_agent::diagnostics::Command::from_args(),
        std::env::args().nth(1).as_deref(),
    ) {
        return tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("初始化单线程 Agent runtime 失败");
    }

    // 主 Agent 同时承担 WSS、任务、制品传输和遥测；保留少量 worker 满足并发，
    // 但不再跟随主机 CPU 数量创建整套线程。
    let worker_threads = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(MAX_AGENT_WORKER_THREADS)
        .clamp(1, MAX_AGENT_WORKER_THREADS);
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(MAX_AGENT_BLOCKING_THREADS)
        .enable_all()
        .build()
        .context("初始化多线程 Agent runtime 失败")
}

fn runtime_is_single_threaded(
    diagnostic: Option<deploy_go_agent::diagnostics::Command>,
    command: Option<&str>,
) -> bool {
    diagnostic.is_some()
        || matches!(
            command,
            Some(
                "runner-service"
                    | "executor-probe"
                    | "executor-release-probe"
                    | "privileged-release-self-test"
                    | "runner-probe"
                    | "runner"
                    | "runner-stdin"
                    | "runner-cancel"
            )
        )
}

async fn agent_main() -> anyhow::Result<()> {
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
    if std::env::args().nth(1).as_deref() == Some("executor-release-probe") {
        let client = deploy_go_agent::executor_client::ExecutorClient::new(
            deploy_go_agent::executor_client::DEFAULT_EXECUTOR_SOCKET_PATH.into(),
        );
        let capabilities = client
            .probe_capabilities()
            .await
            .ok_or_else(|| anyhow::anyhow!("root executor unavailable or incompatible"))?;
        if !capabilities
            .contains(&deploy_go_agent_executor::protocol::ExecutorCapability::DeploymentRelease)
        {
            anyhow::bail!("root executor deployment release capability unavailable");
        }
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("privileged-release-self-test") {
        let client = deploy_go_agent::executor_client::ExecutorClient::new(
            deploy_go_agent::executor_client::DEFAULT_EXECUTOR_SOCKET_PATH.into(),
        );
        match client
            .request(deploy_go_agent_executor::protocol::Request::SelfTest(
                deploy_go_agent_executor::protocol::SelfTestRequest {
                    version: deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
                },
            ))
            .await
        {
            Ok(deploy_go_agent_executor::protocol::Response::SelfTestResult(result))
                if result.succeeded =>
            {
                print!("{}", result.output);
                return Ok(());
            }
            _ => anyhow::bail!("privileged release self-test failed"),
        }
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
    let shared_http_client =
        deploy_go_agent::http_client::new_agent_client(std::time::Duration::from_secs(900));
    let access_provider = Arc::new(CredentialAccessProvider::new(
        credential_store,
        Arc::new(HttpTokenRefresher::with_client(
            config.refresh_url.clone(),
            shared_http_client.clone(),
        )),
    ));
    let mut artifact_api_base = config.refresh_url.clone();
    artifact_api_base.set_path("/");
    artifact_api_base.set_query(None);
    let tasks_root = config.data_dir.join("tasks");
    let executor_client = deploy_go_agent::executor_client::ExecutorClient::new(
        deploy_go_agent::executor_client::DEFAULT_EXECUTOR_SOCKET_PATH.into(),
    );
    let mut task_handler = TaskHandler::new(
        Executor::new(tasks_root.clone())?
            .with_data_dir(config.data_dir.clone())
            .with_runner_service(deploy_go_agent::runner_service::DEFAULT_RUNNER_SOCKET_PATH.into())
            .with_staging_limits(config.staging_size_limit_bytes, config.staging_max_files),
    )
    .with_artifact_transfer(ArtifactTransferClient::with_client(
        artifact_api_base.clone(),
        access_provider.clone(),
        config.artifact_transfer_enabled,
        shared_http_client.clone(),
    ))
    .with_privileged_release_executor(executor_client.clone())
    .with_runtime_probe_client(deploy_go_agent::http_client::new_runtime_probe_client());
    if config.env_sync_enabled {
        // 已由安装器以 2700 创建时保持原样；临时环境可退化为 0700。
        // 不能无条件 chmod 2700：systemd RestrictSUIDSGID 会拒绝 setgid 位。
        deploy_go_agent::dir_guard::ensure_directory_mode(
            &config.secrets_root,
            0o700,
            &[0o2700, 0o700],
        )?;
        task_handler = task_handler.with_env_sync(
            deploy_go_agent::env_sync::EnvSecretClient::with_client(
                artifact_api_base,
                access_provider.clone(),
                true,
                shared_http_client,
            ),
            deploy_go_agent::env_sync::EnvFileStore::new(config.secrets_root.clone())?,
        );
    }
    let terminal = Arc::new(TerminalBridge::new(
        deploy_go_agent::executor_client::DEFAULT_EXECUTOR_SOCKET_PATH.into(),
    ));
    let executor_capabilities = executor_client
        .probe_capabilities()
        .await
        .unwrap_or_default();
    if !executor_capabilities.contains(&ExecutorCapability::PtyTerminal)
        || !executor_capabilities.contains(&ExecutorCapability::DeploymentRelease)
    {
        anyhow::bail!("root executor missing required PTY or deployment release capability");
    }
    let capabilities = vec![
        AgentCapability::PtyTerminal,
        AgentCapability::PrivilegedRelease,
        AgentCapability::SecretEnvironmentV1,
        AgentCapability::RuntimeProbeV1,
    ];
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
    .with_terminal_bridge(terminal)
    .with_telemetry_factory(Arc::new(LinuxTelemetryFactory::new(config.work_root)));
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let signal = tokio::spawn(async move {
        if shutdown_signal().await.is_ok() {
            let _ = shutdown_tx.send(true);
        }
    });
    let storage = StorageCleanup::new(
        config.data_dir.clone(),
        config.task_retention,
        config.deployment_retention,
    );
    let storage_shutdown = shutdown_rx.clone();
    let storage_task = tokio::spawn(async move {
        storage_cleanup_loop(storage, config.storage_cleanup_interval, storage_shutdown).await;
    });
    client.run(shutdown_rx).await;
    signal.abort();
    storage_task.abort();
    Ok(())
}

async fn storage_cleanup_loop(
    cleanup: StorageCleanup,
    interval: std::time::Duration,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    if interval.is_zero() {
        return;
    }
    if !*shutdown.borrow() {
        storage_cleanup_once(cleanup.clone()).await;
    }
    if *shutdown.borrow() {
        return;
    }
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
        }
        if *shutdown.borrow() {
            return;
        }
        storage_cleanup_once(cleanup.clone()).await;
    }
}

async fn storage_cleanup_once(cleanup: StorageCleanup) {
    let report = tokio::task::spawn_blocking(move || cleanup.run_once())
        .await
        .unwrap_or_default();
    tracing::debug!(
        removed_task_dirs = report.removed_task_dirs,
        removed_deployment_dirs = report.removed_deployment_dirs,
        removed_checkout_dirs = report.removed_checkout_dirs,
        removed_artifact_dirs = report.removed_artifact_dirs,
        "执行节点存储回收完成"
    );
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

#[cfg(test)]
mod tests {
    use super::runtime_is_single_threaded;

    #[test]
    fn short_lived_commands_use_single_thread_runtime() {
        assert!(runtime_is_single_threaded(
            Some(deploy_go_agent::diagnostics::Command::Status),
            None
        ));
        for command in [
            "runner-service",
            "executor-probe",
            "executor-release-probe",
            "privileged-release-self-test",
            "runner-probe",
            "runner",
            "runner-stdin",
            "runner-cancel",
        ] {
            assert!(runtime_is_single_threaded(None, Some(command)), "{command}");
        }
    }

    #[test]
    fn agent_service_uses_bounded_multi_thread_runtime() {
        assert!(!runtime_is_single_threaded(None, None));
        assert!(!runtime_is_single_threaded(None, Some("unknown")));
    }
}
