use std::{collections::HashMap, path::Path, process::Stdio, time::Duration};

use async_trait::async_trait;

use crate::{
    config::Config,
    credential_store::{CredentialError, CredentialStore},
    executor_client::{DEFAULT_EXECUTOR_SOCKET_PATH, ExecutorClient},
    runner_service::{DEFAULT_RUNNER_SOCKET_PATH, RunnerServiceClient},
};
use deploy_go_agent_executor::protocol::ExecutorCapability;
use deploy_go_agent_protocol::PROTOCOL_VERSION;

const INSTALLED_CONFIG_PATH: &str = "/etc/deploy-go-agent/config";
const DIAGNOSTIC_FAILURE_EXIT: i32 = 2;
const AGENT_SERVICE: &str = "deploy-go-agent.service";
const RUNNER_SERVICE: &str = "deploy-go-agent-runner.service";
const EXECUTOR_SERVICE: &str = "deploy-go-agent-executor.service";

const KNOWN_CONFIG_KEYS: &[&str] = &[
    "DEPLOY_GO_AGENT_CONTROL_URL",
    "DEPLOY_GO_AGENT_DATA_DIR",
    "DEPLOY_GO_AGENT_HEARTBEAT_SECONDS",
    "DEPLOY_GO_AGENT_STAGING_SIZE_LIMIT_BYTES",
    "DEPLOY_GO_AGENT_STAGING_MAX_FILES",
    "DEPLOY_GO_AGENT_ARTIFACT_TRANSFER_ENABLED",
    "DEPLOY_GO_AGENT_ENV_SYNC_ENABLED",
    "DEPLOY_GO_RUNNER_SOCKET",
    "DEPLOY_GO_RUNNER_TASK_ROOT",
    "DEPLOY_GO_RUNNER_ALLOWED_UID",
    "DEPLOY_GO_RUNNER_ALLOWED_GID",
    "DEPLOY_GO_RUNNER_UID",
    "DEPLOY_GO_RUNNER_GID",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Status,
    Doctor,
}

impl Command {
    pub fn from_args() -> Option<Self> {
        Self::from_arg(std::env::args().nth(1).as_deref())
    }

