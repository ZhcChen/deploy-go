use std::{io::Write, time::Duration};

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use zeroize::Zeroizing;

use super::{
    process,
    ssh::{ProbeError, encode_posix_token},
};

const EXECUTION_TIMEOUT_PADDING: Duration = Duration::from_secs(15);
const CANCEL_TIMEOUT: Duration = Duration::from_secs(55);
const WRAPPER_SHA256: &str = "b0e176cddd6417157d5ccb410448d7de5d3cc30b1104b821a3856f2b23822506";
const WRAPPER: &[u8] = br#"set -eu
deploy_id=$1
work_root=$2
script_path=$3
shift 3
run_dir="$work_root/.deploy-go/$deploy_id"
mkdir -p "$run_dir"
cancel_file="$run_dir/cancel"
status_file="$run_dir/status"
export DEPLOY_ID="$deploy_id" DEPLOY_CANCEL_FILE="$cancel_file"
if [ -e "$cancel_file" ]; then printf 'canceled\n' >"$status_file"; exit 130; fi
setsid "$script_path" "$@" &
pid=$!
printf '%s\n' "$pid" >"$run_dir/pid"
if [ -e "$cancel_file" ]; then kill -TERM -- -"$pid" 2>/dev/null || true; fi
set +e
wait "$pid"
status=$?
set -e
if [ -e "$cancel_file" ]; then printf 'canceled\n' >"$status_file"; else printf 'finished\n' >"$status_file"; fi
exit "$status"
"#;

#[derive(Clone, Debug)]
pub struct OutputChunk {
    pub stream: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub chunks: Vec<OutputChunk>,
    pub exit_code: i32,
}

pub struct ExecutionContext {
    pub deployment_id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub work_root: String,
    pub script_path: String,
    pub argument_tokens: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub trusted_host_key: String,
    pub private_key: Zeroizing<Vec<u8>>,
    pub timeout: Duration,
}

#[async_trait]
pub trait DeploymentExecutor: Send + Sync {
    async fn execute(
        &self,
        context: &ExecutionContext,
        output: tokio::sync::mpsc::Sender<OutputChunk>,
    ) -> Result<i32, ProbeError>;
    async fn cancel(&self, context: &ExecutionContext) -> Result<(), ProbeError>;
}

#[derive(Clone, Debug)]
pub struct OpenSshDeploymentExecutor {
    ssh_program: String,
}

impl Default for OpenSshDeploymentExecutor {
    fn default() -> Self {
        Self {
            ssh_program: "ssh".to_owned(),
        }
    }
}

impl OpenSshDeploymentExecutor {
    pub fn with_program(program: impl Into<String>) -> Self {
        Self {
            ssh_program: program.into(),
        }
    }
}

#[async_trait]
impl DeploymentExecutor for OpenSshDeploymentExecutor {
    async fn execute(
        &self,
        context: &ExecutionContext,
        output: tokio::sync::mpsc::Sender<OutputChunk>,
    ) -> Result<i32, ProbeError> {
        if format!("{:x}", Sha256::digest(WRAPPER)) != WRAPPER_SHA256 {
            return Err(ProbeError::new(
                "wrapper_integrity_failed",
                "部署包装器完整性校验失败",
            ));
        }
        let identity = write_temp(context.private_key.as_slice(), "无法准备部署身份文件")?;
        let known_hosts = write_temp(
            format!("{}\n", context.trusted_host_key).as_bytes(),
            "无法准备部署 host key 文件",
        )?;
        let mut remote_tokens = Vec::new();
        remote_tokens.push("env".to_owned());
        for (key, value) in &context.environment {
            remote_tokens.push(format!("{key}={value}"));
        }
        remote_tokens.push("sh".to_owned());
        remote_tokens.push("-s".to_owned());
        remote_tokens.push("--".to_owned());
        remote_tokens.push(context.deployment_id.clone());
        remote_tokens.push(context.work_root.clone());
        remote_tokens.push(context.script_path.clone());
        remote_tokens.extend(context.argument_tokens.clone());
        let remote_command = remote_tokens
            .iter()
            .map(|token| encode_posix_token(token))
            .collect::<Vec<_>>()
            .join(" ");
        let args = ssh_args(context, identity.path(), known_hosts.path(), remote_command);
        process::run_streaming(
            &self.ssh_program,
            &args,
            Some(WRAPPER),
            context.timeout + EXECUTION_TIMEOUT_PADDING,
            output,
        )
        .await
    }

