use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};

use crate::journal::{Completion, process_start_time};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerSpec {
    pub deployment_id: String,
    pub script_path: PathBuf,
    pub argument_tokens: Vec<String>,
    pub environment_file_references: Vec<(String, PathBuf)>,
    pub timeout_seconds: u32,
    pub log_budget_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub start_time: Option<u64>,
}

pub async fn run_from_args() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(2);
    let spec_path = args.next().context("runner 缺少 spec 路径")?;
    let task_dir = args.next().context("runner 缺少任务目录")?;
    ensure!(args.next().is_none(), "runner 参数过多");
    run(Path::new(&spec_path), Path::new(&task_dir)).await
}

pub async fn run(spec_path: &Path, task_dir: &Path) -> anyhow::Result<()> {
    let spec: RunnerSpec =
        serde_json::from_slice(&fs::read(spec_path).await.context("读取 runner spec 失败")?)
            .context("解析 runner spec 失败")?;
    let stdout_path = task_dir.join("stdout.log");
    let stderr_path = task_dir.join("stderr.log");
    let cancel_path = task_dir.join("cancel");

    let mut command = Command::new(&spec.script_path);
    command
        .args(&spec.argument_tokens)
        .env("DEPLOY_ID", &spec.deployment_id)
        .env("DEPLOY_CANCEL_FILE", &cancel_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false);
    for (key, path) in &spec.environment_file_references {
        command.env(key, path);
    }
    #[cfg(unix)]
    command.process_group(0);

    let mut child = command.spawn().context("启动部署脚本失败")?;
    let pid = child.id().context("部署脚本缺少 PID")?;
    let identity = ProcessIdentity {
        pid,
        start_time: process_start_time(pid).ok(),
    };
    atomic_json(task_dir.join("process.json"), &identity).await?;

    let budget = Arc::new(AtomicU64::new(spec.log_budget_bytes));
    let stdout = child.stdout.take().context("缺少 stdout pipe")?;
    let stderr = child.stderr.take().context("缺少 stderr pipe")?;
    let stdout_copy = tokio::spawn(copy_bounded(stdout, stdout_path, Arc::clone(&budget)));
    let stderr_copy = tokio::spawn(copy_bounded(stderr, stderr_path, Arc::clone(&budget)));

    let mut timed_out = false;
    let status = match tokio::time::timeout(
        Duration::from_secs(u64::from(spec.timeout_seconds)),
        child.wait(),
    )
    .await
    {
        Ok(status) => status.context("等待部署脚本失败")?,
        Err(_) => {
            timed_out = true;
            terminate_process_group(pid, Duration::from_secs(30)).await;
            child.wait().await.context("回收超时部署脚本失败")?
        }
    };
    stdout_copy.await.context("stdout 任务异常")??;
    stderr_copy.await.context("stderr 任务异常")??;

    let canceled = fs::try_exists(&cancel_path).await.unwrap_or(false);
    let completion = if timed_out {
        Completion {
            exit_code: status.code(),
            error_code: Some("task_timeout".to_owned()),
        }
    } else if canceled {
        Completion {
            exit_code: status.code(),
            error_code: Some("task_canceled".to_owned()),
        }
    } else if status.code().is_none() {
        Completion {
            exit_code: None,
            error_code: Some("process_signaled".to_owned()),
        }
    } else {
        Completion {
            exit_code: status.code(),
            error_code: None,
        }
    };
    atomic_json(task_dir.join("completion.json"), &completion).await
}

async fn copy_bounded(
    mut reader: impl AsyncRead + Unpin,
    path: PathBuf,
    budget: Arc<AtomicU64>,
) -> anyhow::Result<()> {
    let mut output = File::create(path).await.context("创建任务输出文件失败")?;
    let mut buffer = vec![0_u8; 16 * 1024];
    loop {
        let read = reader.read(&mut buffer).await.context("读取任务输出失败")?;
        if read == 0 {
            break;
        }
        let allowed = reserve(&budget, read as u64) as usize;
        if allowed > 0 {
            output
                .write_all(&buffer[..allowed])
                .await
                .context("写入任务输出失败")?;
        }
    }
    output.sync_all().await.context("同步任务输出失败")
}

fn reserve(budget: &AtomicU64, requested: u64) -> u64 {
    let mut remaining = budget.load(Ordering::Relaxed);
    loop {
        let allowed = remaining.min(requested);
        match budget.compare_exchange_weak(
            remaining,
            remaining - allowed,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => return allowed,
            Err(actual) => remaining = actual,
        }
    }
}

async fn terminate_process_group(pid: u32, grace: Duration) {
    #[cfg(unix)]
    {
        use nix::{
            sys::signal::{Signal, kill},
            unistd::Pid,
        };
        if let Ok(pid) = i32::try_from(pid) {
            let group = Pid::from_raw(-pid);
            let _ = kill(group, Signal::SIGTERM);
            let deadline = tokio::time::Instant::now() + grace;
            while kill(group, None).is_ok() && tokio::time::Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            if kill(group, None).is_ok() {
                let _ = kill(group, Signal::SIGKILL);
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, grace);
    }
}

async fn atomic_json(path: PathBuf, value: &impl Serialize) -> anyhow::Result<()> {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let bytes = serde_json::to_vec(value).context("序列化 runner 状态失败")?;
    let mut file = File::create(&temporary)
        .await
        .context("创建 runner 状态失败")?;
    file.write_all(&bytes)
        .await
        .context("写入 runner 状态失败")?;
    file.sync_all().await.context("同步 runner 状态失败")?;
    fs::rename(temporary, path)
        .await
        .context("提交 runner 状态失败")
}
