#[cfg(all(unix, not(any(target_os = "linux", test))))]
use std::io::Read;
#[cfg(all(unix, not(any(target_os = "linux", test))))]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
#[cfg(any(target_os = "linux", test))]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
#[cfg(any(target_os = "linux", test))]
use std::sync::Arc;
#[cfg(any(target_os = "linux", test))]
use std::{io::Read, path::Path};

use serde::{Deserialize, Serialize};
#[cfg(any(target_os = "linux", test))]
use tokio::net::UnixListener;
#[cfg(any(target_os = "linux", test))]
use tokio::process::Command;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

const PROTOCOL_VERSION: u16 = 1;
const MAX_FRAME_BYTES: usize = 4096;
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
#[cfg(any(target_os = "linux", test))]
const INACTIVE_TASK_MODE: u32 = 0o3700;
#[cfg(any(target_os = "linux", test))]
const ACTIVE_TASK_MODE: u32 = 0o3770;
pub const DEFAULT_RUNNER_SOCKET_PATH: &str = "/run/deploy-go-agent-runner/runner.sock";
const DEFAULT_RUNNER_HOME: &str = "/var/lib/deploy-go-runner";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchRequest {
    version: u16,
    action: RequestAction,
    task_id: String,
    cancel_grace_millis: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestAction {
    Launch,
    Cancel,
    Version,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchResponse {
    version: u16,
    accepted: bool,
    error_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionResponse {
    version: u16,
    package_version: String,
}

#[cfg(any(target_os = "linux", test))]
enum RunnerReply {
    Launch(LaunchResponse),
    Version(VersionResponse),
}

#[derive(Clone, Debug)]
pub struct RunnerServiceClient {
    socket_path: PathBuf,
}

impl RunnerServiceClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub async fn launch(&self, task_id: &str) -> std::io::Result<()> {
        let response = self.request(RequestAction::Launch, task_id, None).await?;
        accepted(response)
    }

    pub async fn cancel(&self, task_id: &str, grace: std::time::Duration) -> std::io::Result<()> {
        let grace_millis = grace.min(std::time::Duration::from_secs(30)).as_millis() as u64;
        let response = self
            .request(RequestAction::Cancel, task_id, Some(grace_millis))
            .await?;
        accepted(response)
    }

    pub async fn probe(&self) -> bool {
        matches!(
            self.request(RequestAction::Launch, "", None).await,
            Ok(LaunchResponse {
                version: PROTOCOL_VERSION,
                accepted: false,
                error_code: Some(code),
            }) if code == "invalid_request"
        )
    }

    pub async fn probe_version(&self) -> Option<String> {
        match self.request_version().await {
            Ok(VersionResponse {
                version: PROTOCOL_VERSION,
                package_version,
            }) if !package_version.is_empty() => Some(package_version),
            _ => None,
        }
    }

    async fn request(
        &self,
        action: RequestAction,
        task_id: &str,
        cancel_grace_millis: Option<u64>,
    ) -> std::io::Result<LaunchResponse> {
        let timeout = cancel_grace_millis
            .map(|millis| std::time::Duration::from_millis(millis) + REQUEST_TIMEOUT)
            .unwrap_or(REQUEST_TIMEOUT);
        tokio::time::timeout(
            timeout,
            self.request_inner(action, task_id, cancel_grace_millis),
        )
        .await
        .map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::TimedOut, "runner request timed out")
        })?
    }

    async fn request_inner(
        &self,
        action: RequestAction,
        task_id: &str,
        cancel_grace_millis: Option<u64>,
    ) -> std::io::Result<LaunchResponse> {
        let mut stream = UnixStream::connect(&self.socket_path).await?;
        write_frame(
            &mut stream,
            &LaunchRequest {
                version: PROTOCOL_VERSION,
                action,
                task_id: task_id.to_owned(),
                cancel_grace_millis,
            },
        )
        .await?;
        let response: LaunchResponse = read_frame(&mut stream).await?;
        if response.version != PROTOCOL_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "runner protocol mismatch",
            ));
        }
        Ok(response)
    }

    async fn request_version(&self) -> std::io::Result<VersionResponse> {
        tokio::time::timeout(REQUEST_TIMEOUT, async {
            let mut stream = UnixStream::connect(&self.socket_path).await?;
            write_frame(
                &mut stream,
                &LaunchRequest {
                    version: PROTOCOL_VERSION,
                    action: RequestAction::Version,
                    task_id: String::new(),
                    cancel_grace_millis: None,
                },
            )
            .await?;
            let response: VersionResponse = read_frame(&mut stream).await?;
            if response.version != PROTOCOL_VERSION {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "runner protocol mismatch",
                ));
            }
            Ok(response)
        })
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "runner version probe timed out",
            )
        })?
    }
}

fn accepted(response: LaunchResponse) -> std::io::Result<()> {
    if response.accepted {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            response
                .error_code
                .unwrap_or_else(|| "runner_launch_rejected".to_owned()),
        ))
    }
}

