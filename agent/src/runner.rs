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
use deploy_go_agent_protocol::{DeploymentStage, Environment};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::Mutex,
};

use crate::{
    deploy_events::{DeployEventContext, MarkerState, finished_event, process_line, started_event},
    git,
    journal::{Completion, process_start_time},
    staging,
};

const MAX_LOG_LINE_BYTES: usize = 64 * 1024;
const LINE_TRUNCATED_MARKER: &[u8] = b"\n[deploy-go:line_truncated]\n";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerSpec {
    pub deployment_id: String,
    pub script_path: PathBuf,
    pub argument_tokens: Vec<String>,
    pub environment_file_references: Vec<(String, PathBuf)>,
    pub timeout_seconds: u32,
    pub log_budget_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub two_stage: Option<TwoStageRunnerSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TwoStageRunnerSpec {
    pub stage: DeploymentStage,
    pub checkout_dir: PathBuf,
    pub work_root: PathBuf,
    pub repository_url: Option<String>,
    pub commit_sha: String,
    pub credential_file: Option<PathBuf>,
    pub environment: Environment,
    pub release_version: String,
    pub target_code: Option<String>,
    pub modules: Vec<String>,
    pub artifact_dir: Option<PathBuf>,
    pub staging_size_limit_bytes: u64,
    pub staging_max_files: usize,
    pub git_lease_id: Option<String>,
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
    let two_stage = spec.two_stage.clone();
    let event_context = two_stage
        .as_ref()
        .map(|two_stage| event_context_for(two_stage, &spec.deployment_id));
    let events_path = task_dir.join("events.jsonl");
    let state = std::sync::Arc::new(Mutex::new(MarkerState::new()));

    if let Some(context) = &event_context {
        write_event_line(&events_path, &started_event(context)).await?;
        if let Some(two_stage) = &two_stage {
            if two_stage.stage == DeploymentStage::Release
                && let Err(error) = verify_staging(two_stage)
            {
                let diagnostic = format!("发布物校验失败: {error}");
                append_diagnostic(&stderr_path, &diagnostic).await?;
                let mut state = state.lock().await;
                state
                    .violations
                    .push(format!("artifact_verification_failed: {error}"));
                let (event, _) = finished_event(context, &state, false);
                drop(state);
                write_event_line(&events_path, &event).await?;
                return atomic_json(
                    task_dir.join("completion.json"),
                    &Completion {
                        exit_code: Some(1),
                        error_code: Some("artifact_verification_failed".to_owned()),
                    },
                )
                .await;
            }
            if let Some(repository_url) = &two_stage.repository_url
                && let Err(error) = git::checkout_commit(
                    repository_url,
                    &two_stage.commit_sha,
                    &two_stage.checkout_dir,
                    two_stage.credential_file.as_deref(),
                    spec.timeout_seconds,
                )
                .await
            {
                let diagnostic = format!("Git 检出失败: {error}");
                append_diagnostic(&stderr_path, &diagnostic).await?;
                let mut state = state.lock().await;
                state
                    .violations
                    .push(format!("git_checkout_failed: {error}"));
                let (event, _) = finished_event(context, &state, false);
                drop(state);
                write_event_line(&events_path, &event).await?;
                return atomic_json(
                    task_dir.join("completion.json"),
                    &Completion {
                        exit_code: Some(1),
                        error_code: Some("git_checkout_failed".to_owned()),
                    },
                )
                .await;
            }
        }
    }

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
    if let Some(two_stage) = &two_stage {
        let environment = environment_str(&two_stage.environment);
        command
            .env("DEPLOY_ENVIRONMENT", environment)
            .env("DEPLOY_RELEASE_VERSION", &two_stage.release_version)
            .env("DEPLOY_COMMIT_SHA", &two_stage.commit_sha)
            .env("DEPLOY_MODULES", two_stage.modules.join(","));
        if let Some(target_code) = &two_stage.target_code {
            command.env("DEPLOY_TARGET", target_code);
        }
        if let Some(artifact_dir) = &two_stage.artifact_dir {
            let key = if two_stage.stage == DeploymentStage::Prepare {
                "DEPLOY_OUTPUT_DIR"
            } else {
                "DEPLOY_ARTIFACT_DIR"
            };
            command.env(key, artifact_dir);
        }
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
    let stdout_copy = if let Some(context) = &event_context {
        tokio::spawn(copy_stdout_with_events(
            stdout,
            stdout_path,
            events_path.clone(),
            Arc::clone(&budget),
            context.clone(),
            Arc::clone(&state),
        ))
    } else {
        tokio::spawn(copy_bounded(stdout, stdout_path, Arc::clone(&budget)))
    };
    let stderr_copy = tokio::spawn(copy_bounded(
        stderr,
        stderr_path.clone(),
        Arc::clone(&budget),
    ));

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
    let mut completion = if timed_out {
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
    if let (Some(context), Some(two_stage)) = (&event_context, &two_stage) {
        let mut artifact_error = None;
        let exit_ok = status.success() && !canceled && !timed_out;
        if exit_ok
            && two_stage.stage == DeploymentStage::Prepare
            && let Err(error) = verify_staging(two_stage)
        {
            artifact_error = Some(format!("artifact_verification_failed: {error}"));
            append_diagnostic(&stderr_path, artifact_error.as_ref().unwrap()).await?;
        }
        let state = state.lock().await;
        let exit_ok = exit_ok && artifact_error.is_none();
        let (event, event_error) = finished_event(context, &state, exit_ok);
        drop(state);
        write_event_line(&events_path, &event).await?;
        let protocol_error = event_error;
        if let Some(error) = artifact_error.or(protocol_error) {
            if completion.error_code.is_none() {
                completion.error_code = Some(error);
            }
            if completion.exit_code == Some(0) {
                completion.exit_code = Some(1);
            }
        }
    }
    atomic_json(task_dir.join("completion.json"), &completion).await
}

fn event_context_for(spec: &TwoStageRunnerSpec, deployment_id: &str) -> DeployEventContext {
    DeployEventContext {
        deploy_id: deployment_id.to_owned(),
        stage: spec.stage.clone(),
        environment: spec.environment.clone(),
        release_version: spec.release_version.clone(),
        target: spec.target_code.clone(),
    }
}

fn verify_staging(spec: &TwoStageRunnerSpec) -> Result<(), staging::StagingError> {
    let artifact_dir = spec
        .artifact_dir
        .as_deref()
        .ok_or(staging::StagingError::InvalidDirectory)?;
    staging::verify_artifact_dir(
        artifact_dir,
        &spec.release_version,
        &spec.commit_sha,
        &spec.modules,
        &staging::StagingLimits {
            size_limit_bytes: spec.staging_size_limit_bytes,
            max_files: spec.staging_max_files,
        },
    )?;
    Ok(())
}

fn environment_str(environment: &Environment) -> &'static str {
    match environment {
        Environment::Dev => "dev",
        Environment::Test => "test",
        Environment::Staging => "staging",
        Environment::Production => "prod",
    }
}

async fn append_diagnostic(path: &Path, line: &str) -> anyhow::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .context("打开诊断输出失败")?;
    file.write_all(line.as_bytes()).await?;
    file.write_all(b"\n").await?;
    file.sync_all().await?;
    Ok(())
}

async fn write_event_line(
    path: &Path,
    event: &deploy_go_agent_protocol::DeployEvent,
) -> anyhow::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .context("打开事件输出失败")?;
    file.write_all(serde_json::to_string(event)?.as_bytes())
        .await?;
    file.write_all(b"\n").await?;
    file.sync_all().await?;
    Ok(())
}