    fn from_arg(value: Option<&str>) -> Option<Self> {
        match value {
            Some("status") => Some(Self::Status),
            Some("doctor") => Some(Self::Doctor),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Pass,
    Warn,
    Fail,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Check {
    level: Level,
    id: &'static str,
    message: String,
}

impl Check {
    fn pass(id: &'static str, message: impl Into<String>) -> Self {
        Self {
            level: Level::Pass,
            id,
            message: message.into(),
        }
    }

    fn warn(id: &'static str, message: impl Into<String>) -> Self {
        Self {
            level: Level::Warn,
            id,
            message: message.into(),
        }
    }

    fn fail(id: &'static str, message: impl Into<String>) -> Self {
        Self {
            level: Level::Fail,
            id,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ServiceState {
    Active,
    Inactive,
    Unavailable,
}

#[async_trait]
trait Probes: Send + Sync {
    async fn service_state(&self, unit: &str) -> ServiceState;
    async fn https_ready(&self, config: &Config) -> bool;
    async fn runner_ready(&self) -> bool;
    async fn executor_capabilities(&self) -> Option<Vec<ExecutorCapability>>;
}

struct SystemProbes;

#[async_trait]
impl Probes for SystemProbes {
    async fn service_state(&self, unit: &str) -> ServiceState {
        let status = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new("systemctl")
                .args(["is-active", "--quiet", unit])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status(),
        )
        .await;
        match status {
            Ok(Ok(status)) if status.success() => ServiceState::Active,
            Ok(Ok(status)) if matches!(status.code(), Some(3 | 4)) => ServiceState::Inactive,
            Ok(Ok(_)) | Ok(Err(_)) | Err(_) => ServiceState::Unavailable,
        }
    }

    async fn https_ready(&self, config: &Config) -> bool {
        let mut url = config.control_url.clone();
        if url.set_scheme("https").is_err() {
            return false;
        }
        url.set_path("/readyz");
        url.set_query(None);
        url.set_fragment(None);
        anonymous_ready(url).await
    }

    async fn runner_ready(&self) -> bool {
        RunnerServiceClient::new(DEFAULT_RUNNER_SOCKET_PATH.into())
            .probe()
            .await
    }

    async fn executor_capabilities(&self) -> Option<Vec<ExecutorCapability>> {
        ExecutorClient::new(DEFAULT_EXECUTOR_SOCKET_PATH.into())
            .probe_capabilities()
            .await
    }
}

async fn anonymous_ready(url: url::Url) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(8))
        .build()
    else {
        return false;
    };
    matches!(client.get(url).send().await, Ok(response) if response.status().is_success())
}

pub async fn run(command: Command) -> i32 {
    let overrides = environment_overrides();
    let checks = collect(
        command,
        Path::new(INSTALLED_CONFIG_PATH),
        &overrides,
        &SystemProbes,
    )
    .await;
    print!("{}", render(command, &checks));
    exit_code(command, &checks)
}

fn render(command: Command, checks: &[Check]) -> String {
    let mut output = checks
        .iter()
        .map(|check| format!("{} {} {}\n", check.level.label(), check.id, check.message))
        .collect::<String>();
    if command == Command::Doctor {
        output.push_str("INFO NEXT_STEP systemctl status deploy-go-agent deploy-go-agent-runner deploy-go-agent-executor\n");
        output.push_str("INFO NEXT_STEP journalctl -u deploy-go-agent -u deploy-go-agent-runner -u deploy-go-agent-executor --since '30 minutes ago' --no-pager\n");
    }
    output
}

fn exit_code(command: Command, checks: &[Check]) -> i32 {
    if command == Command::Doctor && checks.iter().any(|check| check.level == Level::Fail) {
        DIAGNOSTIC_FAILURE_EXIT
    } else {
        0
    }
}

async fn collect(
    command: Command,
    config_path: &Path,
    overrides: &HashMap<String, String>,
    probes: &dyn Probes,
) -> Vec<Check> {
    let mut checks = vec![Check::pass(
        "VERSION",
        format!(
            "Agent v{}，协议版本 v{}",
            env!("CARGO_PKG_VERSION"),
            PROTOCOL_VERSION
        ),
    )];
    let config = match load_config(config_path, overrides) {
        Ok(config) => {
            checks.push(Check::pass("CONFIG", "安装配置有效"));
            Some(config)
        }
        Err(_) => {
            checks.push(Check::fail("CONFIG", "安装配置缺失或无效"));
            None
        }
    };

    if let Some(config) = &config {
        checks.push(Check::pass("CONTROL_URL", "控制地址格式有效"));
        match CredentialStore::new(config.credential_file.clone()).load() {
            Ok(credentials) => {
                let identity_matches =
                    credential_owner_matches_current_user(&config.credential_file);
                checks.push(Check::pass("CREDENTIALS", "Agent 凭证有效且权限正确"));
                checks.push(if identity_matches {
                    Check::pass("IDENTITY", "当前用户 UID 与 Agent 凭证所有者一致")
                } else {
                    Check::fail("IDENTITY", "请使用 deploy-go-agent 服务用户运行诊断")
                });
                checks.push(Check {
                    level: Level::Pass,
                    id: "AGENT_ID",
                    message: format!("Agent ID {}", credentials.agent_id),
                });
            }
            Err(error) => {
                checks.push(credential_check(error));
                checks.push(Check::warn("IDENTITY", "凭证不可用，未检查运行身份"));
                checks.push(Check::warn("AGENT_ID", "凭证不可用，未读取 Agent ID"));
            }
        }
    } else {
        checks.push(Check::warn("CONTROL_URL", "配置无效，未检查控制地址"));
        checks.push(Check::warn("CREDENTIALS", "配置无效，未检查 Agent 凭证"));
        checks.push(Check::warn("IDENTITY", "凭证不可用，未检查运行身份"));
        checks.push(Check::warn("AGENT_ID", "凭证不可用，未读取 Agent ID"));
    }

    if command == Command::Status {
        return checks;
    }

    checks.push(service_check(
        "AGENT_SERVICE",
        probes.service_state(AGENT_SERVICE).await,
        true,
    ));
    checks.push(service_check(
        "RUNNER_SERVICE",
        probes.service_state(RUNNER_SERVICE).await,
        false,
    ));
    checks.push(service_check(
        "EXECUTOR_SERVICE",
        probes.service_state(EXECUTOR_SERVICE).await,
        false,
    ));
    checks.push(if let Some(config) = &config {
        if probes.https_ready(config).await {
            Check::pass("CONTROL_HTTPS", "主控 HTTPS ready 可达")
        } else {
            Check::fail("CONTROL_HTTPS", "主控 HTTPS ready 不可达")
        }
    } else {
        Check::warn("CONTROL_HTTPS", "配置无效，未检查主控 HTTPS")
    });
    checks.push(Check::warn(
        "CONTROL_CHANNEL_AUTH",
        "未验证 WSS upgrade、Agent 身份与心跳，请结合服务日志确认",
    ));
    checks.push(if probes.runner_ready().await {
        Check::pass("RUNNER_PROTOCOL", "runner broker 协议可用")
    } else {
        Check::warn(
            "RUNNER_PROTOCOL",
            "runner broker 协议不可用，部署能力受影响",
        )
    });
    let executor_capabilities = probes.executor_capabilities().await;
    checks.push(if executor_capabilities.is_some() {
        Check::pass("EXECUTOR_PROTOCOL", "root executor v2 协议可用")
    } else {
        Check::warn(
            "EXECUTOR_PROTOCOL",
            "root executor v2 协议不可用，特权能力受影响",
        )
    });
    checks.push(capability_check(
        "PTY_TERMINAL",
        executor_capabilities.as_deref(),
        ExecutorCapability::PtyTerminal,
        "PTY 终端 capability 可用",
        "PTY 终端 capability 不可用",
    ));
    checks.push(capability_check(
        "PRIVILEGED_RELEASE",
        executor_capabilities.as_deref(),
        ExecutorCapability::DeploymentRelease,
        "结构化特权 release capability 可用",
        "结构化特权 release capability 不可用",
    ));
    checks
}

fn capability_check(
    id: &'static str,
    capabilities: Option<&[ExecutorCapability]>,
    expected: ExecutorCapability,
    available: &'static str,
    unavailable: &'static str,
) -> Check {
    if capabilities.is_some_and(|values| values.contains(&expected)) {
        Check::pass(id, available)
    } else {
        Check::warn(id, unavailable)
    }
}

fn environment_overrides() -> HashMap<String, String> {
    KNOWN_CONFIG_KEYS
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| ((*key).to_owned(), value))
        })
        .collect()
}

