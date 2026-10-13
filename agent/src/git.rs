use std::{fs, io, os::unix::fs::MetadataExt, path::Path, process::Stdio, time::Duration};

use deploy_go_agent_protocol::{SourceMaterialization, SourceMaterializationMode};
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
    #[error("Git sparse checkout 不可用")]
    SparseCheckoutUnavailable,
    #[error("Git 源码物化策略无效")]
    InvalidMaterialization,
    #[error("Git 文件操作失败: {0}")]
    Io(#[from] io::Error),
    #[error("Git checkout 目录清理失败: {0}")]
    CleanupFailed(String),
    /// checkout 目录或 `.git` 的属主不是当前执行身份（Git 的 `dubious ownership` 保护会拒绝它）。
    /// 不使用全局或通配的 `safe.directory` 绕过。
    #[error("检出目录属主不可信: {0}")]
    OwnershipUntrusted(String),
    /// Git 访问本地文件系统被拒绝（与远端认证失败区分）。
    #[error("Git 访问文件系统被拒绝: {0}")]
    FilesystemPermissionDenied(String),
}

impl GitError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Timeout => "git_timeout",
            Self::InvalidCommit => "git_invalid_commit",
            Self::CommitUnavailable => "git_commit_unavailable",
            Self::DirtyWorktree => "git_dirty_worktree",
            Self::InvalidRepository => "git_checkout_directory_invalid",
            Self::AuthenticationFailed => "git_authentication_failed",
            Self::RepositoryUnreachable => "git_repository_unreachable",
            Self::CommandFailed(_) => "git_command_failed",
            Self::Io(_) => "git_io_error",
            Self::CleanupFailed(_) => "git_checkout_cleanup_failed",
            Self::SparseCheckoutUnavailable => "git_sparse_checkout_unavailable",
            Self::InvalidMaterialization => "git_sparse_checkout_invalid",
            Self::OwnershipUntrusted(_) => "git_checkout_ownership_untrusted",
            Self::FilesystemPermissionDenied(_) => "git_checkout_permission_denied",
        }
    }
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
    checkout_commit_with_materialization(
        repository_url,
        commit_sha,
        checkout_dir,
        credential_file,
        timeout_seconds,
        None,
    )
    .await
}

pub async fn checkout_commit_with_materialization(
    repository_url: &str,
    commit_sha: &str,
    checkout_dir: &Path,
    credential_file: Option<&Path>,
    timeout_seconds: u32,
    materialization: Option<&SourceMaterialization>,
) -> Result<(), GitError> {
    if !valid_sha(commit_sha) {
        return Err(GitError::InvalidCommit);
    }
    validate_materialization(materialization)?;
    if checkout_dir.exists() {
        let metadata = fs::symlink_metadata(checkout_dir)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(GitError::InvalidRepository);
        }
        if !checkout_dir.join(".git").exists() {
            fs::remove_dir_all(checkout_dir)
                .map_err(|error| GitError::CleanupFailed(error.to_string()))?;
        }
    }
    if checkout_dir.exists() {
        ensure_checkout_ownership(checkout_dir)?;
        let mut command = git_dir_command(checkout_dir, credential_file);
        command.args(["fetch", "--prune", "origin"]);
        run_command(command, timeout_seconds).await?;
    } else {
        if let Some(parent) = checkout_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        clone_repository(
            repository_url,
            checkout_dir,
            credential_file,
            timeout_seconds,
            materialization,
        )
        .await?;
    }
    if let Some(policy) = materialization
        && matches!(policy.mode, SourceMaterializationMode::Sparse)
    {
        let mut command = git_dir_command(checkout_dir, credential_file);
        command.args(["sparse-checkout", "set", "--no-cone", "--"]);
        // 声明路径都是相对仓库根的（校验已拒绝绝对路径）。non-cone 模式下不含斜杠的名字
        // （如 `api`、`Makefile`）不锚定，会匹配任意层级的同名目录或文件，物化范围会超出声明；
        // 统一加前导 `/` 锚定到仓库根。对文件、目录和 `dir/**` 都成立。
        command.args(policy.paths.iter().map(|path| format!("/{path}")));
        run_command(command, timeout_seconds)
            .await
            .map_err(|error| match error {
                GitError::CommandFailed(_) => GitError::SparseCheckoutUnavailable,
                other => other,
            })?;
    } else if checkout_dir.join(".git/info/sparse-checkout").exists() {
        let mut command = git_dir_command(checkout_dir, credential_file);
        command.args(["sparse-checkout", "disable"]);
        let _ = run_command(command, timeout_seconds).await;
    }
    let mut command = git_dir_command(checkout_dir, credential_file);
    command.args(["checkout", "--detach", commit_sha]);
    run_command(command, timeout_seconds)
        .await
        .map_err(|error| match error {
            GitError::CommandFailed(_) => GitError::CommitUnavailable,
            other => other,
        })?;
    if materialization
        .is_some_and(|policy| matches!(policy.mode, SourceMaterializationMode::Sparse))
    {
        validate_sparse_symlinks(checkout_dir)?;
    }
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

