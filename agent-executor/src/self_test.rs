#[cfg(target_os = "linux")]
use std::{fs, os::unix::fs::PermissionsExt, process::Stdio, time::Duration};

#[cfg(target_os = "linux")]
pub fn run(root: &std::path::Path) -> anyhow::Result<String> {
    run_with_launcher(root, std::fs::read_link("/proc/self/exe")?)
}

#[cfg(target_os = "linux")]
#[doc(hidden)]
pub fn run_with_launcher(
    root: &std::path::Path,
    launcher: std::path::PathBuf,
) -> anyhow::Result<String> {
    fs::create_dir_all(root)?;
    let root_metadata = fs::symlink_metadata(root)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        anyhow::bail!("release self-test root is unsafe");
    }
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    let id = format!("release_SELFTEST_{}", ulid::Ulid::new());
    let directory = root.join(format!(".self-test-{}", ulid::Ulid::new()));
    fs::create_dir_all(&directory)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    let makefile = concat!(
        "deploy-go-release:\n",
        "\t@test \"$$(id -u)\" = 0\n",
        "\t@test \"$$DEPLOY_ID\" = self-test\n",
        "\t@test \"$$DEPLOY_ENVIRONMENT\" = test\n",
        "\t@test \"$$DEPLOY_MODULES\" = self-test\n",
        "\t@test -z \"$${DEPLOY_GO_AGENT_ACCESS_TOKEN:-}\"\n",
        "\t@printf '%s\\n' 'DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}'\n",
        "\t@printf '%s\\n' 'DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}'\n",
        "\t@printf '%s\\n' 'privileged-release-self-test uid=0'\n"
    );
    fs::write(directory.join("Makefile"), makefile)?;
    fs::set_permissions(
        directory.join("Makefile"),
        fs::Permissions::from_mode(0o400),
    )?;
    let cgroup = crate::cgroup::ReleaseCgroup::create_with_launcher(&id, launcher)?;
    let (launcher, arguments) = cgroup.launcher_command();
    let result = std::process::Command::new(launcher)
        .args(arguments)
        .current_dir(&directory)
        .env_clear()
        .env("PATH", crate::release::FIXED_PATH)
        .env("DEPLOY_ID", "self-test")
        .env("DEPLOY_ENVIRONMENT", "test")
        .env("DEPLOY_RELEASE_VERSION", "self-test")
        .env(
            "DEPLOY_COMMIT_SHA",
            "0000000000000000000000000000000000000000",
        )
        .env("DEPLOY_MODULES", "self-test")
        .env("DEPLOY_TARGET", "self-test")
        .env("DEPLOY_ARTIFACT_DIR", directory.join("artifacts"))
        .env("DEPLOY_ENV_DIR", directory.join("env"))
        .env("DEPLOY_CANCEL_FILE", directory.join("cancel"))
        .stdin(Stdio::null())
        .output();
    let _ = cgroup.kill_all();
    let cleanup = cgroup.wait_empty_and_remove(Duration::from_secs(2));
    let _ = fs::remove_dir_all(&directory);
    let result = result?;
    cleanup?;
    let mut output = String::from_utf8_lossy(&result.stdout).into_owned();
    output.push_str(&String::from_utf8_lossy(&result.stderr));
    if !result.status.success() {
        anyhow::bail!("privileged release self-test failed: {output}");
    }
    Ok(output)
}

#[cfg(not(target_os = "linux"))]
pub fn run(_: &std::path::Path) -> anyhow::Result<String> {
    anyhow::bail!("privileged release self-test requires Linux cgroup v2")
}
