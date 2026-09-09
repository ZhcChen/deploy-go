use std::{fs, path::PathBuf};

#[test]
fn skill_has_valid_trigger_metadata_and_core_boundaries() {
    let skill = read("SKILL.md");
    assert!(skill.starts_with("---\nname: deploy-go-deployer\ndescription:"));
    assert!(skill.contains("编辑非正式环境应用"));
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
        "show-app",
        "update-app",
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
    ] {
        assert!(errors.contains(code), "缺少错误码：{code}");
    }
}

#[test]
fn openai_metadata_is_complete_without_tool_dependencies() {
    let metadata = read("agents/openai.yaml");
    assert!(metadata.contains("display_name:"));
    assert!(metadata.contains("short_description:"));
    assert!(metadata.contains("default_prompt:"));
    assert!(metadata.contains("$deploy-go-deployer"));
    assert!(!metadata.contains("dependencies:"));
}

fn package_text() -> String {
    [
        "SKILL.md",
        "agents/openai.yaml",
        "references/commands.md",
        "references/workflows.md",
        "references/errors.md",
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