    async fn cancel(&self, context: &ExecutionContext) -> Result<(), ProbeError> {
        let identity = write_temp(context.private_key.as_slice(), "无法准备取消身份文件")?;
        let known_hosts = write_temp(
            format!("{}\n", context.trusted_host_key).as_bytes(),
            "无法准备取消 host key 文件",
        )?;
        let run_dir = format!(
            "{}/.deploy-go/{}",
            context.work_root.trim_end_matches('/'),
            context.deployment_id
        );
        let command = format!(
            "run_dir={}; mkdir -p \"$run_dir\" || exit 20; touch \"$run_dir/cancel\" || exit 21; i=0; while [ ! -f \"$run_dir/pid\" ] && [ \"$(cat \"$run_dir/status\" 2>/dev/null || true)\" != canceled ] && [ \"$i\" -lt 10 ]; do sleep 1; i=$((i+1)); done; [ \"$(cat \"$run_dir/status\" 2>/dev/null || true)\" = canceled ] && exit 0; [ -f \"$run_dir/pid\" ] || exit 22; pid=$(cat \"$run_dir/pid\") || exit 23; case \"$pid\" in ''|*[!0-9]*) exit 24;; esac; if ! kill -0 -- -\"$pid\" 2>/dev/null; then exit 0; fi; if ! kill -TERM -- -\"$pid\" 2>/dev/null; then kill -0 -- -\"$pid\" 2>/dev/null && exit 25 || exit 0; fi; i=0; while kill -0 -- -\"$pid\" 2>/dev/null && [ \"$i\" -lt 30 ]; do sleep 1; i=$((i+1)); done; if kill -0 -- -\"$pid\" 2>/dev/null; then kill -KILL -- -\"$pid\" 2>/dev/null || exit 26; fi",
            encode_posix_token(&run_dir)
        );
        let args = ssh_args(context, identity.path(), known_hosts.path(), command);
        let output = process::run(&self.ssh_program, &args, None, CANCEL_TIMEOUT).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(ProbeError::new("cancel_failed", "远端取消请求失败"))
        }
    }
}

fn ssh_args(
    context: &ExecutionContext,
    identity: &std::path::Path,
    known_hosts: &std::path::Path,
    remote_command: String,
) -> Vec<String> {
    vec![
        "-F".into(),
        "/dev/null".into(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "IdentitiesOnly=yes".into(),
        "-o".into(),
        "StrictHostKeyChecking=yes".into(),
        "-o".into(),
        format!("UserKnownHostsFile={}", known_hosts.display()),
        "-o".into(),
        "ConnectTimeout=10".into(),
        "-i".into(),
        identity.display().to_string(),
        "-p".into(),
        context.port.to_string(),
        destination(&context.username, &context.host),
        remote_command,
    ]
}
fn destination(username: &str, host: &str) -> String {
    if host.contains(':') {
        format!("{username}@[{host}]")
    } else {
        format!("{username}@{host}")
    }
}
fn write_temp(bytes: &[u8], message: &'static str) -> Result<NamedTempFile, ProbeError> {
    let mut file =
        NamedTempFile::new().map_err(|_| ProbeError::new("temporary_file_failed", message))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|_| ProbeError::new("temporary_file_failed", message))?;
    }
    file.write_all(bytes)
        .map_err(|_| ProbeError::new("temporary_file_failed", message))?;
    Ok(file)
}
