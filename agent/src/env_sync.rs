use std::{
    fs::File,
    io::{self, Read, Write},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};

use nix::{
    errno::Errno,
    fcntl::{OFlag, open, openat, renameat},
    sys::stat::{Mode, mkdirat},
    unistd::{UnlinkatFlags, fsync, unlinkat},
};
use sha2::{Digest, Sha256};
use thiserror::Error;
use ulid::Ulid;
use url::Url;

use crate::token_refresh::AccessProvider;

const FILE_MODE: Mode = Mode::from_bits_truncate(0o600);
const DIRECTORY_MODE: Mode = Mode::from_bits_truncate(0o700);

#[derive(Debug, Error)]
pub enum EnvSyncError {
    #[error("Env 同步未启用")]
    Disabled,
    #[error("Env 同步标识无效")]
    InvalidIdentity,
    #[error("Env 同步摘要不匹配")]
    DigestMismatch,
    #[error("Env 目标不是受控普通文件")]
    UnsafeTarget,
    #[error("Env 同步文件操作失败")]
    Io(#[source] io::Error),
    #[error("Env secret lease 请求失败")]
    Transport,
    #[error("Env secret lease 被拒绝")]
    Rejected,
}

#[derive(Clone)]
pub struct EnvSecretClient {
    client: reqwest::Client,
    api_base: Url,
    access_provider: std::sync::Arc<dyn AccessProvider>,
    enabled: bool,
}

impl EnvSecretClient {
    pub fn new(
        api_base: Url,
        access_provider: std::sync::Arc<dyn AccessProvider>,
        enabled: bool,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("固定 Env HTTPS client 配置有效"),
            api_base,
            access_provider,
            enabled,
        }
    }

    pub async fn fetch(&self, lease_id: &str) -> Result<Vec<u8>, EnvSyncError> {
        if !self.enabled {
            return Err(EnvSyncError::Disabled);
        }
        if !valid_opaque_id(lease_id) {
            return Err(EnvSyncError::InvalidIdentity);
        }
        let mut url = self.api_base.clone();
        url.set_path(&format!("/api/v1/agent/application-env-leases/{lease_id}"));
        let send = |token: String| self.client.get(url.clone()).bearer_auth(token).send();
        let token = self
            .access_provider
            .prepare()
            .await
            .map_err(|_| EnvSyncError::Transport)?
            .access_token;
        let mut response = send(token).await.map_err(|_| EnvSyncError::Transport)?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            let token = self
                .access_provider
                .prepare()
                .await
                .map_err(|_| EnvSyncError::Transport)?
                .access_token;
            response = send(token).await.map_err(|_| EnvSyncError::Transport)?;
        }
        if !response.status().is_success() {
            return Err(EnvSyncError::Rejected);
        }
        let content_length = response.content_length().ok_or(EnvSyncError::Rejected)?;
        if content_length > 1024 * 1024 {
            return Err(EnvSyncError::Rejected);
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| EnvSyncError::Transport)?;
        if bytes.len() as u64 != content_length {
            return Err(EnvSyncError::Transport);
        }
        Ok(bytes.to_vec())
    }
}

#[derive(Clone, Debug)]
pub struct EnvFileStore {
    root: PathBuf,
}

impl EnvFileStore {
    pub fn new(root: PathBuf) -> Result<Self, EnvSyncError> {
        if !root.is_absolute() {
            return Err(EnvSyncError::InvalidIdentity);
        }
        Ok(Self { root })
    }

    pub fn write(
        &self,
        application_slug: &str,
        file_name: &str,
        content: &[u8],
        expected_digest: &str,
    ) -> Result<PathBuf, EnvSyncError> {
        validate_identity(application_slug, file_name, expected_digest)?;
        if hex_digest(content) != expected_digest {
            return Err(EnvSyncError::DigestMismatch);
        }
        let (root, application) = self.open_application(application_slug)?;
        reject_unsafe_existing(&application, file_name)?;

        let temporary = format!(".env-sync-{}", Ulid::new());
        let temporary_fd = openat(
            &application,
            temporary.as_str(),
            OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            FILE_MODE,
        )
        .map_err(nix_io)?;
        let mut temporary_file = File::from(temporary_fd);
        let operation = (|| -> Result<(), EnvSyncError> {
            temporary_file
                .write_all(content)
                .map_err(EnvSyncError::Io)?;
            temporary_file.sync_all().map_err(EnvSyncError::Io)?;
            let metadata = temporary_file.metadata().map_err(EnvSyncError::Io)?;
            if !metadata.is_file() || metadata.nlink() != 1 {
                return Err(EnvSyncError::UnsafeTarget);
            }
            reject_unsafe_existing(&application, file_name)?;
            renameat(&application, temporary.as_str(), &application, file_name).map_err(nix_io)?;
            fsync(&application).map_err(nix_io)?;
            fsync(&root).map_err(nix_io)?;
            Ok(())
        })();
        if operation.is_err() {
            let _ = unlinkat(&application, temporary.as_str(), UnlinkatFlags::NoRemoveDir);
        }
        operation?;
        Ok(self.root.join(application_slug).join(file_name))
    }