async fn copy_stdout_with_events(
    mut reader: impl AsyncRead + Unpin,
    stdout_path: PathBuf,
    events_path: PathBuf,
    budget: Arc<std::sync::atomic::AtomicU64>,
    context: DeployEventContext,
    state: Arc<Mutex<MarkerState>>,
) -> anyhow::Result<()> {
    let mut output = File::create(stdout_path)
        .await
        .context("创建任务输出文件失败")?;
    let mut buffer = vec![0_u8; 16 * 1024];
    let mut line = Vec::with_capacity(MAX_LOG_LINE_BYTES);
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await.context("读取任务输出失败")?;
        if read == 0 {
            break;
        }
        for &byte in &buffer[..read] {
            if byte == b'\n' {
                write_with_budget(&mut output, &line, &budget).await?;
                write_with_budget(&mut output, b"\n", &budget).await?;
                if truncated {
                    write_with_budget(&mut output, LINE_TRUNCATED_MARKER, &budget).await?;
                }
                process_marker_line(&line, &context, &state, &events_path).await?;
                line.clear();
                truncated = false;
            } else if line.len() < MAX_LOG_LINE_BYTES {
                line.push(byte);
            } else {
                truncated = true;
            }
        }
    }
    if !line.is_empty() {
        write_with_budget(&mut output, &line, &budget).await?;
    }
    if truncated {
        write_with_budget(&mut output, LINE_TRUNCATED_MARKER, &budget).await?;
    }
    process_marker_line(&line, &context, &state, &events_path).await?;
    output.sync_all().await.context("同步任务输出失败")
}