/// 为 runner 的首次物化清理受管 checkout 残留。
///
/// checkout 必须由实际执行 Git 和业务脚本的 runner 用户创建；Agent 只
/// 清理旧版本或失败任务留下的目录，不在这里创建或改变新 checkout 的 owner。
pub fn reset_checkout_for_runner(checkout_dir: &Path) -> Result<(), GitError> {
    let metadata = match fs::symlink_metadata(checkout_dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(GitError::Io(error)),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(GitError::InvalidRepository);
    }
    fs::remove_dir_all(checkout_dir).map_err(|error| GitError::CleanupFailed(error.to_string()))
}

/// 首次克隆。仅显式 sparse 策略使用 blob 过滤的部分克隆（只下载提交与树对象，工作区内容在
/// `checkout` 时按需获取），且 sparse 下过滤不可用时不会静默回退为全量克隆；
/// full 模式保持历史行为（不带过滤的完整克隆，提交 8157194 恢复）。
async fn clone_repository(
    repository_url: &str,
    checkout_dir: &Path,
    credential_file: Option<&Path>,
    timeout_seconds: u32,
    materialization: Option<&SourceMaterialization>,
) -> Result<(), GitError> {
    let result = run_clone(
        repository_url,
        checkout_dir,
        credential_file,
        timeout_seconds,
        materialization
            .is_some_and(|policy| matches!(policy.mode, SourceMaterializationMode::Sparse)),
    )
    .await;
    if result.is_err()
        && checkout_dir.exists()
        && let Err(error) = fs::remove_dir_all(checkout_dir)
    {
        return Err(GitError::CleanupFailed(error.to_string()));
    }
    if materialization
        .is_some_and(|policy| matches!(policy.mode, SourceMaterializationMode::Sparse))
    {
        result.map_err(|error| match error {
            GitError::CommandFailed(_) => GitError::SparseCheckoutUnavailable,
            other => other,
        })
    } else {
        result
    }
}

fn validate_materialization(
    materialization: Option<&SourceMaterialization>,
) -> Result<(), GitError> {
    let Some(policy) = materialization else {
        return Ok(());
    };
    if policy.paths.len() > 128 || policy.paths.iter().map(String::len).sum::<usize>() > 16 * 1024 {
        return Err(GitError::InvalidMaterialization);
    }
    let mut previous = None;
    for path in &policy.paths {
        if path.is_empty()
            || path.len() > 4096
            || path.starts_with('/')
            || path.contains('\\')
            || path.chars().any(char::is_control)
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(GitError::InvalidMaterialization);
        }
        let base = path.strip_suffix("/**").unwrap_or(path);
        if base.is_empty()
            || base.contains('*')
            || base.contains('?')
            || base.contains('[')
            || base.contains(']')
        {
            return Err(GitError::InvalidMaterialization);
        }
        if previous.is_some_and(|value| value >= path.as_str()) {
            return Err(GitError::InvalidMaterialization);
        }
        previous = Some(path.as_str());
    }
    if matches!(policy.mode, SourceMaterializationMode::Sparse) && policy.paths.is_empty() {
        return Err(GitError::InvalidMaterialization);
    }
    Ok(())
}