#[cfg(target_os = "linux")]
pub async fn serve_from_env() -> anyhow::Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        anyhow::bail!("runner broker 必须以 root 运行");
    }
    let socket_path = required_absolute_env("DEPLOY_GO_RUNNER_SOCKET")?;
    let task_root = required_absolute_env("DEPLOY_GO_RUNNER_TASK_ROOT")?;
    let allowed_uid = required_id_env("DEPLOY_GO_RUNNER_ALLOWED_UID")?;
    let allowed_gid = required_id_env("DEPLOY_GO_RUNNER_ALLOWED_GID")?;
    let runner_uid = required_id_env("DEPLOY_GO_RUNNER_UID")?;
    let runner_gid = required_id_env("DEPLOY_GO_RUNNER_GID")?;
    ensure_runner_home(&runner_home_path(), runner_uid, runner_gid)?;
    serve(
        &socket_path,
        &task_root,
        allowed_uid,
        allowed_gid,
        runner_uid,
        runner_gid,
    )
    .await
}

#[cfg(unix)]
pub async fn run_cancel_from_args() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};

    ensure!(
        unsafe { libc::geteuid() } != 0,
        "runner-cancel 不得以 root 运行"
    );
    let mut args = std::env::args_os().skip(2);
    let task_dir = PathBuf::from(args.next().context("runner-cancel 缺少任务目录")?);
    let grace_millis: u64 = args
        .next()
        .context("runner-cancel 缺少宽限时间")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("runner-cancel 宽限时间无效"))?
        .parse()
        .context("runner-cancel 宽限时间无效")?;
    ensure!(args.next().is_none(), "runner-cancel 参数过多");
    ensure!(grace_millis <= 30_000, "runner-cancel 宽限时间过长");

    cancel_as_runner(&task_dir, grace_millis).await
}

