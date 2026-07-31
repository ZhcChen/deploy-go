use std::{fmt, io::Write, time::Duration};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ssh_key::{HashAlg, PublicKey};
use tempfile::NamedTempFile;

use super::process;

const PROCESS_TIMEOUT: Duration = Duration::from_secs(15);
const CAPABILITY_SCRIPT: &[u8] = br#"set -eu
root=$1
test -d "$root"
os_name=$(uname -s)
architecture=$(uname -m)
disk_available_kib=$(df -Pk "$root" | awk 'NR == 2 { print $4 }')
printf 'os_name=%s\narchitecture=%s\ndisk_available_bytes=%s\n' "$os_name" "$architecture" "$((disk_available_kib * 1024))"
"#;

#[derive(Clone, Debug)]
pub struct NodeProbeInput {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub work_root: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScannedHostKey {
    pub host_key: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CapabilityReport {
    pub os_name: String,
    pub architecture: String,
    pub disk_available_bytes: u64,
}

#[derive(Clone)]
pub struct ProbeError {
    pub code: &'static str,
    pub message: &'static str,
}

impl ProbeError {
    pub fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

impl fmt::Debug for ProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProbeError")
            .field("code", &self.code)
            .field("message", &self.message)
            .finish()
    }
}

#[async_trait]
pub trait NodeProbe: Send + Sync {
    async fn scan_host_key(&self, node: &NodeProbeInput) -> Result<ScannedHostKey, ProbeError>;

    async fn check(
        &self,
        node: &NodeProbeInput,
        private_key: &[u8],
        trusted_host_key: &str,
    ) -> Result<CapabilityReport, ProbeError>;
}

#[derive(Clone, Debug)]
pub struct OpenSshProbe {
    ssh_program: String,
    keyscan_program: String,
}

impl Default for OpenSshProbe {
    fn default() -> Self {
        Self {
            ssh_program: "ssh".to_owned(),
            keyscan_program: "ssh-keyscan".to_owned(),
        }
    }
}

impl OpenSshProbe {
    pub fn with_programs(
        ssh_program: impl Into<String>,
        keyscan_program: impl Into<String>,
    ) -> Self {
        Self {
            ssh_program: ssh_program.into(),
            keyscan_program: keyscan_program.into(),
        }
    }
}

#[async_trait]
impl NodeProbe for OpenSshProbe {
    async fn scan_host_key(&self, node: &NodeProbeInput) -> Result<ScannedHostKey, ProbeError> {
        validate_connection(node)?;
        let args = vec![
            "-T".to_owned(),
            "5".to_owned(),
            "-p".to_owned(),
            node.port.to_string(),
            "-t".to_owned(),
            "ed25519".to_owned(),
            node.host.clone(),
        ];
        let output = process::run(&self.keyscan_program, &args, None, PROCESS_TIMEOUT).await?;
        if !output.status.success() {
            return Err(classify_process_error(&output.stderr));
        }
        let mut scanned = parse_keyscan(&output.stdout)?;
        let key_fields: Vec<&str> = scanned.host_key.split_whitespace().collect();
        if key_fields.len() != 3 {
            return Err(ProbeError::new("invalid_host_key", "host key 格式无效"));
        }
        scanned.host_key = format!(
            "{} {} {}",
            known_hosts_name(&node.host, node.port),
            key_fields[1],
            key_fields[2]
        );
        Ok(scanned)
    }

    async fn check(
        &self,
        node: &NodeProbeInput,
        private_key: &[u8],
        trusted_host_key: &str,
    ) -> Result<CapabilityReport, ProbeError> {
        validate_connection(node)?;
        let mut identity = secure_temp_file()?;
        identity
            .write_all(private_key)
            .map_err(|_| ProbeError::new("identity_write_failed", "无法准备 SSH 身份文件"))?;
        let mut known_hosts = secure_temp_file()?;
        writeln!(known_hosts, "{trusted_host_key}")
            .map_err(|_| ProbeError::new("known_hosts_write_failed", "无法准备 host key 文件"))?;
        let remote_command = format!("sh -s -- {}", encode_posix_token(&node.work_root));
        let args = vec![
            "-F".to_owned(),
            "/dev/null".to_owned(),
            "-o".to_owned(),
            "BatchMode=yes".to_owned(),
            "-o".to_owned(),
            "IdentitiesOnly=yes".to_owned(),
            "-o".to_owned(),
            "StrictHostKeyChecking=yes".to_owned(),
            "-o".to_owned(),
            format!("UserKnownHostsFile={}", known_hosts.path().display()),
            "-o".to_owned(),
            "ConnectTimeout=10".to_owned(),
            "-i".to_owned(),
            identity.path().display().to_string(),
            "-p".to_owned(),
            node.port.to_string(),
            ssh_destination(&node.username, &node.host),
            remote_command,
        ];
        let output = process::run(
            &self.ssh_program,
            &args,
            Some(CAPABILITY_SCRIPT),
            PROCESS_TIMEOUT,
        )
        .await?;
        if !output.status.success() {
            return Err(classify_process_error(&output.stderr));
        }
        parse_capabilities(&output.stdout)
    }
}

pub fn validate_connection(node: &NodeProbeInput) -> Result<(), ProbeError> {
    if node.host.is_empty()
        || node.host.starts_with('-')
        || !node
            .host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-'))
        || node.username.is_empty()
        || node.username.starts_with('-')
        || !node
            .username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        || !node.work_root.starts_with('/')
        || node.work_root.chars().any(char::is_control)
    {
        return Err(ProbeError::new(
            "invalid_connection_parameters",
            "节点 SSH 参数不合法",
        ));
    }
    Ok(())
}

fn ssh_destination(username: &str, host: &str) -> String {
    if host.contains(':') {
        format!("{username}@[{host}]")
    } else {
        format!("{username}@{host}")
    }
}

fn known_hosts_name(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_owned()
    } else {
        format!("[{host}]:{port}")
    }
}

