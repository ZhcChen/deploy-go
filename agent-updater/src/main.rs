mod transaction;

use anyhow::{Context, bail};
use chrono::Utc;
use clap::Parser;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
use transaction::{
    Transaction, TransactionState, TransactionStore, validate_digest, validate_id, validate_version,
};

const TRANSACTION_ROOT: &str = "/var/lib/deploy-go-agent-updater/transactions";
const REQUEST_ROOT: &str = "/var/lib/deploy-go-agent-updater/requests";
const STAGING_ROOT: &str = "/var/lib/deploy-go-agent/upgrades";

#[derive(Debug, Parser)]
struct Arguments {
    #[arg(long, conflicts_with = "resume")]
    job_id: Option<String>,
    #[arg(long)]
    resume: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpgradeRequest {
    #[serde(default, rename = "version")]
    _version: Option<u16>,
    job_id: String,
    target_version: String,
    manifest_digest: String,
    authorization: String,
    deadline_at: i64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Agent updater 执行失败: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let arguments = Arguments::parse();
    let job_id = match (arguments.job_id, arguments.resume) {
        (Some(job_id), false) => job_id,
        (None, true) => latest_job_id().context("没有可恢复的升级事务")?,
        _ => bail!("必须指定 --job-id 或 --resume"),
    };
    validate_id(&job_id).context("升级 job_id 无效")?;
    let request_path = PathBuf::from(REQUEST_ROOT).join(format!("{}.json", job_id));
    let request: UpgradeRequest = serde_json::from_slice(&fs::read(&request_path)?)?;
    if request.job_id != job_id {
        bail!("升级请求 job_id 不一致");
    }
    validate_version(&request.target_version).context("升级目标版本无效")?;
    validate_digest(&request.manifest_digest).context("升级 manifest 摘要无效")?;
    if request.authorization.is_empty() || request.authorization.len() > 8192 {
        bail!("升级授权无效");
    }
    if request.deadline_at < Utc::now().timestamp() {
        bail!("升级授权已过期");
    }

    let store = TransactionStore::new(TRANSACTION_ROOT.into());
    if store
        .read(&job_id)
        .ok()
        .is_some_and(|transaction| transaction.state == TransactionState::Committed)
    {
        return Ok(());
    }
    let transaction = Transaction::new(
        request.job_id,
        request.target_version,
        request.manifest_digest.clone(),
        request.deadline_at,
    );
    store.prepare(&transaction)?;
    let staging = PathBuf::from(STAGING_ROOT).join(&job_id).join("staging");
    validate_staging(&staging, &request.manifest_digest)?;
    store.update_state(&job_id, TransactionState::Staged)?;
    let backup = PathBuf::from(TRANSACTION_ROOT).join(&job_id).join("backup");
    fs::create_dir_all(&backup)?;
    backup_files(&backup)?;
    store.update_state(&job_id, TransactionState::Stopped)?;
    stop_services()?;
    if let Err(error) = install_files(&staging) {
        let _ = rollback(&backup);
        let _ = store.update_state(&job_id, TransactionState::RolledBack);
        return Err(error);
    }
    store.update_state(&job_id, TransactionState::Switched)?;
    if let Err(error) = restart_services() {
        let _ = rollback(&backup);
        let _ = store.update_state(&job_id, TransactionState::RolledBack);
        return Err(error);
    }
    store.update_state(&job_id, TransactionState::Verified)?;
    store.update_state(&job_id, TransactionState::Committed)?;
    eprintln!("Agent updater 已完成: job_id={job_id}");
    Ok(())
}

fn validate_staging(staging: &Path, manifest_digest: &str) -> anyhow::Result<()> {
    let manifest = staging
        .parent()
        .context("升级 manifest 缺失")?
        .join("manifest.json");
    let manifest = fs::canonicalize(manifest)?;
    let bytes = fs::read(&manifest)?;
    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    if digest != manifest_digest {
        bail!("升级 manifest 摘要不匹配");
    }
    let required = [
        ("agent", 0o755),
        ("executor", 0o755),
        ("updater", 0o755),
        ("unit-agent", 0o644),
        ("unit-runner", 0o644),
        ("unit-executor", 0o644),
        ("unit-updater", 0o644),
        ("executor-config", 0o600),
    ];
    for (name, _) in required {
        let path = staging.join(name);
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_file() {
            bail!("升级 staging 文件类型无效: {name}");
        }
    }
    Ok(())
}

fn backup_files(backup: &Path) -> anyhow::Result<()> {
    for (source, name) in managed_files() {
        let target = Path::new(source);
        if target.is_file() {
            fs::copy(target, backup.join(name))?;
        }
    }
    Ok(())
}

fn install_files(staging: &Path) -> anyhow::Result<()> {
    for (target, name) in managed_files() {
        let source = staging.join(name);
        let temporary = Path::new(target).with_extension("deploy-go-part");
        fs::copy(&source, &temporary)?;
        let mode = if name == "executor-config" {
            0o600
        } else if name.starts_with("unit-") {
            0o644
        } else {
            0o755
        };
        fs::set_permissions(&temporary, fs::Permissions::from_mode(mode))?;
        fs::rename(temporary, target)?;
    }
    run_systemctl(&["daemon-reload"])?;
    Ok(())
}

fn rollback(backup: &Path) -> anyhow::Result<()> {
    for (target, name) in managed_files() {
        let source = backup.join(name);
        if source.is_file() {
            fs::copy(source, target)?;
        }
    }
    run_systemctl(&["daemon-reload"])?;
    let _ = restart_services();
    Ok(())
}

fn managed_files() -> [(&'static str, &'static str); 7] {
    [
        ("/usr/local/bin/deploy-go-agent", "agent"),
        ("/usr/local/bin/deploy-go-agent-executor", "executor"),
        ("/usr/local/bin/deploy-go-agent-updater", "updater"),
        ("/etc/systemd/system/deploy-go-agent.service", "unit-agent"),
        (
            "/etc/systemd/system/deploy-go-agent-runner.service",
            "unit-runner",
        ),
        (
            "/etc/systemd/system/deploy-go-agent-executor.service",
            "unit-executor",
        ),
        (
            "/etc/systemd/system/deploy-go-agent-updater.service",
            "unit-updater",
        ),
    ]
}

fn stop_services() -> anyhow::Result<()> {
    run_systemctl(&[
        "stop",
        "deploy-go-agent.service",
        "deploy-go-agent-runner.service",
        "deploy-go-agent-executor.service",
    ])
}

fn restart_services() -> anyhow::Result<()> {
    run_systemctl(&["restart", "deploy-go-agent-executor.service"])?;
    run_systemctl(&["restart", "deploy-go-agent-runner.service"])?;
    run_systemctl(&["restart", "deploy-go-agent.service"])?;
    run_systemctl(&["is-active", "deploy-go-agent-executor.service"])?;
    run_systemctl(&["is-active", "deploy-go-agent-runner.service"])?;
    run_systemctl(&["is-active", "deploy-go-agent.service"])
}

fn run_systemctl(arguments: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("systemctl").args(arguments).status()?;
    if !status.success() {
        bail!("systemctl 执行失败: {}", arguments.join(" "));
    }
    Ok(())
}

fn latest_job_id() -> anyhow::Result<String> {
    let mut jobs = fs::read_dir(REQUEST_ROOT)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".json").map(str::to_owned)
        })
        .filter(|job_id| job_id.starts_with("upgrade_"))
        .collect::<Vec<_>>();
    jobs.sort();
    jobs.pop()
        .ok_or_else(|| anyhow::anyhow!("没有可恢复的升级事务"))
}