#[cfg(unix)]
async fn cancel_as_runner(task_dir: &std::path::Path, grace_millis: u64) -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };

    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let file = options
        .open(task_dir.join("process.json"))
        .context("读取任务进程身份失败")?;
    let metadata = file.metadata().context("读取任务进程元数据失败")?;
    let current_uid = unsafe { libc::geteuid() };
    ensure!(
        metadata.is_file()
            && metadata.uid() == current_uid
            && metadata.nlink() == 1
            && metadata.mode() & 0o007 == 0,
        "任务进程身份文件不可信"
    );
    let identity: crate::runner::ProcessIdentity =
        serde_json::from_reader(file.take(4097)).context("解析任务进程身份失败")?;
    let expected_start = identity.start_time.context("任务进程缺少启动时间")?;
    ensure!(
        crate::journal::process_start_time(identity.pid).ok() == Some(expected_start),
        "任务进程身份不匹配"
    );
    let raw_pid = i32::try_from(identity.pid).context("任务 PID 无效")?;
    let group = Pid::from_raw(-raw_pid);
    kill(group, Signal::SIGTERM).context("发送 SIGTERM 失败")?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(grace_millis);
    while crate::journal::process_start_time(identity.pid).ok() == Some(expected_start)
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    if crate::journal::process_start_time(identity.pid).ok() == Some(expected_start) {
        kill(group, Signal::SIGKILL).context("发送 SIGKILL 失败")?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub async fn run_cancel_from_args() -> anyhow::Result<()> {
    anyhow::bail!("runner-cancel 仅支持 Unix")
}

#[cfg(not(target_os = "linux"))]
pub async fn serve_from_env() -> anyhow::Result<()> {
    anyhow::bail!("runner broker 仅支持 Linux")
}

#[cfg(any(target_os = "linux", test))]
async fn serve(
    socket_path: &Path,
    task_root: &Path,
    allowed_uid: u32,
    allowed_gid: u32,
    runner_uid: u32,
    runner_gid: u32,
) -> anyhow::Result<()> {
    let parent = socket_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("runner socket 缺少父目录"))?;
    std::fs::create_dir_all(parent)?;
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
    let recovered = recover_active_task(task_root, allowed_uid, runner_uid, runner_gid)?;
    let active_task = Arc::new(tokio::sync::Mutex::new(
        recovered.as_ref().map(|(task_id, _, _)| task_id.clone()),
    ));
    if let Some((task_id, pid, start_time)) = recovered {
        let recovered_task = Arc::clone(&active_task);
        let recovered_root = task_root.to_owned();
        let recovered_dir = task_root.join(&task_id);
        tokio::spawn(async move {
            while crate::journal::process_start_time(pid).ok() == Some(start_time) {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            revoke_and_clear_task(
                &recovered_task,
                &task_id,
                &recovered_root,
                &recovered_dir,
                allowed_uid,
                runner_gid,
            )
            .await;
        });
    }
    #[cfg(unix)]
    {
        std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o660))?;
        let path = std::ffi::CString::new(socket_path.as_os_str().as_encoded_bytes())?;
        if unsafe { libc::chown(path.as_ptr(), u32::MAX, allowed_gid) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    loop {
        let (mut stream, _) = listener.accept().await?;
        let task_root = task_root.to_owned();
        let active_task = Arc::clone(&active_task);
        tokio::spawn(async move {
            let reply = match authorize_peer(&stream, allowed_uid, allowed_gid) {
                Ok(()) => {
                    handle_request(
                        &mut stream,
                        &task_root,
                        allowed_uid,
                        runner_uid,
                        runner_gid,
                        active_task,
                    )
                    .await
                }
                Err(_) => Err("unauthorized_peer"),
            };
            match reply {
                Ok(RunnerReply::Launch(response)) => {
                    let _ = write_frame(&mut stream, &response).await;
                }
                Ok(RunnerReply::Version(response)) => {
                    let _ = write_frame(&mut stream, &response).await;
                }
                Err(code) => {
                    let response = LaunchResponse {
                        version: PROTOCOL_VERSION,
                        accepted: false,
                        error_code: Some(code.to_owned()),
                    };
                    let _ = write_frame(&mut stream, &response).await;
                }
            }
        });
    }
}

#[cfg(any(target_os = "linux", test))]
async fn handle_request(
    stream: &mut UnixStream,
    task_root: &Path,
    allowed_uid: u32,
    runner_uid: u32,
    runner_gid: u32,
    active_task: Arc<tokio::sync::Mutex<Option<String>>>,
) -> Result<RunnerReply, &'static str> {
    let request: LaunchRequest = read_frame(stream).await.map_err(|_| "invalid_request")?;
    if matches!(request.action, RequestAction::Version) {
        if request.version != PROTOCOL_VERSION
            || !request.task_id.is_empty()
            || request.cancel_grace_millis.is_some()
        {
            return Err("invalid_request");
        }
        return Ok(RunnerReply::Version(VersionResponse {
            version: PROTOCOL_VERSION,
            package_version: env!("CARGO_PKG_VERSION").to_owned(),
        }));
    }
    if request.version != PROTOCOL_VERSION || !valid_task_id(&request.task_id) {
        return Err("invalid_request");
    }
    if matches!(request.action, RequestAction::Cancel) {
        let grace = request.cancel_grace_millis.ok_or("invalid_request")?;
        if grace > 30_000 {
            return Err("invalid_request");
        }
        if active_task.lock().await.as_deref() != Some(request.task_id.as_str()) {
            return Err("runner_task_not_active");
        }
        let task_dir = task_root.join(&request.task_id);
        validate_task_dir(
            task_root,
            &task_dir,
            allowed_uid,
            runner_gid,
            ACTIVE_TASK_MODE,
        )?;
        let executable = runner_executable()?;
        let mut command = Command::new(executable);
        command
            .arg("runner-cancel")
            .arg(&task_dir)
            .arg(grace.to_string())
            .env_clear()
            .env(
                "PATH",
                "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            )
            .env("LANG", "C.UTF-8")
            .env("HOME", runner_home_path())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        set_runner_identity(&mut command, runner_uid, runner_gid);
        return match command.status().await {
            Ok(status) if status.success() => Ok(RunnerReply::Launch(LaunchResponse {
                version: PROTOCOL_VERSION,
                accepted: true,
                error_code: None,
            })),
            _ => Err("runner_cancel_failed"),
        };
    }
    if request.cancel_grace_millis.is_some() {
        return Err("invalid_request");
    }
    let task_dir = task_root.join(&request.task_id);
    let spec_path = task_dir.join("runner-spec.json");
    let spec = read_owned_spec(task_root, &task_dir, &spec_path, allowed_uid, runner_gid)?;
    let executable = runner_executable()?;
    {
        let mut active = active_task.lock().await;
        if active.is_some() {
            return Err("runner_busy");
        }
        *active = Some(request.task_id.clone());
    }
    let spec = match prepare_runner_credential(&spec, &task_dir, runner_uid, runner_gid) {
        Ok(spec) => spec,
        Err(code) => {
            clear_active_task(&active_task, &request.task_id).await;
            return Err(code);
        }
    };
    if set_task_mode(
        task_root,
        &task_dir,
        allowed_uid,
        runner_gid,
        INACTIVE_TASK_MODE,
        ACTIVE_TASK_MODE,
    )
    .is_err()
    {
        let _ = std::fs::remove_file(task_dir.join("runner-git-key"));
        clear_active_task(&active_task, &request.task_id).await;
        return Err("task_permission_failed");
    }
    let launch_marker = task_dir.join("runner-launch.lock");
    let mut marker_options = std::fs::OpenOptions::new();
    marker_options.write(true).create_new(true);
    #[cfg(unix)]
    marker_options.mode(0o640);
    let marker = match marker_options.open(&launch_marker) {
        Ok(marker) => marker,
        Err(_) => {
            let _ = std::fs::remove_file(task_dir.join("runner-git-key"));
            revoke_and_clear_task(
                &active_task,
                &request.task_id,
                task_root,
                &task_dir,
                allowed_uid,
                runner_gid,
            )
            .await;
            return Err("runner_already_launched");
        }
    };
    drop(marker);
    let mut command = Command::new(executable);
    command
        .arg("runner-stdin")
        .arg(&task_dir)
        .env_clear()
        .env(
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        )
        .env("LANG", "C.UTF-8")
        .env("HOME", runner_home_path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(false);
    set_runner_identity(&mut command, runner_uid, runner_gid);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            let _ = std::fs::remove_file(&launch_marker);
            let _ = std::fs::remove_file(task_dir.join("runner-git-key"));
            revoke_and_clear_task(
                &active_task,
                &request.task_id,
                task_root,
                &task_dir,
                allowed_uid,
                runner_gid,
            )
            .await;
            return Err("runner_spawn_failed");
        }
    };
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill().await;
        let _ = std::fs::remove_file(&launch_marker);
        let _ = std::fs::remove_file(task_dir.join("runner-git-key"));
        revoke_and_clear_task(
            &active_task,
            &request.task_id,
            task_root,
            &task_dir,
            allowed_uid,
            runner_gid,
        )
        .await;
        return Err("runner_spawn_failed");
    };
    if stdin.write_all(&spec).await.is_err() || stdin.shutdown().await.is_err() {
        let _ = child.kill().await;
        let _ = std::fs::remove_file(launch_marker);
        let _ = std::fs::remove_file(task_dir.join("runner-git-key"));
        revoke_and_clear_task(
            &active_task,
            &request.task_id,
            task_root,
            &task_dir,
            allowed_uid,
            runner_gid,
        )
        .await;
        return Err("runner_spawn_failed");
    }
    let process_identity = task_dir.join("process.json");
    let task_id = request.task_id;
    let task_root = task_root.to_owned();
    tokio::spawn(async move {
        let _ = child.wait().await;
        if !process_identity.is_file() {
            let _ = std::fs::remove_file(launch_marker);
        }
        revoke_and_clear_task(
            &active_task,
            &task_id,
            &task_root,
            &task_dir,
            allowed_uid,
            runner_gid,
        )
        .await;
    });
    Ok(RunnerReply::Launch(LaunchResponse {
        version: PROTOCOL_VERSION,
        accepted: true,
        error_code: None,
    }))
}

