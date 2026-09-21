mod transaction;

use anyhow::{Context, bail};
use chrono::Utc;
use clap::Parser;
use serde::Deserialize;
use std::{fs, path::PathBuf};
use transaction::{Transaction, TransactionStore, validate_digest, validate_id, validate_version};

const TRANSACTION_ROOT: &str = "/var/lib/deploy-go-agent-updater/transactions";
const REQUEST_ROOT: &str = "/var/lib/deploy-go-agent-updater/requests";

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
    job_id: String,
    target_version: String,
    manifest_digest: String,
    authorization: String,
    deadline_at: i64,
}

fn main() -> anyhow::Result<()> {
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
    let transaction = Transaction::new(
        request.job_id,
        request.target_version,
        request.manifest_digest,
        request.deadline_at,
    );
    store.prepare(&transaction)?;
    // 具体的发布物替换和 systemd 编排在 staging 校验接入后执行；在此之前
    // 只落盘受控事务，不允许 updater 把任意路径或脚本当作安装输入。
    bail!("升级 staging 尚未就绪")
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
