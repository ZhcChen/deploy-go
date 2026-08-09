#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    process::Command,
};

const PROTOCOL_VERSION: u16 = 1;
const MAX_FRAME_BYTES: usize = 4096;
pub const DEFAULT_RUNNER_SOCKET_PATH: &str = "/run/deploy-go-agent-runner/runner.sock";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchRequest {
    version: u16,
    task_id: String,
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
        let mut stream = UnixStream::connect(&self.socket_path).await?;
        write_frame(
            &mut stream,
            &LaunchRequest {
                version: PROTOCOL_VERSION,
                task_id: task_id.to_owned(),
            },
        )
        .await?;
        let response: LaunchResponse = read_frame(&mut stream).await?;
        if response.version == PROTOCOL_VERSION && response.accepted {
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
}

pub async fn serve_from_env() -> anyhow::Result<()> {
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
        tokio::spawn(async move {
            let response = match authorize_peer(&stream, allowed_uid, allowed_gid) {
                Ok(()) => {
                    handle_launch(&mut stream, &task_root, allowed_uid, runner_uid, runner_gid)
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

async fn handle_launch(
    stream: &mut UnixStream,
    task_root: &Path,
    allowed_uid: u32,
    runner_uid: u32,
    runner_gid: u32,
) -> Result<(), &'static str> {
    let request: LaunchRequest = read_frame(stream).await.map_err(|_| "invalid_request")?;
    if request.version != PROTOCOL_VERSION || !valid_task_id(&request.task_id) {
        return Err("invalid_request");
    }
    let task_dir = task_root.join(&request.task_id);
    let spec_path = task_dir.join("runner-spec.json");
    let spec = read_owned_spec(task_root, &task_dir, &spec_path, allowed_uid)?;
    let launch_marker = task_dir.join("runner-launch.lock");
    let marker = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&launch_marker)
        .map_err(|_| "runner_already_launched")?;
    drop(marker);
    let executable = std::env::current_exe().map_err(|_| "runner_unavailable")?;
    let mut command = Command::new(executable);
    command
        .arg("runner-stdin")
        .arg(&task_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(false);
    #[cfg(unix)]
    unsafe {
        command.pre_exec(move || {
            if libc::setgid(runner_gid) != 0 || libc::setuid(runner_uid) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            let _ = std::fs::remove_file(&launch_marker);
            return Err("runner_spawn_failed");
        }
    };
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill().await;
        let _ = std::fs::remove_file(&launch_marker);
        return Err("runner_spawn_failed");
    };
    if stdin.write_all(&spec).await.is_err() || stdin.shutdown().await.is_err() {
        let _ = child.kill().await;
        let _ = std::fs::remove_file(launch_marker);
        return Err("runner_spawn_failed");
    }
    Ok(())
}

fn read_owned_spec(
    task_root: &Path,
    task_dir: &Path,
    spec_path: &Path,
    allowed_uid: u32,
) -> Result<Vec<u8>, &'static str> {
    let root = std::fs::canonicalize(task_root).map_err(|_| "task_root_invalid")?;
    let task_metadata = std::fs::symlink_metadata(task_dir).map_err(|_| "task_invalid")?;
    if task_metadata.file_type().is_symlink() || !task_metadata.is_dir() {
        return Err("task_invalid");
    }
    let canonical_task = std::fs::canonicalize(task_dir).map_err(|_| "task_invalid")?;
    if canonical_task.parent() != Some(root.as_path()) {
        return Err("task_invalid");
    }
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

#[cfg(not(target_os = "linux"))]
fn authorize_peer(
    _stream: &UnixStream,
    _allowed_uid: u32,
    _allowed_gid: u32,
) -> std::io::Result<()> {
    Ok(())
}

fn valid_task_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn required_absolute_env(name: &str) -> anyhow::Result<PathBuf> {
    let value = std::env::var_os(name).ok_or_else(|| anyhow::anyhow!("缺少 {name}"))?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        anyhow::bail!("{name} 必须是绝对路径");
    }
    Ok(path)
}

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

        RunnerServiceClient::new(socket)
            .launch("task_01ABC")
            .await
            .unwrap();
        assert!(task_dir.join("runner-launch.lock").is_file());
        service.abort();
    }
}
