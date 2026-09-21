use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::executor_client::ExecutorClient;
use chrono::Utc;
use deploy_go_agent_executor::protocol::{Request, Response, UpgradeStartRequest};
use deploy_go_agent_protocol::{
    AgentUpgradeCommand, AgentUpgradeErrorCode, AgentUpgradePhase, AgentUpgradeProgress,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::mpsc;
use url::Url;

const STATE_FILE: &str = "upgrade-state.json";
const STATE_SCHEMA_VERSION: u16 = 1;
const RELEASE_SCHEMA_VERSION: u32 = 4;
const AGENT_PROTOCOL_VERSION: u64 = 17;
const EXECUTOR_PROTOCOL_VERSION: u64 = 4;
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_COMPONENT_BYTES: usize = 512 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeState {
    pub schema_version: u16,
    pub job_id: String,
    pub target_version: String,
    pub manifest_digest: String,
    pub phase: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct UpgradeStateStore {
    root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpgradeReleaseManifest {
    pub schema_version: u32,
    pub agent_version: String,
    pub executor_version: String,
    #[serde(default)]
    pub runner_protocol: Option<u64>,
    pub executor_protocol: u64,
    pub protocol: ProtocolRange,
    pub systemd_units: SystemdUnits,
    pub executor_config: ReleaseFile,
    pub artifacts: Vec<ReleaseArtifact>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProtocolRange {
    pub minimum: u64,
    pub maximum: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SystemdUnits {
    pub agent: ReleaseFile,
    pub runner: ReleaseFile,
    pub executor: ReleaseFile,
    pub updater: ReleaseFile,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseFile {
    pub url: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseArtifact {
    pub component: String,
    pub os: String,
    pub architecture: String,
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, Error)]
pub enum UpgradeStateError {
    #[error("升级状态无效")]
    Invalid,
    #[error("升级状态文件读写失败: {0}")]
    Io(#[from] io::Error),
    #[error("升级状态 JSON 无效")]
    Json(#[from] serde_json::Error),
}

impl UpgradeStateStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn load(&self) -> Result<Option<UpgradeState>, UpgradeStateError> {
        let path = self.root.join(STATE_FILE);
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let state: UpgradeState = serde_json::from_slice(&bytes)?;
        validate_state(&state)?;
        Ok(Some(state))
    }

    pub fn save(&self, state: &UpgradeState) -> Result<(), UpgradeStateError> {
        validate_state(state)?;
        fs::create_dir_all(&self.root)?;
        let path = self.root.join(STATE_FILE);
        let temporary = self.root.join(format!(".{STATE_FILE}.part"));
        let bytes = serde_json::to_vec(state)?;
        fs::write(&temporary, bytes)?;
        let file = fs::OpenOptions::new().read(true).open(&temporary)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &path)?;
        let directory = fs::File::open(&self.root)?;
        directory.sync_all()?;
        Ok(())
    }
}

pub fn new_state(job_id: &str, target_version: &str, manifest_digest: &str) -> UpgradeState {
    UpgradeState {
        schema_version: STATE_SCHEMA_VERSION,
        job_id: job_id.to_owned(),
        target_version: target_version.to_owned(),
        manifest_digest: manifest_digest.to_owned(),
        phase: "validating".to_owned(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

pub fn sha256_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn validate_release_manifest(
    bytes: &[u8],
    expected_version: &str,
    expected_digest: &str,
) -> Result<UpgradeReleaseManifest, UpgradeStateError> {
    if sha256_digest(bytes) != expected_digest {
        return Err(UpgradeStateError::Invalid);
    }
    let manifest: UpgradeReleaseManifest = serde_json::from_slice(bytes)?;
    let artifacts = &manifest.artifacts;
    let components = artifacts
        .iter()
        .map(|artifact| artifact.component.as_str())
        .collect::<std::collections::HashSet<_>>();
    if manifest.schema_version != RELEASE_SCHEMA_VERSION
        || manifest.agent_version != expected_version
        || manifest.executor_version != expected_version
        || manifest.executor_protocol != EXECUTOR_PROTOCOL_VERSION
        || manifest.protocol.minimum > AGENT_PROTOCOL_VERSION
        || manifest.protocol.maximum < AGENT_PROTOCOL_VERSION
        || artifacts.len() != 3
        || components.len() != 3
        || !["agent", "executor", "updater"]
            .iter()
            .all(|component| components.contains(component))
        || artifacts.iter().any(|artifact| {
            artifact.os != "linux"
                || artifact.architecture != "x86_64"
                || !valid_digest(&format!("sha256:{}", artifact.sha256))
        })
        || !valid_release_file(&manifest.systemd_units.agent)
        || !valid_release_file(&manifest.systemd_units.runner)
        || !valid_release_file(&manifest.systemd_units.executor)
        || !valid_release_file(&manifest.systemd_units.updater)
        || !valid_release_file(&manifest.executor_config)
    {
        return Err(UpgradeStateError::Invalid);
    }
    Ok(manifest)
}

fn valid_release_file(file: &ReleaseFile) -> bool {
    valid_digest(&format!("sha256:{}", file.sha256))
        && file.url.starts_with("https://")
        && file.url.len() <= 2048
}

fn validate_state(state: &UpgradeState) -> Result<(), UpgradeStateError> {
    if state.schema_version != STATE_SCHEMA_VERSION
        || !valid_id(&state.job_id, "upgrade_")
        || state.target_version.is_empty()
        || state.target_version.len() > 64
        || !valid_digest(&state.manifest_digest)
        || !matches!(
            state.phase.as_str(),
            "validating"
                | "downloading"
                | "staged"
                | "installing"
                | "executor_restart"
                | "reconnecting"
                | "succeeded"
                | "failed"
        )
        || state.updated_at.is_empty()
    {
        return Err(UpgradeStateError::Invalid);
    }
    Ok(())
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn valid_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

pub fn verify_file(path: &Path, expected_digest: &str, expected_size: u64) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() != expected_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "升级文件大小或类型无效",
        ));
    }
    let bytes = fs::read(path)?;
    if sha256_digest(&bytes) != expected_digest {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "升级文件摘要不匹配",
        ));
    }
    Ok(())
}

/// Agent 侧升级下载器。所有 URL 都由控制面地址和固定组件名拼接，manifest 中的 URL
/// 只作为展示字段，避免发布物把 Agent 引向任意外部地址。
pub async fn stage_upgrade(
    command: &AgentUpgradeCommand,
    control_url: &Url,
    data_root: &Path,
    client: &Client,
    executor: &ExecutorClient,
    outbound: &mpsc::Sender<deploy_go_agent_protocol::Message>,
) -> Result<(), AgentUpgradeErrorCode> {
    if command.connection_generation == 0 || command.authorization.is_empty() {
        return Err(AgentUpgradeErrorCode::UpgradeExecutorRejected);
    }
    let mut base = control_url.clone();
    base.set_scheme("https")
        .map_err(|_| AgentUpgradeErrorCode::UpgradeManifestInvalid)?;
    base.set_path(&format!(
        "/api/v1/agent/download/{}/manifest.json",
        command.target_version
    ));
    base.set_query(None);
    base.set_fragment(None);
    let state_store = UpgradeStateStore::new(data_root.to_owned());
    let mut state = new_state(
        &command.job_id,
        &command.target_version,
        &command.manifest_digest,
    );
    state_store
        .save(&state)
        .map_err(|_| AgentUpgradeErrorCode::UpgradeInstallFailed)?;
    let _ = send_progress(
        outbound,
        command,
        1,
        AgentUpgradePhase::Validating,
        None,
        None,
    )
    .await;
    let response = client
        .get(base)
        .send()
        .await
        .map_err(|_| AgentUpgradeErrorCode::UpgradeDownloadFailed)?;
    if !response.status().is_success() {
        return Err(AgentUpgradeErrorCode::UpgradeDownloadFailed);
    }
    let manifest_bytes = bounded_response_bytes(response, MAX_MANIFEST_BYTES)
        .await
        .map_err(|_| AgentUpgradeErrorCode::UpgradeDownloadFailed)?;
    let manifest = validate_release_manifest(
        &manifest_bytes,
        &command.target_version,
        &command.manifest_digest,
    )
    .map_err(|_| AgentUpgradeErrorCode::UpgradeManifestInvalid)?;
    state.phase = "downloading".to_owned();
    state.updated_at = Utc::now().to_rfc3339();
    state_store
        .save(&state)
        .map_err(|_| AgentUpgradeErrorCode::UpgradeInstallFailed)?;
    let job_root = data_root.join("upgrades").join(&command.job_id);
    let staging = job_root.join("staging");
    fs::create_dir_all(&staging).map_err(|_| AgentUpgradeErrorCode::UpgradeInstallFailed)?;
    write_atomic(&job_root.join("manifest.json"), &manifest_bytes)
        .map_err(|_| AgentUpgradeErrorCode::UpgradeInstallFailed)?;
    let _ = send_progress(
        outbound,
        command,
        2,
        AgentUpgradePhase::Downloading,
        None,
        None,
    )
    .await;

    let mut files = Vec::new();
    for artifact in &manifest.artifacts {
        if artifact.component != "agent"
            && artifact.component != "executor"
            && artifact.component != "updater"
        {
            return Err(AgentUpgradeErrorCode::UpgradeManifestInvalid);
        }
        files.push((
            artifact.component.as_str(),
            format!("{}/{}", artifact.component, artifact.architecture),
            artifact.sha256.clone(),
        ));
    }
    files.push((
        "unit-agent",
        "systemd-unit/agent".to_owned(),
        manifest.systemd_units.agent.sha256.clone(),
    ));
    files.push((
        "unit-runner",
        "systemd-unit/runner".to_owned(),
        manifest.systemd_units.runner.sha256.clone(),
    ));
    files.push((
        "unit-executor",
        "systemd-unit/executor".to_owned(),
        manifest.systemd_units.executor.sha256.clone(),
    ));
    files.push((
        "unit-updater",
        "systemd-unit/updater".to_owned(),
        manifest.systemd_units.updater.sha256.clone(),
    ));
    files.push((
        "executor-config",
        "executor-config".to_owned(),
        manifest.executor_config.sha256.clone(),
    ));
    for (name, suffix, digest) in files {
        let mut url = control_url.clone();
        url.set_scheme("https")
            .map_err(|_| AgentUpgradeErrorCode::UpgradeManifestInvalid)?;
        url.set_path(&format!(
            "/api/v1/agent/download/{}/{}",
            command.target_version, suffix
        ));
        url.set_query(None);
        url.set_fragment(None);
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|_| AgentUpgradeErrorCode::UpgradeDownloadFailed)?;
        if !response.status().is_success() {
            return Err(AgentUpgradeErrorCode::UpgradeDownloadFailed);
        }
        let bytes = bounded_response_bytes(response, MAX_COMPONENT_BYTES)
            .await
            .map_err(|_| AgentUpgradeErrorCode::UpgradeDownloadFailed)?;
        let expected = format!("sha256:{digest}");
        if sha256_digest(&bytes) != expected {
            return Err(AgentUpgradeErrorCode::UpgradeManifestInvalid);
        }
        let path = staging.join(name);
        write_atomic(&path, &bytes).map_err(|_| AgentUpgradeErrorCode::UpgradeInstallFailed)?;
    }
    state.phase = "staged".to_owned();
    state.updated_at = Utc::now().to_rfc3339();
    state_store
        .save(&state)
        .map_err(|_| AgentUpgradeErrorCode::UpgradeInstallFailed)?;
    let _ = send_progress(outbound, command, 3, AgentUpgradePhase::Staged, None, None).await;
    let deadline_at = chrono::DateTime::parse_from_rfc3339(&command.deadline_at)
        .map_err(|_| AgentUpgradeErrorCode::UpgradeDeadlineExceeded)?
        .timestamp();
    let request = Request::UpgradeStart(UpgradeStartRequest {
        version: None,
        job_id: command.job_id.clone(),
        target_version: command.target_version.clone(),
        manifest_digest: command.manifest_digest.clone(),
        authorization: command.authorization.clone(),
        deadline_at,
    });
    match executor
        .request_with_timeout(request, std::time::Duration::from_secs(10))
        .await
    {
        Ok(Response::UpgradeAccepted(_)) => {
            state.phase = "installing".to_owned();
            state.updated_at = Utc::now().to_rfc3339();
            state_store
                .save(&state)
                .map_err(|_| AgentUpgradeErrorCode::UpgradeInstallFailed)?;
            let _ = send_progress(
                outbound,
                command,
                4,
                AgentUpgradePhase::Installing,
                None,
                None,
            )
            .await;
            Ok(())
        }
        _ => Err(AgentUpgradeErrorCode::UpgradeExecutorRejected),
    }
}

async fn bounded_response_bytes(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, ()> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(());
    }
    let bytes = response.bytes().await.map_err(|_| ())?;
    if bytes.len() > limit {
        return Err(());
    }
    Ok(bytes.to_vec())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension("part");
    fs::write(&temporary, bytes)?;
    let file = fs::OpenOptions::new().read(true).open(&temporary)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path)?;
    Ok(())
}

async fn send_progress(
    outbound: &mpsc::Sender<deploy_go_agent_protocol::Message>,
    command: &AgentUpgradeCommand,
    sequence: u64,
    phase: AgentUpgradePhase,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
) -> Result<(), ()> {
    outbound
        .send(deploy_go_agent_protocol::Message::AgentUpgradeProgress(
            AgentUpgradeProgress {
                job_id: command.job_id.clone(),
                sequence,
                phase,
                downloaded_bytes,
                total_bytes,
            },
        ))
        .await
        .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_store_is_atomic_and_round_trips() {
        let directory = tempfile::tempdir().unwrap();
        let store = UpgradeStateStore::new(directory.path().to_owned());
        let state = new_state(
            "upgrade_01TEST",
            "0.3.7",
            &format!("sha256:{}", "a".repeat(64)),
        );
        store.save(&state).unwrap();
        assert_eq!(store.load().unwrap(), Some(state));
        assert!(!directory.path().join(".upgrade-state.json.part").exists());
    }

    #[test]
    fn invalid_state_and_file_digest_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let store = UpgradeStateStore::new(directory.path().to_owned());
        let mut state = new_state(
            "upgrade_01TEST",
            "0.3.7",
            &format!("sha256:{}", "b".repeat(64)),
        );
        state.job_id = "../escape".into();
        assert!(matches!(
            store.save(&state),
            Err(UpgradeStateError::Invalid)
        ));
        let file = directory.path().join("agent");
        fs::write(&file, b"agent").unwrap();
        assert!(verify_file(&file, &sha256_digest(b"other"), 5).is_err());
        assert!(verify_file(&file, &sha256_digest(b"agent"), 5).is_ok());
    }

    #[test]
    fn release_manifest_requires_the_complete_amd64_v4_set() {
        let manifest = serde_json::json!({
            "schema_version": 4,
            "agent_version": "0.3.7",
            "executor_version": "0.3.7",
            "runner_protocol": 1,
            "executor_protocol": 4,
            "protocol": {"minimum": 11, "maximum": 17},
            "systemd_units": {
                "agent": {"url": "https://release.test/agent.service", "sha256": "a".repeat(64)},
                "runner": {"url": "https://release.test/runner.service", "sha256": "b".repeat(64)},
                "executor": {"url": "https://release.test/executor.service", "sha256": "c".repeat(64)},
                "updater": {"url": "https://release.test/updater.service", "sha256": "d".repeat(64)}
            },
            "executor_config": {"url": "https://release.test/executor.json.in", "sha256": "e".repeat(64)},
            "artifacts": [
                {"component": "agent", "os": "linux", "architecture": "x86_64", "url": "https://release.test/agent", "sha256": "f".repeat(64)},
                {"component": "executor", "os": "linux", "architecture": "x86_64", "url": "https://release.test/executor", "sha256": "1".repeat(64)},
                {"component": "updater", "os": "linux", "architecture": "x86_64", "url": "https://release.test/updater", "sha256": "2".repeat(64)}
            ]
        });
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let digest = sha256_digest(&bytes);
        assert!(validate_release_manifest(&bytes, "0.3.7", &digest).is_ok());
        assert!(validate_release_manifest(&bytes, "0.3.8", &digest).is_err());
    }
}
