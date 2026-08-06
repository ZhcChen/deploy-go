use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use deploy_go_agent_protocol::{
    DeploymentExecuteTask, DeploymentPrepareTask, DeploymentReleaseTask, DeploymentStage,
    GitRefsQueryTask,
};
use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;

use crate::{
    git,
    journal::{
        Completion, JournalError, JournalState, JournalStore, RecoveryState, TaskJournal,
        apply_completion, process_start_time,
    },
    runner::{ProcessIdentity, RunnerSpec, TwoStageRunnerSpec},
};

pub const WRAPPER_VERSION: &str = "1";
pub const DEFAULT_LOG_BUDGET_BYTES: u64 = 50 * 1024 * 1024;
pub const DEFAULT_STAGING_SIZE_LIMIT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const DEFAULT_STAGING_MAX_FILES: usize = 4096;

#[derive(Clone, Debug)]
pub struct Executor {
    journal: JournalStore,
    runner_binary: PathBuf,
    cancel_grace: Duration,
    log_budget_bytes: u64,
    staging_size_limit_bytes: u64,
    staging_max_files: usize,
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
            staging_size_limit_bytes: DEFAULT_STAGING_SIZE_LIMIT_BYTES,
            staging_max_files: DEFAULT_STAGING_MAX_FILES,
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

    pub fn with_staging_limits(mut self, size_limit_bytes: u64, max_files: usize) -> Self {
        self.staging_size_limit_bytes = size_limit_bytes;
        self.staging_max_files = max_files;
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
        self.execute_spec(task_id, idempotency_key, payload_digest, spec)
            .await
    }

    pub async fn execute_prepare(
        &self,
        task_id: &str,
        idempotency_key: &str,
        payload_digest: &str,
        task: &DeploymentPrepareTask,
        credential_file: Option<PathBuf>,
    ) -> Result<TaskJournal, ExecuteError> {
        validate_two_stage_paths(&task.work_root, &task.checkout_dir, Some(&task.output_dir))?;
        validate_git_source(&task.repository_url, &task.commit_sha)?;
        validate_release_metadata(&task.release_version, &task.modules, task.timeout_seconds)?;
        let spec = RunnerSpec {
            deployment_id: task.deployment_id.clone(),
            script_path: PathBuf::from("make"),
            argument_tokens: vec![
                "--no-print-directory".to_owned(),
                "-C".to_owned(),
                task.checkout_dir.clone(),
                "deploy-go-prepare".to_owned(),
            ],
            environment_file_references: Vec::new(),
            timeout_seconds: task.timeout_seconds,
            log_budget_bytes: self.log_budget_bytes,
            two_stage: Some(TwoStageRunnerSpec {
                stage: DeploymentStage::Prepare,
                checkout_dir: PathBuf::from(&task.checkout_dir),
                work_root: PathBuf::from(&task.work_root),
                repository_url: Some(task.repository_url.clone()),
                commit_sha: task.commit_sha.clone(),
                credential_file,
                environment: task.environment.clone(),
                release_version: task.release_version.clone(),
                target_code: None,
                modules: task.modules.clone(),
                artifact_dir: Some(PathBuf::from(&task.output_dir)),
                staging_size_limit_bytes: self.staging_size_limit_bytes,
                staging_max_files: self.staging_max_files,
                git_lease_id: task.git_credential_lease_id.clone(),
            }),
        };
        self.execute_spec(task_id, idempotency_key, payload_digest, spec)
            .await
    }

    pub async fn execute_release(
        &self,
        task_id: &str,
        idempotency_key: &str,
        payload_digest: &str,
        task: &DeploymentReleaseTask,
    ) -> Result<TaskJournal, ExecuteError> {
        validate_two_stage_paths(
            &task.work_root,
            &task.checkout_dir,
            Some(&task.artifact_dir),
        )?;
        validate_release_metadata(&task.release_version, &task.modules, task.timeout_seconds)?;
        if task.target_code.is_empty()
            || task.target_code.len() > 256
            || !task
                .target_code
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        {
            return Err(ExecuteError::InvalidTask);
        }
        let spec = RunnerSpec {
            deployment_id: task.deployment_id.clone(),
            script_path: PathBuf::from("make"),
            argument_tokens: vec![
                "--no-print-directory".to_owned(),
                "-C".to_owned(),
                task.checkout_dir.clone(),
                "deploy-go-release".to_owned(),
            ],
            environment_file_references: Vec::new(),
            timeout_seconds: task.timeout_seconds,
            log_budget_bytes: self.log_budget_bytes,
            two_stage: Some(TwoStageRunnerSpec {
                stage: DeploymentStage::Release,
                checkout_dir: PathBuf::from(&task.checkout_dir),
                work_root: PathBuf::from(&task.work_root),
                repository_url: None,
                commit_sha: task.commit_sha.clone(),
                credential_file: None,
                environment: task.environment.clone(),
                release_version: task.release_version.clone(),
                target_code: Some(task.target_code.clone()),
                modules: task.modules.clone(),
                artifact_dir: Some(PathBuf::from(&task.artifact_dir)),
                staging_size_limit_bytes: self.staging_size_limit_bytes,
                staging_max_files: self.staging_max_files,
                git_lease_id: None,
            }),
        };
        self.execute_spec(task_id, idempotency_key, payload_digest, spec)
            .await
    }

