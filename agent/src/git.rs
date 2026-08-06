use std::{fs, io, path::Path, process::Stdio, time::Duration};

use thiserror::Error;
use tokio::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteHead {
    pub name: String,
    pub sha: String,
}

#[derive(Debug, Error)]
pub enum GitError {
    #[error("Git 命令执行失败: {0}")]
    CommandFailed(String),
    #[error("Git 认证失败")]
    AuthenticationFailed,
    #[error("Git 仓库不可达")]
    RepositoryUnreachable,
    #[error("Git 命令超时")]
    Timeout,
    #[error("commit SHA 无效")]
    InvalidCommit,
    #[error("目标 commit 不可用")]
    CommitUnavailable,
    #[error("检出目录不是干净的工作区")]
    DirtyWorktree,
    #[error("检出目录不是 Git 仓库")]
    InvalidRepository,
    #[error("Git 文件操作失败: {0}")]
    Io(#[from] io::Error),
}

pub async fn list_remote_heads(
    repository_url: &str,
    credential_file: Option<&Path>,
    timeout_seconds: u32,
) -> Result<Vec<RemoteHead>, GitError> {
    let mut command = git_command(credential_file);
    command
        .args(["ls-remote", "--heads"])
        .arg(repository_url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = run_command(command, timeout_seconds).await?;
    let mut heads = Vec::new();
    for line in output.lines() {
        let Some((sha, ref_name)) = line.split_once('\t') else {
            continue;
        };
        let Some(name) = ref_name.strip_prefix("refs/heads/") else {
            continue;
        };
        if !valid_sha(sha) || name.is_empty() || name.contains("..") || name.contains('@') {
            continue;
        }
        heads.push(RemoteHead {
            name: name.to_owned(),
            sha: sha.to_owned(),
        });
    }
    heads.sort_by(|left, right| left.name.cmp(&right.name));
    heads.dedup_by(|left, right| left.name == right.name);
    Ok(heads)
}

pub async fn checkout_commit(
    repository_url: &str,
    commit_sha: &str,
    checkout_dir: &Path,
    credential_file: Option<&Path>,
    timeout_seconds: u32,
) -> Result<(), GitError> {
    if !valid_sha(commit_sha) {
        return Err(GitError::InvalidCommit);
    }
    if checkout_dir.exists() {
        let mut command = git_dir_command(checkout_dir, credential_file);
        command.args(["fetch", "--prune", "origin"]);
        run_command(command, timeout_seconds)
            .await
            .map_err(|error| match error {
                GitError::CommandFailed(_) | GitError::Timeout => GitError::InvalidRepository,
                other => other,
            })?;
    } else {
        if let Some(parent) = checkout_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut command = git_command(credential_file);
        command
            .arg("clone")
            .arg("--no-checkout")
            .arg(repository_url)
            .arg(checkout_dir);
        let result = run_command(command, timeout_seconds).await;
        if result.is_err() {
            let _ = fs::remove_dir_all(checkout_dir);
            return Err(GitError::InvalidRepository);
        }
    }
    let mut command = git_dir_command(checkout_dir, credential_file);
    command.args(["checkout", "--detach", commit_sha]);
    run_command(command, timeout_seconds)
        .await
        .map_err(|error| match error {
            GitError::CommandFailed(_) | GitError::Timeout => GitError::CommitUnavailable,
            other => other,
        })?;
    let mut command = git_dir_command(checkout_dir, credential_file);
    command.args(["rev-parse", "HEAD"]);
    let head = run_command(command, timeout_seconds).await?;
    if !head.trim().eq_ignore_ascii_case(commit_sha) {
        return Err(GitError::CommitUnavailable);
    }
    let mut command = git_dir_command(checkout_dir, credential_file);
    command.args(["status", "--porcelain"]);
    let status = run_command(command, timeout_seconds).await?;
    if !status.trim().is_empty() {
        return Err(GitError::DirtyWorktree);
    }
    Ok(())
}

fn git_command(credential_file: Option<&Path>) -> Command {
    let mut command = Command::new("git");
    if let Some(key) = credential_file {
        command.arg("-c").arg(format!(
            "core.sshCommand=ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
            key.display()
        ));
    }
    command
}

fn git_dir_command(checkout_dir: &Path, credential_file: Option<&Path>) -> Command {
    let mut command = git_command(credential_file);
    command.arg("-C").arg(checkout_dir);
    command
}

async fn run_command(mut command: Command, timeout_seconds: u32) -> Result<String, GitError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = tokio::time::timeout(
        Duration::from_secs(u64::from(timeout_seconds)),
        command.output(),
    )
    .await
    .map_err(|_| GitError::Timeout)?
    .map_err(GitError::Io)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = format!(
            "exit={} stdout={} stderr={}",
            output
                .status
                .code()
                .map_or_else(|| "signal".to_owned(), |code| code.to_string()),
            stdout.trim(),
            stderr.trim()
        );
        return Err(classify_failure(&stderr, detail));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn classify_failure(stderr: &str, detail: String) -> GitError {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("permission denied")
        || lower.contains("publickey")
        || lower.contains("authentication failed")
        || lower.contains("could not read from remote repository")
        || lower.contains("host key verification failed")
    {
        GitError::AuthenticationFailed
    } else if lower.contains("could not resolve host")
        || lower.contains("could not resolve hostname")
        || lower.contains("connection refused")
        || lower.contains("connection timed out")
        || lower.contains("connection reset")
        || lower.contains("network is unreachable")
        || lower.contains("no route to host")
        || lower.contains("name or service not known")
    {
        GitError::RepositoryUnreachable
    } else {
        GitError::CommandFailed(detail)
    }
}

fn valid_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::classify_failure;
    use crate::git::GitError;

    #[test]
    fn classifies_authentication_and_reachability_failures() {
        assert!(matches!(
            classify_failure(
                "git@git.example.test: Permission denied (publickey).",
                "detail".to_owned()
            ),
            GitError::AuthenticationFailed
        ));
        assert!(matches!(
            classify_failure(
                "fatal: Could not read from remote repository.",
                "detail".to_owned()
            ),
            GitError::AuthenticationFailed
        ));
        assert!(matches!(
            classify_failure(
                "ssh: connect to host git.example.test port 22: Connection refused",
                "detail".to_owned()
            ),
            GitError::RepositoryUnreachable
        ));
        assert!(matches!(
            classify_failure(
                "ssh: Could not resolve hostname git.example.test",
                "detail".to_owned()
            ),
            GitError::RepositoryUnreachable
        ));
        assert!(matches!(
            classify_failure("fatal: repository not found", "detail".to_owned()),
            GitError::CommandFailed(_)
        ));
    }
}
