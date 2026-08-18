use std::{
    collections::HashMap,
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use deploy_go_agent_protocol::{
    Message, SecretEnvironmentLeaseRef, SecretEnvironmentLeaseRequest,
    SecretEnvironmentLeaseResponse, SecretEnvironmentVariable, SecretLeasePurpose,
    SecretLeaseRequest, SecretLeaseResponse,
};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::{Mutex, mpsc, oneshot};
use zeroize::Zeroizing;

const LEASE_TIMEOUT: Duration = Duration::from_secs(15);
const ENVIRONMENT_LEASE_ATTEMPTS: usize = 3;
const KEY_FILE: &str = "git-key";

type EnvironmentLeaseKey = (String, String);
type EnvironmentLeaseWaiter = oneshot::Sender<Result<Vec<SecretEnvironmentVariable>, String>>;

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
    pending_environment: Mutex<HashMap<EnvironmentLeaseKey, EnvironmentLeaseWaiter>>,
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

    pub async fn fetch_environment(
        &self,
        task_id: &str,
        lease: &SecretEnvironmentLeaseRef,
        payload_digest: &str,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<Vec<SecretEnvironmentVariable>, SecretLeaseError> {
        let mut last_error = SecretLeaseError::RequestFailed;
        for _ in 0..ENVIRONMENT_LEASE_ATTEMPTS {
            match self
                .fetch_environment_once(task_id, lease, payload_digest, outbound)
                .await
            {
                Ok(variables) => return Ok(variables),
                Err(error @ SecretLeaseError::Rejected(_)) => return Err(error),
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    async fn fetch_environment_once(
        &self,
        task_id: &str,
        lease: &SecretEnvironmentLeaseRef,
        payload_digest: &str,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<Vec<SecretEnvironmentVariable>, SecretLeaseError> {
        let delivery_nonce = format!("secret_delivery_{}", ulid::Ulid::new());
        let key = (lease.lease_id.clone(), delivery_nonce.clone());
        let (sender, receiver) = oneshot::channel();
        self.pending_environment
            .lock()
            .await
            .insert(key.clone(), sender);
        let sent = outbound
            .send(Message::SecretEnvironmentLeaseRequest(
                SecretEnvironmentLeaseRequest {
                    task_id: task_id.to_owned(),
                    lease_id: lease.lease_id.clone(),
                    payload_digest: payload_digest.to_owned(),
                    descriptor_digest: lease.descriptor.descriptor_digest.clone(),
                    delivery_nonce: delivery_nonce.clone(),
                },
            ))
            .await;
        if sent.is_err() {
            self.pending_environment.lock().await.remove(&key);
            return Err(SecretLeaseError::RequestFailed);
        }
        let response = match tokio::time::timeout(LEASE_TIMEOUT, receiver).await {
            Ok(Ok(response)) => response.map_err(SecretLeaseError::Rejected),
            Ok(Err(_)) => Err(SecretLeaseError::RequestFailed),
            Err(_) => Err(SecretLeaseError::Timeout),
        };
        if response.is_err() {
            self.pending_environment.lock().await.remove(&key);
        }
        response
    }

    pub async fn resolve_environment(&self, response: SecretEnvironmentLeaseResponse) {
        let key = (response.lease_id.clone(), response.delivery_nonce.clone());
        let Some(sender) = self.pending_environment.lock().await.remove(&key) else {
            return;
        };
        let result = match response.error_code {
            Some(code) => Err(format!("secret_lease_{code}")),
            None => {
                let unexpired = chrono::DateTime::parse_from_rfc3339(&response.expires_at)
                    .is_ok_and(|expires_at| expires_at > chrono::Utc::now());
                if !unexpired {
                    let _ = sender.send(Err("secret_lease_expired".to_owned()));
                    return;
                }
                let mut variables = response.variables;
                variables.sort_by(|left, right| left.name.cmp(&right.name));
                let encoded = Zeroizing::new(serde_json::to_vec(&variables).unwrap_or_default());
                let digest = format!("sha256:{:x}", Sha256::digest(encoded.as_slice()));
                if response.value_digest.as_deref() != Some(digest.as_str()) {
                    Err("secret_lease_value_digest_mismatch".to_owned())
                } else {
                    Ok(variables)
                }
            }
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

    fn environment_lease() -> SecretEnvironmentLeaseRef {
        SecretEnvironmentLeaseRef {
            lease_id: "lease_environment_01".into(),
            descriptor: deploy_go_agent_protocol::SecretEnvironmentDescriptor {
                purpose: deploy_go_agent_protocol::SecretEnvironmentPurpose::ConfigCenterConnection,
                variable_names: vec![
                    "DEPLOY_CONFIG_CENTER_ENDPOINTS".into(),
                    "DEPLOY_CONFIG_CENTER_PASSWORD".into(),
                    "DEPLOY_CONFIG_CENTER_PREFIX".into(),
                    "DEPLOY_CONFIG_CENTER_TYPE".into(),
                    "DEPLOY_CONFIG_CENTER_USERNAME".into(),
                ],
                credential_version: 1,
                template_id: "etcd".into(),
                template_version: "3.6".into(),
                template_digest: "sha256:template".into(),
                release_stage: deploy_go_agent_protocol::DeploymentStage::Release,
                executor_audience: "release_executor".into(),
                target_process: "deploy-release".into(),
                descriptor_digest: "sha256:descriptor".into(),
            },
        }
    }

    fn environment_variables() -> Vec<SecretEnvironmentVariable> {
        vec![
            SecretEnvironmentVariable {
                name: "DEPLOY_CONFIG_CENTER_TYPE".into(),
                value: "etcd".into(),
            },
            SecretEnvironmentVariable {
                name: "DEPLOY_CONFIG_CENTER_ENDPOINTS".into(),
                value: "[\"http://127.0.0.1:2379\"]".into(),
            },
            SecretEnvironmentVariable {
                name: "DEPLOY_CONFIG_CENTER_PREFIX".into(),
                value: "/deploy-go/apps/app/test/".into(),
            },
            SecretEnvironmentVariable {
                name: "DEPLOY_CONFIG_CENTER_USERNAME".into(),
                value: "app_test".into(),
            },
            SecretEnvironmentVariable {
                name: "DEPLOY_CONFIG_CENTER_PASSWORD".into(),
                value: "password".into(),
            },
        ]
    }

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

    #[tokio::test]
    async fn environment_response_requires_matching_delivery_nonce_and_digest() {
        let broker = std::sync::Arc::new(SecretLeaseBroker::new());
        let (sender, mut receiver) = mpsc::channel(4);
        let lease = environment_lease();
        let fetch_broker = std::sync::Arc::clone(&broker);
        let fetch_lease = lease.clone();
        let fetch = tokio::spawn(async move {
            fetch_broker
                .fetch_environment(
                    "task_environment_01",
                    &fetch_lease,
                    "sha256:payload",
                    &sender,
                )
                .await
        });
        let Message::SecretEnvironmentLeaseRequest(request) = receiver.recv().await.unwrap() else {
            panic!("预期收到敏感环境租约请求");
        };
        let variables = environment_variables();
        broker
            .resolve_environment(SecretEnvironmentLeaseResponse {
                lease_id: request.lease_id.clone(),
                delivery_nonce: "secret_delivery_stale".into(),
                value_digest: Some("sha256:stale".into()),
                variables: variables.clone(),
                expires_at: "2099-08-16T00:01:00Z".into(),
                error_code: None,
            })
            .await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!fetch.is_finished());

        let mut sorted = variables;
        sorted.sort_by(|left, right| left.name.cmp(&right.name));
        let value_digest = format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&sorted).unwrap())
        );
        broker
            .resolve_environment(SecretEnvironmentLeaseResponse {
                lease_id: request.lease_id,
                delivery_nonce: request.delivery_nonce,
                value_digest: Some(value_digest),
                variables: sorted,
                expires_at: "2099-08-16T00:01:00Z".into(),
                error_code: None,
            })
            .await;
        assert!(fetch.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn environment_response_rejects_value_digest_mismatch() {
        let broker = std::sync::Arc::new(SecretLeaseBroker::new());
        let (sender, mut receiver) = mpsc::channel(4);
        let fetch_broker = std::sync::Arc::clone(&broker);
        let fetch = tokio::spawn(async move {
            fetch_broker
                .fetch_environment(
                    "task_environment_02",
                    &environment_lease(),
                    "sha256:payload",
                    &sender,
                )
                .await
        });
        let Message::SecretEnvironmentLeaseRequest(request) = receiver.recv().await.unwrap() else {
            panic!("预期收到敏感环境租约请求");
        };
        broker
            .resolve_environment(SecretEnvironmentLeaseResponse {
                lease_id: request.lease_id,
                delivery_nonce: request.delivery_nonce,
                value_digest: Some("sha256:wrong".into()),
                variables: environment_variables(),
                expires_at: "2099-08-16T00:01:00Z".into(),
                error_code: None,
            })
            .await;
        assert!(matches!(
            fetch.await.unwrap(),
            Err(SecretLeaseError::Rejected(code)) if code == "secret_lease_value_digest_mismatch"
        ));
    }
}
