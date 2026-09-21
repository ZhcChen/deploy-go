use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const MAX_ID_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionState {
    Prepared,
    Staged,
    Stopped,
    Switched,
    Verified,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub schema_version: u16,
    pub job_id: String,
    pub target_version: String,
    pub manifest_digest: String,
    pub state: TransactionState,
    pub updated_at: String,
}

impl Transaction {
    pub fn new(
        job_id: String,
        target_version: String,
        manifest_digest: String,
        _deadline_at: i64,
    ) -> Self {
        Self {
            schema_version: 1,
            job_id,
            target_version,
            manifest_digest,
            state: TransactionState::Prepared,
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionStore {
    root: PathBuf,
}

impl TransactionStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn prepare(&self, transaction: &Transaction) -> anyhow::Result<()> {
        validate_id(&transaction.job_id)?;
        validate_version(&transaction.target_version)?;
        validate_digest(&transaction.manifest_digest)?;
        fs::create_dir_all(&self.root)?;
        let path = self.path(&transaction.job_id);
        if path.exists() {
            let existing = self.read(&transaction.job_id)?;
            if existing.job_id != transaction.job_id
                || existing.target_version != transaction.target_version
                || existing.manifest_digest != transaction.manifest_digest
            {
                anyhow::bail!("升级事务已存在且内容不一致");
            }
            return Ok(());
        }
        self.write_atomic(&path, transaction)
    }

    pub fn read(&self, job_id: &str) -> anyhow::Result<Transaction> {
        validate_id(job_id)?;
        Ok(serde_json::from_slice(&fs::read(self.path(job_id))?)?)
    }

    pub fn update_state(
        &self,
        job_id: &str,
        state: TransactionState,
    ) -> anyhow::Result<Transaction> {
        let mut transaction = self.read(job_id)?;
        transaction.state = state;
        transaction.updated_at = Utc::now().to_rfc3339();
        self.write_atomic(&self.path(job_id), &transaction)?;
        Ok(transaction)
    }

    fn path(&self, job_id: &str) -> PathBuf {
        self.root.join(format!("{job_id}.json"))
    }

    fn write_atomic(&self, path: &std::path::Path, value: &Transaction) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec(value)?;
        let digest = Sha256::digest(&bytes);
        let temporary = path.with_extension(format!("{}.part", hex(&digest)));
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, path)?;
        Ok(())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn validate_id(value: &str) -> anyhow::Result<()> {
    if value.len() > MAX_ID_BYTES
        || !value.starts_with("upgrade_")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        anyhow::bail!("标识符格式无效");
    }
    Ok(())
}

pub fn validate_version(value: &str) -> anyhow::Result<()> {
    if value.is_empty() || value.len() > 64 || value.chars().any(char::is_control) {
        anyhow::bail!("版本格式无效");
    }
    Ok(())
}

pub fn validate_digest(value: &str) -> anyhow::Result<()> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        anyhow::bail!("摘要格式无效");
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!("摘要格式无效");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_is_idempotent_and_rejects_conflicts() {
        let directory = tempfile::tempdir().unwrap();
        let store = TransactionStore::new(directory.path().to_owned());
        let transaction = Transaction::new(
            "upgrade_01TEST".into(),
            "0.3.7".into(),
            format!("sha256:{}", "a".repeat(64)),
            2_000_000_000,
        );
        store.prepare(&transaction).unwrap();
        store.prepare(&transaction).unwrap();
        let mut conflict = transaction.clone();
        conflict.target_version = "0.3.8".into();
        assert!(store.prepare(&conflict).is_err());
    }

    #[test]
    fn validation_rejects_paths_and_non_sha256_digests() {
        assert!(validate_id("upgrade_01/../../tmp").is_err());
        assert!(validate_digest("file:///tmp/manifest").is_err());
        assert!(validate_digest("sha256:ZZ").is_err());
    }
}
