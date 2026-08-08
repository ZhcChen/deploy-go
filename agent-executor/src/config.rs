use serde::Deserialize;
use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

pub const DEFAULT_SOCKET_PATH: &str = "/run/deploy-go-agent/executor.sock";
pub const DEFAULT_CONFIG_PATH: &str = "/etc/deploy-go-agent/executor.json";

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
}

impl From<LocalConfig> for ExecutorConfig {
    fn from(value: LocalConfig) -> Self {
        let mut config = Self::system(value.allowed_uid, value.allowed_gid);
        config.shell = value.shell;
        config.home = value.home;
        config.allowed_executable = value.allowed_executable;
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
