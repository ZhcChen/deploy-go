use std::{fs, path::PathBuf};

#[test]
fn skill_has_valid_trigger_metadata_and_core_boundaries() {
    let skill = read("SKILL.md");
    assert!(skill.starts_with("---\nname: deploy-go-deployer\ndescription:"));
    assert!(skill.contains("编辑非正式环境应用"));
    assert!(skill.contains("create-app"));
    assert!(skill.contains("正式环境（`prod`）会被服务端拒绝"));
    assert!(skill.contains("references/commands.md"));
}

#[test]
fn skill_uses_self_contained_cli_and_one_level_references() {
    let skill = read("SKILL.md");
    for reference in ["commands.md", "workflows.md", "errors.md"] {
        assert!(skill.contains(&format!("references/{reference}")));
        assert!(skill_root().join("references").join(reference).is_file());
    }
    assert!(skill.contains("<skill-dir>/scripts/deploy-go-deployer"));
    assert!(skill.contains("<skill-dir>\\scripts\\deploy-go-deployer.exe"));

    let forbidden = [
        "docs/".to_string(),
        "tools/".to_string(),
        "/Users/".to_string(),
        "mcp/".to_string(),
    ];
    for forbidden in forbidden {
        assert!(
            !package_text().contains(&forbidden),
            "Skill 包包含仓库耦合内容：{forbidden}"
        );
    }
}

#[test]
fn command_reference_covers_supported_surface_and_update_boundary() {
    let commands = read("references/commands.md");
    for command in [
        "list-apps",
        "create-app",
        "show-app",
        "update-app",
        "list-env-files",
        "inspect-env-file",
        "register-env-file",
        "update-env-file",
        "delete-env-file",
        "list-targets",
        "create-target",
        "update-target",
        "set-target-status",
        "show-workspace-source",
        "set-workspace-source",
        "list-build-agents",
        "list-git-credentials",
        "show-git-source",
        "set-git-source",
        "refresh-git-branches",
        "show-git-refresh",
        "set-git-branch",
        "set-app-status",
        "retry-env-sync",
        "list-config-files",
        "preview-deploy",
        "probe-runtime",
        "show-runtime-status",
        "deploy",
        "status",
        "cancel",
        "openapi",
    ] {
        assert!(commands.contains(command), "缺少命令：{command}");
    }
    for option in [
        "--version",
        "--parameter-schema-file",
        "--verification-config-file",
        "--clear-tags",
        "--idempotency-key",
        "--content-file",
        "--confirm-file-name",
        "--node-id",
        "--script-path",
        "--timeout-seconds",
        "--secret-file",
        "--build-agent-id",
        "--workspace-path",
        "--environment",
        "--repository-url",
        "--git-credential-id",
        "--materialization",
        "--sparse-path",
        "--branch",
        "--wait",
        "--snapshot-hash",
    ] {
        assert!(commands.contains(option), "缺少参数：{option}");
    }
}

#[test]
fn workflows_and_errors_cover_write_boundaries() {
    let workflows = read("references/workflows.md");
    assert!(workflows.contains("正式环境"));
    assert!(workflows.contains("非正式环境"));
    assert!(workflows.contains("不自动重试"));

    let errors = read("references/errors.md");
    for code in [
        "external_production_deployment_forbidden",
        "external_production_application_forbidden",
        "external_production_environment_forbidden",
        "resource_version_conflict",
        "external_node_not_allowed",
        "git_ref_discovery_expired",
        "git_branch_not_found",
        "preview_expired",
        "external_application_reactivation_forbidden",
    ] {
        assert!(errors.contains(code), "缺少错误码：{code}");
    }
}

#[test]
fn installer_tracks_stable_release_by_default() {
    let installer = read("scripts/install.sh");
    assert!(installer.contains("DEPLOY_GO_DEPLOYER_VERSION:-stable"));
    assert!(!installer.contains("DEPLOY_GO_DEPLOYER_VERSION:-0.3.4"));
}

fn package_text() -> String {
    [
        "SKILL.md",
        "references/commands.md",
        "references/workflows.md",
        "references/errors.md",
        "scripts/install.sh",
    ]
    .map(read)
    .join("\n")
}

fn read(relative: &str) -> String {
    fs::read_to_string(skill_root().join(relative))
        .unwrap_or_else(|error| panic!("读取 {relative} 失败：{error}"))
}

fn skill_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skills/deploy-go-deployer")
}
