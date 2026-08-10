use std::{
    collections::{HashMap, hash_map::Entry},
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use chrono::Utc;
use deploy_go_agent_protocol::{
    ArtifactPrepared, ArtifactUploadAuthorized, DeployEvent, DeploymentPrepareTask,
    DeploymentReleaseTask, EnvSyncAction, EnvSyncTask, Envelope, GitRefsQueryTask, Message,
    OutputStream, ReconcileReport, ReconciledTask, ReconciledTaskState,
    ReleaseAuthorizationRequest, ReleaseAuthorizationResponse, SystemInspectTask, TaskAck,
    TaskAckDisposition, TaskCancel, TaskDispatch, TaskLifecycleState, TaskOutput, TaskPayload,
    TaskProgress, TaskResult, TaskState, TaskTerminalStatus,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::{
    artifact_transfer::{ArchivePreparation, ArtifactTransferClient, ArtifactTransferError},
    connection::{ConnectionError, MessageHandler},
    env_sync::{EnvFileStore, EnvSecretClient, EnvSyncError},
    executor::{ExecuteError, Executor},
    executor_client::ExecutorClient,
    journal::{JournalState, RecoveryState, TaskJournal},
    secret_lease::{SecretLeaseBroker, SecretLeaseError},
};

const OUTPUT_CHUNK_BYTES: usize = 32 * 1024;

#[derive(Debug)]
enum PreparedArtifactTransferError {
    Configuration(&'static str),
    Deadline,
    Prepare(ArtifactTransferError),
    Authorization(&'static str),
    Upload(ArtifactTransferError),
    UploadTimeout,
    Canceled,
}

impl PreparedArtifactTransferError {
    fn code(&self) -> &'static str {
        match self {
            Self::Configuration(_) => "artifact_transfer_unavailable",
            Self::Deadline => "artifact_transfer_deadline_exceeded",
            Self::Prepare(_) => "artifact_prepare_failed",
            Self::Authorization("timeout") => "artifact_authorization_timeout",
            Self::Authorization(_) => "artifact_authorization_failed",
            Self::Upload(_) => "artifact_transfer_failed",
            Self::UploadTimeout => "artifact_transfer_timeout",
            Self::Canceled => "task_canceled",
        }
    }

    fn detail(&self) -> String {
        match self {
            Self::Configuration(detail) | Self::Authorization(detail) => (*detail).to_owned(),
            Self::Prepare(error) | Self::Upload(error) => error.to_string(),
            Self::Deadline => "任务截止时间已到".to_owned(),
            Self::UploadTimeout => "artifact 上传超时".to_owned(),
            Self::Canceled => "任务已取消".to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct TaskHandler {
    executor: Arc<Executor>,
    event_lock: Arc<Mutex<()>>,
    secret_lease: Arc<SecretLeaseBroker>,
    artifact_transfer: Option<Arc<ArtifactTransferClient>>,
    env_secret: Option<Arc<EnvSecretClient>>,
    env_store: Option<Arc<EnvFileStore>>,
    artifact_authorizations: Arc<Mutex<HashMap<String, oneshot::Sender<ArtifactUploadAuthorized>>>>,
    release_authorizations:
        Arc<Mutex<HashMap<String, oneshot::Sender<ReleaseAuthorizationResponse>>>>,
    transfer_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    privileged_release_executor: Option<Arc<ExecutorClient>>,
}

impl TaskHandler {
    pub fn new(executor: Executor) -> Self {
        Self {
            executor: Arc::new(executor),
            event_lock: Arc::new(Mutex::new(())),
            secret_lease: Arc::new(SecretLeaseBroker::new()),
            artifact_transfer: None,
            env_secret: None,
            env_store: None,
            artifact_authorizations: Arc::new(Mutex::new(HashMap::new())),
            release_authorizations: Arc::new(Mutex::new(HashMap::new())),
            transfer_locks: Arc::new(Mutex::new(HashMap::new())),
            privileged_release_executor: None,
        }
    }

    pub fn with_artifact_transfer(mut self, client: ArtifactTransferClient) -> Self {
        self.artifact_transfer = Some(Arc::new(client));
        self
    }

    pub fn with_env_sync(mut self, client: EnvSecretClient, store: EnvFileStore) -> Self {
        self.env_secret = Some(Arc::new(client));
        self.env_store = Some(Arc::new(store));
        self
    }

    pub fn with_privileged_release_executor(mut self, client: ExecutorClient) -> Self {
        self.privileged_release_executor = Some(Arc::new(client));
        self
    }

    async fn transfer_lock(&self, task_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.transfer_locks.lock().await;
        locks
            .entry(task_id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn dispatch(&self, dispatch: TaskDispatch, outbound: mpsc::Sender<Message>) {
        if self
            .executor
            .validate_dispatch_identity(
                &dispatch.task_id,
                &dispatch.idempotency_key,
                &dispatch.payload_digest,
            )
            .is_err()
        {
            let _ = send_ack(
                &outbound,
                &dispatch,
                TaskAckDisposition::Rejected,
                Some("invalid_task_identity"),
            )
            .await;
            return;
        }
        if deadline_expired(&dispatch.deadline_at) {
            let _ = send_ack(
                &outbound,
                &dispatch,
                TaskAckDisposition::Rejected,
                Some("deadline_expired"),
            )
            .await;
            return;
        }
        let task = match &dispatch.task {
            TaskPayload::SystemInspect(task) => {
                self.inspect(&dispatch, task, outbound).await;
                return;
            }
            TaskPayload::GitRefsQuery(task) => {
                self.refs_query(&dispatch, task, outbound).await;
                return;
            }
            TaskPayload::DeploymentPrepare(task) => {
                self.prepare(&dispatch, task, outbound).await;
                return;
            }
            TaskPayload::DeploymentRelease(task) => {
                self.release(&dispatch, task, outbound).await;
                return;
            }
            TaskPayload::EnvSync(task) => {
                self.env_sync(&dispatch, task, outbound).await;
                return;
            }
            TaskPayload::DeploymentExecute(task) => task,
            TaskPayload::HealthDiagnose(_) => {
                let _ = send_ack(
                    &outbound,
                    &dispatch,
                    TaskAckDisposition::Rejected,
                    Some("unsupported_task_type"),
                )
                .await;
                return;
            }
        };

        match self
            .executor
            .execute(
                &dispatch.task_id,
                &dispatch.idempotency_key,
                &dispatch.payload_digest,
                task,
            )
            .await
        {
            Ok(mut journal) => {
                if send_ack(&outbound, &dispatch, TaskAckDisposition::Accepted, None)
                    .await
                    .is_err()
                {
                    return;
                }
                if send_state(
                    &self.executor,
                    &self.event_lock,
                    &outbound,
                    &mut journal,
                    TaskLifecycleState::Accepted,
                )
                .await
                .is_err()
                    || send_state(
                        &self.executor,
                        &self.event_lock,
                        &outbound,
                        &mut journal,
                        TaskLifecycleState::Running,
                    )
                    .await
                    .is_err()
                {
                    return;
                }
                monitor(
                    self.executor.clone(),
                    self.event_lock.clone(),
                    journal,
                    outbound,
                )
                .await;
            }
            Err(ExecuteError::Duplicate) => {
                let Ok(journal) = self.executor.load(&dispatch.task_id) else {
                    let _ = send_ack(
                        &outbound,
                        &dispatch,
                        TaskAckDisposition::Rejected,
                        Some("idempotency_conflict"),
                    )
                    .await;
                    return;
                };
                if journal.payload_digest != dispatch.payload_digest {
                    let _ = send_ack(
                        &outbound,
                        &dispatch,
                        TaskAckDisposition::Rejected,
                        Some("payload_conflict"),
                    )
                    .await;
                    return;
                }
                if send_ack(&outbound, &dispatch, TaskAckDisposition::Duplicate, None)
                    .await
                    .is_ok()
                {
                    replay(
                        self.executor.clone(),
                        self.event_lock.clone(),
                        journal,
                        outbound,
                    )
                    .await;
                }
            }
            Err(error) => {
                let code = execute_error_code(&error);
                let _ = send_ack(
                    &outbound,
                    &dispatch,
                    TaskAckDisposition::Rejected,
                    Some(code),
                )
                .await;
            }
        }
    }

    async fn env_sync(
        &self,
        dispatch: &TaskDispatch,
        task: &EnvSyncTask,
        outbound: mpsc::Sender<Message>,
    ) {
        let (Some(client), Some(store)) = (self.env_secret.clone(), self.env_store.clone()) else {
            let _ = send_ack(
                &outbound,
                dispatch,
                TaskAckDisposition::Rejected,
                Some("env_sync_disabled"),
            )
            .await;
            return;
        };
        let mut journal = match self
            .executor
            .create_transfer_task(
                &dispatch.task_id,
                &dispatch.idempotency_key,
                &dispatch.payload_digest,
                crate::journal::TransferPhase::EnvSync,
            )
            .await
        {
            Ok(journal) => journal,
            Err(ExecuteError::Duplicate) => {
                self.handle_existing(dispatch, outbound).await;
                return;
            }
            Err(error) => {
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some(execute_error_code(&error)),
                )
                .await;
                return;
            }
        };
        if send_ack(&outbound, dispatch, TaskAckDisposition::Accepted, None)
            .await
            .is_err()
            || send_state(
                &self.executor,
                &self.event_lock,
                &outbound,
                &mut journal,
                TaskLifecycleState::Running,
            )
            .await
            .is_err()
        {
            return;
        }

        let result = match task.action {
            EnvSyncAction::Write => match client.fetch(&task.lease_id).await {
                Ok(mut content) => {
                    let store = store.clone();
                    let application_slug = task.application_slug.clone();
                    let file_name = task.file_name.clone();
                    let digest = task.digest.clone();
                    let written = tokio::task::spawn_blocking(move || {
                        let result = store.write(&application_slug, &file_name, &content, &digest);
                        content.fill(0);
                        result
                    })
                    .await
                    .map_err(|_| EnvSyncError::Transport)
                    .and_then(|result| result);
                    written.map(|_| ())
                }
                Err(error) => Err(error),
            },
            EnvSyncAction::Delete => {
                let store = store.clone();
                let application_slug = task.application_slug.clone();
                let file_name = task.file_name.clone();
                tokio::task::spawn_blocking(move || store.delete(&application_slug, &file_name))
                    .await
                    .map_err(|_| EnvSyncError::Transport)
                    .and_then(|result| result)
            }
        };
        let result_data = result.is_ok().then(|| {
            json!({
                "env_sync_id": task.env_sync_id,
                "env_version": task.env_version,
                "digest": task.digest,
                "action": task.action,
            })
        });
        let error_code = result.err().map(env_sync_error_code);
        if let Ok(mut completed) = self.executor.complete_task(
            &dispatch.task_id,
            if error_code.is_some() {
                JournalState::Failed
            } else {
                JournalState::Succeeded
            },
            error_code,
            result_data,
        ) {
            let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut completed).await;
        }
    }

    async fn monitor_prepare_transfer(
        &self,
        mut journal: TaskJournal,
        task: DeploymentPrepareTask,
        deadline_at: String,
        outbound: mpsc::Sender<Message>,
    ) {
        let lock = self.transfer_lock(&journal.task_id).await;
        let _guard = lock.lock().await;
        if let Ok(current) = self.executor.load(&journal.task_id) {
            if terminal(&current.state) {
                replay(
                    self.executor.clone(),
                    self.event_lock.clone(),
                    current,
                    outbound,
                )
                .await;
                return;
            }
            journal = current;
        }
        loop {
            if drain_outputs(&self.executor, &self.event_lock, &outbound, &mut journal)
                .await
                .is_err()
                || drain_events(&self.executor, &self.event_lock, &outbound, &mut journal)
                    .await
                    .is_err()
            {
                return;
            }
            match self.executor.poll_completion(&journal.task_id) {
                Ok(Some(mut current)) => {
                    let _ =
                        drain_outputs(&self.executor, &self.event_lock, &outbound, &mut current)
                            .await;
                    let _ = drain_events(&self.executor, &self.event_lock, &outbound, &mut current)
                        .await;
                    if current.state == JournalState::Succeeded {
                        current.state = JournalState::Running;
                        current.transfer_phase = Some(crate::journal::TransferPhase::PrepareUpload);
                        if self.executor.store_journal(&current).is_ok() {
                            match self
                                .transfer_prepared_artifact(
                                    &task,
                                    &current.task_id,
                                    &deadline_at,
                                    &outbound,
                                )
                                .await
                            {
                                Ok(()) => {
                                    if let Ok(completed) = self.executor.complete_task(
                                        &current.task_id,
                                        JournalState::Succeeded,
                                        None,
                                        None,
                                    ) {
                                        current = completed;
                                    }
                                }
                                Err(error) => {
                                    let canceled = self
                                        .executor
                                        .is_cancel_requested(&current.task_id)
                                        || matches!(error, PreparedArtifactTransferError::Canceled);
                                    tracing::error!(
                                        task_id = %current.task_id,
                                        deployment_id = %task.deployment_id,
                                        stage = error.code(),
                                        error = %error.detail(),
                                        "prepared artifact transfer failed"
                                    );
                                    if let Ok(failed) = self.executor.complete_task(
                                        &current.task_id,
                                        if canceled {
                                            JournalState::Canceled
                                        } else {
                                            JournalState::Failed
                                        },
                                        (!canceled).then(|| error.code().to_owned()),
                                        None,
                                    ) {
                                        current = failed;
                                    }
                                }
                            }
                        }
                    }
                    let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut current)
                        .await;
                    return;
                }
                Ok(None) => {
                    if let Ok(current) = self.executor.load(&journal.task_id) {
                        journal = current;
                    }
                }
                Err(_) => return,
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn transfer_prepared_artifact(
        &self,
        task: &DeploymentPrepareTask,
        task_id: &str,
        deadline_at: &str,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<(), PreparedArtifactTransferError> {
        let request =
            task.artifact_upload
                .as_ref()
                .ok_or(PreparedArtifactTransferError::Configuration(
                    "artifact upload request missing",
                ))?;
        let client = self
            .artifact_transfer
            .as_ref()
            .filter(|item| item.enabled())
            .ok_or(PreparedArtifactTransferError::Configuration(
                "artifact transfer client disabled",
            ))?;
        remaining_budget(deadline_at).map_err(|_| PreparedArtifactTransferError::Deadline)?;
        let archive_path = self.executor.task_dir(task_id).join("artifact.tar");
        let limits = self.executor.staging_limits();
        let archive = client
            .prepare_archive(ArchivePreparation {
                task_id,
                authorization_id: &request.authorization_id,
                deployment_id: &task.deployment_id,
                artifact_dir: Path::new(&task.output_dir),
                archive_path: &archive_path,
                expected_release: &task.release_version,
                expected_commit: &task.commit_sha,
                expected_modules: &task.modules,
                limits: &limits,
            })
            .map_err(PreparedArtifactTransferError::Prepare)?;
        remaining_budget(deadline_at).map_err(|_| PreparedArtifactTransferError::Deadline)?;
        let lease_id = self
            .authorize_artifact_upload(archive.notice.clone(), deadline_at, outbound)
            .await?;
        match lease_id {
            Some(lease_id) => {
                let budget = remaining_budget(deadline_at)
                    .map_err(|_| PreparedArtifactTransferError::Deadline)?;
                tokio::select! {
                    result = tokio::time::timeout(budget, client.upload(&lease_id, &archive)) => {
                        result
                            .map_err(|_| PreparedArtifactTransferError::UploadTimeout)?
                            .map_err(PreparedArtifactTransferError::Upload)
                    },
                    _ = wait_for_cancel(self.executor.clone(), task_id.to_owned()) => {
                        Err(PreparedArtifactTransferError::Canceled)
                    },
                }
            }
            None => Ok(()),
        }
    }

    async fn authorize_artifact_upload(
        &self,
        notice: ArtifactPrepared,
        deadline_at: &str,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<Option<String>, PreparedArtifactTransferError> {
        let key = notice.authorization_id.clone();
        let task_id = notice.task_id.clone();
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self.artifact_authorizations.lock().await;
            match pending.entry(key.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(sender);
                }
                Entry::Occupied(_) => {
                    return Err(PreparedArtifactTransferError::Authorization(
                        "duplicate authorization request",
                    ));
                }
            }
        }
        if outbound
            .send(Message::ArtifactPrepared(notice))
            .await
            .is_err()
        {
            self.artifact_authorizations.lock().await.remove(&key);
            return Err(PreparedArtifactTransferError::Authorization(
                "control channel unavailable",
            ));
        }
        let budget = remaining_budget(deadline_at)
            .map_err(|_| PreparedArtifactTransferError::Deadline)?
            .min(Duration::from_secs(30));
        let response = tokio::select! {
            response = tokio::time::timeout(budget, receiver) => response,
            _ = wait_for_cancel(self.executor.clone(), task_id.clone()) => {
                self.artifact_authorizations.lock().await.remove(&key);
                return Err(PreparedArtifactTransferError::Canceled);
            }
        };
        if response.is_err() {
            self.artifact_authorizations.lock().await.remove(&key);
        }
        let response = response
            .map_err(|_| PreparedArtifactTransferError::Authorization("timeout"))?
            .map_err(|_| PreparedArtifactTransferError::Authorization("response channel closed"))?;
        if response.task_id != task_id || response.authorization_id != key {
            return Err(PreparedArtifactTransferError::Authorization(
                "response identity mismatch",
            ));
        }
        match (response.lease_id, response.error_code) {
            (Some(lease_id), None) => Ok(Some(lease_id)),
            (None, Some(code)) if code == "artifact_already_verified" => Ok(None),
            (None, Some(_)) => Err(PreparedArtifactTransferError::Authorization(
                "server rejected authorization",
            )),
            _ => Err(PreparedArtifactTransferError::Authorization(
                "invalid authorization response",
            )),
        }
    }

    async fn inspect(
        &self,
        dispatch: &TaskDispatch,
        task: &SystemInspectTask,
        outbound: mpsc::Sender<Message>,
    ) {
        let mut journal = match self
            .executor
            .create_task(
                &dispatch.task_id,
                &dispatch.idempotency_key,
                &dispatch.payload_digest,
            )
            .await
        {
            Ok(journal) => journal,
            Err(ExecuteError::Duplicate) => {
                let Ok(journal) = self.executor.load(&dispatch.task_id) else {
                    return;
                };
                if send_ack(&outbound, dispatch, TaskAckDisposition::Duplicate, None)
                    .await
                    .is_ok()
                {
                    replay(
                        self.executor.clone(),
                        self.event_lock.clone(),
                        journal,
                        outbound,
                    )
                    .await;
                }
                return;
            }
            Err(error) => {
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some(execute_error_code(&error)),
                )
                .await;
                return;
            }
        };
        if send_ack(&outbound, dispatch, TaskAckDisposition::Accepted, None)
            .await
            .is_err()
            || send_state(
                &self.executor,
                &self.event_lock,
                &outbound,
                &mut journal,
                TaskLifecycleState::Accepted,
            )
            .await
            .is_err()
            || send_state(
                &self.executor,
                &self.event_lock,
                &outbound,
                &mut journal,
                TaskLifecycleState::Running,
            )
            .await
            .is_err()
        {
            return;
        }
        let (state, error_code, data) = match inspect_system(task) {
            Ok(data) => (JournalState::Succeeded, None, Some(data)),
            Err(code) => (JournalState::Failed, Some(code.to_owned()), None),
        };
        let Ok(mut completed) =
            self.executor
                .complete_task(&dispatch.task_id, state, error_code, data)
        else {
            return;
        };
        let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut completed).await;
    }

    async fn refs_query(
        &self,
        dispatch: &TaskDispatch,
        task: &GitRefsQueryTask,
        outbound: mpsc::Sender<Message>,
    ) {
        let mut journal = match self
            .executor
            .create_task(
                &dispatch.task_id,
                &dispatch.idempotency_key,
                &dispatch.payload_digest,
            )
            .await
        {
            Ok(journal) => journal,
            Err(ExecuteError::Duplicate) => {
                let Ok(journal) = self.executor.load(&dispatch.task_id) else {
                    return;
                };
                if send_ack(&outbound, dispatch, TaskAckDisposition::Duplicate, None)
                    .await
                    .is_ok()
                {
                    replay(
                        self.executor.clone(),
                        self.event_lock.clone(),
                        journal,
                        outbound,
                    )
                    .await;
                }
                return;
            }
            Err(error) => {
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some(execute_error_code(&error)),
                )
                .await;
                return;
            }
        };
        if send_ack(&outbound, dispatch, TaskAckDisposition::Accepted, None)
            .await
            .is_err()
            || send_state(
                &self.executor,
                &self.event_lock,
                &outbound,
                &mut journal,
                TaskLifecycleState::Accepted,
            )
            .await
            .is_err()
            || send_state(
                &self.executor,
                &self.event_lock,
                &outbound,
                &mut journal,
                TaskLifecycleState::Running,
            )
            .await
            .is_err()
        {
            return;
        }
        let credential = match self
            .fetch_secret_before_deadline(
                dispatch,
                task.git_credential_lease_id.as_deref(),
                &outbound,
            )
            .await
        {
            Ok(credential) => credential,
            Err(code) => {
                let Ok(mut completed) = self.executor.complete_task(
                    &dispatch.task_id,
                    JournalState::Failed,
                    Some(code),
                    None,
                ) else {
                    return;
                };
                let _ =
                    send_result(&self.executor, &self.event_lock, &outbound, &mut completed).await;
                return;
            }
        };
        let Ok(mut completed) = self
            .executor
            .run_refs_query(&dispatch.task_id, task, credential)
            .await
        else {
            self.executor.cleanup_secret(&dispatch.task_id);
            return;
        };
        let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut completed).await;
    }

    async fn prepare(
        &self,
        dispatch: &TaskDispatch,
        task: &DeploymentPrepareTask,
        outbound: mpsc::Sender<Message>,
    ) {
        if self.executor.load(&dispatch.task_id).is_ok() {
            if task.artifact_upload.is_some()
                && let Ok(journal) = self.executor.load(&dispatch.task_id)
                && journal.result_sequence.is_none()
            {
                self.monitor_prepare_transfer(
                    journal,
                    task.clone(),
                    dispatch.deadline_at.clone(),
                    outbound,
                )
                .await;
                return;
            }
            self.handle_existing(dispatch, outbound).await;
            return;
        }
        let credential = match self
            .fetch_secret_before_deadline(
                dispatch,
                task.git_credential_lease_id.as_deref(),
                &outbound,
            )
            .await
        {
            Ok(credential) => credential,
            Err(code) => {
                self.executor.cleanup_secret(&dispatch.task_id);
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some(&code),
                )
                .await;
                return;
            }
        };
        let mut effective = task.clone();
        effective.timeout_seconds =
            match remaining_timeout_seconds(&dispatch.deadline_at, task.timeout_seconds) {
                Ok(timeout) => timeout,
                Err(_) => {
                    self.executor.cleanup_secret(&dispatch.task_id);
                    let _ = send_ack(
                        &outbound,
                        dispatch,
                        TaskAckDisposition::Rejected,
                        Some("deadline_expired"),
                    )
                    .await;
                    return;
                }
            };
        match self
            .executor
            .execute_prepare(
                &dispatch.task_id,
                &dispatch.idempotency_key,
                &dispatch.payload_digest,
                &effective,
                credential,
            )
            .await
        {
            Ok(mut journal) => {
                if send_ack(&outbound, dispatch, TaskAckDisposition::Accepted, None)
                    .await
                    .is_err()
                {
                    return;
                }
                if send_state(
                    &self.executor,
                    &self.event_lock,
                    &outbound,
                    &mut journal,
                    TaskLifecycleState::Accepted,
                )
                .await
                .is_err()
                {
                    return;
                }
                if send_state(
                    &self.executor,
                    &self.event_lock,
                    &outbound,
                    &mut journal,
                    TaskLifecycleState::Running,
                )
                .await
                .is_err()
                {
                    return;
                }
                if task.artifact_upload.is_some() {
                    self.monitor_prepare_transfer(
                        journal,
                        task.clone(),
                        dispatch.deadline_at.clone(),
                        outbound,
                    )
                    .await;
                } else {
                    monitor(
                        self.executor.clone(),
                        self.event_lock.clone(),
                        journal,
                        outbound,
                    )
                    .await;
                }
            }
            Err(ExecuteError::Duplicate) => {
                self.handle_existing(dispatch, outbound).await;
            }
            Err(error) => {
                self.executor.cleanup_secret(&dispatch.task_id);
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some(execute_error_code(&error)),
                )
                .await;
            }
        }
    }

    async fn release(
        &self,
        dispatch: &TaskDispatch,
        task: &DeploymentReleaseTask,
        outbound: mpsc::Sender<Message>,
    ) {
        if !task.required_env.is_empty() {
            let (Some(store), Some(application_slug)) =
                (self.env_store.clone(), task.application_slug.clone())
            else {
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some("env_gate_failed"),
                )
                .await;
                return;
            };
            let required = task.required_env.clone();
            let verified = tokio::task::spawn_blocking(move || {
                required.iter().all(|item| match item.action {
                    EnvSyncAction::Write => store
                        .verify(&application_slug, &item.file_name, &item.digest)
                        .is_ok(),
                    EnvSyncAction::Delete => store
                        .verify_absent(&application_slug, &item.file_name)
                        .is_ok(),
                })
            })
            .await
            .unwrap_or(false);
            if !verified {
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some("env_gate_failed"),
                )
                .await;
                return;
            }
        }
        if task.privileged && task.artifact_download.is_none() {
            let _ = send_ack(
                &outbound,
                dispatch,
                TaskAckDisposition::Rejected,
                Some("privileged_release_artifact_required"),
            )
            .await;
            return;
        }
        if task.artifact_download.is_none() {
            self.release_legacy(dispatch, task, outbound).await;
            return;
        }
        if self
            .executor
            .validate_cross_node_release_payload(task)
            .is_err()
        {
            let _ = send_ack(
                &outbound,
                dispatch,
                TaskAckDisposition::Rejected,
                Some("invalid_release_paths"),
            )
            .await;
            return;
        }
        let mut journal = match self
            .executor
            .create_transfer_task(
                &dispatch.task_id,
                &dispatch.idempotency_key,
                &dispatch.payload_digest,
                crate::journal::TransferPhase::ReleaseDownload,
            )
            .await
        {
            Ok(journal) => journal,
            Err(ExecuteError::Duplicate) => {
                self.resume_existing_release(dispatch, task, outbound).await;
                return;
            }
            Err(error) => {
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some(execute_error_code(&error)),
                )
                .await;
                return;
            }
        };
        if send_ack(&outbound, dispatch, TaskAckDisposition::Accepted, None)
            .await
            .is_err()
            || send_state(
                &self.executor,
                &self.event_lock,
                &outbound,
                &mut journal,
                TaskLifecycleState::Accepted,
            )
            .await
            .is_err()
            || send_state(
                &self.executor,
                &self.event_lock,
                &outbound,
                &mut journal,
                TaskLifecycleState::Running,
            )
            .await
            .is_err()
        {
            return;
        }
        self.resume_cross_node_release(dispatch, task, journal, outbound)
            .await;
    }

    async fn release_legacy(
        &self,
        dispatch: &TaskDispatch,
        task: &DeploymentReleaseTask,
        outbound: mpsc::Sender<Message>,
    ) {
        let environment_directory = if self.executor.load(&dispatch.task_id).is_ok() {
            None
        } else {
            match self.materialize_release_env(dispatch, task).await {
                Ok(path) => path,
                Err(code) => {
                    self.executor.cleanup_secret(&dispatch.task_id);
                    let _ = send_ack(
                        &outbound,
                        dispatch,
                        TaskAckDisposition::Rejected,
                        Some(&code),
                    )
                    .await;
                    return;
                }
            }
        };
        match self
            .executor
            .execute_release(
                &dispatch.task_id,
                &dispatch.idempotency_key,
                &dispatch.payload_digest,
                task,
                environment_directory,
            )
            .await
        {
            Ok(mut journal) => {
                if send_ack(&outbound, dispatch, TaskAckDisposition::Accepted, None)
                    .await
                    .is_err()
                {
                    return;
                }
                if send_state(
                    &self.executor,
                    &self.event_lock,
                    &outbound,
                    &mut journal,
                    TaskLifecycleState::Accepted,
                )
                .await
                .is_err()
                    || send_state(
                        &self.executor,
                        &self.event_lock,
                        &outbound,
                        &mut journal,
                        TaskLifecycleState::Running,
                    )
                    .await
                    .is_err()
                {
                    return;
                }
                monitor(
                    self.executor.clone(),
                    self.event_lock.clone(),
                    journal,
                    outbound,
                )
                .await;
            }
            Err(ExecuteError::Duplicate) => {
                self.handle_existing(dispatch, outbound).await;
            }
            Err(error) => {
                self.executor.cleanup_secret(&dispatch.task_id);
                let _ = send_ack(
                    &outbound,
                    dispatch,
                    TaskAckDisposition::Rejected,
                    Some(execute_error_code(&error)),
                )
                .await;
            }
        }
    }

    async fn resume_existing_release(
        &self,
        dispatch: &TaskDispatch,
        task: &DeploymentReleaseTask,
        outbound: mpsc::Sender<Message>,
    ) {
        let Ok(journal) = self.executor.load(&dispatch.task_id) else {
            return;
        };
        if journal.payload_digest != dispatch.payload_digest {
            let _ = send_ack(
                &outbound,
                dispatch,
                TaskAckDisposition::Rejected,
                Some("payload_conflict"),
            )
            .await;
            return;
        }
        if send_ack(&outbound, dispatch, TaskAckDisposition::Duplicate, None)
            .await
            .is_err()
        {
            return;
        }
        if terminal(&journal.state) || journal.pid.is_some() {
            replay(
                self.executor.clone(),
                self.event_lock.clone(),
                journal,
                outbound,
            )
            .await;
        } else {
            self.resume_cross_node_release(dispatch, task, journal, outbound)
                .await;
        }
    }

    async fn resume_cross_node_release(
        &self,
        dispatch: &TaskDispatch,
        task: &DeploymentReleaseTask,
        journal: TaskJournal,
        outbound: mpsc::Sender<Message>,
    ) {
        let lock = self.transfer_lock(&dispatch.task_id).await;
        let _guard = lock.lock().await;
        let Ok(current) = self.executor.load(&dispatch.task_id) else {
            return;
        };
        if terminal(&current.state) || current.pid.is_some() {
            replay(
                self.executor.clone(),
                self.event_lock.clone(),
                current,
                outbound,
            )
            .await;
            return;
        }
        if current.transfer_phase == Some(crate::journal::TransferPhase::PrivilegedRelease) {
            if let Some(client) = self.privileged_release_executor.clone() {
                monitor_privileged_release(
                    client,
                    self.executor.clone(),
                    self.event_lock.clone(),
                    dispatch.task_id.clone(),
                    dispatch.payload_digest.clone(),
                    task.clone(),
                    outbound,
                )
                .await;
            }
            return;
        }
        let result = self
            .prepare_cross_node_release(dispatch, task, &outbound)
            .await;
        let credential = match result {
            Ok(credential) => credential,
            Err(code) => {
                let canceled = self.executor.is_cancel_requested(&journal.task_id);
                if let Ok(mut failed) = self.executor.complete_task(
                    &journal.task_id,
                    if canceled {
                        JournalState::Canceled
                    } else {
                        JournalState::Failed
                    },
                    (!canceled).then_some(code),
                    None,
                ) {
                    let _ =
                        send_result(&self.executor, &self.event_lock, &outbound, &mut failed).await;
                }
                return;
            }
        };
        if self.executor.is_cancel_requested(&dispatch.task_id) {
            if let Ok(mut canceled) =
                self.executor
                    .complete_task(&dispatch.task_id, JournalState::Canceled, None, None)
            {
                let _ =
                    send_result(&self.executor, &self.event_lock, &outbound, &mut canceled).await;
            }
            return;
        }
        let mut effective = derived_release_task(task, &self.executor.task_dir(&dispatch.task_id));
        effective.timeout_seconds =
            match remaining_timeout_seconds(&dispatch.deadline_at, task.timeout_seconds) {
                Ok(timeout) => timeout,
                Err(_) => {
                    self.executor.cleanup_secret(&dispatch.task_id);
                    if let Ok(mut failed) = self.executor.complete_task(
                        &dispatch.task_id,
                        JournalState::Failed,
                        Some("deadline_expired".to_owned()),
                        None,
                    ) {
                        let _ =
                            send_result(&self.executor, &self.event_lock, &outbound, &mut failed)
                                .await;
                    }
                    return;
                }
            };
        let environment_directory = match self.materialize_release_env(dispatch, task).await {
            Ok(path) => path,
            Err(code) => {
                if let Ok(mut failed) = self.executor.complete_task(
                    &dispatch.task_id,
                    JournalState::Failed,
                    Some(code),
                    None,
                ) {
                    let _ =
                        send_result(&self.executor, &self.event_lock, &outbound, &mut failed).await;
                }
                return;
            }
        };
        if task.privileged {
            let repository_url = match effective.repository_url.as_deref() {
                Some(value) => value,
                None => {
                    self.fail_release_task(
                        &dispatch.task_id,
                        "privileged_release_context_missing",
                        &outbound,
                    )
                    .await;
                    return;
                }
            };
            let checkout_timeout =
                match remaining_timeout_seconds(&dispatch.deadline_at, effective.timeout_seconds) {
                    Ok(value) => value,
                    Err(_) => {
                        self.fail_release_task(&dispatch.task_id, "deadline_expired", &outbound)
                            .await;
                        return;
                    }
                };
            if crate::git::checkout_commit(
                repository_url,
                &effective.commit_sha,
                Path::new(&effective.checkout_dir),
                credential.as_deref(),
                checkout_timeout,
            )
            .await
            .is_err()
            {
                self.fail_release_task(&dispatch.task_id, "git_checkout_failed", &outbound)
                    .await;
                return;
            }
            if let Err(code) = self
                .start_privileged_release(dispatch, &effective, environment_directory, &outbound)
                .await
            {
                self.fail_release_task(&dispatch.task_id, &code, &outbound)
                    .await;
            }
            return;
        }
        match self
            .executor
            .start_admitted_cross_node_release(
                &dispatch.task_id,
                &dispatch.payload_digest,
                &effective,
                credential,
                environment_directory,
            )
            .await
        {
            Ok(journal) => {
                monitor(
                    self.executor.clone(),
                    self.event_lock.clone(),
                    journal,
                    outbound,
                )
                .await;
            }
            Err(error) => {
                if let Ok(mut failed) = self.executor.complete_task(
                    &dispatch.task_id,
                    JournalState::Failed,
                    Some(execute_error_code(&error).to_owned()),
                    None,
                ) {
                    let _ =
                        send_result(&self.executor, &self.event_lock, &outbound, &mut failed).await;
                }
            }
        }
    }

    async fn prepare_cross_node_release(
        &self,
        dispatch: &TaskDispatch,
        task: &DeploymentReleaseTask,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<Option<std::path::PathBuf>, String> {
        let client = self
            .artifact_transfer
            .as_ref()
            .filter(|item| item.enabled())
            .ok_or_else(|| "cross_node_artifacts_disabled".to_owned())?;
        let download = task
            .artifact_download
            .as_ref()
            .ok_or_else(|| "artifact_download_missing".to_owned())?;
        let task_dir = self.executor.task_dir(&dispatch.task_id);
        fs::create_dir_all(&task_dir).map_err(|_| "artifact_staging_failed".to_owned())?;
        self.executor
            .set_transfer_phase(
                &dispatch.task_id,
                Some(crate::journal::TransferPhase::ReleaseDownload),
            )
            .map_err(|_| "artifact_staging_failed".to_owned())?;
        let archive_path = task_dir.join("artifact.tar");
        let budget = remaining_budget(&dispatch.deadline_at)
            .map_err(|_| "artifact_download_timeout".to_owned())?;
        tokio::select! {
            result = tokio::time::timeout(
                budget,
                client.download(&download.lease_id, &archive_path, &download.archive_digest),
            ) => result
                .map_err(|_| "artifact_download_timeout".to_owned())?
                .map_err(|_| "artifact_download_failed".to_owned())?,
            _ = wait_for_cancel(self.executor.clone(), dispatch.task_id.clone()) => {
                return Err("deployment_canceled".to_owned());
            }
        }
        remaining_budget(&dispatch.deadline_at).map_err(|_| "deadline_expired".to_owned())?;
        self.executor
            .set_transfer_phase(
                &dispatch.task_id,
                Some(crate::journal::TransferPhase::ReleaseExtract),
            )
            .map_err(|_| "artifact_staging_failed".to_owned())?;
        remaining_budget(&dispatch.deadline_at).map_err(|_| "deadline_expired".to_owned())?;
        let effective = derived_release_task(task, &task_dir);
        crate::artifact_transfer::extract_archive_atomic_verified(
            &archive_path,
            Path::new(&effective.artifact_dir),
            |temporary| {
                let mut candidate = effective.clone();
                candidate.artifact_dir = temporary.to_string_lossy().into_owned();
                verify_downloaded_artifact(&candidate, &download.manifest_digest, &self.executor)
                    .map_err(|_| crate::artifact_transfer::ArtifactTransferError::Verification)
            },
        )
        .map_err(|_| "artifact_extract_failed".to_owned())?;
        remaining_budget(&dispatch.deadline_at).map_err(|_| "deadline_expired".to_owned())?;
        let existing_key = crate::secret_lease::key_path(&task_dir);
        if existing_key.is_file() {
            return Ok(Some(existing_key));
        }
        let secret = self
            .fetch_secret_before_deadline(
                dispatch,
                task.git_credential_lease_id.as_deref(),
                outbound,
            )
            .await?;
        remaining_budget(&dispatch.deadline_at).map_err(|_| "deadline_expired".to_owned())?;
        Ok(secret)
    }

    async fn materialize_release_env(
        &self,
        dispatch: &TaskDispatch,
        task: &DeploymentReleaseTask,
    ) -> Result<Option<PathBuf>, String> {
        let files = task
            .required_env
            .iter()
            .filter(|item| item.action == EnvSyncAction::Write)
            .map(|item| (item.file_name.clone(), item.digest.clone()))
            .collect::<Vec<_>>();
        if files.is_empty() {
            return Ok(None);
        }
        let store = self
            .env_store
            .clone()
            .ok_or_else(|| "env_gate_failed".to_owned())?;
        let application_slug = task
            .application_slug
            .clone()
            .ok_or_else(|| "env_gate_failed".to_owned())?;
        let task_dir = self.executor.task_dir(&dispatch.task_id);
        tokio::task::spawn_blocking(move || store.materialize(&application_slug, &files, &task_dir))
            .await
            .map_err(|_| "env_materialization_failed".to_owned())?
            .map(Some)
            .map_err(|_| "env_materialization_failed".to_owned())
    }

    async fn fail_release_task(&self, task_id: &str, code: &str, outbound: &mpsc::Sender<Message>) {
        if let Ok(mut failed) =
            self.executor
                .complete_task(task_id, JournalState::Failed, Some(code.to_owned()), None)
        {
            let _ = send_result(&self.executor, &self.event_lock, outbound, &mut failed).await;
        }
    }

    async fn start_privileged_release(
        &self,
        dispatch: &TaskDispatch,
        task: &DeploymentReleaseTask,
        environment_directory: Option<PathBuf>,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<(), String> {
        let client = self
            .privileged_release_executor
            .clone()
            .ok_or_else(|| "privileged_release_executor_unavailable".to_owned())?;
        let context = task
            .privileged_context
            .as_ref()
            .ok_or_else(|| "privileged_release_context_missing".to_owned())?;
        let task_dir = self.executor.task_dir(&dispatch.task_id);
        let env_dir = environment_directory.unwrap_or_else(|| task_dir.join("env"));
        if !env_dir.exists() {
            fs::create_dir(&env_dir).map_err(|_| "env_materialization_failed".to_owned())?;
        }
        let checkout_dir = PathBuf::from(&task.checkout_dir);
        let artifact_dir = PathBuf::from(&task.artifact_dir);
        let facts_task = task.clone();
        let facts_context = context.clone();
        let facts_task_id = dispatch.task_id.clone();
        let facts = tokio::task::spawn_blocking(move || {
            privileged_release_facts(
                &facts_task_id,
                &facts_task,
                &facts_context,
                &checkout_dir,
                &artifact_dir,
                &env_dir,
            )
        })
        .await
        .map_err(|_| "privileged_release_admission_failed".to_owned())??;
        let authorization = self
            .request_release_authorization(facts.authorization_request.clone(), outbound)
            .await?;
        let deadline_at = chrono::DateTime::parse_from_rfc3339(&dispatch.deadline_at)
            .map_err(|_| "deadline_expired".to_owned())?
            .timestamp();
        let request = deploy_go_agent_executor::protocol::ReleaseStartRequest {
            version: deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
            job_id: facts.job_id,
            authorization,
            deployment_id: task.deployment_id.clone(),
            target_run_id: context.target_run_id.clone(),
            target_id: context.target_id.clone(),
            node_id: context.node_id.clone(),
            agent_id: context.agent_id.clone(),
            snapshot_hash: context.snapshot_hash.clone(),
            commit_sha: task.commit_sha.clone(),
            checkout_dir: task.checkout_dir.clone(),
            artifact_dir: task.artifact_dir.clone(),
            env_dir: facts.env_dir.to_string_lossy().into_owned(),
            cancel_file: facts.cancel_file.to_string_lossy().into_owned(),
            environment: environment_name(&task.environment).to_owned(),
            release_version: task.release_version.clone(),
            modules: task.modules.clone(),
            target_code: task.target_code.clone(),
            task_payload_digest: dispatch.payload_digest.clone(),
            deadline_at,
        };
        fs::write(
            task_dir.join("privileged-release-task.json"),
            serde_json::to_vec(task).map_err(|_| "privileged_release_journal_failed".to_owned())?,
        )
        .map_err(|_| "privileged_release_journal_failed".to_owned())?;
        let mut journal = self
            .executor
            .set_transfer_phase(
                &dispatch.task_id,
                Some(crate::journal::TransferPhase::PrivilegedRelease),
            )
            .map_err(|_| "privileged_release_journal_failed".to_owned())?;
        journal.external_output_sequence = 0;
        self.executor
            .store_journal(&journal)
            .map_err(|_| "privileged_release_journal_failed".to_owned())?;
        match client
            .request(deploy_go_agent_executor::protocol::Request::ReleaseStart(
                request,
            ))
            .await
        {
            Ok(deploy_go_agent_executor::protocol::Response::ReleaseStarted(_)) => {}
            Ok(deploy_go_agent_executor::protocol::Response::Error(error)) => {
                return Err(error.code);
            }
            _ => return Err("privileged_release_executor_protocol".to_owned()),
        }
        monitor_privileged_release(
            client,
            self.executor.clone(),
            self.event_lock.clone(),
            dispatch.task_id.clone(),
            dispatch.payload_digest.clone(),
            task.clone(),
            outbound.clone(),
        )
        .await;
        Ok(())
    }

    async fn request_release_authorization(
        &self,
        request: ReleaseAuthorizationRequest,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<String, String> {
        let (sender, receiver) = oneshot::channel();
        self.release_authorizations
            .lock()
            .await
            .insert(request.authorization_id.clone(), sender);
        if outbound
            .send(Message::ReleaseAuthorizationRequest(request.clone()))
            .await
            .is_err()
        {
            self.release_authorizations
                .lock()
                .await
                .remove(&request.authorization_id);
            return Err("release_authorization_request_failed".to_owned());
        }
        let response = match tokio::time::timeout(Duration::from_secs(10), receiver).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => return Err("release_authorization_failed".to_owned()),
            Err(_) => {
                self.release_authorizations
                    .lock()
                    .await
                    .remove(&request.authorization_id);
                return Err("release_authorization_timeout".to_owned());
            }
        };
        if response.task_id != request.task_id {
            return Err("release_authorization_binding_mismatch".to_owned());
        }
        response.authorization.ok_or_else(|| {
            response
                .error_code
                .unwrap_or_else(|| "release_authorization_failed".to_owned())
        })
    }

    async fn handle_existing(&self, dispatch: &TaskDispatch, outbound: mpsc::Sender<Message>) {
        let Ok(journal) = self.executor.load(&dispatch.task_id) else {
            let _ = send_ack(
                &outbound,
                dispatch,
                TaskAckDisposition::Rejected,
                Some("idempotency_conflict"),
            )
            .await;
            return;
        };
        if journal.payload_digest != dispatch.payload_digest {
            let _ = send_ack(
                &outbound,
                dispatch,
                TaskAckDisposition::Rejected,
                Some("payload_conflict"),
            )
            .await;
            return;
        }
        if send_ack(&outbound, dispatch, TaskAckDisposition::Duplicate, None)
            .await
            .is_ok()
        {
            replay(
                self.executor.clone(),
                self.event_lock.clone(),
                journal,
                outbound,
            )
            .await;
        }
    }

    async fn fetch_secret(
        &self,
        dispatch: &TaskDispatch,
        lease_id: Option<&str>,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<Option<std::path::PathBuf>, String> {
        let Some(lease_id) = lease_id else {
            return Ok(None);
        };
        self.secret_lease
            .fetch(
                &dispatch.task_id,
                lease_id,
                &dispatch.payload_digest,
                &self.executor.task_dir(&dispatch.task_id),
                outbound,
            )
            .await
            .map(Some)
            .map_err(|error| match error {
                SecretLeaseError::RequestFailed => "secret_lease_request_failed".to_owned(),
                SecretLeaseError::Timeout => "secret_lease_timeout".to_owned(),
                SecretLeaseError::Rejected(code) => code,
                SecretLeaseError::Io(_) => "secret_lease_io_error".to_owned(),
            })
    }

    async fn fetch_secret_before_deadline(
        &self,
        dispatch: &TaskDispatch,
        lease_id: Option<&str>,
        outbound: &mpsc::Sender<Message>,
    ) -> Result<Option<std::path::PathBuf>, String> {
        let budget =
            remaining_budget(&dispatch.deadline_at).map_err(|_| "deadline_expired".to_owned())?;
        tokio::time::timeout(budget, self.fetch_secret(dispatch, lease_id, outbound))
            .await
            .map_err(|_| "deadline_expired".to_owned())?
    }

    async fn cancel(&self, cancel: TaskCancel, outbound: mpsc::Sender<Message>) {
        let Ok(mut journal) = self.executor.load(&cancel.task_id) else {
            return;
        };
        if terminal(&journal.state) {
            let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut journal).await;
            return;
        }
        if self.executor.request_cancel(&cancel.task_id).await.is_err() {
            return;
        }
        if send_state(
            &self.executor,
            &self.event_lock,
            &outbound,
            &mut journal,
            TaskLifecycleState::Canceling,
        )
        .await
        .is_err()
        {
            return;
        }
        if journal.transfer_phase == Some(crate::journal::TransferPhase::PrivilegedRelease) {
            let Some(client) = self.privileged_release_executor.clone() else {
                return;
            };
            let _ = client
                .request(deploy_go_agent_executor::protocol::Request::ReleaseCancel(
                    deploy_go_agent_executor::protocol::ReleaseCancelRequest {
                        version: deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
                        job_id: format!("release_{}", cancel.task_id),
                        task_payload_digest: journal.payload_digest.clone(),
                        reason: cancel.reason,
                    },
                ))
                .await;
            return;
        }
        if let Ok(mut completed) = self.executor.cancel(&cancel.task_id).await {
            let _ =
                drain_outputs(&self.executor, &self.event_lock, &outbound, &mut completed).await;
            let _ = drain_events(&self.executor, &self.event_lock, &outbound, &mut completed).await;
            let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut completed).await;
        }
    }

    async fn reconcile(&self, task_ids: Vec<String>, outbound: mpsc::Sender<Message>) {
        let mut tasks = Vec::with_capacity(task_ids.len());
        let mut privileged = Vec::new();
        for task_id in task_ids {
            let item = match self.executor.recover(&task_id) {
                Ok(state) => {
                    let item = reconciled(state);
                    if let Ok(journal) = self.executor.load(&task_id)
                        && journal.transfer_phase
                            == Some(crate::journal::TransferPhase::PrivilegedRelease)
                        && let Ok(bytes) = fs::read(
                            self.executor
                                .task_dir(&task_id)
                                .join("privileged-release-task.json"),
                        )
                        && let Ok(task) = serde_json::from_slice::<DeploymentReleaseTask>(&bytes)
                    {
                        privileged.push((task_id.clone(), journal.payload_digest.clone(), task));
                    }
                    item
                }
                Err(_) => ReconciledTask {
                    task_id,
                    payload_digest: String::new(),
                    state: ReconciledTaskState::Unknown,
                    last_sequence: 0,
                    result: None,
                },
            };
            tasks.push(item);
        }
        let _ = outbound
            .send(Message::ReconcileReport(ReconcileReport { tasks }))
            .await;
        if let Some(client) = self.privileged_release_executor.clone() {
            for (task_id, payload_digest, task) in privileged {
                let executor = self.executor.clone();
                let event_lock = self.event_lock.clone();
                let outbound = outbound.clone();
                let client = client.clone();
                tokio::spawn(async move {
                    monitor_privileged_release(
                        client,
                        executor,
                        event_lock,
                        task_id,
                        payload_digest,
                        task,
                        outbound,
                    )
                    .await;
                });
            }
        }
    }
}