    pub fn delete(&self, application_slug: &str, file_name: &str) -> Result<(), EnvSyncError> {
        validate_names(application_slug, file_name)?;
        let (root, application) = self.open_application(application_slug)?;
        match reject_unsafe_existing(&application, file_name) {
            Ok(true) => {
                unlinkat(&application, file_name, UnlinkatFlags::NoRemoveDir).map_err(nix_io)?;
                fsync(&application).map_err(nix_io)?;
                fsync(&root).map_err(nix_io)?;
            }
            Ok(false) => {}
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub fn verify(
        &self,
        application_slug: &str,
        file_name: &str,
        expected_digest: &str,
    ) -> Result<(), EnvSyncError> {
        validate_identity(application_slug, file_name, expected_digest)?;
        let (_, application) = self.open_application(application_slug)?;
        let descriptor = openat(
            &application,
            file_name,
            OFlag::O_RDONLY | OFlag::O_NONBLOCK | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| EnvSyncError::UnsafeTarget)?;
        let mut file = File::from(descriptor);
        let metadata = file.metadata().map_err(EnvSyncError::Io)?;
        if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > 1024 * 1024 {
            return Err(EnvSyncError::UnsafeTarget);
        }
        let mut content = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut content).map_err(EnvSyncError::Io)?;
        let matches = hex_digest(&content) == expected_digest;
        content.fill(0);
        if !matches {
            return Err(EnvSyncError::DigestMismatch);
        }
        Ok(())
    }

    pub fn verify_absent(
        &self,
        application_slug: &str,
        file_name: &str,
    ) -> Result<(), EnvSyncError> {
        validate_names(application_slug, file_name)?;
        let (_, application) = self.open_application(application_slug)?;
        match reject_unsafe_existing(&application, file_name)? {
            false => Ok(()),
            true => Err(EnvSyncError::DigestMismatch),
        }
    }

    fn open_application(
        &self,
        application_slug: &str,
    ) -> Result<(std::os::fd::OwnedFd, std::os::fd::OwnedFd), EnvSyncError> {
        let root = open(
            &self.root,
            OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(nix_io)?;
        match mkdirat(&root, application_slug, DIRECTORY_MODE) {
            Ok(()) | Err(Errno::EEXIST) => {}
            Err(error) => return Err(nix_io(error)),
        }
        let application = openat(
            &root,
            application_slug,
            OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| EnvSyncError::UnsafeTarget)?;
        Ok((root, application))
    }
}

fn reject_unsafe_existing<Fd: std::os::fd::AsFd>(
    directory: &Fd,
    file_name: &str,
) -> Result<bool, EnvSyncError> {
    let descriptor = match openat(
        directory,
        file_name,
        OFlag::O_RDONLY | OFlag::O_NONBLOCK | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    ) {
        Ok(descriptor) => descriptor,
        Err(Errno::ENOENT) => return Ok(false),
        Err(_) => return Err(EnvSyncError::UnsafeTarget),
    };
    let metadata = File::from(descriptor)
        .metadata()
        .map_err(EnvSyncError::Io)?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(EnvSyncError::UnsafeTarget);
    }
    Ok(true)
}

fn validate_identity(
    application_slug: &str,
    file_name: &str,
    digest: &str,
) -> Result<(), EnvSyncError> {
    validate_names(application_slug, file_name)?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(EnvSyncError::InvalidIdentity);
    }
    Ok(())
}

fn validate_names(application_slug: &str, file_name: &str) -> Result<(), EnvSyncError> {
    let valid_component = |value: &str, max: usize| {
        !value.is_empty()
            && value.len() <= max
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && !value.contains("..")
    };
    if !valid_component(application_slug, 128)
        || !valid_component(file_name, 132)
        || !file_name.ends_with(".env")
    {
        return Err(EnvSyncError::InvalidIdentity);
    }
    Ok(())
}

fn valid_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn hex_digest(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}

fn nix_io(error: Errno) -> EnvSyncError {
    EnvSyncError::Io(io::Error::from_raw_os_error(error as i32))
}