fn load_config(path: &Path, overrides: &HashMap<String, String>) -> Result<Config, ()> {
    let content = std::fs::read_to_string(path).map_err(|_| ())?;
    let mut values = HashMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(())?;
        if !KNOWN_CONFIG_KEYS.contains(&key)
            || values.contains_key(key)
            || value.chars().any(char::is_control)
        {
            return Err(());
        }
        values.insert(key.to_owned(), value.to_owned());
    }
    values.extend(overrides.clone());
    let control_url = values.get("DEPLOY_GO_AGENT_CONTROL_URL").ok_or(())?;
    let data_dir = values
        .get("DEPLOY_GO_AGENT_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/var/lib/deploy-go-agent".into());
    let heartbeat = values
        .get("DEPLOY_GO_AGENT_HEARTBEAT_SECONDS")
        .map(|value| value.parse().map_err(|_| ()))
        .transpose()?
        .unwrap_or(30);
    Config::parse(control_url, data_dir, heartbeat).map_err(|_| ())
}

fn credential_check(error: CredentialError) -> Check {
    match error {
        CredentialError::Missing => Check::fail("CREDENTIALS", "Agent 凭证不存在"),
        CredentialError::UnsafeDirectoryPermissions | CredentialError::UnsafeFilePermissions => {
            Check::fail("CREDENTIALS", "Agent 凭证权限不安全")
        }
        CredentialError::Invalid => Check::fail("CREDENTIALS", "Agent 凭证内容无效"),
        CredentialError::Io(_) => Check::fail("CREDENTIALS", "Agent 凭证无法读取"),
    }
}

fn service_check(id: &'static str, state: ServiceState, decisive: bool) -> Check {
    match state {
        ServiceState::Active => Check::pass(id, "systemd 服务正在运行"),
        ServiceState::Inactive if decisive => Check::fail(id, "Agent systemd 服务未运行"),
        ServiceState::Inactive => Check::warn(id, "systemd 服务未运行，附加能力受影响"),
        ServiceState::Unavailable => Check::warn(id, "当前环境无法查询 systemd 服务状态"),
    }
}

