use std::{
    fmt,
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

#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentCredentials {
    pub agent_id: String,
    pub refresh_token: String,
}

impl fmt::Debug for AgentCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentCredentials")
            .field("agent_id", &self.agent_id)
            .field("refresh_token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct CredentialStore {
    path: PathBuf,
}

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("凭证文件不存在")]
    Missing,
    #[error("凭证目录权限必须为 0700")]
    UnsafeDirectoryPermissions,
    #[error("凭证文件权限必须为 0600")]
    UnsafeFilePermissions,
    #[error("凭证内容无效")]
    Invalid,
    #[error("凭证文件操作失败")]
    Io(#[source] io::Error),
}

impl CredentialStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<AgentCredentials, CredentialError> {
        let parent = self.parent()?;
        verify_directory(parent)?;
        let file = read_file(&self.path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                CredentialError::Missing
            } else {
                CredentialError::Io(error)
            }
        })?;
        verify_file(&file.metadata().map_err(CredentialError::Io)?)?;
        let credentials: AgentCredentials =
            serde_json::from_reader(file).map_err(|_| CredentialError::Invalid)?;
        validate(&credentials)?;
        Ok(credentials)
    }

    pub fn store(&self, credentials: &AgentCredentials) -> Result<(), CredentialError> {
        validate(credentials)?;
        let parent = self.parent()?;
        ensure_directory(parent)?;
        verify_directory(parent)?;
        let bytes = serde_json::to_vec(credentials).map_err(|_| CredentialError::Invalid)?;
        let temporary = temporary_path(parent, &self.path);

        let result = (|| {
            let mut file = secure_file(&temporary)?;
            file.write_all(&bytes).map_err(CredentialError::Io)?;
            file.sync_all().map_err(CredentialError::Io)?;
            fs::rename(&temporary, &self.path).map_err(CredentialError::Io)?;
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(CredentialError::Io)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn parent(&self) -> Result<&Path, CredentialError> {
        self.path.parent().ok_or(CredentialError::Invalid)
    }
}

fn validate(credentials: &AgentCredentials) -> Result<(), CredentialError> {
    if credentials.agent_id.trim().is_empty()
        || credentials.agent_id.chars().any(char::is_control)
        || credentials.refresh_token.len() < 32
        || credentials.refresh_token.chars().any(char::is_whitespace)
    {
        Err(CredentialError::Invalid)
    } else {
        Ok(())
    }
}

fn ensure_directory(path: &Path) -> Result<(), CredentialError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(CredentialError::Io)?;
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(CredentialError::Io)?;
    }
    Ok(())
}

#[cfg(unix)]
fn verify_directory(path: &Path) -> Result<(), CredentialError> {
    let metadata = fs::symlink_metadata(path).map_err(CredentialError::Io)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o777 != 0o700
    {
        return Err(CredentialError::UnsafeDirectoryPermissions);
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_directory(path: &Path) -> Result<(), CredentialError> {
    if fs::metadata(path).map_err(CredentialError::Io)?.is_dir() {
        Ok(())
    } else {
        Err(CredentialError::UnsafeDirectoryPermissions)
    }
}

#[cfg(unix)]
fn verify_file(metadata: &fs::Metadata) -> Result<(), CredentialError> {
    if !metadata.is_file() || metadata.permissions().mode() & 0o777 != 0o600 {
        Err(CredentialError::UnsafeFilePermissions)
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn verify_file(metadata: &fs::Metadata) -> Result<(), CredentialError> {
    if metadata.is_file() {
        Ok(())
    } else {
        Err(CredentialError::UnsafeFilePermissions)
    }
}

fn secure_file(path: &Path) -> Result<File, CredentialError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options.open(path).map_err(CredentialError::Io)
}

fn temporary_path(parent: &Path, destination: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("credentials");
    parent.join(format!(
        ".{name}.tmp-{}-{timestamp}-{sequence}",
        std::process::id()
    ))
}

fn read_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    options.open(path)
}
