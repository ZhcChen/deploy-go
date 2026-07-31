use std::str::FromStr;

use anyhow::Context;
use deploy_go_api::{AppState, app, config::Config, db, http::shutdown_signal};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env().context("加载配置失败")?;
    let connect_options = SqliteConnectOptions::from_str(&config.database_url)
        .context("解析 SQLite URL 失败")?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(connect_options)
        .await
        .context("连接 SQLite 失败")?;
    db::migrate(&pool)
        .await
        .context("执行数据库 migration 失败")?;

    if std::env::args().nth(1).as_deref() == Some("migrate") {
        tracing::info!("database migrations completed");
        return Ok(());
    }

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .context("监听 API 地址失败")?;
    tracing::info!(address = %config.bind_addr, "Deploy Go API started");

    axum::serve(listener, app(AppState::new(pool)))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("API 服务异常退出")?;

    Ok(())
}
