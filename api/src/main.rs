use std::str::FromStr;

use anyhow::Context;
use deploy_go_api::{
    AppState, app, config::Config, crypto::MasterKeyRing, db, deployer::DeployerInstallation,
    git_credentials, http::shutdown_signal, ssh_credentials,
};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (runtime_logs, runtime_log_layer) = deploy_go_api::runtime_logs::RuntimeLogStore::start();
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .with(runtime_log_layer)
        .init();

    let process_mode = std::env::args().nth(1);
    if matches!(
        process_mode.as_deref(),
        Some("openapi" | "openapi-check" | "external-openapi" | "external-openapi-check")
    ) {
        return handle_openapi(process_mode.as_deref().unwrap());
    }

    let config = Config::from_env().context("加载配置失败")?;
    let connect_options = SqliteConnectOptions::from_str(&config.database_url)
        .context("解析 SQLite URL 失败")?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5))
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
    let pool = SqlitePool::connect_with(connect_options)
        .await
        .context("连接 SQLite 失败")?;
    db::migrate(&pool)
        .await
        .context("执行数据库 migration 失败")?;
    if process_mode.as_deref() == Some("migrate") {
        tracing::info!("database migrations completed");
        return Ok(());
    }

    let master_key_ring = MasterKeyRing::from_env().context("加载 SSH 凭证主密钥失败")?;
    if process_mode.as_deref() == Some("credential-reencrypt") {
        let migrated = ssh_credentials::reencrypt_all(&pool, &master_key_ring)
            .await
            .context("重加密 SSH 凭证失败")?;
        let git_migrated = git_credentials::reencrypt_all(&pool, &master_key_ring)
            .await
            .context("重加密 Git 凭证失败")?;
        tracing::info!(migrated, git_migrated, "credential re-encryption completed");
        return Ok(());
    }
    let terminal_signer = deploy_go_api::terminal_capability::signer_from_env()
        .context("加载终端 capability 签名密钥失败")?;
    let release_signer = deploy_go_api::release_authorization::signer_from_env()
        .context("加载特权发布签名密钥失败")?;

    deploy_go_api::agents::websocket::reset_online_state(&pool)
        .await
        .context("重置 Agent 节点连接状态失败")?;
    deploy_go_api::terminals::interrupt_active_sessions(&pool)
        .await
        .context("收敛 API 重启前终端会话失败")?;

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .context("监听 API 地址失败")?;
    tracing::info!(address = %config.bind_addr, "Deploy Go API started");

    let agent_installation = if let Some(release) = &config.agent_release {
        Some(
            deploy_go_api::agents::AgentInstallation::from_dir(
                release.public_base_url.clone(),
                release.release_dir.clone(),
            )
            .context("加载 Agent 发布配置失败")?,
        )
    } else {
        None
    };
    let deployer_installation = if let Some(release) = &config.deployer_release {
        Some(
            DeployerInstallation::from_dir(
                release.public_base_url.clone(),
                release.release_dir.clone(),
            )
            .context("加载 deployer 发布配置失败")?,
        )
    } else {
        None
    };

    let mut state = AppState::with_runtime_logs(pool, runtime_logs)
        .with_allowed_origins(config.allowed_origins)
        .with_cookie_secure(config.cookie_secure)
        .with_master_key_ring(master_key_ring)
        .with_terminal_signer(terminal_signer)
        .with_release_signer(release_signer)
        .with_cross_node_artifacts_enabled(config.cross_node_artifacts_enabled)
        .with_artifact_store(
            deploy_go_api::artifacts::ArtifactStore::initialize(config.artifacts)
                .context("初始化制品存储失败")?,
        );
    if let Some(agent_installation) = agent_installation {
        state = state.with_agent_installation(agent_installation);
    }
    if let Some(deployer_installation) = deployer_installation {
        state = state.with_deployer_installation(deployer_installation);
    }
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let signal_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = signal_tx.send(true);
    });
    let worker = tokio::spawn(deploy_go_api::deployments::run_worker(
        state.clone(),
        shutdown_rx.clone(),
    ));

    axum::serve(listener, app(state))
        .with_graceful_shutdown(wait_for_shutdown(shutdown_rx))
        .await
        .context("API 服务异常退出")?;
    let _ = shutdown_tx.send(true);
    tokio::time::timeout(std::time::Duration::from_secs(5), worker)
        .await
        .context("等待部署 worker 停止超时")?
        .context("部署 worker 异常退出")?;

    Ok(())
}

async fn wait_for_shutdown(mut shutdown: tokio::sync::watch::Receiver<bool>) {
    if *shutdown.borrow() {
        return;
    }
    let _ = shutdown.changed().await;
}

fn handle_openapi(mode: &str) -> anyhow::Result<()> {
    let internal = deploy_go_api::openapi_document();
    let external = deploy_go_api::external::external_openapi_document();
    match mode {
        "openapi" => {
            write_openapi("api/openapi/openapi.json", &internal)?;
            write_openapi("api/openapi/external.json", &external)?;
        }
        "openapi-check" => {
            check_openapi("api/openapi/openapi.json", &internal)?;
            check_openapi("api/openapi/external.json", &external)?;
        }
        "external-openapi" => {
            write_openapi("api/openapi/external.json", &external)?;
        }
        "external-openapi-check" => {
            check_openapi("api/openapi/external.json", &external)?;
        }
        _ => anyhow::bail!("未知 OpenAPI 模式：{mode}"),
    }
    Ok(())
}

fn write_openapi(path: &str, document: &serde_json::Value) -> anyhow::Result<()> {
    let mut generated = serde_json::to_string_pretty(document).context("序列化 OpenAPI 失败")?;
    generated.push('\n');
    let path = std::path::Path::new(path);
    std::fs::create_dir_all(path.parent().unwrap()).context("创建 OpenAPI 目录失败")?;
    std::fs::write(path, generated).context("写入 OpenAPI 产物失败")?;
    tracing::info!(path = %path.display(), "OpenAPI document generated");
    Ok(())
}

fn check_openapi(path: &str, document: &serde_json::Value) -> anyhow::Result<()> {
    let mut generated = serde_json::to_string_pretty(document).context("序列化 OpenAPI 失败")?;
    generated.push('\n');
    let current = std::fs::read_to_string(path)
        .context("读取 OpenAPI 产物失败，请先运行 make api-openapi")?;
    anyhow::ensure!(
        current == generated,
        "OpenAPI 产物已过期，请运行 make api-openapi"
    );
    tracing::info!(path, "OpenAPI document is current");
    Ok(())
}
