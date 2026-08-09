use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use deploy_go_agent_protocol::{
    Message, SecretLeasePurpose, SecretLeaseRequest, SecretLeaseResponse,
};
use thiserror::Error;
use tokio::sync::{Mutex, mpsc, oneshot};

const LEASE_TIMEOUT: Duration = Duration::from_secs(15);
const KEY_FILE: &str = "git-key";

#[derive(Debug, Error)]
pub enum SecretLeaseError {
    #[error("无法发送 secret lease 请求")]
    RequestFailed,
    #[error("secret lease 响应超时")]
    Timeout,
    #[error("secret lease 被拒绝: {0}")]
    Rejected(String),
    #[error("secret lease 密钥写入失败: {0}")]
    Io(#[from] io::Error),
}

#[derive(Default)]
pub struct SecretLeaseBroker {
    pending: Mutex<HashMap<String, oneshot::Sender<Result<String, String>>>>,
}

impl SecretLeaseBroker {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn fetch(
        &self,
        task_id: &str,
        lease_id: &str,
        payload_digest: &str,
        task_dir: &Path,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<PathBuf, SecretLeaseError> {
        let (sender, receiver) = oneshot::channel();
        self.pending
            .lock()
            .await
            .insert(lease_id.to_owned(), sender);
        let sent = outbound
            .send(Message::SecretLeaseRequest(SecretLeaseRequest {
                task_id: task_id.to_owned(),
                lease_id: lease_id.to_owned(),
                payload_digest: payload_digest.to_owned(),
                purpose: SecretLeasePurpose::GitCredential,
            }))
            .await;
        if sent.is_err() {
            self.pending.lock().await.remove(lease_id);
            return Err(SecretLeaseError::RequestFailed);
        }
        let private_key = tokio::time::timeout(LEASE_TIMEOUT, receiver)
            .await
            .map_err(|_| SecretLeaseError::Timeout);
        if private_key.is_err() {
            self.pending.lock().await.remove(lease_id);
        }
        let private_key = private_key?
            .map_err(|_| SecretLeaseError::RequestFailed)?
            .map_err(SecretLeaseError::Rejected)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(parent) = task_dir.parent() {
                fs::create_dir_all(parent)?;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o3770))?;
            }
            fs::create_dir_all(task_dir)?;
            fs::set_permissions(task_dir, fs::Permissions::from_mode(0o3770))?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(task_dir)?;
        let key_path = task_dir.join(KEY_FILE);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o640);
        }
        let mut file = options.open(&key_path)?;
        io::Write::write_all(&mut file, private_key.as_bytes())?;
        file.sync_all()?;
        Ok(key_path)
    }

    pub async fn resolve(&self, response: SecretLeaseResponse) {
        let Some(sender) = self.pending.lock().await.remove(&response.lease_id) else {
            return;
        };
        let result = match response.error_code {
            Some(code) => Err(format!("secret_lease_{code}")),
            None => Ok(response.private_key),
        };
        let _ = sender.send(result);
    }
}

pub fn key_path(task_dir: &Path) -> PathBuf {
    task_dir.join(KEY_FILE)
}