fn validate_sparse_symlinks(checkout_dir: &Path) -> Result<(), GitError> {
    let root = fs::canonicalize(checkout_dir)?;
    validate_sparse_symlinks_inner(checkout_dir, &root)
}

fn validate_sparse_symlinks_inner(path: &Path, root: &Path) -> Result<(), GitError> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)?;
        if metadata.file_type().is_symlink() {
            let target =
                fs::canonicalize(&entry_path).map_err(|_| GitError::InvalidMaterialization)?;
            if !target.starts_with(root) {
                return Err(GitError::InvalidMaterialization);
            }
        } else if metadata.is_dir() {
            validate_sparse_symlinks_inner(&entry_path, root)?;
        }
    }
    Ok(())
}

async fn run_clone(
    repository_url: &str,
    checkout_dir: &Path,
    credential_file: Option<&Path>,
    timeout_seconds: u32,
    filtered: bool,
) -> Result<(), GitError> {
    let mut command = git_command(credential_file);
    command.arg("clone").arg("--no-checkout");
    if filtered {
        command.arg("--filter=blob:none");
    }
    command.arg(repository_url).arg(checkout_dir);
    run_command(command, timeout_seconds).await.map(|_| ())
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
            sanitize_git_output(stdout.trim()),
            sanitize_git_output(stderr.trim())
        );
        return Err(classify_failure(&stderr, detail));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn classify_failure(stderr: &str, detail: String) -> GitError {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("dubious ownership") {
        GitError::OwnershipUntrusted(detail)
    } else if is_remote_authentication_failure(&lower) {
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
    } else if lower.contains("permission denied") {
        // 走到这里的 permission denied 没有 SSH 认证的特征，是本地文件系统权限问题
        // （如 `Unable to create '.git/index.lock': Permission denied`），不能报成“认证失败”。
        GitError::FilesystemPermissionDenied(detail)
    } else {
        GitError::CommandFailed(detail)
    }
}

/// 远端认证失败的特征。SSH 的 `Permission denied (publickey)` 属于这一类，
/// 单独的 `permission denied` 则不是（见 `classify_failure`）。
fn is_remote_authentication_failure(lower: &str) -> bool {
    lower.contains("publickey")
        || lower.contains("authentication failed")
        || lower.contains("could not read from remote repository")
        || lower.contains("host key verification failed")
        || lower.contains("permission denied, please try again")
        || lower.contains("permission denied (")
}

/// 把 Git 的原始输出整理成可以写进诊断的单行文本：去掉 URL 里的用户信息（`scheme://user:secret@host`），
/// 转义控制字符，限制长度。凭证本来就不应出现在 URL 里（凭证只经文件传递），这里是兜底。
fn sanitize_git_output(text: &str) -> String {
    const MAX_CHARS: usize = 1500;
    let mut redacted = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find("://") {
        let (head, tail) = rest.split_at(index + 3);
        redacted.push_str(head);
        let end = tail
            .find(|character: char| {
                character.is_whitespace() || matches!(character, '/' | '\'' | '"')
            })
            .unwrap_or(tail.len());
        match tail[..end].rfind('@') {
            Some(at) => {
                redacted.push_str("***@");
                rest = &tail[at + 1..];
            }
            None => rest = tail,
        }
    }
    redacted.push_str(rest);

    let mut output = String::new();
    for (count, character) in redacted.chars().enumerate() {
        if count >= MAX_CHARS {
            output.push('…');
            break;
        }
        match character {
            '\n' => output.push_str(" | "),
            character if character.is_control() => {
                output.push_str(&format!("\\u{{{:x}}}", character as u32));
            }
            character => output.push(character),
        }
    }
    output
}