#[cfg(unix)]
fn credential_owner_matches_current_user(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(path)
        .map(|metadata| {
            metadata.file_type().is_file() && metadata.uid() == unsafe { libc::geteuid() }
        })
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn credential_owner_matches_current_user(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        http::{HeaderMap, StatusCode},
        response::Redirect,
        routing::get,
    };
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    struct FakeProbes {
        agent_service: ServiceState,
        component_services: ServiceState,
        https: bool,
        runner: bool,
        executor: Option<Vec<ExecutorCapability>>,
    }

    #[async_trait]
    impl Probes for FakeProbes {
        async fn service_state(&self, unit: &str) -> ServiceState {
            if unit == AGENT_SERVICE {
                self.agent_service
            } else {
                self.component_services
            }
        }

        async fn https_ready(&self, _: &Config) -> bool {
            self.https
        }

        async fn runner_ready(&self) -> bool {
            self.runner
        }

        async fn executor_capabilities(&self) -> Option<Vec<ExecutorCapability>> {
            self.executor.clone()
        }
    }

    fn fixture() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().unwrap();
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = temp.path().join("config");
        std::fs::write(
            &config,
            format!(
                "DEPLOY_GO_AGENT_CONTROL_URL=wss://deploy.example.test/api/v1/agent/control\nDEPLOY_GO_AGENT_DATA_DIR={}\nDEPLOY_GO_RUNNER_SOCKET=/run/runner.sock\n",
                temp.path().display()
            ),
        ).unwrap();
        let credentials = temp.path().join("credentials.json");
        std::fs::write(
            &credentials,
            r#"{"agent_id":"agent_test","refresh_token":"abcdefghijklmnopqrstuvwxyz1234567890"}"#,
        )
        .unwrap();
        std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o600)).unwrap();
        (temp, config)
    }

    fn healthy_probes() -> FakeProbes {
        FakeProbes {
            agent_service: ServiceState::Active,
            component_services: ServiceState::Active,
            https: true,
            runner: true,
            executor: Some(vec![
                ExecutorCapability::PtyTerminal,
                ExecutorCapability::DeploymentRelease,
            ]),
        }
    }

    async fn serve(router: Router) -> url::Url {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        url::Url::parse(&format!("http://{address}/readyz")).unwrap()
    }

    #[tokio::test]
    async fn doctor_reports_control_failures_and_component_warnings() {
        let (_temp, config) = fixture();
        let checks = collect(
            Command::Doctor,
            &config,
            &HashMap::new(),
            &FakeProbes {
                agent_service: ServiceState::Inactive,
                component_services: ServiceState::Inactive,
                https: false,
                runner: false,
                executor: None,
            },
        )
        .await;
        assert!(
            checks
                .iter()
                .any(|check| check.id == "AGENT_SERVICE" && check.level == Level::Fail)
        );
        assert!(
            checks
                .iter()
                .any(|check| check.id == "CONTROL_HTTPS" && check.level == Level::Fail)
        );
        for id in [
            "RUNNER_SERVICE",
            "EXECUTOR_SERVICE",
            "RUNNER_PROTOCOL",
            "EXECUTOR_PROTOCOL",
            "PTY_TERMINAL",
            "PRIVILEGED_RELEASE",
            "CONTROL_CHANNEL_AUTH",
        ] {
            assert!(
                checks
                    .iter()
                    .any(|check| check.id == id && check.level == Level::Warn),
                "{id} 应为 WARN"
            );
        }
        assert_eq!(exit_code(Command::Doctor, &checks), 2);
        assert_eq!(exit_code(Command::Status, &checks), 0);
    }

    #[test]
    fn config_parser_rejects_unknown_duplicate_and_shell_lines() {
        let temp = TempDir::new().unwrap();
        for (index, content) in [
            "UNKNOWN=value\n",
            "DEPLOY_GO_AGENT_CONTROL_URL=wss://one.test/control\nDEPLOY_GO_AGENT_CONTROL_URL=wss://two.test/control\n",
            "export DEPLOY_GO_AGENT_CONTROL_URL=wss://one.test/control\n",
            "DEPLOY_GO_AGENT_CONTROL_URL=wss://one.test/control\nBROKEN\n",
        ]
        .into_iter()
        .enumerate()
        {
            let path = temp.path().join(format!("config-{index}"));
            std::fs::write(&path, content).unwrap();
            assert!(load_config(&path, &HashMap::new()).is_err());
        }
    }

    #[test]
    fn environment_override_wins_without_evaluating_file_values() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config");
        std::fs::write(&path, "DEPLOY_GO_AGENT_CONTROL_URL=$(echo secret)\n").unwrap();
        let overrides = HashMap::from([(
            "DEPLOY_GO_AGENT_CONTROL_URL".to_owned(),
            "wss://deploy.example.test/api/v1/agent/control".to_owned(),
        )]);
        assert_eq!(
            load_config(&path, &overrides)
                .unwrap()
                .control_url
                .host_str(),
            Some("deploy.example.test")
        );
    }

    #[tokio::test]
    async fn output_messages_never_include_credential_contents() {
        let (temp, config) = fixture();
        std::fs::write(
            temp.path().join("credentials.json"),
            "secret-token-in-invalid-json",
        )
        .unwrap();
        let checks = collect(Command::Status, &config, &HashMap::new(), &healthy_probes()).await;
        let output = render(Command::Status, &checks);
        assert!(!output.contains("secret-token-in-invalid-json"));
        assert!(
            checks
                .iter()
                .any(|check| check.id == "CREDENTIALS" && check.level == Level::Fail)
        );
        assert!(checks.iter().any(|check| check.id == "IDENTITY"));
        assert!(checks.iter().any(|check| check.id == "AGENT_ID"));
    }

    #[tokio::test]
    async fn doctor_output_has_stable_order_and_next_steps() {
        let (_temp, config) = fixture();
        let checks = collect(Command::Doctor, &config, &HashMap::new(), &healthy_probes()).await;
        assert_eq!(
            checks.iter().map(|check| check.id).collect::<Vec<_>>(),
            vec![
                "VERSION",
                "CONFIG",
                "CONTROL_URL",
                "CREDENTIALS",
                "IDENTITY",
                "AGENT_ID",
                "AGENT_SERVICE",
                "RUNNER_SERVICE",
                "EXECUTOR_SERVICE",
                "CONTROL_HTTPS",
                "CONTROL_CHANNEL_AUTH",
                "RUNNER_PROTOCOL",
                "EXECUTOR_PROTOCOL",
                "PTY_TERMINAL",
                "PRIVILEGED_RELEASE",
            ]
        );
        let output = render(Command::Doctor, &checks);
        assert!(output.contains(&format!("协议版本 v{PROTOCOL_VERSION}")));
        assert!(output.contains("WARN CONTROL_CHANNEL_AUTH"));
        assert!(output.contains("INFO NEXT_STEP systemctl status"));
        assert!(output.contains("INFO NEXT_STEP journalctl"));
        assert_eq!(exit_code(Command::Doctor, &checks), 0);
    }

    #[tokio::test]
    async fn unavailable_systemd_is_warning_only() {
        let (_temp, config) = fixture();
        let checks = collect(
            Command::Doctor,
            &config,
            &HashMap::new(),
            &FakeProbes {
                agent_service: ServiceState::Unavailable,
                component_services: ServiceState::Unavailable,
                ..healthy_probes()
            },
        )
        .await;
        for id in ["AGENT_SERVICE", "RUNNER_SERVICE", "EXECUTOR_SERVICE"] {
            assert!(
                checks
                    .iter()
                    .any(|check| check.id == id && check.level == Level::Warn)
            );
        }
        assert_eq!(exit_code(Command::Doctor, &checks), 0);
    }

    #[tokio::test]
    async fn ready_probe_is_anonymous_and_does_not_follow_redirects() {
        async fn ready(headers: HeaderMap) -> StatusCode {
            if headers.contains_key(reqwest::header::AUTHORIZATION)
                || headers.contains_key(reqwest::header::COOKIE)
            {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::OK
            }
        }

        let ready_url = serve(
            Router::new()
                .route("/readyz", get(ready))
                .route(
                    "/redirect",
                    get(|| async { Redirect::temporary("/readyz") }),
                )
                .route(
                    "/unavailable",
                    get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
                ),
        )
        .await;
        assert!(anonymous_ready(ready_url.clone()).await);

        let mut redirect_url = ready_url.clone();
        redirect_url.set_path("/redirect");
        assert!(!anonymous_ready(redirect_url).await);

        let mut unavailable_url = ready_url;
        unavailable_url.set_path("/unavailable");
        assert!(!anonymous_ready(unavailable_url).await);
    }

    #[test]
    fn diagnostic_dispatch_does_not_capture_internal_commands() {
        assert_eq!(Command::from_arg(Some("status")), Some(Command::Status));
        assert_eq!(Command::from_arg(Some("doctor")), Some(Command::Doctor));
        for command in [
            "runner-service",
            "runner-probe",
            "executor-probe",
            "runner",
            "runner-stdin",
            "runner-cancel",
        ] {
            assert_eq!(Command::from_arg(Some(command)), None);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn credential_permissions_are_checked_without_exposing_contents() {
        let (temp, config) = fixture();
        let credentials = temp.path().join("credentials.json");
        std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o644)).unwrap();
        let checks = collect(Command::Status, &config, &HashMap::new(), &healthy_probes()).await;
        assert!(checks.iter().any(|check| {
            check.id == "CREDENTIALS"
                && check.level == Level::Fail
                && check.message == "Agent 凭证权限不安全"
        }));
    }
}
