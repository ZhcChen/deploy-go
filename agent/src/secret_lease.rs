use std::{
    collections::HashMap,
    fs::OpenOptions,
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
            if let Some(parent) = task_dir.parent() {
                crate::dir_guard::ensure_directory_mode(parent, 0o3710, &[0o3710])?;
            }
            crate::dir_guard::ensure_directory_mode(task_dir, 0o3700, &[0o3700, 0o3770])?;
        }
        #[cfg(not(unix))]
        std::fs::create_dir_all(task_dir)?;
        let key_path = task_dir.join(KEY_FILE);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // OpenSSH 强制私钥文件仅属主可读写；runner 使用由
            // runner broker 以 root 身份生成的私有副本 runner-git-key。
            options.mode(0o600);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn git_key_is_private_to_owner() {
        use std::sync::Arc;

        let directory = tempfile::tempdir().unwrap();
        let task_dir = directory.path().join("tasks/task_lease");
        let broker = Arc::new(SecretLeaseBroker::new());
        let (sender, mut receiver) = mpsc::channel(4);

        let fetch_broker = Arc::clone(&broker);
        let fetch = tokio::spawn(async move {
            fetch_broker
                .fetch("task_lease", "lease_01", "sha256:1234", &task_dir, &sender)
                .await
                .unwrap()
        });

        let request = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .expect("等待 secret lease 请求超时")
            .expect("发送方提前关闭");
        let Message::SecretLeaseRequest(SecretLeaseRequest { lease_id, .. }) = request else {
            panic!("预期收到 secret lease 请求");
        };
        broker
            .resolve(SecretLeaseResponse {
                lease_id,
                private_key:
                    "-----BEGIN OPENSSH PRIVATE KEY-----\nfake\n-----END OPENSSH PRIVATE KEY-----\n"
                        .to_owned(),
                expires_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
                error_code: None,
            })
            .await;

        let key_path = tokio::time::timeout(Duration::from_secs(5), fetch)
            .await
            .expect("获取私钥任务超时")
            .expect("获取私钥失败");
        let metadata = std::fs::metadata(&key_path).unwrap();
        assert!(metadata.len() > 0);
        #[cfg(unix)]
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }
}