#[cfg(any(target_os = "linux", test))]
async fn clear_active_task(active_task: &tokio::sync::Mutex<Option<String>>, task_id: &str) {
    let mut active = active_task.lock().await;
    if active.as_deref() == Some(task_id) {
        *active = None;
    }
}

#[cfg(any(target_os = "linux", test))]
async fn revoke_and_clear_task(
    active_task: &tokio::sync::Mutex<Option<String>>,
    task_id: &str,
    task_root: &Path,
    task_dir: &Path,
    allowed_uid: u32,
    shared_gid: u32,
) {
    if revoke_task_access(task_root, task_dir, allowed_uid, shared_gid).is_ok() {
        clear_active_task(active_task, task_id).await;
    }
}

#[cfg(any(target_os = "linux", test))]
fn recover_active_task(
    task_root: &Path,
    allowed_uid: u32,
    runner_uid: u32,
    runner_gid: u32,
) -> anyhow::Result<Option<(String, u32, u64)>> {
    let mut recovered = None;
    for entry in std::fs::read_dir(task_root)? {
        let entry = entry?;
        let Some(task_id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !valid_task_id(&task_id) {
            continue;
        }
        let task_dir = entry.path();
        let Ok(metadata) = std::fs::symlink_metadata(&task_dir) else {
            continue;
        };
        let mode = metadata.mode() & 0o7777;
        if mode == INACTIVE_TASK_MODE {
            continue;
        }
        validate_task_dir(
            task_root,
            &task_dir,
            allowed_uid,
            runner_gid,
            ACTIVE_TASK_MODE,
        )
        .map_err(|code| anyhow::anyhow!("活动任务目录不可信：{task_id} ({code})"))?;
        let identity_path = task_dir.join("process.json");
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        let file = options
            .open(identity_path)
            .map_err(|_| anyhow::anyhow!("活动任务进程身份缺失：{task_id}"))?;
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != runner_uid
            || metadata.nlink() != 1
            || metadata.mode() & 0o007 != 0
        {
            anyhow::bail!("活动任务进程身份不可信：{task_id}");
        }
        let identity =
            serde_json::from_reader::<_, crate::runner::ProcessIdentity>(file.take(4097))
                .map_err(|_| anyhow::anyhow!("活动任务进程身份无效：{task_id}"))?;
        let start_time = identity
            .start_time
            .ok_or_else(|| anyhow::anyhow!("活动任务进程缺少启动时间：{task_id}"))?;
        if crate::journal::process_start_time(identity.pid).ok() != Some(start_time)
            || process_uid(identity.pid) != Some(runner_uid)
        {
            revoke_task_access(task_root, &task_dir, allowed_uid, runner_gid)
                .map_err(|_| anyhow::anyhow!("陈旧 runner 目录权限恢复失败：{task_id}"))?;
            continue;
        }
        if recovered.is_some() {
            anyhow::bail!("检测到多个活动 runner，拒绝启动 broker");
        }
        recovered = Some((task_id, identity.pid, start_time));
    }
    Ok(recovered)
}

#[cfg(any(target_os = "linux", test))]
fn process_uid(pid: u32) -> Option<u32> {
    std::fs::metadata(format!("/proc/{pid}"))
        .ok()
        .map(|metadata| metadata.uid())
}

#[cfg(any(target_os = "linux", test))]
fn set_runner_identity(command: &mut Command, runner_uid: u32, runner_gid: u32) {
    unsafe {
        command.pre_exec(move || {
            libc::umask(0o027);
            if (libc::geteuid() == 0 && libc::setgroups(0, std::ptr::null()) != 0)
                || libc::setgid(runner_gid) != 0
                || libc::setuid(runner_uid) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(any(target_os = "linux", test))]
fn runner_executable() -> Result<PathBuf, &'static str> {
    let current = std::env::current_exe().map_err(|_| "runner_unavailable")?;
    resolve_runner_executable(current)
}

#[cfg(any(target_os = "linux", test))]
fn runner_home_path() -> PathBuf {
    std::env::var_os("DEPLOY_GO_RUNNER_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RUNNER_HOME))
}

#[cfg(target_os = "linux")]
fn ensure_runner_home(path: &Path, runner_uid: u32, runner_gid: u32) -> anyhow::Result<()> {
    use std::ffi::CString;
    use std::os::unix::fs::PermissionsExt;

    if path.is_symlink() {
        anyhow::bail!("runner home 不能是符号链接: {}", path.display());
    }
    std::fs::create_dir_all(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    let path_c = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| anyhow::anyhow!("runner home 路径无效"))?;
    if unsafe { libc::chown(path_c.as_ptr(), runner_uid, runner_gid) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn resolve_runner_executable(current: PathBuf) -> Result<PathBuf, &'static str> {
    if current.exists() {
        return Ok(current);
    }
    let Some(stripped) = current
        .to_str()
        .and_then(|value| value.strip_suffix(" (deleted)"))
    else {
        return Err("runner_unavailable");
    };
    let fallback = PathBuf::from(stripped);
    if fallback.exists() {
        Ok(fallback)
    } else {
        Err("runner_unavailable")
    }
}

#[cfg(any(target_os = "linux", test))]
fn read_owned_spec(
    task_root: &Path,
    task_dir: &Path,
    spec_path: &Path,
    allowed_uid: u32,
    shared_gid: u32,
) -> Result<Vec<u8>, &'static str> {
    validate_task_dir(
        task_root,
        task_dir,
        allowed_uid,
        shared_gid,
        INACTIVE_TASK_MODE,
    )?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let file = options.open(spec_path).map_err(|_| "spec_invalid")?;
    let metadata = file.metadata().map_err(|_| "spec_invalid")?;
    if !metadata.is_file() {
        return Err("spec_invalid");
    }
    #[cfg(unix)]
    if metadata.uid() != allowed_uid || metadata.nlink() != 1 || metadata.mode() & 0o007 != 0 {
        return Err("spec_invalid");
    }
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "spec_invalid")?;
    if bytes.len() > 64 * 1024 {
        return Err("spec_invalid");
    }
    let spec =
        serde_json::from_slice::<crate::runner::RunnerSpec>(&bytes).map_err(|_| "spec_invalid")?;
    validate_task_secret_references(&spec, task_dir, allowed_uid, shared_gid)?;
    Ok(bytes)
}

#[cfg(any(target_os = "linux", test))]
fn validate_task_secret_references(
    spec: &crate::runner::RunnerSpec,
    task_dir: &Path,
    allowed_uid: u32,
    shared_gid: u32,
) -> Result<(), &'static str> {
    if let Some(directory) = &spec.environment_directory {
        if directory != &task_dir.join("env") {
            return Err("spec_secret_path_invalid");
        }
        validate_shared_path(directory, allowed_uid, shared_gid, true)?;
    }
    for (_, path) in &spec.environment_file_references {
        if path.parent() != Some(task_dir.join("refs").as_path()) {
            return Err("spec_secret_path_invalid");
        }
        validate_shared_path(path, allowed_uid, shared_gid, false)?;
    }
    if let Some(path) = spec
        .two_stage
        .as_ref()
        .and_then(|two_stage| two_stage.credential_file.as_ref())
    {
        if path != &task_dir.join("git-key") {
            return Err("spec_secret_path_invalid");
        }
        validate_private_path(path, allowed_uid, shared_gid)?;
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn validate_private_path(
    path: &Path,
    allowed_uid: u32,
    shared_gid: u32,
) -> Result<(), &'static str> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "spec_secret_path_invalid")?;
    if metadata.file_type().is_symlink()
        || metadata.uid() != allowed_uid
        || metadata.gid() != shared_gid
        || metadata.mode() & 0o077 != 0
        || !metadata.is_file()
        || metadata.nlink() != 1
    {
        return Err("spec_secret_path_invalid");
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn prepare_runner_credential(
    spec_bytes: &[u8],
    task_dir: &Path,
    runner_uid: u32,
    runner_gid: u32,
) -> Result<Vec<u8>, &'static str> {
    let mut spec: crate::runner::RunnerSpec =
        serde_json::from_slice(spec_bytes).map_err(|_| "runner_spec_invalid")?;
    let Some(source) = spec
        .two_stage
        .as_ref()
        .and_then(|two_stage| two_stage.credential_file.as_ref())
        .cloned()
    else {
        return Ok(spec_bytes.to_vec());
    };
    if source != task_dir.join("git-key") {
        return Err("spec_secret_path_invalid");
    }
    let target = task_dir.join("runner-git-key");
    let _ = std::fs::remove_file(&target);
    let mut input_options = std::fs::OpenOptions::new();
    input_options.read(true);
    #[cfg(unix)]
    input_options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let input = input_options
        .open(&source)
        .map_err(|_| "runner_credential_failed")?;
    let input_metadata = input.metadata().map_err(|_| "runner_credential_failed")?;
    #[cfg(unix)]
    if !input_metadata.is_file()
        || input_metadata.nlink() != 1
        || input_metadata.mode() & 0o077 != 0
        || input_metadata.len() > 1024 * 1024
    {
        return Err("runner_credential_failed");
    }
    #[cfg(not(unix))]
    if !input_metadata.is_file() || input_metadata.len() > 1024 * 1024 {
        return Err("runner_credential_failed");
    }
    let mut output_options = std::fs::OpenOptions::new();
    output_options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        output_options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let mut output = output_options
        .open(&target)
        .map_err(|_| "runner_credential_failed")?;
    std::io::copy(&mut input.take(1024 * 1024 + 1), &mut output)
        .map_err(|_| "runner_credential_failed")?;
    let output_metadata = output.metadata().map_err(|_| "runner_credential_failed")?;
    if output_metadata.len() > 1024 * 1024 {
        let _ = std::fs::remove_file(&target);
        return Err("runner_credential_failed");
    }
    output.sync_all().map_err(|_| "runner_credential_failed")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if output_metadata.uid() != runner_uid || output_metadata.gid() != runner_gid {
            let path = std::ffi::CString::new(target.as_os_str().as_encoded_bytes())
                .map_err(|_| "runner_credential_failed")?;
            if unsafe { libc::chown(path.as_ptr(), runner_uid, runner_gid) } != 0 {
                let _ = std::fs::remove_file(&target);
                return Err("runner_credential_failed");
            }
        }
    }
    if let Some(two_stage) = spec.two_stage.as_mut() {
        two_stage.credential_file = Some(target);
    }
    serde_json::to_vec(&spec).map_err(|_| "runner_credential_failed")
}

#[cfg(any(target_os = "linux", test))]
fn validate_shared_path(
    path: &Path,
    allowed_uid: u32,
    shared_gid: u32,
    directory: bool,
) -> Result<(), &'static str> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "spec_secret_path_invalid")?;
    if metadata.file_type().is_symlink()
        || metadata.uid() != allowed_uid
        || metadata.gid() != shared_gid
        || metadata.mode() & 0o007 != 0
        || (directory && !metadata.is_dir())
        || (!directory && (!metadata.is_file() || metadata.nlink() != 1))
    {
        return Err("spec_secret_path_invalid");
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn validate_task_dir(
    task_root: &Path,
    task_dir: &Path,
    allowed_uid: u32,
    shared_gid: u32,
    expected_mode: u32,
) -> Result<(), &'static str> {
    let root = std::fs::canonicalize(task_root).map_err(|_| "task_root_invalid")?;
    let task_metadata = std::fs::symlink_metadata(task_dir).map_err(|_| "task_invalid")?;
    if task_metadata.file_type().is_symlink() || !task_metadata.is_dir() {
        return Err("task_invalid");
    }
    #[cfg(unix)]
    if task_metadata.uid() != allowed_uid
        || task_metadata.gid() != shared_gid
        || task_metadata.mode() & 0o7777 != expected_mode
    {
        return Err("task_invalid");
    }
    let canonical_task = std::fs::canonicalize(task_dir).map_err(|_| "task_invalid")?;
    if canonical_task.parent() != Some(root.as_path()) {
        return Err("task_invalid");
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn set_task_mode(
    task_root: &Path,
    task_dir: &Path,
    allowed_uid: u32,
    shared_gid: u32,
    current_mode: u32,
    new_mode: u32,
) -> Result<(), &'static str> {
    validate_task_dir(task_root, task_dir, allowed_uid, shared_gid, current_mode)?;
    std::fs::set_permissions(task_dir, std::fs::Permissions::from_mode(new_mode))
        .map_err(|_| "task_permission_failed")
}

#[cfg(any(target_os = "linux", test))]
fn revoke_task_access(
    task_root: &Path,
    task_dir: &Path,
    allowed_uid: u32,
    shared_gid: u32,
) -> Result<(), &'static str> {
    set_task_mode(
        task_root,
        task_dir,
        allowed_uid,
        shared_gid,
        ACTIVE_TASK_MODE,
        INACTIVE_TASK_MODE,
    )
}

#[cfg(target_os = "linux")]
fn authorize_peer(stream: &UnixStream, allowed_uid: u32, allowed_gid: u32) -> std::io::Result<()> {
    let credentials = stream.peer_cred()?;
    if credentials.uid() == allowed_uid && credentials.gid() == allowed_gid {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "unauthorized runner peer",
        ))
    }
}

#[cfg(all(not(target_os = "linux"), test))]
fn authorize_peer(
    _stream: &UnixStream,
    _allowed_uid: u32,
    _allowed_gid: u32,
) -> std::io::Result<()> {
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn valid_task_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(target_os = "linux")]
fn required_absolute_env(name: &str) -> anyhow::Result<PathBuf> {
    let value = std::env::var_os(name).ok_or_else(|| anyhow::anyhow!("缺少 {name}"))?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        anyhow::bail!("{name} 必须是绝对路径");
    }
    Ok(path)
}

