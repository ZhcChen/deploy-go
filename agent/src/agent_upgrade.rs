use std::{
    fs, io,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const STATE_FILE: &str = "upgrade-state.json";
const STATE_SCHEMA_VERSION: u16 = 1;

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
}