struct PrivilegedReleaseFacts {
    job_id: String,
    env_dir: PathBuf,
    cancel_file: PathBuf,
    authorization_request: ReleaseAuthorizationRequest,
}

#[derive(serde::Deserialize)]
struct PrivilegedArtifactManifest {
    artifacts: Vec<PrivilegedArtifactEntry>,
}

#[derive(serde::Deserialize)]
struct PrivilegedArtifactEntry {
    path: String,
    sha256: String,
}

fn privileged_release_facts(
    task_id: &str,
    task: &DeploymentReleaseTask,
    context: &deploy_go_agent_protocol::PrivilegedReleaseContext,
    checkout_dir: &Path,
    artifact_dir: &Path,
    env_dir: &Path,
) -> Result<PrivilegedReleaseFacts, String> {
    let manifest_path = artifact_dir.join(deploy_go_agent_executor::release::ARTIFACT_MANIFEST);
    let manifest_digest = deploy_go_agent_executor::release::file_digest(&manifest_path)
        .map_err(|_| "privileged_release_admission_failed".to_owned())?;
    let manifest: PrivilegedArtifactManifest = serde_json::from_slice(
        &fs::read(&manifest_path).map_err(|_| "privileged_release_admission_failed".to_owned())?,
    )
    .map_err(|_| "privileged_release_admission_failed".to_owned())?;
    let artifacts = manifest
        .artifacts
        .into_iter()
        .map(|item| deploy_go_agent_protocol::ReleaseFileDigest {
            relative_path: item.path,
            digest: item.sha256,
        })
        .collect::<Vec<_>>();
    let env_files = task
        .required_env
        .iter()
        .filter(|item| item.action == EnvSyncAction::Write)
        .map(|item| {
            let digest =
                deploy_go_agent_executor::release::file_digest(&env_dir.join(&item.file_name))
                    .map_err(|_| "privileged_release_admission_failed".to_owned())?;
            if !digest.eq_ignore_ascii_case(&item.digest) {
                return Err("privileged_release_admission_failed".to_owned());
            }
            Ok(deploy_go_agent_protocol::ReleaseFileDigest {
                relative_path: item.file_name.clone(),
                digest,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let task_root = checkout_dir
        .parent()
        .ok_or_else(|| "privileged_release_admission_failed".to_owned())?;
    let cancel_file = task_root.join("cancel");
    Ok(PrivilegedReleaseFacts {
        job_id: format!("release_{task_id}"),
        env_dir: env_dir.to_path_buf(),
        cancel_file: cancel_file.clone(),
        authorization_request: ReleaseAuthorizationRequest {
            task_id: task_id.to_owned(),
            authorization_id: format!("release_auth_{}", ulid::Ulid::new()),
            target_run_id: context.target_run_id.clone(),
            target_id: context.target_id.clone(),
            snapshot_hash: context.snapshot_hash.clone(),
            checkout_tree_digest: deploy_go_agent_executor::release::directory_digest(
                checkout_dir,
                true,
            )
            .map_err(|_| "privileged_release_admission_failed".to_owned())?,
            artifact_manifest_digest: manifest_digest,
            artifacts,
            env_files,
            cancel_file: cancel_file.to_string_lossy().into_owned(),
        },
    })
}

fn environment_name(environment: &deploy_go_agent_protocol::Environment) -> &'static str {
    match environment {
        deploy_go_agent_protocol::Environment::Dev => "dev",
        deploy_go_agent_protocol::Environment::Test => "test",
        deploy_go_agent_protocol::Environment::Staging => "staging",
        deploy_go_agent_protocol::Environment::Production => "prod",
    }
}

async fn monitor_privileged_release(
    client: Arc<ExecutorClient>,
    executor: Arc<Executor>,
    event_lock: Arc<Mutex<()>>,
    task_id: String,
    payload_digest: String,
    task: DeploymentReleaseTask,
    outbound: mpsc::Sender<Message>,
) {
    let job_id = format!("release_{task_id}");
    loop {
        let after_sequence = executor
            .load(&task_id)
            .map(|journal| journal.external_output_sequence)
            .unwrap_or_default();
        let output = client
            .request(deploy_go_agent_executor::protocol::Request::ReleaseOutput(
                deploy_go_agent_executor::protocol::ReleaseOutputRequest {
                    version: deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
                    job_id: job_id.clone(),
                    task_payload_digest: payload_digest.clone(),
                    after_sequence,
                    max_frames: 128,
                },
            ))
            .await;
        let Ok(deploy_go_agent_executor::protocol::Response::ReleaseOutput(batch)) = output else {
            return;
        };
        let mut journal = match executor.load(&task_id) {
            Ok(journal) => journal,
            Err(_) => return,
        };
        for frame in batch.frames {
            if frame.sequence != journal.external_output_sequence.saturating_add(1) {
                return;
            }
            let stream = match frame.stream {
                deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout => {
                    OutputStream::Stdout
                }
                deploy_go_agent_executor::protocol::ReleaseOutputStream::Stderr => {
                    OutputStream::Stderr
                }
            };
            if executor
                .persist_external_output(&task_id, frame.sequence, stream, &frame.data)
                .is_err()
            {
                return;
            }
            journal.external_output_sequence = frame.sequence;
        }
        let output_incomplete = batch.truncated;
        if executor.store_journal(&journal).is_err()
            || rebuild_privileged_events(&executor.task_dir(&task_id), &task, None).is_err()
            || drain_outputs(&executor, &event_lock, &outbound, &mut journal)
                .await
                .is_err()
            || drain_events(&executor, &event_lock, &outbound, &mut journal)
                .await
                .is_err()
        {
            return;
        }
        let status = client
            .request(deploy_go_agent_executor::protocol::Request::ReleaseStatus(
                deploy_go_agent_executor::protocol::ReleaseStatusRequest {
                    version: deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
                    job_id: job_id.clone(),
                    task_payload_digest: payload_digest.clone(),
                },
            ))
            .await;
        match status {
            Ok(deploy_go_agent_executor::protocol::Response::ReleaseStatus(_)) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Ok(deploy_go_agent_executor::protocol::Response::ReleaseExited(exited)) => {
                if output_incomplete || exited.last_sequence > journal.external_output_sequence {
                    continue;
                }
                let exit_ok =
                    exited.state == deploy_go_agent_executor::protocol::ReleaseJobState::Succeeded;
                let protocol_error =
                    rebuild_privileged_events(&executor.task_dir(&task_id), &task, Some(exit_ok))
                        .ok()
                        .flatten();
                let (state, error_code) = match exited.state {
                    deploy_go_agent_executor::protocol::ReleaseJobState::Succeeded
                        if protocol_error.is_none() =>
                    {
                        (JournalState::Succeeded, None)
                    }
                    deploy_go_agent_executor::protocol::ReleaseJobState::Canceled => {
                        (JournalState::Canceled, Some("task_canceled".to_owned()))
                    }
                    deploy_go_agent_executor::protocol::ReleaseJobState::TimedOut => {
                        (JournalState::Failed, Some("task_timeout".to_owned()))
                    }
                    _ => (
                        JournalState::Failed,
                        protocol_error.or_else(|| Some(exited.reason.clone())),
                    ),
                };
                if let Ok(mut completed) = executor.complete_task(&task_id, state, error_code, None)
                {
                    completed.exit_code = exited.exit_code;
                    let _ = executor.store_journal(&completed);
                    let _ = drain_outputs(&executor, &event_lock, &outbound, &mut completed).await;
                    let _ = drain_events(&executor, &event_lock, &outbound, &mut completed).await;
                    let _ = send_result(&executor, &event_lock, &outbound, &mut completed).await;
                }
                return;
            }
            _ => return,
        }
    }
}

fn rebuild_privileged_events(
    task_dir: &Path,
    task: &DeploymentReleaseTask,
    terminal_exit_ok: Option<bool>,
) -> Result<Option<String>, io::Error> {
    let context = crate::deploy_events::DeployEventContext {
        deploy_id: task.deployment_id.clone(),
        stage: deploy_go_agent_protocol::DeploymentStage::Release,
        environment: task.environment.clone(),
        release_version: task.release_version.clone(),
        target: Some(task.target_code.clone()),
    };
    let mut state = crate::deploy_events::MarkerState::new();
    let mut events = vec![crate::deploy_events::started_event(&context)];
    let stdout = fs::read(task_dir.join("stdout.log")).unwrap_or_default();
    let complete = if terminal_exit_ok.is_some() {
        stdout.as_slice()
    } else if let Some(index) = stdout.iter().rposition(|byte| *byte == b'\n') {
        &stdout[..index]
    } else {
        &[]
    };
    if !complete.is_empty() {
        for line in complete.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            let line = String::from_utf8_lossy(line);
            match crate::deploy_events::process_line(&line, &context, &mut state) {
                Ok(Some(event)) => events.push(event),
                Ok(None) => {}
                Err(error) => state.violations.push(error.to_string()),
            }
        }
    }
    let mut protocol_error = None;
    if let Some(exit_ok) = terminal_exit_ok {
        let (event, error) = crate::deploy_events::finished_event(&context, &state, exit_ok);
        events.push(event);
        protocol_error = error;
    }
    let mut encoded = Vec::new();
    for event in events {
        serde_json::to_writer(&mut encoded, &event).map_err(io::Error::other)?;
        encoded.push(b'\n');
    }
    fs::write(task_dir.join("events.jsonl"), encoded)?;
    Ok(protocol_error)
}

#[async_trait]
impl MessageHandler for TaskHandler {
    async fn handle(
        &self,
        envelope: Envelope,
        outbound: mpsc::Sender<Message>,
    ) -> Result<(), ConnectionError> {
        match envelope.message {
            Message::TaskDispatch(dispatch) => {
                let handler = self.clone();
                tokio::spawn(async move {
                    handler.dispatch(dispatch, outbound).await;
                });
                Ok(())
            }
            Message::TaskCancel(cancel) => {
                let handler = self.clone();
                tokio::spawn(async move {
                    handler.cancel(cancel, outbound).await;
                });
                Ok(())
            }
            Message::ReconcileRequest(request) => {
                let handler = self.clone();
                tokio::spawn(async move {
                    handler.reconcile(request.task_ids, outbound).await;
                });
                Ok(())
            }
            Message::SecretLeaseResponse(response) => {
                let broker = Arc::clone(&self.secret_lease);
                tokio::spawn(async move {
                    broker.resolve(response).await;
                });
                Ok(())
            }
            Message::ArtifactUploadAuthorized(response) => {
                let pending = Arc::clone(&self.artifact_authorizations);
                tokio::spawn(async move {
                    if let Some(sender) = pending.lock().await.remove(&response.authorization_id) {
                        let _ = sender.send(response);
                    }
                });
                Ok(())
            }
            Message::ReleaseAuthorizationResponse(response) => {
                let pending = Arc::clone(&self.release_authorizations);
                tokio::spawn(async move {
                    if let Some(sender) = pending.lock().await.remove(&response.authorization_id) {
                        let _ = sender.send(response);
                    }
                });
                Ok(())
            }
            Message::HeartbeatAck(_) | Message::ProtocolError(_) => Ok(()),
            _ => Err(ConnectionError::InvalidMessage),
        }
    }

    fn active_task_ids(&self) -> Vec<String> {
        self.executor.active_task_ids().unwrap_or_default()
    }
}

async fn monitor(
    executor: Arc<Executor>,
    event_lock: Arc<Mutex<()>>,
    mut journal: TaskJournal,
    outbound: mpsc::Sender<Message>,
) {
    loop {
        if drain_outputs(&executor, &event_lock, &outbound, &mut journal)
            .await
            .is_err()
        {
            return;
        }
        if drain_events(&executor, &event_lock, &outbound, &mut journal)
            .await
            .is_err()
        {
            return;
        }
        match executor.poll_completion(&journal.task_id) {
            Ok(Some(mut current)) => {
                let _ = drain_outputs(&executor, &event_lock, &outbound, &mut current).await;
                let _ = drain_events(&executor, &event_lock, &outbound, &mut current).await;
                let _ = send_result(&executor, &event_lock, &outbound, &mut current).await;
                return;
            }
            Ok(None) => {
                if let Ok(current) = executor.load(&journal.task_id) {
                    journal = current;
                }
            }
            Err(_) => return,
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn replay(
    executor: Arc<Executor>,
    event_lock: Arc<Mutex<()>>,
    journal: TaskJournal,
    outbound: mpsc::Sender<Message>,
) {
    if terminal(&journal.state) {
        let _ = resend_result(&outbound, &journal).await;
    } else {
        monitor(executor, event_lock, journal, outbound).await;
    }
}

async fn send_ack(
    outbound: &mpsc::Sender<Message>,
    dispatch: &TaskDispatch,
    disposition: TaskAckDisposition,
    error_code: Option<&str>,
) -> Result<(), ()> {
    outbound
        .send(Message::TaskAck(TaskAck {
            task_id: dispatch.task_id.clone(),
            payload_digest: dispatch.payload_digest.clone(),
            disposition,
            error_code: error_code.map(str::to_owned),
        }))
        .await
        .map_err(|_| ())
}

async fn send_state(
    executor: &Executor,
    event_lock: &Mutex<()>,
    outbound: &mpsc::Sender<Message>,
    journal: &mut TaskJournal,
    state: TaskLifecycleState,
) -> Result<(), ()> {
    let _guard = event_lock.lock().await;
    *journal = executor.load(&journal.task_id).map_err(|_| ())?;
    let sequence = journal.last_sequence + 1;
    outbound
        .send(Message::TaskState(TaskState {
            task_id: journal.task_id.clone(),
            sequence,
            state: state.clone(),
        }))
        .await
        .map_err(|_| ())?;
    journal.last_sequence = sequence;
    journal.state = match state {
        TaskLifecycleState::Running => JournalState::Running,
        TaskLifecycleState::Accepted => JournalState::Accepted,
        TaskLifecycleState::Canceling => journal.state.clone(),
    };
    executor.store_journal(journal).map_err(|_| ())
}

async fn drain_outputs(
    executor: &Executor,
    event_lock: &Mutex<()>,
    outbound: &mpsc::Sender<Message>,
    journal: &mut TaskJournal,
) -> Result<(), ()> {
    drain_stream(
        executor,
        event_lock,
        outbound,
        journal,
        OutputStream::Stdout,
    )
    .await?;
    drain_stream(
        executor,
        event_lock,
        outbound,
        journal,
        OutputStream::Stderr,
    )
    .await?;
    Ok(())
}

async fn drain_events(
    executor: &Executor,
    event_lock: &Mutex<()>,
    outbound: &mpsc::Sender<Message>,
    journal: &mut TaskJournal,
) -> Result<(), ()> {
    let _guard = event_lock.lock().await;
    *journal = executor.load(&journal.task_id).map_err(|_| ())?;
    let bytes = match fs::read(executor.task_dir(&journal.task_id).join("events.jsonl")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(()),
    };
    let start = usize::try_from(journal.events_offset)
        .map_err(|_| ())?
        .min(bytes.len());
    let mut cursor = start;
    for chunk in bytes[start..].split_inclusive(|byte| *byte == b'\n') {
        if !chunk.ends_with(b"\n") {
            break;
        }
        let line = &chunk[..chunk.len() - 1];
        cursor += chunk.len();
        if !line.is_empty() {
            let event: DeployEvent = serde_json::from_slice(line).map_err(|_| ())?;
            let sequence = journal.last_sequence + 1;
            outbound
                .send(Message::TaskProgress(TaskProgress {
                    task_id: journal.task_id.clone(),
                    sequence,
                    event,
                }))
                .await
                .map_err(|_| ())?;
            journal.last_sequence = sequence;
        }
        journal.events_offset = cursor as u64;
        executor.store_journal(journal).map_err(|_| ())?;
    }
    Ok(())
}

async fn drain_stream(
    executor: &Executor,
    event_lock: &Mutex<()>,
    outbound: &mpsc::Sender<Message>,
    journal: &mut TaskJournal,
    stream: OutputStream,
) -> Result<(), ()> {
    let _guard = event_lock.lock().await;
    *journal = executor.load(&journal.task_id).map_err(|_| ())?;
    let (filename, mut offset) = match stream {
        OutputStream::Stdout => ("stdout.log", journal.stdout_offset),
        OutputStream::Stderr => ("stderr.log", journal.stderr_offset),
    };
    let bytes = match fs::read(executor.task_dir(&journal.task_id).join(filename)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(()),
    };
    let start = usize::try_from(offset).map_err(|_| ())?.min(bytes.len());
    for chunk in bytes[start..].chunks(OUTPUT_CHUNK_BYTES) {
        let sequence = journal.last_sequence + 1;
        outbound
            .send(Message::TaskOutput(TaskOutput {
                task_id: journal.task_id.clone(),
                sequence,
                stream: stream.clone(),
                text: String::from_utf8_lossy(chunk).into_owned(),
            }))
            .await
            .map_err(|_| ())?;
        journal.last_sequence = sequence;
        offset += chunk.len() as u64;
        match stream {
            OutputStream::Stdout => journal.stdout_offset = offset,
            OutputStream::Stderr => journal.stderr_offset = offset,
        }
        executor.store_journal(journal).map_err(|_| ())?;
    }
    Ok(())
}

async fn send_result(
    executor: &Executor,
    event_lock: &Mutex<()>,
    outbound: &mpsc::Sender<Message>,
    journal: &mut TaskJournal,
) -> Result<(), ()> {
    let _guard = event_lock.lock().await;
    *journal = executor.load(&journal.task_id).map_err(|_| ())?;
    if journal.result_sequence.is_some() {
        return resend_result(outbound, journal).await;
    }
    let sequence = journal.last_sequence + 1;
    let result = result_for(journal, sequence);
    outbound
        .send(Message::TaskResult(result))
        .await
        .map_err(|_| ())?;
    journal.last_sequence = sequence;
    journal.result_sequence = Some(sequence);
    executor.store_journal(journal).map_err(|_| ())
}

async fn resend_result(outbound: &mpsc::Sender<Message>, journal: &TaskJournal) -> Result<(), ()> {
    outbound
        .send(Message::TaskResult(result_for(
            journal,
            journal.result_sequence.unwrap_or(journal.last_sequence),
        )))
        .await
        .map_err(|_| ())
}

fn result_for(journal: &TaskJournal, sequence: u64) -> TaskResult {
    let status = match journal.state {
        JournalState::Succeeded => TaskTerminalStatus::Succeeded,
        JournalState::Canceled => TaskTerminalStatus::Canceled,
        JournalState::Interrupted => TaskTerminalStatus::Interrupted,
        _ => TaskTerminalStatus::Failed,
    };
    TaskResult {
        task_id: journal.task_id.clone(),
        sequence,
        status,
        exit_code: journal.exit_code,
        error_code: journal.error_code.clone(),
        summary: None,
        data: journal.result_data.clone(),
    }
}

fn verify_downloaded_artifact(
    task: &DeploymentReleaseTask,
    expected_manifest_digest: &str,
    executor: &Executor,
) -> Result<(), ()> {
    let manifest =
        fs::read(Path::new(&task.artifact_dir).join("deploy-go-artifact.json")).map_err(|_| ())?;
    if format!("{:x}", Sha256::digest(&manifest)) != expected_manifest_digest {
        return Err(());
    }
    crate::staging::verify_artifact_dir(
        Path::new(&task.artifact_dir),
        &task.release_version,
        &task.commit_sha,
        &task.modules,
        &executor.staging_limits(),
    )
    .map(|_| ())
    .map_err(|_| ())
}

fn derived_release_task(task: &DeploymentReleaseTask, task_dir: &Path) -> DeploymentReleaseTask {
    let mut derived = task.clone();
    derived.work_root = task_dir.to_string_lossy().into_owned();
    derived.checkout_dir = task_dir.join("checkout").to_string_lossy().into_owned();
    derived.artifact_dir = task_dir.join("staging").to_string_lossy().into_owned();
    derived.cancel_file = task_dir.join("cancel").to_string_lossy().into_owned();
    derived
}

fn reconciled(state: RecoveryState) -> ReconciledTask {
    let journal = match state {
        RecoveryState::Accepted(journal)
        | RecoveryState::Running(journal)
        | RecoveryState::Terminal(journal)
        | RecoveryState::Interrupted(journal) => journal,
    };
    let state = if terminal(&journal.state) {
        ReconciledTaskState::Terminal
    } else if journal.transfer_phase.is_some() {
        ReconciledTaskState::Accepted
    } else if journal.state == JournalState::Running {
        ReconciledTaskState::Running
    } else {
        ReconciledTaskState::Accepted
    };
    let result = terminal(&journal.state).then(|| {
        result_for(
            &journal,
            journal.result_sequence.unwrap_or(journal.last_sequence + 1),
        )
    });
    ReconciledTask {
        task_id: journal.task_id.clone(),
        payload_digest: journal.payload_digest.clone(),
        state,
        last_sequence: journal.last_sequence,
        result,
    }
}

async fn wait_for_cancel(executor: Arc<Executor>, task_id: String) {
    while !executor.is_cancel_requested(&task_id) {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

fn remaining_budget(deadline_at: &str) -> Result<Duration, ()> {
    let deadline = chrono::DateTime::parse_from_rfc3339(deadline_at)
        .map_err(|_| ())?
        .with_timezone(&Utc);
    (deadline - Utc::now()).to_std().map_err(|_| ())
}

fn remaining_timeout_seconds(deadline_at: &str, configured: u32) -> Result<u32, ()> {
    let seconds = remaining_budget(deadline_at)?.as_secs();
    if seconds == 0 {
        return Err(());
    }
    u32::try_from(seconds.min(u64::from(configured))).map_err(|_| ())
}

fn terminal(state: &JournalState) -> bool {
    matches!(
        state,
        JournalState::Succeeded
            | JournalState::Failed
            | JournalState::Canceled
            | JournalState::Interrupted
    )
}

fn deadline_expired(deadline: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(deadline).map_or(true, |deadline| deadline <= Utc::now())
}

fn execute_error_code(error: &ExecuteError) -> &'static str {
    match error {
        ExecuteError::PayloadConflict => "payload_conflict",
        ExecuteError::UnsupportedWrapper => "unsupported_wrapper",
        ExecuteError::PathOutsideWorkRoot => "path_outside_work_root",
        ExecuteError::InaccessiblePath => "inaccessible_path",
        _ => "invalid_task",
    }
}

fn env_sync_error_code(error: EnvSyncError) -> String {
    match error {
        EnvSyncError::Disabled => "env_sync_disabled",
        EnvSyncError::InvalidIdentity => "env_sync_invalid_identity",
        EnvSyncError::DigestMismatch => "env_sync_digest_mismatch",
        EnvSyncError::UnsafeTarget => "env_sync_unsafe_target",
        EnvSyncError::Io(_) => "env_sync_io_failed",
        EnvSyncError::Transport => "env_sync_transport_failed",
        EnvSyncError::Rejected => "env_sync_lease_rejected",
    }
    .to_owned()
}

fn inspect_system(task: &SystemInspectTask) -> Result<serde_json::Value, &'static str> {
    let work_root = inspect_directory(&task.work_root).map_err(|_| "work_root_inaccessible")?;
    inspect_directory(&task.secrets_root).map_err(|_| "secrets_root_inaccessible")?;
    let filesystem =
        nix::sys::statvfs::statvfs(&work_root).map_err(|_| "disk_inspection_failed")?;
    let disk_available_bytes =
        (filesystem.blocks_available() as u64).saturating_mul(filesystem.block_size() as u64);
    let system = crate::system_info::collect();
    Ok(json!({
        "os_name": system.os,
        "architecture": system.architecture,
        "hostname": system.hostname,
        "disk_available_bytes": disk_available_bytes,
        "work_root_accessible": true,
        "secrets_root_accessible": true
    }))
}

fn inspect_directory(path: &str) -> Result<std::path::PathBuf, ()> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err(());
    }
    let canonical = fs::canonicalize(path).map_err(|_| ())?;
    if !canonical.is_dir()
        || nix::unistd::access(
            &canonical,
            nix::unistd::AccessFlags::R_OK | nix::unistd::AccessFlags::X_OK,
        )
        .is_err()
    {
        return Err(());
    }
    Ok(canonical)
}

#[cfg(test)]
mod deadline_tests {
    use super::remaining_timeout_seconds;

    #[test]
    fn consumed_budget_prevents_start_and_remaining_budget_clamps_runner_timeout() {
        let expired = (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339();
        assert!(remaining_timeout_seconds(&expired, 60).is_err());

        let short = (chrono::Utc::now() + chrono::Duration::seconds(3)).to_rfc3339();
        let timeout = remaining_timeout_seconds(&short, 60).unwrap();
        assert!((1..=3).contains(&timeout));
        assert!(timeout < 60);
    }
}

#[cfg(test)]
mod privileged_bridge_tests {
    use super::*;
    use deploy_go_agent_executor::protocol::{
        MAX_FRAME_BYTES, PROTOCOL_VERSION, ReleaseExitedResponse, ReleaseJobState,
        ReleaseOutputFrame, ReleaseOutputResponse, ReleaseOutputStream, Request, Response,
        read_request, write_message,
    };
    use tokio::net::UnixListener;

    fn bridge_task() -> DeploymentReleaseTask {
        DeploymentReleaseTask {
            deployment_id: "deployment".into(),
            target_code: "test".into(),
            work_root: "/srv/work".into(),
            checkout_dir: "/srv/work/checkout".into(),
            artifact_dir: "/srv/work/artifact".into(),
            environment: deploy_go_agent_protocol::Environment::Test,
            release_version: "release-1".into(),
            commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            modules: vec!["api".into()],
            make_target: deploy_go_agent_protocol::MakeTarget::DeployGoRelease,
            timeout_seconds: 60,
            cancel_file: "/srv/work/cancel".into(),
            privileged: true,
            privileged_context: None,
            artifact_download: None,
            repository_url: None,
            git_credential_lease_id: None,
            application_slug: None,
            required_env: Vec::new(),
        }
    }

    #[tokio::test]
    async fn executor_output_events_and_terminal_results_use_existing_state_machine() {
        for (state, exit_code, expected) in [
            (
                ReleaseJobState::Succeeded,
                Some(0),
                TaskTerminalStatus::Succeeded,
            ),
            (ReleaseJobState::Failed, Some(2), TaskTerminalStatus::Failed),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let socket = directory.path().join("executor.sock");
            let listener = UnixListener::bind(&socket).unwrap();
            let server = tokio::spawn(async move {
                let (mut output, _) = listener.accept().await.unwrap();
                assert!(matches!(
                    read_request(&mut output, MAX_FRAME_BYTES).await.unwrap(),
                    Some(Request::ReleaseOutput(_))
                ));
                write_message(
                    &mut output,
                    &Response::ReleaseOutput(ReleaseOutputResponse {
                        version: PROTOCOL_VERSION,
                        job_id: "release_task_bridge".into(),
                        frames: vec![ReleaseOutputFrame {
                            sequence: 1,
                            stream: ReleaseOutputStream::Stdout,
                            data: concat!(
                                "release log\n",
                                "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                                "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n"
                            )
                            .as_bytes()
                            .to_vec(),
                        }],
                        truncated: false,
                    }),
                    MAX_FRAME_BYTES,
                )
                .await
                .unwrap();
                let (mut status, _) = listener.accept().await.unwrap();
                assert!(matches!(
                    read_request(&mut status, MAX_FRAME_BYTES).await.unwrap(),
                    Some(Request::ReleaseStatus(_))
                ));
                write_message(
                    &mut status,
                    &Response::ReleaseExited(ReleaseExitedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: "release_task_bridge".into(),
                        state,
                        exit_code,
                        reason: if state == ReleaseJobState::Succeeded {
                            "process_exited".into()
                        } else {
                            "nonzero_exit".into()
                        },
                        last_sequence: 1,
                    }),
                    MAX_FRAME_BYTES,
                )
                .await
                .unwrap();
            });
            let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
            executor
                .create_transfer_task(
                    "task_bridge",
                    "idem_bridge_0123456789",
                    "sha256:abcdef0123456789",
                    crate::journal::TransferPhase::PrivilegedRelease,
                )
                .await
                .unwrap();
            let task = bridge_task();
            let (outbound, mut received) = mpsc::channel(32);
            monitor_privileged_release(
                Arc::new(ExecutorClient::new(socket)),
                executor,
                Arc::new(Mutex::new(())),
                "task_bridge".into(),
                "sha256:abcdef0123456789".into(),
                task,
                outbound,
            )
            .await;
            server.await.unwrap();
            let mut messages = Vec::new();
            while let Ok(message) = received.try_recv() {
                messages.push(message);
            }
            assert!(messages.iter().any(|message| matches!(message, Message::TaskOutput(output) if output.text.contains("release log"))));
            assert!(
                messages
                    .iter()
                    .filter(|message| matches!(message, Message::TaskProgress(_)))
                    .count()
                    >= 3,
                "{messages:?}"
            );
            let result = messages
                .iter()
                .find_map(|message| match message {
                    Message::TaskResult(result) => Some(result),
                    _ => None,
                })
                .unwrap();
            assert_eq!(result.status, expected);
            assert_eq!(result.exit_code, exit_code);
        }
    }

    #[tokio::test]
    async fn terminal_status_waits_until_all_paginated_executor_output_is_persisted() {
        const FRAME_COUNT: u64 = 300;
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = tokio::spawn(async move {
            let mut next_sequence = 1_u64;
            while next_sequence <= FRAME_COUNT {
                let (mut output, _) = listener.accept().await.unwrap();
                let request = read_request(&mut output, MAX_FRAME_BYTES)
                    .await
                    .unwrap()
                    .unwrap();
                let Request::ReleaseOutput(request) = request else {
                    panic!("expected release output request");
                };
                assert_eq!(request.after_sequence, next_sequence - 1);
                let end = (next_sequence + 127).min(FRAME_COUNT);
                let frames = (next_sequence..=end)
                    .map(|sequence| ReleaseOutputFrame {
                        sequence,
                        stream: ReleaseOutputStream::Stdout,
                        data: if sequence == 1 {
                            concat!(
                                "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                                "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n"
                            )
                            .as_bytes()
                            .to_vec()
                        } else {
                            format!("frame-{sequence}\n").into_bytes()
                        },
                    })
                    .collect();
                write_message(
                    &mut output,
                    &Response::ReleaseOutput(ReleaseOutputResponse {
                        version: PROTOCOL_VERSION,
                        job_id: "release_task_paginated".into(),
                        frames,
                        truncated: false,
                    }),
                    MAX_FRAME_BYTES,
                )
                .await
                .unwrap();
                next_sequence = end + 1;

                let (mut status, _) = listener.accept().await.unwrap();
                assert!(matches!(
                    read_request(&mut status, MAX_FRAME_BYTES).await.unwrap(),
                    Some(Request::ReleaseStatus(_))
                ));
                write_message(
                    &mut status,
                    &Response::ReleaseExited(ReleaseExitedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: "release_task_paginated".into(),
                        state: ReleaseJobState::Succeeded,
                        exit_code: Some(0),
                        reason: "process_exited".into(),
                        last_sequence: FRAME_COUNT,
                    }),
                    MAX_FRAME_BYTES,
                )
                .await
                .unwrap();
            }
        });
        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_paginated",
                "idem_paginated_0123456789",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let (outbound, mut received) = mpsc::channel(512);
        monitor_privileged_release(
            Arc::new(ExecutorClient::new(socket)),
            Arc::clone(&executor),
            Arc::new(Mutex::new(())),
            "task_paginated".into(),
            "sha256:abcdef0123456789".into(),
            bridge_task(),
            outbound,
        )
        .await;
        server.await.unwrap();

        let journal = executor.load("task_paginated").unwrap();
        assert_eq!(journal.external_output_sequence, FRAME_COUNT);
        assert_eq!(journal.state, JournalState::Succeeded);
        let mut result_count = 0;
        while let Ok(message) = received.try_recv() {
            if matches!(message, Message::TaskResult(_)) {
                result_count += 1;
            }
        }
        assert_eq!(result_count, 1);
    }
}
