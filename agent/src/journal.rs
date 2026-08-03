use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
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
#[serde(deny_unknown_fields)]
pub struct TaskJournal {
    pub task_id: String,
    pub payload_digest: String,
    pub state: JournalState,
    pub pid: Option<u32>,
    pub process_start_time: Option<u64>,
    pub stdout_offset: u64,
    pub stderr_offset: u64,
    pub last_sequence: u64,
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

    pub fn create(&self, task_id: &str, payload_digest: &str) -> Result<TaskJournal, JournalError> {
        validate_identity(task_id, payload_digest)?;
        let task = TaskJournal {
            task_id: task_id.to_owned(),
            payload_digest: payload_digest.to_owned(),
            state: JournalState::Accepted,
            pid: None,
            process_start_time: None,
            stdout_offset: 0,
            stderr_offset: 0,
            last_sequence: 0,
            exit_code: None,
            error_code: None,
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
        validate_identity(&task.task_id, &task.payload_digest)?;
        if task.task_id != task_id {
            return Err(JournalError::InvalidJournal);
        }
        Ok(task)
    }

    pub fn store(&self, task: &TaskJournal) -> Result<(), JournalError> {
        validate_identity(&task.task_id, &task.payload_digest)?;
        ensure_private_directory(&self.root)?;
        let task_dir = self.task_dir(&task.task_id);
        ensure_private_directory(&task_dir)?;
        atomic_write(&task_dir.join("journal.json"), task)
    }

    pub fn recover(&self, task_id: &str) -> Result<RecoveryState, JournalError> {
        let mut task = self.load(task_id)?;
        match task.state {
            JournalState::Accepted => Ok(RecoveryState::Accepted(task)),
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

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.root.join(task_id)
    }
}

pub fn process_start_time(pid: u32) -> io::Result<u64> {
    #[cfg(target_os = "linux")]
    {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
        let closing = stat
            .rfind(')')
            .ok_or_else(|| io::Error::other("invalid proc stat"))?;
        let fields = stat[closing + 1..].split_whitespace().collect::<Vec<_>>();
        return fields
            .get(19)
            .ok_or_else(|| io::Error::other("missing proc start time"))?
            .parse()
            .map_err(|_| io::Error::other("invalid proc start time"));
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

fn validate_identity(task_id: &str, payload_digest: &str) -> Result<(), JournalError> {
    validate_task_id(task_id)?;
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