/// 失败时附带的执行身份与 checkout 目录元数据（只有数字形式的 uid/gid/mode，没有路径和内容），
/// 用来区分“属主不对”“权限不足”“目录不存在”这几类现场问题。
pub fn checkout_diagnostics(checkout_dir: &Path) -> String {
    fn describe(path: &Path) -> String {
        match fs::symlink_metadata(path) {
            Ok(metadata) => format!(
                "uid={} gid={} mode={:04o}",
                metadata.uid(),
                metadata.gid(),
                metadata.mode() & 0o7777
            ),
            Err(error) => format!("不可读({:?})", error.kind()),
        }
    }
    let (euid, egid) = unsafe { (libc::geteuid(), libc::getegid()) };
    format!(
        "执行身份 euid={euid} egid={egid}; checkout[{}]; .git[{}]; 父目录[{}]",
        describe(checkout_dir),
        describe(&checkout_dir.join(".git")),
        checkout_dir
            .parent()
            .map_or_else(|| "无".to_owned(), describe)
    )
}

/// 已存在的仓库，其工作树目录与 `.git` 的属主必须是当前执行身份；否则 Git 会以 `dubious ownership` 拒绝，
/// 与其等 Git 报一个泛化错误，不如直接给出独立错误码和诊断。
fn ensure_checkout_ownership(checkout_dir: &Path) -> Result<(), GitError> {
    let euid = unsafe { libc::geteuid() };
    for path in [checkout_dir.to_path_buf(), checkout_dir.join(".git")] {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.uid() != euid {
            return Err(GitError::OwnershipUntrusted(checkout_diagnostics(
                checkout_dir,
            )));
        }
    }
    Ok(())
}