    pub async fn run_refs_query(
        &self,
        task_id: &str,
        task: &GitRefsQueryTask,
        credential_file: Option<PathBuf>,
    ) -> Result<TaskJournal, ExecuteError> {
        let heads = git::list_remote_heads(
            &task.repository_url,
            credential_file.as_deref(),
            task.timeout_seconds,
        )
        .await;
        match heads {
            Ok(heads) => {
                let refs = heads
                    .iter()
                    .map(|head| {
                        serde_json::json!({"name": head.name, "ref": format!("refs/heads/{}", head.name), "sha": head.sha})
                    })
                    .collect::<Vec<_>>();
                self.complete_task(
                    task_id,
                    JournalState::Succeeded,
                    None,
                    Some(serde_json::json!({"refs": refs})),
                )
            }
            Err(error) => self.complete_task(
                task_id,
                JournalState::Failed,
                Some(git_error_code(&error)),
                None,
            ),
        }
    }

    async fn execute_spec(
        &self,
        task_id: &str,
        idempotency_key: &str,
        payload_digest: &str,
        spec: RunnerSpec,
    ) -> Result<TaskJournal, ExecuteError> {
        let _admission = self.admission_lock.lock().await;
        match self.journal.load(task_id) {
            Ok(existing) if existing.payload_digest != payload_digest => {
                return Err(ExecuteError::PayloadConflict);
            }
            Ok(_) => {
                return Err(ExecuteError::Duplicate);
            }
            Err(JournalError::Missing) => {}
            Err(error) => {
                return Err(error.into());
            }
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
        journal.git_lease_id = spec
            .two_stage
            .as_ref()
            .and_then(|two_stage| two_stage.git_lease_id.clone());
        self.journal.store(&journal)?;
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
        cleanup_secret(&task_dir);
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

    pub fn load(&self, task_id: &str) -> Result<TaskJournal, ExecuteError> {
        Ok(self.journal.load(task_id)?)
    }

    pub fn recover(&self, task_id: &str) -> Result<RecoveryState, ExecuteError> {
        let state = self.journal.recover(task_id)?;
        if matches!(
            state,
            RecoveryState::Terminal(_) | RecoveryState::Interrupted(_)
        ) {
            cleanup_secret(&self.journal.task_dir(task_id));
        }
        Ok(state)
    }

    pub fn poll_completion(&self, task_id: &str) -> Result<Option<TaskJournal>, ExecuteError> {
        let task_dir = self.journal.task_dir(task_id);
        let completion_path = task_dir.join("completion.json");
        if !completion_path.exists() {
            return Ok(None);
        }
        let mut journal = self.journal.load(task_id)?;
        let completion: Completion = read_json(&completion_path)?;
        apply_completion(&mut journal, completion);
        self.journal.store(&journal)?;
        cleanup_secret(&task_dir);
        Ok(Some(journal))
    }

    pub fn active_task_ids(&self) -> Result<Vec<String>, ExecuteError> {
        Ok(self.journal.active_task_ids()?)
    }

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.journal.task_dir(task_id)
    }

    pub fn cleanup_secret(&self, task_id: &str) {
        cleanup_secret(&self.journal.task_dir(task_id));
    }

    pub fn store_journal(&self, journal: &TaskJournal) -> Result<(), ExecuteError> {
        Ok(self.journal.store(journal)?)
    }

    pub async fn create_task(
        &self,
        task_id: &str,
        idempotency_key: &str,
        payload_digest: &str,
    ) -> Result<TaskJournal, ExecuteError> {
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
        Ok(self
            .journal
            .create(task_id, idempotency_key, payload_digest)?)
    }

    pub fn complete_task(
        &self,
        task_id: &str,
        state: JournalState,
        error_code: Option<String>,
        result_data: Option<Value>,
    ) -> Result<TaskJournal, ExecuteError> {
        if !matches!(
            state,
            JournalState::Succeeded | JournalState::Failed | JournalState::Interrupted
        ) {
            return Err(ExecuteError::InvalidState);
        }
        let mut journal = self.journal.load(task_id)?;
        journal.state = state;
        journal.error_code = error_code;
        journal.result_data = result_data;
        self.journal.store(&journal)?;
        cleanup_secret(&self.journal.task_dir(task_id));
        Ok(journal)
    }
}

fn validate_task(task: &DeploymentExecuteTask) -> Result<RunnerSpec, ExecuteError> {
    if task.wrapper_version != WRAPPER_VERSION {
        return Err(ExecuteError::UnsupportedWrapper);
    }
    if task.deployment_id.is_empty()
        || task.deployment_id.len() > 128
        || !task
            .deployment_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || !(1..=86_400).contains(&task.timeout_seconds)
        || task.argument_tokens.len() > 128
        || task
            .argument_tokens
            .iter()
            .any(|token| token.len() > 4096 || token.chars().any(char::is_control))
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
    let mut environment_keys = HashSet::new();
    for reference in &task.environment_file_references {
        if !valid_environment_key(&reference.environment_key)
            || matches!(
                reference.environment_key.as_str(),
                "DEPLOY_ID" | "DEPLOY_CANCEL_FILE"
            )
            || !environment_keys.insert(reference.environment_key.as_str())
        {
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
        two_stage: None,
    })
}

fn validate_two_stage_paths(
    work_root: &str,
    checkout_dir: &str,
    artifact_dir: Option<&str>,
) -> Result<(), ExecuteError> {
    let root = PathBuf::from(work_root);
    let checkout = PathBuf::from(checkout_dir);
    if !root.is_absolute()
        || !checkout.is_absolute()
        || !absolute_path_within(&checkout, &root)
        || checkout == root
    {
        return Err(ExecuteError::PathOutsideWorkRoot);
    }
    if let Some(artifact_dir) = artifact_dir {
        let artifact = PathBuf::from(artifact_dir);
        if !artifact.is_absolute()
            || !absolute_path_within(&artifact, &root)
            || artifact == checkout
            || absolute_path_within(&artifact, &checkout)
            || absolute_path_within(&checkout, &artifact)
        {
            return Err(ExecuteError::PathOutsideWorkRoot);
        }
    }
    Ok(())
}

fn absolute_path_within(path: &Path, root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    !relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    })
}

