#[cfg(target_os = "linux")]
use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::{fs::OpenOptionsExt, process::CommandExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
const CGROUP_MOUNT: &str = "/sys/fs/cgroup";
#[cfg(target_os = "linux")]
const LAUNCHER_ARG: &str = "__terminal-session-launch";
#[cfg(target_os = "linux")]
const RELEASE_LAUNCHER_ARG: &str = "__deployment-release-launch";

#[cfg(target_os = "linux")]
pub struct SessionCgroup {
    path: PathBuf,
    launcher: PathBuf,
    cleaned: AtomicBool,
    cleanup_gate: Option<std::sync::Arc<crate::session_claim::SessionRegistry>>,
}

#[cfg(target_os = "linux")]
impl SessionCgroup {
    pub fn create(session_id: &str) -> anyhow::Result<Self> {
        Self::create_with_launcher(session_id, std::fs::read_link("/proc/self/exe")?)
    }

    #[doc(hidden)]
    pub fn create_with_launcher(session_id: &str, launcher: PathBuf) -> anyhow::Result<Self> {
        if !valid_session_id(session_id) {
            anyhow::bail!("invalid terminal session id for cgroup");
        }
        let parent = current_cgroup_dir()?;
        let path = parent.join(format!("terminal-{session_id}"));
        std::fs::create_dir(&path)?;
        let session = Self {
            path,
            launcher,
            cleaned: AtomicBool::new(false),
            cleanup_gate: None,
        };
        if !session.path.join("cgroup.kill").is_file()
            || !session.path.join("cgroup.procs").is_file()
            || !session.path.join("cgroup.events").is_file()
        {
            let _ = std::fs::remove_dir(&session.path);
            anyhow::bail!("cgroup v2 kill interface unavailable");
        }
        Ok(session)
    }

    pub fn with_cleanup_gate(
        mut self,
        cleanup_gate: std::sync::Arc<crate::session_claim::SessionRegistry>,
    ) -> Self {
        self.cleanup_gate = Some(cleanup_gate);
        self
    }

    pub fn launcher_command(&self, shell: &Path) -> anyhow::Result<(PathBuf, Vec<String>)> {
        Ok((
            self.launcher.clone(),
            vec![
                LAUNCHER_ARG.to_owned(),
                self.path.to_string_lossy().into_owned(),
                shell.to_string_lossy().into_owned(),
            ],
        ))
    }

    pub fn kill_all(&self) -> anyhow::Result<()> {
        write_control(&self.path.join("cgroup.kill"), b"1\n")
    }

    pub fn wait_empty_and_remove(&self, timeout: Duration) -> anyhow::Result<()> {
        let deadline = Instant::now() + timeout;
        loop {
            let events = std::fs::read_to_string(self.path.join("cgroup.events"))?;
            if events.lines().any(|line| line == "populated 0") {
                std::fs::remove_dir(&self.path)?;
                self.cleaned.store(true, Ordering::Release);
                return Ok(());
            }
            if Instant::now() >= deadline {
                anyhow::bail!("terminal cgroup remained populated");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for SessionCgroup {
    fn drop(&mut self) {
        if self.cleaned.load(Ordering::Acquire) {
            return;
        }
        let result = self
            .kill_all()
            .and_then(|()| self.wait_empty_and_remove(Duration::from_millis(250)));
        if result.is_err()
            && let Some(gate) = self.cleanup_gate.as_ref()
        {
            gate.block_after_cleanup_failure();
        }
    }
}

#[cfg(target_os = "linux")]
pub struct ReleaseCgroup {
    path: PathBuf,
    launcher: PathBuf,
    cleaned: AtomicBool,
}

#[cfg(target_os = "linux")]
impl ReleaseCgroup {
    pub fn create(job_id: &str) -> anyhow::Result<Self> {
        Self::create_with_launcher(job_id, std::fs::read_link("/proc/self/exe")?)
    }

    #[doc(hidden)]
    pub fn create_with_launcher(job_id: &str, launcher: PathBuf) -> anyhow::Result<Self> {
        if !valid_release_job_id(job_id) {
            anyhow::bail!("invalid release job id for cgroup");
        }
        let parent = current_cgroup_dir()?;
        let path = parent.join(format!("deployment-{job_id}"));
        std::fs::create_dir(&path)?;
        let cgroup = Self {
            path,
            launcher,
            cleaned: AtomicBool::new(false),
        };
        if !cgroup.path.join("cgroup.kill").is_file()
            || !cgroup.path.join("cgroup.procs").is_file()
            || !cgroup.path.join("cgroup.events").is_file()
        {
            let _ = std::fs::remove_dir(&cgroup.path);
            anyhow::bail!("cgroup v2 kill interface unavailable");
        }
        Ok(cgroup)
    }

    pub fn launcher_command(&self) -> (PathBuf, Vec<String>) {
        (
            self.launcher.clone(),
            vec![
                RELEASE_LAUNCHER_ARG.to_owned(),
                self.path.to_string_lossy().into_owned(),
            ],
        )
    }

    pub fn kill_all(&self) -> anyhow::Result<()> {
        write_control(&self.path.join("cgroup.kill"), b"1\n")
    }

    pub fn wait_empty_and_remove(&self, timeout: Duration) -> anyhow::Result<()> {
        wait_empty_and_remove(&self.path, timeout)?;
        self.cleaned.store(true, Ordering::Release);
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for ReleaseCgroup {
    fn drop(&mut self) {
        if self.cleaned.load(Ordering::Acquire) {
            return;
        }
        let _ = self
            .kill_all()
            .and_then(|()| self.wait_empty_and_remove(Duration::from_millis(500)));
    }
}

#[cfg(target_os = "linux")]
pub fn run_launcher_if_requested() -> anyhow::Result<bool> {
    let mut args = std::env::args_os();
    let _program = args.next();
    let operation = args.next();
    if operation.as_deref() == Some(std::ffi::OsStr::new(RELEASE_LAUNCHER_ARG)) {
        let cgroup = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing release cgroup path"))?;
        if args.next().is_some() {
            anyhow::bail!("unexpected release launcher argument");
        }
        let cgroup = PathBuf::from(cgroup);
        validate_release_launcher_target(&cgroup)?;
        write_control(
            &cgroup.join("cgroup.procs"),
            format!("{}\n", std::process::id()).as_bytes(),
        )?;
        let error = std::process::Command::new("/usr/bin/make")
            .args(["--no-print-directory", "deploy-go-release"])
            .exec();
        return Err(error.into());
    }
    if operation.as_deref() != Some(std::ffi::OsStr::new(LAUNCHER_ARG)) {
        return Ok(false);
    }
    let cgroup = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing terminal cgroup path"))?;
    let shell = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing terminal shell path"))?;
    if args.next().is_some() {
        anyhow::bail!("unexpected terminal launcher argument");
    }
    let cgroup = PathBuf::from(cgroup);
    validate_launcher_target(&cgroup)?;
    write_control(
        &cgroup.join("cgroup.procs"),
        format!("{}\n", std::process::id()).as_bytes(),
    )?;
    let error = std::process::Command::new(shell).arg("-l").exec();
    Err(error.into())
}

#[cfg(target_os = "linux")]
fn validate_release_launcher_target(path: &Path) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("release cgroup has no parent"))?;
    if parent != current_cgroup_dir()? {
        anyhow::bail!("release cgroup is outside executor cgroup");
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !name
        .strip_prefix("deployment-")
        .is_some_and(valid_release_job_id)
    {
        anyhow::bail!("release cgroup name is invalid");
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        anyhow::bail!("release cgroup path is invalid");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn wait_empty_and_remove(path: &Path, timeout: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        let events = std::fs::read_to_string(path.join("cgroup.events"))?;
        if events.lines().any(|line| line == "populated 0") {
            std::fs::remove_dir(path)?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!("executor cgroup remained populated");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn current_cgroup_dir() -> anyhow::Result<PathBuf> {
    let current = std::fs::read_to_string("/proc/self/cgroup")?;
    let relative = parse_unified_path(&current)
        .ok_or_else(|| anyhow::anyhow!("executor is not running in cgroup v2"))?;
    let relative = relative.strip_prefix("/").unwrap_or(relative);
    let path = Path::new(CGROUP_MOUNT).join(relative);
    let metadata = std::fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        anyhow::bail!("executor cgroup path is invalid");
    }
    Ok(path)
}

#[cfg(target_os = "linux")]
fn validate_launcher_target(path: &Path) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("terminal cgroup has no parent"))?;
    if parent != current_cgroup_dir()? {
        anyhow::bail!("terminal cgroup is outside executor cgroup");
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !name.strip_prefix("terminal-").is_some_and(valid_session_id) {
        anyhow::bail!("terminal cgroup name is invalid");
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        anyhow::bail!("terminal cgroup path is invalid");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn write_control(path: &Path, value: &[u8]) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    file.write_all(value)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn parse_unified_path(value: &str) -> Option<&Path> {
    value
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .map(Path::new)
}

#[cfg(target_os = "linux")]
fn valid_session_id(value: &str) -> bool {
    value.starts_with("term_")
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(target_os = "linux")]
fn valid_release_job_id(value: &str) -> bool {
    value.starts_with("release_")
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn parses_unified_cgroup_and_validates_session_ids() {
        assert_eq!(
            parse_unified_path("0::/system.slice/deploy-go.service\n"),
            Some(Path::new("/system.slice/deploy-go.service"))
        );
        assert!(valid_session_id("term_01TEST"));
        assert!(!valid_session_id("../term_01TEST"));
        assert!(!valid_session_id("session_01TEST"));
    }
}