fn valid_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{
        checkout_diagnostics, classify_failure, ensure_checkout_ownership,
        reset_checkout_for_runner, sanitize_git_output,
    };
    use crate::git::GitError;
    use std::fs;

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

    #[test]
    fn exposes_stable_error_codes_without_error_details() {
        assert_eq!(
            GitError::AuthenticationFailed.error_code(),
            "git_authentication_failed"
        );
        assert_eq!(
            GitError::RepositoryUnreachable.error_code(),
            "git_repository_unreachable"
        );
        assert_eq!(GitError::Timeout.error_code(), "git_timeout");
        assert_eq!(
            GitError::InvalidRepository.error_code(),
            "git_checkout_directory_invalid"
        );
        assert_eq!(
            GitError::CleanupFailed("secret path".to_owned()).error_code(),
            "git_checkout_cleanup_failed"
        );
    }

    #[test]
    fn reset_checkout_for_runner_removes_managed_residue() {
        let directory = tempfile::tempdir().unwrap();
        let checkout = directory.path().join("checkout");
        fs::create_dir_all(checkout.join(".git")).unwrap();
        fs::write(checkout.join("stale"), b"stale").unwrap();

        reset_checkout_for_runner(&checkout).unwrap();

        assert!(!checkout.exists());
    }

    #[test]
    fn reset_checkout_for_runner_rejects_symlinks_and_files() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("checkout");
        fs::write(&file, b"not a directory").unwrap();
        assert!(matches!(
            reset_checkout_for_runner(&file),
            Err(super::GitError::InvalidRepository)
        ));

        #[cfg(unix)]
        {
            let target = directory.path().join("target");
            fs::create_dir(&target).unwrap();
            let link = directory.path().join("link");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert!(matches!(
                reset_checkout_for_runner(&link),
                Err(super::GitError::InvalidRepository)
            ));
        }
    }

    #[test]
    fn dubious_ownership_is_its_own_error_not_a_generic_command_failure() {
        let stderr = "fatal: detected dubious ownership in repository at '/srv/work/checkout'\n\
                      To add an exception for this directory, call:\n\
                      \tgit config --global --add safe.directory /srv/work/checkout";
        let error = classify_failure(stderr, "detail".to_owned());
        assert!(
            matches!(error, GitError::OwnershipUntrusted(_)),
            "{error:?}"
        );
        assert_eq!(error.error_code(), "git_checkout_ownership_untrusted");
    }

    #[test]
    fn local_permission_denied_is_not_reported_as_authentication_failure() {
        // 这些是文件系统权限问题，此前会被误报成 git_authentication_failed，把排查引向错误方向。
        for stderr in [
            "fatal: Unable to create '/srv/work/checkout/.git/index.lock': Permission denied",
            "error: cannot open '.git/FETCH_HEAD': Permission denied",
            "fatal: could not create work tree dir 'checkout': Permission denied",
        ] {
            let error = classify_failure(stderr, "detail".to_owned());
            assert!(
                matches!(error, GitError::FilesystemPermissionDenied(_)),
                "{stderr} => {error:?}"
            );
            assert_eq!(error.error_code(), "git_checkout_permission_denied");
        }
        // SSH 认证失败仍然是认证失败。
        for stderr in [
            "git@host: Permission denied (publickey,password).",
            "Permission denied, please try again.",
            "fatal: Authentication failed for 'https://host/repo.git'",
        ] {
            assert!(
                matches!(
                    classify_failure(stderr, "detail".to_owned()),
                    GitError::AuthenticationFailed
                ),
                "{stderr}"
            );
        }
    }

    #[test]
    fn sanitize_git_output_redacts_url_userinfo_and_escapes_control_characters() {
        let cleaned = sanitize_git_output(
            "fatal: unable to access 'https://deploy-user:s3cr3t-token@git.example.test/org/repo.git/': 403\nremote: see ssh://git:key@host:22/x",
        );
        assert!(!cleaned.contains("s3cr3t-token"), "{cleaned}");
        assert!(!cleaned.contains("deploy-user"), "{cleaned}");
        assert!(!cleaned.contains("git:key"), "{cleaned}");
        assert!(
            cleaned.contains("https://***@git.example.test/org/repo.git/"),
            "{cleaned}"
        );
        assert!(cleaned.contains("ssh://***@host:22/x"), "{cleaned}");
        assert!(!cleaned.contains('\n'), "多行输出必须折叠成单行：{cleaned}");

        // 没有用户信息的 URL 原样保留；控制字符被转义；长度受限。
        assert_eq!(
            sanitize_git_output("fetching https://git.example.test/org/repo.git"),
            "fetching https://git.example.test/org/repo.git"
        );
        assert!(sanitize_git_output("a\u{1b}[2Jb").contains("\\u{1b}"));
        assert!(sanitize_git_output(&"x".repeat(5000)).chars().count() <= 1501);
    }

    #[test]
    fn diagnostics_contain_only_numeric_identity_and_mode_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let checkout = directory.path().join("checkout");
        fs::create_dir_all(checkout.join(".git")).unwrap();
        let text = checkout_diagnostics(&checkout);
        assert!(text.contains("euid=") && text.contains("egid="), "{text}");
        assert!(
            text.contains("checkout[uid=") && text.contains("mode="),
            "{text}"
        );
        assert!(text.contains(".git[uid="), "{text}");
        assert!(
            !text.contains(directory.path().to_string_lossy().as_ref()),
            "诊断不得包含节点上的绝对路径：{text}"
        );
        // 不存在的目录给出“不可读”，而不是 panic 或泄露路径。
        assert!(checkout_diagnostics(&directory.path().join("missing/x")).contains("不可读"));
    }

    #[test]
    fn existing_repository_owned_by_the_current_identity_passes_the_ownership_check() {
        let directory = tempfile::tempdir().unwrap();
        let checkout = directory.path().join("checkout");
        fs::create_dir_all(checkout.join(".git")).unwrap();
        assert!(ensure_checkout_ownership(&checkout).is_ok());
        // 缺少 .git 时是 Io 错误（调用方此前已保证存在），而不是误判为属主问题。
        let missing = directory.path().join("no-git");
        fs::create_dir_all(&missing).unwrap();
        assert!(matches!(
            ensure_checkout_ownership(&missing),
            Err(GitError::Io(_))
        ));
    }
}
