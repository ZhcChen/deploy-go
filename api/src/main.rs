use std::str::FromStr;

use anyhow::Context;
use deploy_go_api::{
    AppState, app, config::Config, crypto::MasterKeyRing, db, http::shutdown_signal,
    ssh_credentials,
};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let process_mode = std::env::args().nth(1);
    if matches!(process_mode.as_deref(), Some("openapi" | "openapi-check")) {
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
        tracing::info!(migrated, "SSH credential re-encryption completed");
        return Ok(());
    }

    deploy_go_api::agents::websocket::reset_online_state(&pool)
        .await
        .context("重置 Agent 节点连接状态失败")?;

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .context("监听 API 地址失败")?;
    tracing::info!(address = %config.bind_addr, "Deploy Go API started");

    let mut state = AppState::new(pool)
        .with_allowed_origin(config.allowed_origin)
        .with_cookie_secure(config.cookie_secure)
        .with_master_key_ring(master_key_ring);
    if let Some(setup_token) = config.setup_token {
        state = state.with_setup_token(setup_token);
    }

    let worker = tokio::spawn(deploy_go_api::deployments::run_worker(state.clone()));

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("API 服务异常退出")?;
    worker.abort();

    Ok(())
}

fn handle_openapi(mode: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new("api/openapi/openapi.json");
    let mut generated = serde_json::to_string_pretty(&deploy_go_api::openapi_document())
        .context("序列化 OpenAPI 失败")?;
    generated.push('\n');
    if mode == "openapi" {
        std::fs::create_dir_all(path.parent().unwrap()).context("创建 OpenAPI 目录失败")?;
        std::fs::write(path, generated).context("写入 OpenAPI 产物失败")?;
        tracing::info!(path = %path.display(), "OpenAPI document generated");
        return Ok(());
    }
    let current = std::fs::read_to_string(path)
        .context("读取 OpenAPI 产物失败，请先运行 make api-openapi")?;
    anyhow::ensure!(
        current == generated,
        "OpenAPI 产物已过期，请运行 make api-openapi"
    );
    tracing::info!(path = %path.display(), "OpenAPI document is current");
    Ok(())
}
