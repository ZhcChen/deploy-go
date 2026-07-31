use std::{process::Stdio, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::mpsc,
    time::timeout,
};

use super::{deployment::OutputChunk, ssh::ProbeError};

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

pub async fn run_streaming(
    program: &str,
    args: &[String],
    stdin: Option<&[u8]>,
    timeout_duration: Duration,
    output: mpsc::Sender<OutputChunk>,
) -> Result<i32, ProbeError> {
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
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ProbeError::new("process_start_failed", "无法读取 SSH 工具输出"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ProbeError::new("process_start_failed", "无法读取 SSH 工具错误输出"))?;
    let stdout_task = tokio::spawn(forward(stdout, "stdout", output.clone()));
    let stderr_task = tokio::spawn(forward(stderr, "stderr", output));
    let status = match timeout(timeout_duration, child.wait()).await {
        Ok(result) => {
            result.map_err(|_| ProbeError::new("process_io_failed", "SSH 工具执行失败"))?
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            return Err(ProbeError::new("timeout", "SSH 操作超时"));
        }
    };
    stdout_task
        .await
        .map_err(|_| ProbeError::new("process_io_failed", "SSH 标准输出读取失败"))??;
    stderr_task
        .await
        .map_err(|_| ProbeError::new("process_io_failed", "SSH 标准错误读取失败"))??;
    Ok(status.code().unwrap_or(255))
}

async fn forward(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    stream: &'static str,
    output: mpsc::Sender<OutputChunk>,
) -> Result<(), ProbeError> {
    let mut buffer = vec![0_u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|_| ProbeError::new("process_io_failed", "SSH 输出读取失败"))?;
        if read == 0 {
            return Ok(());
        }
        if output
            .send(OutputChunk {
                stream,
                bytes: buffer[..read].to_vec(),
            })
            .await
            .is_err()
        {
            return Err(ProbeError::new(
                "process_io_failed",
                "SSH 输出消费者已经停止",
            ));
        }
    }
}
