use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalState {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Canceled,
    Interrupted,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferPhase {
    PrepareUpload,
    ReleaseDownload,
    ReleaseExtract,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskJournal {
    pub task_id: String,
    pub idempotency_key: String,
    pub payload_digest: String,
    pub state: JournalState,
    pub pid: Option<u32>,
    pub process_start_time: Option<u64>,
    pub stdout_offset: u64,
    pub stderr_offset: u64,
    #[serde(default)]
    pub events_offset: u64,
    pub last_sequence: u64,
    #[serde(default)]
    pub result_sequence: Option<u64>,
    #[serde(default)]
    pub git_lease_id: Option<String>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    #[serde(default)]
    pub result_data: Option<Value>,
    #[serde(default)]
    pub transfer_phase: Option<TransferPhase>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RecoveryState {
    Accepted(TaskJournal),
    Running(TaskJournal),
    Terminal(TaskJournal),
    Interrupted(TaskJournal),
}

#[derive(Clone, Debug)]
pub struct JournalStore {
    root: PathBuf,
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("任务 ID 或摘要无效")]
    InvalidIdentity,
    #[error("任务 journal 不存在")]
    Missing,
    #[error("任务 journal 内容无效")]
    InvalidJournal,
    #[error("任务 journal 文件操作失败")]
    Io(#[source] io::Error),
}

impl JournalStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn create(
        &self,
        task_id: &str,
        idempotency_key: &str,
        payload_digest: &str,
    ) -> Result<TaskJournal, JournalError> {
        validate_identity(task_id, idempotency_key, payload_digest)?;
        let task = TaskJournal {
            task_id: task_id.to_owned(),
            idempotency_key: idempotency_key.to_owned(),
            payload_digest: payload_digest.to_owned(),
            state: JournalState::Accepted,
            pid: None,
            process_start_time: None,
            stdout_offset: 0,
            stderr_offset: 0,
            events_offset: 0,
            last_sequence: 0,
            result_sequence: None,
            git_lease_id: None,
            exit_code: None,
            error_code: None,
            result_data: None,
            transfer_phase: None,
        };
        self.store(&task)?;
        Ok(task)
    }

    pub fn load(&self, task_id: &str) -> Result<TaskJournal, JournalError> {
        validate_task_id(task_id)?;
        let path = self.task_dir(task_id).join("journal.json");
        let bytes = fs::read(path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                JournalError::Missing
            } else {
                JournalError::Io(error)
            }
        })?;
        let task: TaskJournal =
            serde_json::from_slice(&bytes).map_err(|_| JournalError::InvalidJournal)?;
        validate_identity(&task.task_id, &task.idempotency_key, &task.payload_digest)?;
        if task.task_id != task_id {
            return Err(JournalError::InvalidJournal);
        }
        Ok(task)
    }

    pub fn store(&self, task: &TaskJournal) -> Result<(), JournalError> {
        validate_identity(&task.task_id, &task.idempotency_key, &task.payload_digest)?;
        ensure_private_directory(&self.root)?;
        let task_dir = self.task_dir(&task.task_id);
        ensure_private_directory(&task_dir)?;
        atomic_write(&task_dir.join("journal.json"), task)
    }

    pub fn recover(&self, task_id: &str) -> Result<RecoveryState, JournalError> {
        let mut task = self.load(task_id)?;
        if task.transfer_phase.is_some() {
            return Ok(RecoveryState::Running(task));
        }
        let completion_path = self.task_dir(task_id).join("completion.json");
        if completion_path.exists() {
            let completion: Completion =
                serde_json::from_slice(&fs::read(completion_path).map_err(JournalError::Io)?)
                    .map_err(|_| JournalError::InvalidJournal)?;
            apply_completion(&mut task, completion);
            self.store(&task)?;
            return Ok(RecoveryState::Terminal(task));
        }
        match task.state {
            JournalState::Accepted => {
                task.state = JournalState::Interrupted;
                task.error_code = Some("runner_not_started".to_owned());
                self.store(&task)?;
                Ok(RecoveryState::Interrupted(task))
            }
            JournalState::Running => {
                if owns_process(&task) {
                    Ok(RecoveryState::Running(task))
                } else {
                    task.state = JournalState::Interrupted;
                    task.pid = None;
                    task.process_start_time = None;
                    task.error_code = Some("process_identity_lost".to_owned());
                    self.store(&task)?;
                    Ok(RecoveryState::Interrupted(task))
                }
            }
            JournalState::Succeeded
            | JournalState::Failed
            | JournalState::Canceled
            | JournalState::Interrupted => Ok(RecoveryState::Terminal(task)),
        }
    }

    pub fn find_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<TaskJournal>, JournalError> {
        validate_idempotency_key(idempotency_key)?;
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(JournalError::Io(error)),
        };
        for entry in entries {
            let entry = entry.map_err(JournalError::Io)?;
            if !entry.file_type().map_err(JournalError::Io)?.is_dir() {
                continue;
            }
            let task_id = entry
                .file_name()
                .to_str()
                .map(str::to_owned)
                .ok_or(JournalError::InvalidJournal)?;
            let task = match self.load(&task_id) {
                Err(JournalError::Missing) => continue,
                result => result?,
            };
            if task.idempotency_key == idempotency_key {
                return Ok(Some(task));
            }
        }
        Ok(None)
    }

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.root.join(task_id)
    }

    pub fn active_task_ids(&self) -> Result<Vec<String>, JournalError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(JournalError::Io(error)),
        };
        let mut task_ids = Vec::new();
        for entry in entries {
            let entry = entry.map_err(JournalError::Io)?;
            if !entry.file_type().map_err(JournalError::Io)?.is_dir() {
                continue;
            }
            let Some(task_id) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let task = match self.load(&task_id) {
                Err(JournalError::Missing) => continue,
                result => result?,
            };
            if matches!(task.state, JournalState::Accepted | JournalState::Running) {
                task_ids.push(task_id);
            }
        }
        task_ids.sort();
        Ok(task_ids)
    }
}

