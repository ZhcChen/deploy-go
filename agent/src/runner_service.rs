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
pub const DEFAULT_RUNNER_SOCKET_PATH: &str = "/run/deploy-go-agent-runner/runner.sock";

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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchResponse {
    version: u16,
    accepted: bool,
    error_code: Option<String>,
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
    let active_task = Arc::new(tokio::sync::Mutex::new(None::<String>));
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
            let response = match authorize_peer(&stream, allowed_uid, allowed_gid) {
                Ok(()) => {
                    handle_launch(
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
            let response = match response {
                Ok(()) => LaunchResponse {
                    version: PROTOCOL_VERSION,
                    accepted: true,
                    error_code: None,
                },
                Err(code) => LaunchResponse {
                    version: PROTOCOL_VERSION,
                    accepted: false,
                    error_code: Some(code.to_owned()),
                },
            };
            let _ = write_frame(&mut stream, &response).await;
        });
    }
}

#[cfg(any(target_os = "linux", test))]
async fn handle_launch(
    stream: &mut UnixStream,
    task_root: &Path,
    allowed_uid: u32,
    runner_uid: u32,
    runner_gid: u32,
    active_task: Arc<tokio::sync::Mutex<Option<String>>>,
) -> Result<(), &'static str> {
    let request: LaunchRequest = read_frame(stream).await.map_err(|_| "invalid_request")?;
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
        validate_task_dir(task_root, &task_dir, allowed_uid, runner_gid)?;
        let executable = std::env::current_exe().map_err(|_| "runner_unavailable")?;
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
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        set_runner_identity(&mut command, runner_uid, runner_gid);
        return match command.status().await {
            Ok(status) if status.success() => Ok(()),
            _ => Err("runner_cancel_failed"),
        };
    }
    if request.cancel_grace_millis.is_some() {
        return Err("invalid_request");
    }
    let task_dir = task_root.join(&request.task_id);
    let spec_path = task_dir.join("runner-spec.json");
    let spec = read_owned_spec(task_root, &task_dir, &spec_path, allowed_uid, runner_gid)?;
    let executable = std::env::current_exe().map_err(|_| "runner_unavailable")?;
    {
        let mut active = active_task.lock().await;
        if active.is_some() {
            return Err("runner_busy");
        }
        *active = Some(request.task_id.clone());
    }
    let launch_marker = task_dir.join("runner-launch.lock");
    let mut marker_options = std::fs::OpenOptions::new();
    marker_options.write(true).create_new(true);
    #[cfg(unix)]
    marker_options.mode(0o640);
    let marker = match marker_options.open(&launch_marker) {
        Ok(marker) => marker,
        Err(_) => {
            clear_active_task(&active_task, &request.task_id).await;
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
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(false);
    set_runner_identity(&mut command, runner_uid, runner_gid);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            let _ = std::fs::remove_file(&launch_marker);
            clear_active_task(&active_task, &request.task_id).await;
            return Err("runner_spawn_failed");
        }
    };
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill().await;
        let _ = std::fs::remove_file(&launch_marker);
        clear_active_task(&active_task, &request.task_id).await;
        return Err("runner_spawn_failed");
    };
    if stdin.write_all(&spec).await.is_err() || stdin.shutdown().await.is_err() {
        let _ = child.kill().await;
        let _ = std::fs::remove_file(launch_marker);
        clear_active_task(&active_task, &request.task_id).await;
        return Err("runner_spawn_failed");
    }
    let process_identity = task_dir.join("process.json");
    let task_id = request.task_id;
    tokio::spawn(async move {
        let _ = child.wait().await;
        if !process_identity.is_file() {
            let _ = std::fs::remove_file(launch_marker);
        }
        clear_active_task(&active_task, &task_id).await;
    });
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
async fn clear_active_task(active_task: &tokio::sync::Mutex<Option<String>>, task_id: &str) {
    let mut active = active_task.lock().await;
    if active.as_deref() == Some(task_id) {
        *active = None;
    }
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
fn read_owned_spec(
    task_root: &Path,
    task_dir: &Path,
    spec_path: &Path,
    allowed_uid: u32,
    shared_gid: u32,
) -> Result<Vec<u8>, &'static str> {
    validate_task_dir(task_root, task_dir, allowed_uid, shared_gid)?;
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
    if bytes.len() > 64 * 1024
        || serde_json::from_slice::<crate::runner::RunnerSpec>(&bytes).is_err()
    {
        return Err("spec_invalid");
    }
    Ok(bytes)
}

#[cfg(any(target_os = "linux", test))]
fn validate_task_dir(
    task_root: &Path,
    task_dir: &Path,
    allowed_uid: u32,
    shared_gid: u32,
) -> Result<(), &'static str> {
    let root = std::fs::canonicalize(task_root).map_err(|_| "task_root_invalid")?;
    let task_metadata = std::fs::symlink_metadata(task_dir).map_err(|_| "task_invalid")?;
    if task_metadata.file_type().is_symlink() || !task_metadata.is_dir() {
        return Err("task_invalid");
    }
    #[cfg(unix)]
    if task_metadata.uid() != allowed_uid
        || task_metadata.gid() != shared_gid
        || task_metadata.mode() & 0o7777 != 0o3770
    {
        return Err("task_invalid");
    }
    let canonical_task = std::fs::canonicalize(task_dir).map_err(|_| "task_invalid")?;
    if canonical_task.parent() != Some(root.as_path()) {
        return Err("task_invalid");
    }
    Ok(())
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
    fn task_ids_and_paths_are_bounded() {
        assert!(valid_task_id("task_01ABC"));
        assert!(!valid_task_id("../task"));
        assert!(!valid_task_id(""));
    }

    #[tokio::test]
    async fn client_launches_only_a_fixed_task_spec() {
        let fixture = tempfile::tempdir().unwrap();
        let task_root = fixture.path().join("tasks");
        let task_dir = task_root.join("task_01ABC");
        std::fs::create_dir_all(&task_dir).unwrap();
        std::fs::create_dir(task_root.join("probe")).unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(&task_dir, std::fs::Permissions::from_mode(0o3770)).unwrap();
        let spec_path = task_dir.join("runner-spec.json");
        let spec = crate::runner::RunnerSpec {
            deployment_id: "deployment_01ABC".to_owned(),
            script_path: PathBuf::from("/bin/true"),
            argument_tokens: vec![],
            environment_file_references: vec![],
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
