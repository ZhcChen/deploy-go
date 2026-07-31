use std::{process::Stdio, time::Duration};

use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

use super::ssh::ProbeError;

pub async fn run(
    program: &str,
    args: &[String],
    stdin: Option<&[u8]>,
    timeout_duration: Duration,
) -> Result<std::process::Output, ProbeError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|_| ProbeError::new("process_start_failed", "无法启动 SSH 工具"))?;
    if let Some(input) = stdin {
        let mut child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| ProbeError::new("process_start_failed", "无法写入 SSH 工具"))?;
        child_stdin
            .write_all(input)
            .await
            .map_err(|_| ProbeError::new("process_io_failed", "SSH 工具输入失败"))?;
    }
    timeout(timeout_duration, child.wait_with_output())
        .await
        .map_err(|_| ProbeError::new("timeout", "SSH 操作超时"))?
        .map_err(|_| ProbeError::new("process_io_failed", "SSH 工具执行失败"))
}