fn parse_keyscan(output: &[u8]) -> Result<ScannedHostKey, ProbeError> {
    let text = String::from_utf8_lossy(output);
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let mut fields = line.split_whitespace();
        let host = fields.next();
        let algorithm = fields.next();
        let key = fields.next();
        if let (Some(host), Some("ssh-ed25519"), Some(key)) = (host, algorithm, key) {
            let encoded = format!("ssh-ed25519 {key}");
            let public = PublicKey::from_openssh(&encoded)
                .map_err(|_| ProbeError::new("invalid_host_key", "host key 格式无效"))?;
            return Ok(ScannedHostKey {
                host_key: format!("{host} ssh-ed25519 {key}"),
                fingerprint: public.fingerprint(HashAlg::Sha256).to_string(),
            });
        }
    }
    Err(ProbeError::new(
        "host_key_missing",
        "未取得 Ed25519 host key",
    ))
}

fn parse_capabilities(output: &[u8]) -> Result<CapabilityReport, ProbeError> {
    let text = String::from_utf8(output.to_vec())
        .map_err(|_| ProbeError::new("invalid_output", "节点检查输出不是 UTF-8"))?;
    let value = |name: &str| {
        text.lines()
            .find_map(|line| line.split_once('=').filter(|(key, _)| *key == name))
            .map(|(_, value)| value.to_owned())
    };
    Ok(CapabilityReport {
        os_name: value("os_name")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ProbeError::new("invalid_output", "节点检查缺少系统信息"))?,
        architecture: value("architecture")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ProbeError::new("invalid_output", "节点检查缺少架构信息"))?,
        disk_available_bytes: value("disk_available_bytes")
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| ProbeError::new("invalid_output", "节点检查缺少磁盘信息"))?,
    })
}

fn classify_process_error(stderr: &[u8]) -> ProbeError {
    let stderr = String::from_utf8_lossy(stderr);
    if stderr.contains("Permission denied") {
        ProbeError::new("authentication_failed", "SSH 身份验证失败")
    } else if stderr.contains("Could not resolve hostname") {
        ProbeError::new("dns_failed", "节点域名解析失败")
    } else if stderr.contains("REMOTE HOST IDENTIFICATION HAS CHANGED") {
        ProbeError::new("host_key_changed", "节点 host key 已变化")
    } else if stderr.contains("timed out") || stderr.contains("Connection timed out") {
        ProbeError::new("timeout", "SSH 连接超时")
    } else {
        ProbeError::new("connection_failed", "SSH 连接失败")
    }
}

fn secure_temp_file() -> Result<NamedTempFile, ProbeError> {
    let file = NamedTempFile::new()
        .map_err(|_| ProbeError::new("temporary_file_failed", "无法创建 SSH 临时文件"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|_| ProbeError::new("temporary_file_failed", "无法保护 SSH 临时文件"))?;
    }
    Ok(file)
}

pub fn encode_posix_token(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::{
        NodeProbeInput, classify_process_error, encode_posix_token, known_hosts_name,
        parse_capabilities, parse_keyscan, ssh_destination, validate_connection,
    };

    #[test]
    fn token_encoder_handles_single_quotes() {
        assert_eq!(encode_posix_token("/srv/a'b"), "'/srv/a'\\''b'");
    }

    #[test]
    fn connection_parameters_reject_option_injection() {
        let mut node = NodeProbeInput {
            id: "node_1".to_owned(),
            host: "example.test".to_owned(),
            port: 22,
            username: "deploy".to_owned(),
            work_root: "/srv/apps".to_owned(),
        };
        assert!(validate_connection(&node).is_ok());
        node.host = "-oProxyCommand=bad".to_owned();
        assert!(validate_connection(&node).is_err());
        node.host = "example.test".to_owned();
        node.username = "-oProxyCommand".to_owned();
        assert!(validate_connection(&node).is_err());
        assert_eq!(ssh_destination("deploy", "::1"), "deploy@[::1]");
        assert_eq!(known_hosts_name("node.test", 22), "node.test");
        assert_eq!(known_hosts_name("node.test", 2222), "[node.test]:2222");
    }

    #[test]
    fn capability_parser_requires_all_fields() {
        assert!(
            parse_capabilities(b"os_name=Linux\narchitecture=x86_64\ndisk_available_bytes=4096\n")
                .is_ok()
        );
        assert!(parse_capabilities(b"os_name=Linux\n").is_err());
    }

    #[test]
    fn process_errors_have_stable_categories() {
        assert_eq!(
            classify_process_error(b"Permission denied").code,
            "authentication_failed"
        );
        assert_eq!(
            classify_process_error(b"Could not resolve hostname").code,
            "dns_failed"
        );
        assert_eq!(
            classify_process_error(b"REMOTE HOST IDENTIFICATION HAS CHANGED").code,
            "host_key_changed"
        );
        assert_eq!(
            classify_process_error(b"Connection timed out").code,
            "timeout"
        );
        assert_eq!(
            classify_process_error(b"Connection refused").code,
            "connection_failed"
        );
    }

    #[test]
    fn keyscan_parser_accepts_only_ed25519() {
        let parsed = parse_keyscan(b"[node.example.test]:22 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILM+rvN+ot98qgEN796jTiQfZfG1KaT0PtFDJ/XFSqti\n").unwrap();
        assert!(parsed.fingerprint.starts_with("SHA256:"));
        assert!(parse_keyscan(b"node.test ssh-rsa AAAA\n").is_err());
    }
}