pub fn apply_completion(task: &mut TaskJournal, completion: Completion) {
    task.pid = None;
    task.process_start_time = None;
    task.exit_code = completion.exit_code;
    task.error_code = completion.error_code;
    task.state = if task.error_code.as_deref() == Some("task_canceled") {
        JournalState::Canceled
    } else if task.exit_code == Some(0) && task.error_code.is_none() {
        JournalState::Succeeded
    } else {
        JournalState::Failed
    };
}

pub fn process_start_time(pid: u32) -> io::Result<u64> {
    #[cfg(target_os = "linux")]
    {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
        let closing = stat
            .rfind(')')
            .ok_or_else(|| io::Error::other("invalid proc stat"))?;
        let fields = stat[closing + 1..].split_whitespace().collect::<Vec<_>>();
        fields
            .get(19)
            .ok_or_else(|| io::Error::other("missing proc start time"))?
            .parse()
            .map_err(|_| io::Error::other("invalid proc start time"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "process identity is only supported on Linux",
        ))
    }
}

fn owns_process(task: &TaskJournal) -> bool {
    match (task.pid, task.process_start_time) {
        (Some(pid), Some(expected)) => {
            process_start_time(pid).is_ok_and(|actual| actual == expected)
        }
        _ => false,
    }
}

pub(crate) fn validate_identity(
    task_id: &str,
    idempotency_key: &str,
    payload_digest: &str,
) -> Result<(), JournalError> {
    validate_task_id(task_id)?;
    validate_idempotency_key(idempotency_key)?;
    if payload_digest.len() < 16
        || payload_digest.len() > 128
        || !payload_digest
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_'))
    {
        return Err(JournalError::InvalidIdentity);
    }
    Ok(())
}

fn validate_idempotency_key(key: &str) -> Result<(), JournalError> {
    if key.len() < 16
        || key.len() > 128
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
    {
        Err(JournalError::InvalidIdentity)
    } else {
        Ok(())
    }
}

fn validate_task_id(task_id: &str) -> Result<(), JournalError> {
    if task_id.is_empty()
        || task_id.len() > 128
        || !task_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Err(JournalError::InvalidIdentity)
    } else {
        Ok(())
    }
}

fn ensure_private_directory(path: &Path) -> Result<(), JournalError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(JournalError::Io)?;
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(JournalError::Io)?;
    }
    #[cfg(unix)]
    {
        let metadata = fs::symlink_metadata(path).map_err(JournalError::Io)?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.permissions().mode() & 0o777 != 0o700
        {
            return Err(JournalError::Io(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "unsafe journal directory",
            )));
        }
    }
    Ok(())
}

fn atomic_write(path: &Path, task: &TaskJournal) -> Result<(), JournalError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary =
        path.with_extension(format!("tmp-{}-{timestamp}-{sequence}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let result = (|| {
        let mut file = options.open(&temporary).map_err(JournalError::Io)?;
        let bytes = serde_json::to_vec(task).map_err(|_| JournalError::InvalidJournal)?;
        file.write_all(&bytes).map_err(JournalError::Io)?;
        file.sync_all().map_err(JournalError::Io)?;
        fs::rename(&temporary, path).map_err(JournalError::Io)?;
        File::open(path.parent().ok_or(JournalError::InvalidJournal)?)
            .and_then(|directory| directory.sync_all())
            .map_err(JournalError::Io)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