async fn process_marker_line(
    line: &[u8],
    context: &DeployEventContext,
    state: &Arc<Mutex<MarkerState>>,
    events_path: &Path,
) -> anyhow::Result<()> {
    if !line.starts_with(b"DEPLOY_GO_EVENT ") {
        return Ok(());
    }
    let text = String::from_utf8_lossy(line);
    let mut state = state.lock().await;
    match process_line(text.trim_end(), context, &mut state) {
        Ok(Some(event)) => {
            drop(state);
            write_event_line(events_path, &event).await?;
        }
        Ok(None) => {}
        Err(violation) => state
            .violations
            .push(format!("{}: {}", violation.kind, violation.message)),
    }
    Ok(())
}

async fn copy_bounded(
    mut reader: impl AsyncRead + Unpin,
    path: PathBuf,
    budget: Arc<AtomicU64>,
) -> anyhow::Result<()> {
    let mut output = File::create(path).await.context("创建任务输出文件失败")?;
    let mut buffer = vec![0_u8; 16 * 1024];
    let mut line = Vec::with_capacity(MAX_LOG_LINE_BYTES);
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await.context("读取任务输出失败")?;
        if read == 0 {
            break;
        }
        for &byte in &buffer[..read] {
            if byte == b'\n' {
                write_with_budget(&mut output, &line, &budget).await?;
                write_with_budget(&mut output, b"\n", &budget).await?;
                if truncated {
                    write_with_budget(&mut output, LINE_TRUNCATED_MARKER, &budget).await?;
                }
                line.clear();
                truncated = false;
            } else if line.len() < MAX_LOG_LINE_BYTES {
                line.push(byte);
            } else {
                truncated = true;
            }
        }
    }
    if !line.is_empty() {
        write_with_budget(&mut output, &line, &budget).await?;
    }
    if truncated {
        write_with_budget(&mut output, LINE_TRUNCATED_MARKER, &budget).await?;
    }
    output.sync_all().await.context("同步任务输出失败")
}

async fn write_with_budget(
    output: &mut File,
    bytes: &[u8],
    budget: &AtomicU64,
) -> anyhow::Result<()> {
    let allowed = reserve(budget, bytes.len() as u64) as usize;
    if allowed > 0 {
        output
            .write_all(&bytes[..allowed])
            .await
            .context("写入任务输出失败")?;
    }
    Ok(())
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
