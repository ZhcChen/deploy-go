use std::{fs, path::Path, process::Command as StdCommand};

use deploy_go_agent::git::{GitError, checkout_commit, list_remote_heads};

fn git(repo: &Path, args: &[&str]) -> String {
    let output = StdCommand::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git 命令执行失败");
    assert!(
        output.status.success(),
        "git {:?} 失败: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn init_repo(root: &Path) -> String {
    fs::create_dir_all(root).unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    git(root, &["config", "commit.gpgsign", "false"]);
    fs::write(root.join("README.md"), "hello\n").unwrap();
    git(root, &["add", "README.md"]);
    git(root, &["commit", "-q", "-m", "initial"]);
    git(root, &["rev-parse", "HEAD"])
}

#[tokio::test]
async fn list_remote_heads_returns_sorted_branch_heads() {
    let directory = tempfile::tempdir().unwrap();
    let repo = directory.path().join("repo");
    init_repo(&repo);
    git(&repo, &["checkout", "-q", "-b", "feature/x"]);
    fs::write(repo.join("feature.txt"), "feature\n").unwrap();
    git(&repo, &["add", "feature.txt"]);
    git(&repo, &["commit", "-q", "-m", "feature"]);
    git(&repo, &["checkout", "-q", "master"]);

    let heads = list_remote_heads(repo.to_str().unwrap(), None, 60)
        .await
        .unwrap();
    assert!(heads.iter().any(|head| head.name == "master"));
    assert!(heads.iter().any(|head| head.name == "feature/x"));
    assert!(heads.windows(2).all(|pair| pair[0].name < pair[1].name));
    assert!(heads.iter().all(|head| head.sha.len() == 40));
}

#[tokio::test]
async fn checkout_commit_clones_detached_head_and_verifies_clean_worktree() {
    let directory = tempfile::tempdir().unwrap();
    let repo = directory.path().join("repo");
    let sha = init_repo(&repo);
    let checkout_dir = directory.path().join("checkout");

    checkout_commit(repo.to_str().unwrap(), &sha, &checkout_dir, None, 60)
        .await
        .unwrap();
    assert_eq!(git(&checkout_dir, &["rev-parse", "HEAD"]), sha);
    assert_eq!(git(&checkout_dir, &["status", "--porcelain"]), "");
}

#[tokio::test]
async fn checkout_commit_rejects_invalid_and_unavailable_commits() {
    let directory = tempfile::tempdir().unwrap();
    let repo = directory.path().join("repo");
    init_repo(&repo);
    let checkout_dir = directory.path().join("checkout");

    assert!(matches!(
        checkout_commit(
            repo.to_str().unwrap(),
            "not-a-commit",
            &checkout_dir,
            None,
            60
        )
        .await,
        Err(GitError::InvalidCommit)
    ));
    assert!(matches!(
        checkout_commit(
            repo.to_str().unwrap(),
            "0000000000000000000000000000000000000000",
            &checkout_dir,
            None,
            60
        )
        .await,
        Err(GitError::CommitUnavailable)
    ));
}

#[tokio::test]
async fn dirty_worktree_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let repo = directory.path().join("repo");
    let sha = init_repo(&repo);
    let checkout_dir = directory.path().join("checkout");
    checkout_commit(repo.to_str().unwrap(), &sha, &checkout_dir, None, 60)
        .await
        .unwrap();
    fs::write(checkout_dir.join("untracked.txt"), "dirty\n").unwrap();
    assert!(matches!(
        checkout_commit(repo.to_str().unwrap(), &sha, &checkout_dir, None, 60).await,
        Err(GitError::DirtyWorktree)
    ));
}