#[cfg(test)]
mod tests {
    use super::{UpgradeRequest, managed_files, validate_staging};
    use sha2::{Digest, Sha256};
    use std::fs;

    fn request_json(version: &str) -> String {
        format!(
            r#"{{{version}"job_id":"upgrade_01TEST","target_version":"0.3.10","manifest_digest":"sha256:{}","authorization":"authorization","deadline_at":2000000000}}"#,
            "a".repeat(64)
        )
    }

    #[test]
    fn updater_request_accepts_missing_version() {
        let request: UpgradeRequest = serde_json::from_str(&request_json("")).unwrap();
        assert_eq!(request.job_id, "upgrade_01TEST");
    }

    #[test]
    fn updater_request_accepts_legacy_null_version() {
        let request: UpgradeRequest =
            serde_json::from_str(&request_json(r#""version":null,"#)).unwrap();
        assert_eq!(request.job_id, "upgrade_01TEST");
    }

    #[test]
    fn staging_validation_reads_manifest_from_job_root() {
        let directory = tempfile::tempdir().unwrap();
        let job_root = directory.path().join("upgrade_01TEST");
        let staging = job_root.join("staging");
        fs::create_dir_all(&staging).unwrap();
        let manifest = b"manifest";
        fs::write(job_root.join("manifest.json"), manifest).unwrap();
        for name in [
            "agent",
            "executor",
            "updater",
            "unit-agent",
            "unit-runner",
            "unit-executor",
            "unit-updater",
            "executor-config",
        ] {
            let path = staging.join(name);
            fs::write(&path, name).unwrap();
        }
        let digest = format!("sha256:{:x}", Sha256::digest(manifest));

        validate_staging(&staging, &digest).unwrap();
    }

    #[test]
    fn updater_preserves_node_bound_executor_config() {
        assert!(managed_files().iter().all(|(target, name)| *target
            != "/etc/deploy-go-agent/executor.json"
            && *name != "executor-config"));
    }
}
