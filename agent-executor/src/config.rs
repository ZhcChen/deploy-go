use serde::Deserialize;
use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

pub const DEFAULT_SOCKET_PATH: &str = "/run/deploy-go-agent/executor.sock";
pub const DEFAULT_CONFIG_PATH: &str = "/etc/deploy-go-agent/executor.json";
pub const DEFAULT_RELEASE_GLOBAL_STORAGE_BYTES: u64 = 50 * 1024 * 1024 * 1024;
pub const DEFAULT_RELEASE_MINIMUM_FREE_BYTES: u64 = 512 * 1024 * 1024;
pub const DEFAULT_RELEASE_RETENTION_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalConfig {
    pub allowed_uid: u32,
    pub allowed_gid: u32,
    pub allowed_executable: PathBuf,
    #[serde(default = "default_shell")]
    pub shell: PathBuf,
    #[serde(default = "default_home")]
    pub home: PathBuf,
    pub node_id: String,
    pub agent_id: String,
    pub capability_public_key: String,
    pub capability_replay_dir: PathBuf,
    pub release_public_key: String,
    pub release_jobs_dir: PathBuf,
    #[serde(default = "default_release_global_storage_bytes")]
    pub release_global_storage_bytes: u64,
    #[serde(default = "default_release_minimum_free_bytes")]
    pub release_minimum_free_bytes: u64,
    #[serde(default = "default_release_retention_seconds")]
    pub release_retention_seconds: u64,
}

fn default_release_global_storage_bytes() -> u64 {
    DEFAULT_RELEASE_GLOBAL_STORAGE_BYTES
}

fn default_release_minimum_free_bytes() -> u64 {
    DEFAULT_RELEASE_MINIMUM_FREE_BYTES
}

fn default_release_retention_seconds() -> u64 {
    DEFAULT_RELEASE_RETENTION_SECONDS
}

fn default_shell() -> PathBuf {
    "/bin/sh".into()
}

fn default_home() -> PathBuf {
    "/root".into()
}

#[derive(Clone, Debug)]
pub struct ExecutorConfig {
    pub socket_path: PathBuf,
    pub allowed_uid: u32,
    pub allowed_gid: u32,
    pub allowed_executable: PathBuf,
    pub shell: PathBuf,
    pub home: PathBuf,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub close_grace: Duration,
    pub max_frame_bytes: usize,
    pub output_buffer_frames: usize,
    pub node_id: String,
    pub agent_id: String,
    pub capability_public_key: String,
    pub capability_replay_dir: PathBuf,
    pub release_public_key: String,
    pub release_jobs_dir: PathBuf,
    pub release_global_storage_bytes: u64,
    pub release_minimum_free_bytes: u64,
    pub release_retention: Duration,
}

impl From<LocalConfig> for ExecutorConfig {
    fn from(value: LocalConfig) -> Self {
        let mut config = Self::system(value.allowed_uid, value.allowed_gid);
        config.shell = value.shell;
        config.home = value.home;
        config.allowed_executable = value.allowed_executable;
        config.node_id = value.node_id;
        config.agent_id = value.agent_id;
        config.capability_public_key = value.capability_public_key;
        config.capability_replay_dir = value.capability_replay_dir;
        config.release_public_key = value.release_public_key;
        config.release_jobs_dir = value.release_jobs_dir;
        config.release_global_storage_bytes = value.release_global_storage_bytes;
        config.release_minimum_free_bytes = value.release_minimum_free_bytes;
        config.release_retention = Duration::from_secs(value.release_retention_seconds);
        config
    }
}

impl ExecutorConfig {
    pub fn system(allowed_uid: u32, allowed_gid: u32) -> Self {
        Self {
            socket_path: DEFAULT_SOCKET_PATH.into(),
            allowed_uid,
            allowed_gid,
            allowed_executable: "/usr/local/bin/deploy-go-agent".into(),
            shell: "/bin/sh".into(),
            home: "/root".into(),
            idle_timeout: Duration::from_secs(15 * 60),
            max_lifetime: Duration::from_secs(4 * 60 * 60),
            close_grace: Duration::from_secs(2),
            max_frame_bytes: 64 * 1024,
            output_buffer_frames: 128,
            node_id: "node_test".into(),
            agent_id: "agent_test".into(),
            capability_public_key: deploy_go_terminal_capability::CapabilitySigner::from_seed(
                [0_u8; 32],
            )
            .public_key_base64(),
            capability_replay_dir: "/var/lib/deploy-go-agent-executor/used-capabilities".into(),
            release_public_key: deploy_go_release_authorization::ReleaseSigner::from_seed(
                [1_u8; 32],
            )
            .public_key_base64(),
            release_jobs_dir: "/var/lib/deploy-go-agent-executor/release-jobs".into(),
            release_global_storage_bytes: DEFAULT_RELEASE_GLOBAL_STORAGE_BYTES,
            release_minimum_free_bytes: DEFAULT_RELEASE_MINIMUM_FREE_BYTES,
            release_retention: Duration::from_secs(DEFAULT_RELEASE_RETENTION_SECONDS),
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.shell.is_absolute() {
            anyhow::bail!("executor shell must be an absolute path");
        }
        let metadata = std::fs::metadata(&self.shell)?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            anyhow::bail!("executor shell must be an executable regular file");
        }
        if !self.home.is_absolute() || !std::fs::metadata(&self.home)?.is_dir() {
            anyhow::bail!("executor home must be an absolute directory");
        }
        if self.allowed_executable != std::path::Path::new("/usr/local/bin/deploy-go-agent") {
            anyhow::bail!("executor allowed executable must be the managed Agent binary");
        }
        if !self.node_id.starts_with("node_")
            || !self.agent_id.starts_with("agent_")
            || self.capability_public_key.is_empty()
            || !self.capability_replay_dir.is_absolute()
            || self.release_public_key.is_empty()
            || !self.release_jobs_dir.is_absolute()
            || self.release_global_storage_bytes == 0
            || self.release_minimum_free_bytes == 0
            || self.release_retention.is_zero()
        {
            anyhow::bail!("executor capability configuration is invalid");
        }
        deploy_go_terminal_capability::CapabilityVerifier::from_base64(
            &self.capability_public_key,
        )?;
        deploy_go_release_authorization::ReleaseVerifier::from_base64(&self.release_public_key)?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn validate_allowed_executable(&self) -> anyhow::Result<()> {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(&self.allowed_executable)?;
        if !metadata.is_file() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            anyhow::bail!("managed Agent binary must be root-owned and not group/world writable");
        }
        Ok(())
    }
}

pub fn set_owned_permissions(path: &std::path::Path, gid: u32, mode: u32) -> std::io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let raw = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains NUL"))?;
    // SAFETY: raw is a valid NUL-terminated path and this helper runs in the root executor.
    if unsafe { libc::chown(raw.as_ptr(), 0, gid) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}