fn validate_git_source(repository_url: &str, commit_sha: &str) -> Result<(), ExecuteError> {
    if repository_url.is_empty()
        || repository_url.len() > 2048
        || repository_url.chars().any(char::is_control)
        || repository_url.split("://").nth(1).is_some_and(|authority| {
            authority
                .split('/')
                .next()
                .is_some_and(|host| host.contains('@'))
        })
        || !(commit_sha.len() == 40 && commit_sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(ExecuteError::InvalidTask);
    }
    Ok(())
}

fn validate_release_metadata(
    release_version: &str,
    modules: &[String],
    timeout_seconds: u32,
) -> Result<(), ExecuteError> {
    if release_version.is_empty()
        || release_version.len() > 256
        || release_version.chars().any(char::is_control)
        || !(1..=86_400).contains(&timeout_seconds)
        || modules.is_empty()
        || modules.len() > 128
    {
        return Err(ExecuteError::InvalidTask);
    }
    let mut seen = HashSet::new();
    for module in modules {
        if module.is_empty()
            || module.len() > 128
            || !module.bytes().enumerate().all(|(index, byte)| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || (index > 0 && matches!(byte, b'.' | b'-' | b'_'))
            })
            || !seen.insert(module.as_str())
        {
            return Err(ExecuteError::InvalidTask);
        }
    }
    Ok(())
}

fn git_error_code(error: &git::GitError) -> String {
    match error {
        git::GitError::Timeout => "git_timeout".to_owned(),
        git::GitError::InvalidCommit => "git_invalid_commit".to_owned(),
        git::GitError::CommitUnavailable => "git_commit_unavailable".to_owned(),
        git::GitError::DirtyWorktree => "git_dirty_worktree".to_owned(),
        git::GitError::InvalidRepository => "git_invalid_repository".to_owned(),
        git::GitError::AuthenticationFailed => "git_authentication_failed".to_owned(),
        git::GitError::RepositoryUnreachable => "git_repository_unreachable".to_owned(),
        git::GitError::CommandFailed(_) => "git_command_failed".to_owned(),
        git::GitError::Io(_) => "git_io_error".to_owned(),
    }
}

fn cleanup_secret(task_dir: &Path) {
    let _ = fs::remove_file(task_dir.join("git-key"));
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
    let completion_path = task_dir.join("completion.json");
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if path.exists() {
            return read_json(&path);
        }
        if completion_path.exists() {
            // runner 可能在记录进程身份前就因校验失败退出，任务已完成，无需 PID。
            return Ok(ProcessIdentity {
                pid: 0,
                start_time: None,
            });
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