#[cfg(target_os = "linux")]
fn required_id_env(name: &str) -> anyhow::Result<u32> {
    std::env::var(name)
        .map_err(anyhow::Error::from)?
        .parse()
        .map_err(anyhow::Error::from)
}

async fn write_frame<T: Serialize>(stream: &mut UnixStream, value: &T) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "runner frame too large",
        ));
    }
    stream.write_u32(bytes.len() as u32).await?;
    stream.write_all(&bytes).await
}

async fn read_frame<T: for<'de> Deserialize<'de>>(stream: &mut UnixStream) -> std::io::Result<T> {
    let length = stream.read_u32().await? as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid runner frame",
        ));
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await?;
    serde_json::from_slice(&bytes).map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_executable_falls_back_from_deleted_current_exe() {
        let fixture = tempfile::tempdir().unwrap();
        let current = fixture.path().join("deploy-go-agent (deleted)");
        let fallback = fixture.path().join("deploy-go-agent");
        std::fs::write(&fallback, b"binary").unwrap();
        assert_eq!(
            resolve_runner_executable(current).unwrap(),
            fallback
        );
    }

    #[test]
    fn task_ids_and_paths_are_bounded() {
        assert!(valid_task_id("task_01ABC"));
        assert!(!valid_task_id("../task"));
        assert!(!valid_task_id(""));
    }

    #[test]
    #[cfg(unix)]
    fn task_secret_references_cannot_escape_the_task_directory() {
        let fixture = tempfile::tempdir().unwrap();
        let task_dir = fixture.path().join("task");
        let env_dir = task_dir.join("env");
        std::fs::create_dir_all(&env_dir).unwrap();
        std::fs::set_permissions(&env_dir, std::fs::Permissions::from_mode(0o2750)).unwrap();
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        let mut spec = crate::runner::RunnerSpec {
            deployment_id: "deployment_01ABC".to_owned(),
            script_path: PathBuf::from("/bin/true"),
            argument_tokens: Vec::new(),
            environment_file_references: Vec::new(),
            environment_directory: Some(env_dir),
            timeout_seconds: 30,
            log_budget_bytes: 1024,
            two_stage: None,
        };
        assert!(validate_task_secret_references(&spec, &task_dir, uid, gid).is_ok());

        spec.environment_directory = Some(fixture.path().join("outside"));
        assert_eq!(
            validate_task_secret_references(&spec, &task_dir, uid, gid),
            Err("spec_secret_path_invalid")
        );
    }

    #[cfg(unix)]
    fn spec_with_git_credential(task_dir: &Path) -> crate::runner::RunnerSpec {
        crate::runner::RunnerSpec {
            deployment_id: "deployment_01ABC".to_owned(),
            script_path: PathBuf::from("/bin/true"),
            argument_tokens: Vec::new(),
            environment_file_references: Vec::new(),
            environment_directory: None,
            timeout_seconds: 30,
            log_budget_bytes: 1024,
            two_stage: Some(crate::runner::TwoStageRunnerSpec {
                stage: deploy_go_agent_protocol::DeploymentStage::Prepare,
                checkout_dir: task_dir.join("checkout"),
                work_root: task_dir.join("work"),
                repository_url: Some("git@example.test:repo.git".to_owned()),
                commit_sha: "0123456789abcdef".to_owned(),
                credential_file: Some(task_dir.join("git-key")),
                environment: deploy_go_agent_protocol::Environment::Test,
                release_version: "20260813".to_owned(),
                target_code: None,
                modules: vec!["api".to_owned()],
                artifact_dir: Some(task_dir.join("staging")),
                staging_size_limit_bytes: 1024,
                staging_max_files: 10,
                git_lease_id: None,
            }),
        }
    }

    #[cfg(unix)]
    #[test]
    fn git_key_requires_0600_and_agent_ownership() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = tempfile::tempdir().unwrap();
        let task_dir = fixture.path().join("task");
        std::fs::create_dir_all(&task_dir).unwrap();
        let key_path = task_dir.join("git-key");
        std::fs::write(&key_path, b"private").unwrap();
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o640)).unwrap();
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        let spec = spec_with_git_credential(&task_dir);
        assert_eq!(
            validate_task_secret_references(&spec, &task_dir, uid, gid),
            Err("spec_secret_path_invalid")
        );

        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(validate_task_secret_references(&spec, &task_dir, uid, gid).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn prepare_runner_credential_creates_private_runner_copy() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = tempfile::tempdir().unwrap();
        let task_dir = fixture.path().join("task");
        std::fs::create_dir_all(&task_dir).unwrap();
        let key_path = task_dir.join("git-key");
        std::fs::write(&key_path, b"private-key").unwrap();
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        let spec = spec_with_git_credential(&task_dir);
        let bytes = serde_json::to_vec(&spec).unwrap();

        let updated = prepare_runner_credential(&bytes, &task_dir, uid, gid).unwrap();
        let parsed: crate::runner::RunnerSpec = serde_json::from_slice(&updated).unwrap();
        let credential = parsed
            .two_stage
            .expect("two_stage spec")
            .credential_file
            .expect("credential path");
        assert_eq!(credential, task_dir.join("runner-git-key"));
        assert_eq!(
            std::fs::metadata(&credential).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(std::fs::read(&credential).unwrap(), b"private-key");
        assert_eq!(
            std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[tokio::test]
    async fn client_launches_only_a_fixed_task_spec() {
        let fixture = tempfile::tempdir().unwrap();
        let task_root = fixture.path().join("tasks");
        let task_dir = task_root.join("task_01ABC");
        std::fs::create_dir_all(&task_dir).unwrap();
        std::fs::create_dir(task_root.join("probe")).unwrap();
        #[cfg(unix)]
        {
            std::fs::set_permissions(&task_root, std::fs::Permissions::from_mode(0o3710)).unwrap();
            std::fs::set_permissions(&task_dir, std::fs::Permissions::from_mode(0o3700)).unwrap();
            std::fs::set_permissions(
                task_root.join("probe"),
                std::fs::Permissions::from_mode(0o3700),
            )
            .unwrap();
        }
        let spec_path = task_dir.join("runner-spec.json");
        let spec = crate::runner::RunnerSpec {
            deployment_id: "deployment_01ABC".to_owned(),
            script_path: PathBuf::from("/bin/true"),
            argument_tokens: vec![],
            environment_file_references: vec![],
            environment_directory: None,
            timeout_seconds: 30,
            log_budget_bytes: 1024,
            two_stage: None,
        };
        std::fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(&spec_path, std::fs::Permissions::from_mode(0o640)).unwrap();
        let socket = fixture.path().join("runner.sock");
        #[cfg(unix)]
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        #[cfg(not(unix))]
        let (uid, gid) = (0, 0);
        let service_socket = socket.clone();
        let service_root = task_root.clone();
        let service =
            tokio::spawn(
                async move { serve(&service_socket, &service_root, uid, gid, uid, gid).await },
            );
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let client = RunnerServiceClient::new(socket);
        assert!(client.probe().await);
        assert_eq!(
            client.probe_version().await.as_deref(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(!task_root.join("probe/runner-launch.lock").exists());
        client.launch("task_01ABC").await.unwrap();
        assert!(task_dir.join("runner-launch.lock").is_file());
        service.abort();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn runner_identity_can_cancel_its_process_group() {
        let fixture = tempfile::tempdir().unwrap();
        let mut command = tokio::process::Command::new("sleep");
        command.arg("30");
        command.process_group(0);
        let mut child = command.spawn().unwrap();
        let pid = child.id().unwrap();
        let identity = crate::runner::ProcessIdentity {
            pid,
            start_time: Some(crate::journal::process_start_time(pid).unwrap()),
        };
        let process_path = fixture.path().join("process.json");
        std::fs::write(&process_path, serde_json::to_vec(&identity).unwrap()).unwrap();
        std::fs::set_permissions(&process_path, std::fs::Permissions::from_mode(0o640)).unwrap();

        cancel_as_runner(fixture.path(), 100).await.unwrap();
        let status = child.wait().await.unwrap();
        assert!(!status.success());
    }
}
