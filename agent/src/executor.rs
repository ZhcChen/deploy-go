use std::{
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use deploy_go_agent_protocol::DeploymentExecuteTask;
use thiserror::Error;
use tokio::process::Command;

use crate::{
    journal::{
        Completion, JournalError, JournalState, JournalStore, TaskJournal, apply_completion,
        process_start_time,
    },
    runner::{ProcessIdentity, RunnerSpec},
};

pub const WRAPPER_VERSION: &str = "1";
pub const DEFAULT_LOG_BUDGET_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Executor {
    journal: JournalStore,
    runner_binary: PathBuf,
    cancel_grace: Duration,
    log_budget_bytes: u64,
    admission_lock: Arc<tokio::sync::Mutex<()>>,
}

#[derive(Debug, Error)]
pub enum ExecuteError {
    #[error("任务字段无效")]
    InvalidTask,
    #[error("脚本路径逃逸工作根目录")]
    PathOutsideWorkRoot,
    #[error("脚本或环境文件不可访问")]
    InaccessiblePath,
    #[error("包装器版本不受支持")]
    UnsupportedWrapper,
    #[error("任务 payload 与已有 journal 冲突")]
    PayloadConflict,
    #[error("任务已存在")]
    Duplicate,
    #[error("任务进程身份无法验证")]
    ProcessIdentityMismatch,
    #[error("任务 journal 操作失败")]
    Journal(#[from] JournalError),
    #[error("任务进程操作失败")]
    Io(#[from] io::Error),
    #[error("任务状态无效")]
    InvalidState,
}

impl Executor {
    pub fn new(journal_root: PathBuf) -> Result<Self, ExecuteError> {
        Ok(Self {
            journal: JournalStore::new(journal_root),
            runner_binary: std::env::current_exe()?,
            cancel_grace: Duration::from_secs(30),
            log_budget_bytes: DEFAULT_LOG_BUDGET_BYTES,
            admission_lock: Arc::new(tokio::sync::Mutex::new(())),
        })
    }

    pub fn with_runner_binary(mut self, path: PathBuf) -> Self {
        self.runner_binary = path;
        self
    }

    pub fn with_cancel_grace(mut self, grace: Duration) -> Self {
        self.cancel_grace = grace;
        self
    }

    pub fn with_log_budget(mut self, bytes: u64) -> Self {
        self.log_budget_bytes = bytes;
        self
    }

    pub async fn execute(
        &self,
        task_id: &str,
        idempotency_key: &str,
        payload_digest: &str,
        task: &DeploymentExecuteTask,
    ) -> Result<TaskJournal, ExecuteError> {
        let mut spec = validate_task(task)?;
        spec.log_budget_bytes = self.log_budget_bytes;
        let _admission = self.admission_lock.lock().await;
        match self.journal.load(task_id) {
            Ok(existing) if existing.payload_digest != payload_digest => {
                return Err(ExecuteError::PayloadConflict);
            }
            Ok(_) => return Err(ExecuteError::Duplicate),
            Err(JournalError::Missing) => {}
            Err(error) => return Err(error.into()),
        }
        if let Some(existing) = self.journal.find_by_idempotency_key(idempotency_key)? {
            return if existing.payload_digest == payload_digest {
                Err(ExecuteError::Duplicate)
            } else {
                Err(ExecuteError::PayloadConflict)
            };
        }
        let mut journal = self
            .journal
            .create(task_id, idempotency_key, payload_digest)?;
        let task_dir = self.journal.task_dir(task_id);
        let spec_path = task_dir.join("runner-spec.json");
        write_private_json(&spec_path, &spec)?;

        let mut runner = Command::new(&self.runner_binary);
        runner
            .arg("runner")
            .arg(&spec_path)
            .arg(&task_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(false);
        runner.spawn()?;

        let identity = wait_for_identity(&task_dir, Duration::from_secs(5)).await?;
        journal.state = JournalState::Running;
        journal.pid = Some(identity.pid);
        journal.process_start_time = identity.start_time;
        self.journal.store(&journal)?;
        Ok(journal)
    }

    pub async fn finish(&self, task_id: &str) -> Result<TaskJournal, ExecuteError> {
        let mut journal = self.journal.load(task_id)?;
        let task_dir = self.journal.task_dir(task_id);
        wait_for_completion(&task_dir, Duration::from_secs(5)).await?;
        let completion: Completion = read_json(&task_dir.join("completion.json"))?;
        apply_completion(&mut journal, completion);
        self.journal.store(&journal)?;
        Ok(journal)
    }

    pub async fn cancel(&self, task_id: &str) -> Result<TaskJournal, ExecuteError> {
        let mut journal = self.journal.load(task_id)?;
        let (pid, expected_start) = match (journal.pid, journal.process_start_time) {
            (Some(pid), Some(start)) => (pid, start),
            _ => return Err(ExecuteError::ProcessIdentityMismatch),
        };
        if process_start_time(pid).ok() != Some(expected_start) {
            return Err(ExecuteError::ProcessIdentityMismatch);
        }
        fs::write(self.journal.task_dir(task_id).join("cancel"), b"")?;
        signal_group(pid, nix::sys::signal::Signal::SIGTERM)?;
        let deadline = tokio::time::Instant::now() + self.cancel_grace;
        while process_start_time(pid).ok() == Some(expected_start)
            && tokio::time::Instant::now() < deadline
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        if process_start_time(pid).ok() == Some(expected_start) {
            signal_group(pid, nix::sys::signal::Signal::SIGKILL)?;
        }
        wait_for_completion(&self.journal.task_dir(task_id), Duration::from_secs(5)).await?;
        journal = self.finish(task_id).await?;
        Ok(journal)
    }
}

fn validate_task(task: &DeploymentExecuteTask) -> Result<RunnerSpec, ExecuteError> {
    if task.wrapper_version != WRAPPER_VERSION {
        return Err(ExecuteError::UnsupportedWrapper);
    }
    if !(1..=86_400).contains(&task.timeout_seconds)
        || task.argument_tokens.len() > 128
        || task
            .argument_tokens
            .iter()
            .any(|token| token.len() > 4096 || token.contains('\0'))
        || task.environment_file_references.len() > 64
    {
        return Err(ExecuteError::InvalidTask);
    }
    let root = canonical_directory(Path::new(&task.work_root))?;
    let script = fs::canonicalize(&task.script_path).map_err(|_| ExecuteError::InaccessiblePath)?;
    if !script.starts_with(&root) || script == root || !script.is_file() {
        return Err(ExecuteError::PathOutsideWorkRoot);
    }
    #[cfg(unix)]
    if nix::unistd::access(&script, nix::unistd::AccessFlags::X_OK).is_err() {
        return Err(ExecuteError::InaccessiblePath);
    }
    let mut references = Vec::with_capacity(task.environment_file_references.len());
    for reference in &task.environment_file_references {
        if !valid_environment_key(&reference.environment_key) {
            return Err(ExecuteError::InvalidTask);
        }
        let path =
            fs::canonicalize(&reference.file_path).map_err(|_| ExecuteError::InaccessiblePath)?;
        if !path.is_file() || OpenOptions::new().read(true).open(&path).is_err() {
            return Err(ExecuteError::InaccessiblePath);
        }
        references.push((reference.environment_key.clone(), path));
    }
    Ok(RunnerSpec {
        deployment_id: task.deployment_id.clone(),
        script_path: script,
        argument_tokens: task.argument_tokens.clone(),
        environment_file_references: references,
        timeout_seconds: task.timeout_seconds,
        log_budget_bytes: DEFAULT_LOG_BUDGET_BYTES,
    })
}

fn canonical_directory(path: &Path) -> Result<PathBuf, ExecuteError> {
    if !path.is_absolute() {
        return Err(ExecuteError::PathOutsideWorkRoot);
    }
    let canonical = fs::canonicalize(path).map_err(|_| ExecuteError::InaccessiblePath)?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(ExecuteError::InaccessiblePath)
    }
}

fn valid_environment_key(key: &str) -> bool {
    (1..=64).contains(&key.len())
        && key.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_uppercase() || byte == b'_' || (index > 0 && byte.is_ascii_digit())
        })
}

fn write_private_json(path: &Path, value: &impl serde::Serialize) -> Result<(), ExecuteError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ExecuteError::InvalidTask)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    std::io::Write::write_all(&mut file, &bytes)?;
    file.sync_all()?;
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, ExecuteError> {
    serde_json::from_slice(&fs::read(path)?).map_err(|_| ExecuteError::InvalidState)
}

async fn wait_for_identity(
    task_dir: &Path,
    timeout: Duration,
) -> Result<ProcessIdentity, ExecuteError> {
    let path = task_dir.join("process.json");
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if path.exists() {
            return read_json(&path);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(ExecuteError::InvalidState);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn wait_for_completion(task_dir: &Path, timeout: Duration) -> Result<(), ExecuteError> {
    let path = task_dir.join("completion.json");
    let deadline = tokio::time::Instant::now() + timeout;
    while !path.exists() {
        if tokio::time::Instant::now() >= deadline {
            return Err(ExecuteError::InvalidState);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

fn signal_group(pid: u32, signal: nix::sys::signal::Signal) -> Result<(), ExecuteError> {
    use nix::{sys::signal::kill, unistd::Pid};
    let pid = i32::try_from(pid).map_err(|_| ExecuteError::ProcessIdentityMismatch)?;
    kill(Pid::from_raw(-pid), signal).map_err(|error| ExecuteError::Io(io::Error::other(error)))
}
