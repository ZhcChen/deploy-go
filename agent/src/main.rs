use anyhow::Context;
use deploy_go_agent::{config::Config, credential_store::CredentialStore, system_info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env().context("加载 Agent 配置失败")?;
    let credentials = CredentialStore::new(config.credential_file.clone())
        .load()
        .context("加载 Agent 凭证失败")?;
    let system = system_info::collect();
    tracing::info!(
        agent_id = %credentials.agent_id,
        os = %system.os,
        architecture = %system.architecture,
        control_url = %config.control_url,
        "Deploy Go Agent initialized"
    );
    Ok(())
}
