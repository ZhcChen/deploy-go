use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_deploy-go-deployer");
const SENTINEL_KEY: &str = "dgx_sentinel_help_output_must_not_leak_0123456789";

/// 运行 CLI 并返回 (stdout, stderr)。
fn run(args: &[&str]) -> (String, String) {
    let output = Command::new(BINARY)
        .args(args)
        .env("DEPLOY_GO_API_KEY", SENTINEL_KEY)
        .env_remove("DEPLOY_GO_API_BASE_URL")
        .output()
        .expect("CLI 启动失败");
    (
        String::from_utf8(output.stdout).expect("stdout 非 UTF-8"),
        String::from_utf8(output.stderr).expect("stderr 非 UTF-8"),
    )
}

#[test]
fn help_hides_api_key_environment_value() {
    let (stdout, stderr) = run(&["--help"]);
    assert!(
        stdout.contains("DEPLOY_GO_API_KEY"),
        "帮助文本应提示可用环境变量名"
    );
    assert!(
        !stdout.contains(SENTINEL_KEY),
        "帮助文本泄露了 DEPLOY_GO_API_KEY 的实际值"
    );
    assert!(
        !stdout.contains("dgx_sentinel"),
        "帮助文本泄露了 API Key 值前缀"
    );
    assert!(stderr.is_empty(), "帮助文本不应写入 stderr");
}

#[test]
fn missing_subcommand_help_also_hides_api_key() {
    let (stdout, stderr) = run(&[]);
    let rendered = format!("{stdout}{stderr}");
    assert!(
        !rendered.contains(SENTINEL_KEY),
        "缺少子命令时的帮助文本泄露了 DEPLOY_GO_API_KEY 的实际值"
    );
}

#[test]
fn help_still_exposes_api_base_default() {
    let (stdout, _) = run(&["--help"]);
    assert!(
        stdout.contains("https://deploy.quanxinfu.com"),
        "帮助文本应保留 API 基础地址默认值"
    );
}
