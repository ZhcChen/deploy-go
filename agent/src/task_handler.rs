use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    io::{self, BufRead, Read},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use chrono::Utc;
use deploy_go_agent_protocol::{
    ArtifactPrepared, ArtifactUploadAuthorized, DeployEvent, DeploymentPrepareTask,
    DeploymentReleaseTask, DeploymentStage, EnvSyncAction, EnvSyncTask, Envelope, GitRefsQueryTask,
    Message, OutputStream, RESERVED_WORKSPACE_ARTIFACT, RESERVED_WORKSPACE_MODULE, ReconcileReport,
    ReconciledTask, ReconciledTaskState, ReleaseAuthorizationRequest, ReleaseAuthorizationResponse,
    ReleaseCheckoutMode, RuntimeProbeTask, RuntimeProbeType, SecretEnvironmentAuthorization,
    SourcePolicy, SystemInspectTask, TaskAck, TaskAckDisposition, TaskCancel, TaskDispatch,
    TaskLifecycleState, TaskOutput, TaskPayload, TaskProgress, TaskResult, TaskState,
    TaskTerminalStatus,
};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, mpsc, oneshot};
use zeroize::Zeroizing;

use crate::{
    agent_upgrade,
    artifact_transfer::{ArchivePreparation, ArtifactTransferClient, ArtifactTransferError},
    connection::{ConnectionError, MessageHandler},
    env_sync::{EnvFileStore, EnvSecretClient, EnvSyncError},
    executor::{ExecuteError, Executor, execute_error_code},
    executor_client::{ExecutorClient, ExecutorClientError},
    journal::{JournalError, JournalState, RecoveryState, TaskJournal},
    secret_lease::{SecretLeaseBroker, SecretLeaseError},
};

const OUTPUT_CHUNK_BYTES: usize = 32 * 1024;
const PLATFORM_CHECKOUT_MAX_FILE_BYTES: u64 = 128 * 1024;
const PLATFORM_CHECKOUT_MAX_ARCHIVE_FILES: usize = 64;
const PLATFORM_CHECKOUT_MAX_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024;

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
            Self::Upload(error)
                if error
                    .upload_failure()
                    .is_some_and(|failure| failure.category == "timeout") =>
            {
                "artifact_transfer_timeout"
            }
            Self::Upload(_) => "artifact_transfer_failed",
            Self::UploadTimeout => "artifact_transfer_timeout",
            Self::Canceled => "task_canceled",
        }
    }

    fn detail(&self) -> String {
        match self {
            Self::Configuration(detail) | Self::Authorization(detail) => (*detail).to_owned(),
            Self::Upload(error) if error.upload_failure().is_some() => error.to_string(),
            Self::Prepare(_) => "artifact 本地打包或校验失败".to_owned(),
            Self::Upload(_) => "artifact 上传失败（本地 IO 或进度校验）".to_owned(),
            Self::Deadline => "任务截止时间已到".to_owned(),
            Self::UploadTimeout => "artifact 上传超时".to_owned(),
            Self::Canceled => "任务已取消".to_owned(),
        }
    }

    fn data(&self, elapsed_ms: u64) -> serde_json::Value {
        let detail = match self {
            Self::Upload(error) if error.upload_failure().is_some() => {
                serde_json::to_value(error.upload_failure().unwrap()).expect("固定诊断结构可序列化")
            }
            _ => {
                let (stage, category) = match self {
                    Self::Configuration(_) => ("configuration", "unavailable"),
                    Self::Prepare(ArtifactTransferError::Io(_)) => ("archive_prepare", "io"),
                    Self::Prepare(_) => ("archive_prepare", "verification"),
                    Self::Authorization("timeout") => ("upload_authorization", "timeout"),
                    Self::Authorization(_) => ("upload_authorization", "authorization"),
                    Self::Deadline => ("transfer", "deadline"),
                    Self::UploadTimeout => ("transfer", "timeout"),
                    Self::Canceled => ("transfer", "canceled"),
                    Self::Upload(ArtifactTransferError::Io(_)) => ("transfer", "io"),
                    Self::Upload(_) => ("transfer", "invalid_response"),
                };
                json!({"stage":stage,"category":category,"http_status":null,
                    "attempts":null,"elapsed_ms":elapsed_ms,"confirmed_offset":null,"cause":null})
            }
        };
        json!({"artifact_transfer":detail,"transfer_elapsed_ms":elapsed_ms})
    }
}

#[derive(Clone)]
pub struct TaskHandler {
    receipt_sender:
        Arc<std::sync::OnceLock<mpsc::Sender<deploy_go_agent_protocol::TaskEventReceipt>>>,
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
    monitor_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    privileged_monitor_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    privileged_release_executor: Option<Arc<ExecutorClient>>,
    runtime_probe_client: Option<reqwest::Client>,
    agent_upgrade_client: Option<reqwest::Client>,
    agent_upgrade_control_url: Option<url::Url>,
    agent_upgrade_data_dir: Option<std::path::PathBuf>,
}

impl TaskHandler {
    pub fn new(executor: Executor) -> Self {
        Self {
            receipt_sender: Default::default(),
            executor: Arc::new(executor),
            event_lock: Arc::new(Mutex::new(())),
            secret_lease: Arc::new(SecretLeaseBroker::new()),
            artifact_transfer: None,
            env_secret: None,
            env_store: None,
            artifact_authorizations: Arc::new(Mutex::new(HashMap::new())),
            release_authorizations: Arc::new(Mutex::new(HashMap::new())),
            transfer_locks: Arc::new(Mutex::new(HashMap::new())),
            monitor_locks: Arc::new(Mutex::new(HashMap::new())),
            privileged_monitor_locks: Arc::new(Mutex::new(HashMap::new())),
            privileged_release_executor: None,
            runtime_probe_client: None,
            agent_upgrade_client: None,
            agent_upgrade_control_url: None,
            agent_upgrade_data_dir: None,
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

    pub fn with_runtime_probe_client(mut self, client: reqwest::Client) -> Self {
        self.runtime_probe_client = Some(client);
        self
    }

    pub fn with_agent_upgrade(
        mut self,
        control_url: url::Url,
        data_dir: std::path::PathBuf,
        client: reqwest::Client,
    ) -> Self {
        self.agent_upgrade_control_url = Some(control_url);
        self.agent_upgrade_data_dir = Some(data_dir);
        self.agent_upgrade_client = Some(client);
        self
    }

    async fn transfer_lock(&self, task_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.transfer_locks.lock().await;
        locks
            .entry(task_id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn privileged_monitor_lock(&self, task_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.privileged_monitor_locks.lock().await;
        locks
            .entry(task_id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn monitor_lock(&self, task_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.monitor_locks.lock().await;
        locks
            .entry(task_id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn run_monitor(&self, journal: TaskJournal, outbound: mpsc::Sender<Message>) {
        let lock = self.monitor_lock(&journal.task_id).await;
        let _guard = lock.lock().await;
        let journal = self.executor.load(&journal.task_id).unwrap_or(journal);
        if terminal(&journal.state) {
            let mut journal = journal;
            let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut journal).await;
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

    async fn replay(&self, journal: TaskJournal, outbound: mpsc::Sender<Message>) {
        if terminal(&journal.state) {
            let mut journal = journal;
            let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut journal).await;
        } else {
            self.run_monitor(journal, outbound).await;
        }
    }

    async fn run_privileged_release_monitor(
        &self,
        task_id: String,
        payload_digest: String,
        task: Option<DeploymentReleaseTask>,
        outbound: mpsc::Sender<Message>,
    ) {
        let Some(client) = self.privileged_release_executor.clone() else {
            log_privileged_release_diagnostic(
                &task_id,
                &format!("release_{task_id}"),
                &task,
                None,
                "monitor_claim",
                "executor_unavailable",
                None,
                None,
                "retain_durable_task",
                false,
            );
            return;
        };
        let lock = self.privileged_monitor_lock(&task_id).await;
        let _guard = lock.lock().await;
        monitor_privileged_release(
            client,
            self.executor.clone(),
            self.event_lock.clone(),
            task_id,
            payload_digest,
            task,
            outbound,
        )
        .await;
    }

    /// reconcile 与取消恢复共用的特权 release 恢复入口，必须 spawn 调用，不能在对账批处理里等待。
    ///
    /// 先等待同一任务既有 monitor owner 退出（旧连接关闭后它会自行退出），再在持有所有权的前提下
    /// 重读 journal、载入并回填 descriptor，最后续接原 job。descriptor 暂时读不到只退避重试，
    /// 不能放弃监控，否则任务会停在 running 直到下一次重连才有人接管。
    async fn recover_privileged_release_monitor(
        &self,
        task_id: String,
        outbound: mpsc::Sender<Message>,
    ) {
        let job_id = format!("release_{task_id}");
        let Some(client) = self.privileged_release_executor.clone() else {
            log_privileged_release_diagnostic(
                &task_id,
                &job_id,
                &None,
                None,
                "monitor_claim",
                "executor_unavailable",
                None,
                None,
                "retain_durable_task",
                false,
            );
            return;
        };
        let lock = self.privileged_monitor_lock(&task_id).await;
        let _owner = lock.lock().await;
        let mut consecutive_errors = 0_u32;
        loop {
            if outbound.is_closed() {
                return;
            }
            let latest = match self.executor.load(&task_id) {
                Ok(journal) => journal,
                Err(error) if unrecoverable_journal_error(&error) => {
                    log_privileged_release_diagnostic(
                        &task_id,
                        &job_id,
                        &None,
                        None,
                        "journal_read",
                        "journal_unavailable",
                        None,
                        None,
                        "fail_closed_unknown",
                        true,
                    );
                    return;
                }
                Err(_) => {
                    if !retry_privileged_release_monitor(
                        &task_id,
                        &job_id,
                        &None,
                        None,
                        &outbound,
                        &mut consecutive_errors,
                        "journal_read",
                        "local_persistence",
                        None,
                        None,
                        "retry_same_job",
                    )
                    .await
                    {
                        return;
                    }
                    continue;
                }
            };
            // 已终态的任务不需要 descriptor：monitor 会按 durable journal 重放同一个 TaskResult。
            let task = if terminal(&latest.state) {
                None
            } else {
                match self.load_privileged_release_descriptor(&latest, &task_id) {
                    PrivilegedReleaseDescriptorState::Valid(task) => Some(*task),
                    PrivilegedReleaseDescriptorState::Unavailable => None,
                    PrivilegedReleaseDescriptorState::RetryLater => {
                        if !retry_privileged_release_monitor(
                            &task_id,
                            &job_id,
                            &None,
                            Some(&latest),
                            &outbound,
                            &mut consecutive_errors,
                            "descriptor_read",
                            "local_persistence",
                            None,
                            None,
                            "retry_same_job",
                        )
                        .await
                        {
                            return;
                        }
                        continue;
                    }
                }
            };
            monitor_privileged_release(
                client,
                self.executor.clone(),
                self.event_lock.clone(),
                task_id,
                latest.payload_digest.clone(),
                task,
                outbound,
            )
            .await;
            return;
        }
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
            TaskPayload::RuntimeProbe(task) => {
                self.runtime_probe(&dispatch, task, outbound).await;
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
                self.run_monitor(journal, outbound).await;
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
                    self.replay(journal, outbound).await;
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
            if terminal(&current.state) && !resume_prepare_transfer(&current) {
                self.replay(current, outbound).await;
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
                            let transfer_started = std::time::Instant::now();
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
                                        Some(json!({"transfer_elapsed_ms":transfer_started.elapsed().as_millis() as u64})),
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
                                        Some(error.data(transfer_started.elapsed().as_millis() as u64)),
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
        Self::add_workspace_artifact(task).map_err(|_| {
            PreparedArtifactTransferError::Configuration("workspace_artifact_pack_failed")
        })?;
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
                    result = tokio::time::timeout(budget + Duration::from_millis(100), client.upload_with_budget(&lease_id, &archive, budget)) => {
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

    fn add_workspace_artifact(task: &DeploymentPrepareTask) -> Result<(), ()> {
        if task.source_policy != SourcePolicy::Workspace {
            return Ok(());
        }
        let output_dir = Path::new(&task.output_dir);
        let checkout_dir = Path::new(&task.checkout_dir);
        if !output_dir.is_dir() || !checkout_dir.is_dir() {
            return Err(());
        }
        let archive_path = output_dir.join(RESERVED_WORKSPACE_ARTIFACT);
        let temporary = archive_path.with_extension("tmp");
        let result = (|| -> Result<(), ()> {
            let file = fs::File::create(&temporary).map_err(|_| ())?;
            let mut encoder = GzEncoder::new(file, Compression::default());
            {
                let mut builder = tar::Builder::new(&mut encoder);
                Self::append_workspace_tree(&mut builder, checkout_dir, Path::new(""))?;
                builder.finish().map_err(|_| ())?;
            }
            let file = encoder.finish().map_err(|_| ())?;
            file.sync_all().map_err(|_| ())?;
            if archive_path.exists() {
                fs::remove_file(&archive_path).map_err(|_| ())?;
            }
            fs::rename(&temporary, &archive_path).map_err(|_| ())?;
            let metadata = fs::metadata(&archive_path).map_err(|_| ())?;
            let manifest_path = output_dir.join("deploy-go-artifact.json");
            let bytes = fs::read(&manifest_path).map_err(|_| ())?;
            let mut manifest: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
            let artifacts = manifest["artifacts"].as_array_mut().ok_or(())?;
            let entry_path = RESERVED_WORKSPACE_ARTIFACT;
            artifacts.retain(|entry| {
                entry.get("path").and_then(serde_json::Value::as_str) != Some(entry_path)
            });
            artifacts.push(json!({
                "module": RESERVED_WORKSPACE_MODULE,
                "path": entry_path,
                "sha256": Self::sha256_file(&archive_path)?,
                "size": metadata.len(),
            }));
            fs::write(
                &manifest_path,
                serde_json::to_vec(&manifest).map_err(|_| ())?,
            )
            .map_err(|_| ())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn append_workspace_tree(
        builder: &mut tar::Builder<&mut GzEncoder<fs::File>>,
        current: &Path,
        relative: &Path,
    ) -> Result<(), ()> {
        for entry in fs::read_dir(current).map_err(|_| ())? {
            let entry = entry.map_err(|_| ())?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|_| ())?;
            if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
                return Err(());
            }
            #[cfg(unix)]
            if metadata.is_file() {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() != 1 {
                    return Err(());
                }
            }
            let child_relative = relative.join(entry.file_name());
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o7777
            };
            #[cfg(not(unix))]
            let mode = 0o644;
            if metadata.is_dir() {
                let mut header = tar::Header::new_gnu();
                header.set_entry_type(tar::EntryType::Directory);
                header.set_mode(mode);
                header.set_size(0);
                header.set_cksum();
                builder
                    .append_data(&mut header, &child_relative, io::empty())
                    .map_err(|_| ())?;
                Self::append_workspace_tree(builder, &path, &child_relative)?;
            } else {
                let mut input = fs::File::open(&path).map_err(|_| ())?;
                let mut header = tar::Header::new_gnu();
                header.set_size(metadata.len());
                header.set_mode(mode);
                header.set_cksum();
                builder
                    .append_data(&mut header, &child_relative, &mut input)
                    .map_err(|_| ())?;
            }
        }
        Ok(())
    }

    fn sha256_file(path: &Path) -> Result<String, ()> {
        let mut input = fs::File::open(path).map_err(|_| ())?;
        let mut hasher = Sha256::new();
        io::copy(&mut input, &mut hasher).map_err(|_| ())?;
        Ok(format!("{:x}", hasher.finalize()))
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
                    self.replay(journal, outbound).await;
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

    async fn runtime_probe(
        &self,
        dispatch: &TaskDispatch,
        task: &RuntimeProbeTask,
        outbound: mpsc::Sender<Message>,
    ) {
        if !task.validate() {
            let _ = send_ack(
                &outbound,
                dispatch,
                TaskAckDisposition::Rejected,
                Some("invalid_runtime_probe_payload"),
            )
            .await;
            return;
        }
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
                    self.replay(journal, outbound).await;
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
        let (state, error_code, data) =
            match perform_runtime_probe(task, self.runtime_probe_client.as_ref()).await {
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
                    self.replay(journal, outbound).await;
                }
                return;
            }
            Err(error) => {
                tracing::warn!(task_id = %dispatch.task_id, error = ?error, "分支发现任务初始化失败");
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
        let credential = if task.source_policy == SourcePolicy::Workspace {
            None
        } else {
            match self
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
                    self.run_monitor(journal, outbound).await;
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
                self.run_monitor(journal, outbound).await;
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
            self.replay(journal, outbound).await;
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
            self.replay(current, outbound).await;
            return;
        }
        if current.transfer_phase == Some(crate::journal::TransferPhase::PrivilegedRelease) {
            self.run_privileged_release_monitor(
                dispatch.task_id.clone(),
                dispatch.payload_digest.clone(),
                Some(task.clone()),
                outbound,
            )
            .await;
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
            let checkout_timeout =
                match remaining_timeout_seconds(&dispatch.deadline_at, effective.timeout_seconds) {
                    Ok(value) => value,
                    Err(_) => {
                        self.fail_release_task(&dispatch.task_id, "deadline_expired", &outbound)
                            .await;
                        return;
                    }
                };
            if task.checkout_mode == ReleaseCheckoutMode::Artifact {
                if materialize_platform_checkout(
                    Path::new(&effective.artifact_dir),
                    Path::new(&effective.checkout_dir),
                    &task.modules,
                )
                .is_err()
                {
                    self.fail_release_task(
                        &dispatch.task_id,
                        "artifact_checkout_failed",
                        &outbound,
                    )
                    .await;
                    return;
                }
            } else if task.checkout_mode == ReleaseCheckoutMode::WorkspaceArtifact {
                // WorkspaceArtifact 只消费已验证发布物中的工作区快照，不允许复用
                // prepare 留在本机的 checkout：同节点部署也由控制面下发 artifact。
                if materialize_workspace_checkout(
                    Path::new(&effective.artifact_dir),
                    Path::new(&effective.checkout_dir),
                    &self.executor.staging_limits(),
                )
                .is_err()
                {
                    self.fail_release_task(
                        &dispatch.task_id,
                        "workspace_checkout_failed",
                        &outbound,
                    )
                    .await;
                    return;
                }
            } else {
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
                if crate::git::checkout_commit_with_materialization(
                    repository_url,
                    &effective.commit_sha,
                    Path::new(&effective.checkout_dir),
                    credential.as_deref(),
                    checkout_timeout,
                    effective.source_materialization.as_ref(),
                )
                .await
                .is_err()
                {
                    self.fail_release_task(&dispatch.task_id, "git_checkout_failed", &outbound)
                        .await;
                    return;
                }
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
                self.run_monitor(journal, outbound).await;
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
        let commit = task.commit_sha.chars().take(12).collect::<String>();
        let commit = if commit.is_empty() { "-" } else { &commit };
        let _ = emit_transfer_line(
            &self.executor,
            &self.event_lock,
            outbound,
            &dispatch.task_id,
            format!(
                "[INFO] 正在下载发布物 release={} commit={commit} modules=[{}]",
                task.release_version,
                task.modules.join(", "),
            ),
        )
        .await;
        self.executor
            .set_transfer_phase(
                &dispatch.task_id,
                Some(crate::journal::TransferPhase::ReleaseDownload),
            )
            .map_err(|_| "artifact_staging_failed".to_owned())?;
        let archive_path = task_dir.join("artifact.tar");
        let budget = remaining_budget(&dispatch.deadline_at)
            .map_err(|_| "artifact_download_timeout".to_owned())?;
        let download_result = tokio::select! {
            result = tokio::time::timeout(
                budget,
                client.download(&download.lease_id, &archive_path, &download.archive_digest),
            ) => result,
            _ = wait_for_cancel(self.executor.clone(), dispatch.task_id.clone()) => {
                return Err("deployment_canceled".to_owned());
            }
        };
        let failure = match download_result {
            Ok(Ok(())) => None,
            Ok(Err(error)) => {
                let code = if matches!(error, ArtifactTransferError::Access { .. }) {
                    "artifact_download_access_failed"
                } else {
                    "artifact_download_failed"
                };
                Some((code, error.download_diagnostic()))
            }
            Err(_) => Some((
                "artifact_download_timeout",
                "stage=download category=timeout".to_owned(),
            )),
        };
        if let Some((code, diagnostic)) = failure {
            tracing::warn!(task_id = %dispatch.task_id, error_code = code, %diagnostic, "发布物下载失败");
            // 诊断先持久化；断线或发送队列背压不能阻塞任务终态落盘。
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                emit_transfer_line(
                    &self.executor,
                    &self.event_lock,
                    outbound,
                    &dispatch.task_id,
                    format!("[ERROR] 发布物下载失败 code={code} {diagnostic}"),
                ),
            )
            .await;
            return Err(code.to_owned());
        }
        remaining_budget(&dispatch.deadline_at).map_err(|_| "deadline_expired".to_owned())?;
        let downloaded_size = fs::metadata(&archive_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let _ = emit_transfer_line(
            &self.executor,
            &self.event_lock,
            outbound,
            &dispatch.task_id,
            format!("[INFO] 发布物下载完成（{downloaded_size} 字节），开始校验并解压"),
        )
        .await;
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

    /// release 准入失败时，把可对外说明的原因写入部署日志与节点诊断。
    /// 只输出 checkout 内的相对路径和条目类型，不读取、不输出任何文件内容。
    async fn report_release_admission_failure(
        &self,
        task_id: &str,
        code: &str,
        checkout_dir: &Path,
        outbound: &mpsc::Sender<Message>,
    ) {
        const MAX_REPORTED_ENTRIES: usize = 5;
        let mut lines = Vec::new();
        let mut reported = 0_usize;
        if code == RELEASE_CHECKOUT_UNSAFE_ENTRY {
            let directory = checkout_dir.to_path_buf();
            let entries = tokio::task::spawn_blocking(move || {
                deploy_go_agent_executor::release::find_unsafe_entries(
                    &directory,
                    true,
                    MAX_REPORTED_ENTRIES + 1,
                )
            })
            .await
            .unwrap_or_default();
            reported = entries.len().min(MAX_REPORTED_ENTRIES);
            lines.push(
                "[ERROR] 发布准入被拒绝：checkout 含 root 执行器不接受的条目（只允许普通文件）；请从仓库中移除后重新部署"
                    .to_owned(),
            );
            for (path, kind) in entries.iter().take(MAX_REPORTED_ENTRIES) {
                lines.push(format!("[ERROR]   {}：{}", kind.label(), path.display()));
            }
            if entries.len() > MAX_REPORTED_ENTRIES {
                lines.push("[ERROR]   …（仅列出前几项）".to_owned());
            }
        } else {
            lines.push(format!("[ERROR] 发布准入被拒绝 code={code}"));
        }
        tracing::warn!(
            diagnostic_event = "privileged_release_admission_rejected",
            task_id = %task_id,
            reason = code,
            unsafe_entries = reported,
            "release admission rejected before executor start"
        );
        for line in lines {
            let _ =
                emit_transfer_line(&self.executor, &self.event_lock, outbound, task_id, line).await;
        }
    }

    async fn fail_release_task(&self, task_id: &str, code: &str, outbound: &mpsc::Sender<Message>) {
        let canceled = self.executor.is_cancel_requested(task_id);
        if let Ok(mut failed) = self.executor.complete_task(
            task_id,
            if canceled {
                JournalState::Canceled
            } else {
                JournalState::Failed
            },
            (!canceled).then(|| code.to_owned()),
            None,
        ) {
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
        let diagnostic_checkout_dir = checkout_dir.clone();
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
        .map_err(|_| "privileged_release_admission_failed".to_owned())?;
        let facts = match facts {
            Ok(facts) => facts,
            Err(code) => {
                self.report_release_admission_failure(
                    &dispatch.task_id,
                    &code,
                    &diagnostic_checkout_dir,
                    outbound,
                )
                .await;
                return Err(code);
            }
        };
        let secret_environment = match task.secret_environment.as_ref() {
            Some(lease) if lease.descriptor.validate() => self
                .secret_lease
                .fetch_environment(&dispatch.task_id, lease, &dispatch.payload_digest, outbound)
                .await
                .map_err(|error| match error {
                    SecretLeaseError::RequestFailed => {
                        "secret_environment_request_failed".to_owned()
                    }
                    SecretLeaseError::Timeout => "secret_environment_timeout".to_owned(),
                    SecretLeaseError::Rejected(code) => code,
                    SecretLeaseError::Io(_) => "secret_environment_io_error".to_owned(),
                })?,
            Some(_) => return Err("secret_environment_descriptor_invalid".to_owned()),
            None => Vec::new(),
        };
        let secret_environment = task.secret_environment.as_ref().map(|lease| {
            let encoded =
                Zeroizing::new(serde_json::to_vec(&secret_environment).unwrap_or_default());
            let value_digest = format!("sha256:{:x}", Sha256::digest(encoded.as_slice()));
            deploy_go_agent_executor::protocol::SecretEnvironmentRequest {
                purpose: match lease.descriptor.purpose {
                    deploy_go_agent_protocol::SecretEnvironmentPurpose::EtcdInit => "etcd-init",
                    deploy_go_agent_protocol::SecretEnvironmentPurpose::ConfigCenterConnection => {
                        "config-center-connection"
                    }
                }
                .to_owned(),
                variable_names: lease.descriptor.variable_names.clone(),
                descriptor_digest: lease.descriptor.descriptor_digest.clone(),
                value_digest,
                credential_version: lease.descriptor.credential_version,
                template_id: lease.descriptor.template_id.clone(),
                template_version: lease.descriptor.template_version.clone(),
                template_digest: lease.descriptor.template_digest.clone(),
                release_stage: match lease.descriptor.release_stage {
                    deploy_go_agent_protocol::DeploymentStage::Prepare => "prepare",
                    deploy_go_agent_protocol::DeploymentStage::Release => "release",
                }
                .to_owned(),
                executor_audience: lease.descriptor.executor_audience.clone(),
                target_process: lease.descriptor.target_process.clone(),
                variables: secret_environment
                    .iter()
                    .map(
                        |variable| deploy_go_agent_executor::protocol::SecretEnvironmentValue {
                            name: variable.name.clone(),
                            value: variable.value.clone(),
                        },
                    )
                    .collect(),
            }
        });
        let mut authorization_request = facts.authorization_request;
        authorization_request.secret_environment = secret_environment
            .as_ref()
            .zip(task.secret_environment.as_ref())
            .map(|(secret, lease)| SecretEnvironmentAuthorization {
                lease_id: lease.lease_id.clone(),
                purpose: lease.descriptor.purpose,
                variable_names: lease.descriptor.variable_names.clone(),
                descriptor_digest: secret.descriptor_digest.clone(),
                value_digest: secret.value_digest.clone(),
                credential_version: secret.credential_version,
                template_id: secret.template_id.clone(),
                template_version: secret.template_version.clone(),
                template_digest: secret.template_digest.clone(),
                release_stage: match task
                    .secret_environment
                    .as_ref()
                    .map(|lease| lease.descriptor.release_stage.clone())
                {
                    Some(deploy_go_agent_protocol::DeploymentStage::Prepare) => {
                        deploy_go_agent_protocol::DeploymentStage::Prepare
                    }
                    Some(deploy_go_agent_protocol::DeploymentStage::Release) | None => {
                        deploy_go_agent_protocol::DeploymentStage::Release
                    }
                },
                executor_audience: secret.executor_audience.clone(),
                target_process: secret.target_process.clone(),
            });
        let authorization = self
            .request_release_authorization(authorization_request, outbound)
            .await?;
        let deadline_at = chrono::DateTime::parse_from_rfc3339(&dispatch.deadline_at)
            .map_err(|_| "deadline_expired".to_owned())?
            .timestamp();
        let request = deploy_go_agent_executor::protocol::ReleaseStartRequest {
            version: deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
            job_id: facts.job_id.clone(),
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
            secret_environment,
        };
        fs::write(
            task_dir.join("privileged-release-task.json"),
            serde_json::to_vec(task).map_err(|_| "privileged_release_journal_failed".to_owned())?,
        )
        .map_err(|_| "privileged_release_journal_failed".to_owned())?;
        self.executor
            .begin_privileged_release(
                &dispatch.task_id,
                &task.deployment_id,
                &context.target_run_id,
            )
            .map_err(|_| "privileged_release_journal_failed".to_owned())?;
        if self.executor.is_cancel_requested(&dispatch.task_id) {
            return Err("task_canceled".to_owned());
        }
        match request_privileged_release_start(&client, request.clone(), &facts.job_id).await {
            Ok(()) => {}
            Err(ReleaseStartFailure::Rejected(error_code)) => {
                tracing::warn!(
                    task_id = %dispatch.task_id,
                    job_id = %facts.job_id,
                    deployment_id = %task.deployment_id,
                    target_run_id = %context.target_run_id,
                    target_id = %context.target_id,
                    error_code,
                    attempt = 1,
                    "privileged release start was definitively rejected"
                );
                return Err(error_code.to_owned());
            }
            Err(ReleaseStartFailure::NotSent(error_code)) => {
                tracing::warn!(
                    task_id = %dispatch.task_id,
                    job_id = %facts.job_id,
                    deployment_id = %task.deployment_id,
                    target_run_id = %context.target_run_id,
                    target_id = %context.target_id,
                    error_code,
                    attempt = 1,
                    "privileged release start request was not sent"
                );
                return Err(error_code.to_owned());
            }
            Err(ReleaseStartFailure::Uncertain(error_code)) => {
                tracing::warn!(
                    task_id = %dispatch.task_id,
                    job_id = %facts.job_id,
                    deployment_id = %task.deployment_id,
                    target_run_id = %context.target_run_id,
                    target_id = %context.target_id,
                    error_code,
                    attempt = 1,
                    "privileged release start outcome is uncertain; retrying the same durable job"
                );
                if self.executor.is_cancel_requested(&dispatch.task_id) {
                    request_privileged_release_cancel(
                        &client,
                        &facts.job_id,
                        &dispatch.payload_digest,
                    )
                    .await;
                } else {
                    match request_privileged_release_start(&client, request, &facts.job_id).await {
                        Ok(()) => tracing::info!(
                            task_id = %dispatch.task_id,
                            job_id = %facts.job_id,
                            deployment_id = %task.deployment_id,
                            target_run_id = %context.target_run_id,
                            target_id = %context.target_id,
                            attempt = 2,
                            "privileged release start reconciled to the existing durable job"
                        ),
                        Err(failure) => tracing::warn!(
                            task_id = %dispatch.task_id,
                            job_id = %facts.job_id,
                            deployment_id = %task.deployment_id,
                            target_run_id = %context.target_run_id,
                            target_id = %context.target_id,
                            error_code = failure.error_code(),
                            attempt = 2,
                            "privileged release start remains unconfirmed; handing off to durable status monitor"
                        ),
                    }
                }
            }
        }
        if self.executor.is_cancel_requested(&dispatch.task_id) {
            request_privileged_release_cancel(&client, &facts.job_id, &dispatch.payload_digest)
                .await;
        }
        self.run_privileged_release_monitor(
            dispatch.task_id.clone(),
            dispatch.payload_digest.clone(),
            Some(task.clone()),
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
            self.replay(journal, outbound).await;
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
        let mut journal = match self.executor.load(&cancel.task_id) {
            Ok(journal) => journal,
            Err(_) => {
                tracing::error!(
                    diagnostic_scope = "task_reconcile",
                    task_id = %cancel.task_id,
                    failure_stage = "cancel_journal_load",
                    error_class = "journal_unavailable",
                    recovery_action = "unknown_no_executor_query",
                    "task journal cannot be trusted during cancellation"
                );
                return;
            }
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
                log_privileged_release_diagnostic(
                    &cancel.task_id,
                    &format!("release_{}", cancel.task_id),
                    &None,
                    Some(&journal),
                    "cancel_reconcile",
                    "executor_unavailable",
                    None,
                    None,
                    "retain_durable_task",
                    false,
                );
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
            // 取消只负责把 ReleaseCancel 送达 executor，终态仍由原 job 的 monitor 收敛。
            // 统一走恢复入口：等待既有 owner 退出，descriptor 暂时不可读时退避重试而不是放弃。
            self.recover_privileged_release_monitor(cancel.task_id.clone(), outbound.clone())
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

    fn load_privileged_release_descriptor(
        &self,
        journal: &TaskJournal,
        task_id: &str,
    ) -> PrivilegedReleaseDescriptorState {
        let path = self
            .executor
            .task_dir(task_id)
            .join("privileged-release-task.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                log_privileged_release_diagnostic(
                    task_id,
                    &format!("release_{task_id}"),
                    &None,
                    Some(journal),
                    "descriptor_read",
                    "descriptor_missing",
                    None,
                    None,
                    "observe_original_job",
                    true,
                );
                return PrivilegedReleaseDescriptorState::Unavailable;
            }
            Err(_) => {
                log_privileged_release_diagnostic(
                    task_id,
                    &format!("release_{task_id}"),
                    &None,
                    Some(journal),
                    "descriptor_read",
                    "local_io",
                    None,
                    None,
                    "retry_reconcile_later",
                    false,
                );
                return PrivilegedReleaseDescriptorState::RetryLater;
            }
        };
        let task = match serde_json::from_slice::<DeploymentReleaseTask>(&bytes) {
            Ok(task) => task,
            Err(_) => {
                log_privileged_release_diagnostic(
                    task_id,
                    &format!("release_{task_id}"),
                    &None,
                    Some(journal),
                    "descriptor_parse",
                    "descriptor_invalid",
                    None,
                    None,
                    "observe_original_job",
                    true,
                );
                return PrivilegedReleaseDescriptorState::Unavailable;
            }
        };
        let Some(target_run_id) = task
            .privileged_context
            .as_ref()
            .map(|context| context.target_run_id.as_str())
            .filter(|value| !value.is_empty())
        else {
            log_privileged_release_diagnostic(
                task_id,
                &format!("release_{task_id}"),
                &Some(task),
                Some(journal),
                "descriptor_validate",
                "association_unavailable",
                None,
                None,
                "observe_original_job",
                true,
            );
            return PrivilegedReleaseDescriptorState::Unavailable;
        };
        if !task.privileged
            || task.deployment_id.is_empty()
            || journal
                .deployment_id
                .as_deref()
                .is_some_and(|value| value != task.deployment_id.as_str())
            || journal
                .target_run_id
                .as_deref()
                .is_some_and(|value| value != target_run_id)
        {
            log_privileged_release_diagnostic(
                task_id,
                &format!("release_{task_id}"),
                &Some(task),
                Some(journal),
                "descriptor_validate",
                "association_conflict",
                None,
                None,
                "observe_original_job",
                true,
            );
            return PrivilegedReleaseDescriptorState::Unavailable;
        }
        let mut updated = journal.clone();
        if updated.deployment_id.is_none() {
            updated.deployment_id = Some(task.deployment_id.clone());
        }
        if updated.target_run_id.is_none() {
            updated.target_run_id = Some(target_run_id.to_owned());
        }
        if updated != *journal && self.executor.store_journal(&updated).is_err() {
            log_privileged_release_diagnostic(
                task_id,
                &format!("release_{task_id}"),
                &Some(task),
                Some(journal),
                "association_backfill",
                "local_persistence",
                None,
                None,
                "retry_reconcile_later",
                false,
            );
            return PrivilegedReleaseDescriptorState::RetryLater;
        }
        PrivilegedReleaseDescriptorState::Valid(Box::new(task))
    }

    async fn reconcile(&self, task_ids: Vec<String>, outbound: mpsc::Sender<Message>) {
        let mut tasks = Vec::with_capacity(task_ids.len());
        let mut privileged = Vec::new();
        let mut monitors = Vec::new();
        for task_id in task_ids {
            let item = match self.executor.recover(&task_id) {
                Ok(state) => {
                    let journal = match &state {
                        RecoveryState::Accepted(journal)
                        | RecoveryState::Running(journal)
                        | RecoveryState::Terminal(journal)
                        | RecoveryState::Interrupted(journal) => journal.clone(),
                    };
                    let resume_prepare = prepare_transfer_pending(&journal, &self.executor);
                    let should_monitor =
                        matches!(state, RecoveryState::Running(_)) && !resume_prepare;
                    let mut item = reconciled(state);
                    if self.executor.reliable_logs_enabled()
                        && crate::log_delivery::enabled(&self.executor.task_dir(&task_id))
                    {
                        item.log_delivery_version = Some(1);
                    }
                    if resume_prepare {
                        // prepare 进程可能已结束但制品尚未上传。报告 Accepted 让主控按
                        // 幂等路径重新下发任务，由 prepare() 恢复制品上传或终态回传。
                        item.state = ReconciledTaskState::Accepted;
                        item.result = None;
                    }
                    if journal.transfer_phase
                        == Some(crate::journal::TransferPhase::PrivilegedRelease)
                    {
                        // 对账批处理不能等待 monitor 所有权（它会持有到终态）。交给独立的恢复任务：
                        // 它等旧 owner 退出后在持锁状态下重读 journal、回填 descriptor 并续接原 job。
                        privileged.push(task_id.clone());
                    } else if should_monitor {
                        monitors.push(journal);
                    }
                    item
                }
                Err(_) => {
                    tracing::error!(
                        diagnostic_scope = "task_reconcile",
                        task_id = %task_id,
                        failure_stage = "journal_reconcile",
                        error_class = "journal_unavailable",
                        recovery_action = "unknown_no_executor_query",
                        "task journal cannot be trusted for recovery"
                    );
                    ReconciledTask {
                        log_delivery_version: None,
                        task_id,
                        payload_digest: String::new(),
                        state: ReconciledTaskState::Unknown,
                        last_sequence: 0,
                        result: None,
                    }
                }
            };
            tasks.push(item);
        }
        let _ = outbound
            .send(Message::ReconcileReport(ReconcileReport { tasks }))
            .await;
        // 普通任务的 runner 仍可在 Agent 重连后继续运行，需要重新挂接 monitor；
        // transfer/特权任务由各自的恢复路径接管，避免重复上传或重复启动。
        for task_id in privileged {
            let handler = self.clone();
            let outbound = outbound.clone();
            tokio::spawn(async move {
                handler
                    .recover_privileged_release_monitor(task_id, outbound)
                    .await;
            });
        }
        for journal in monitors {
            let handler = self.clone();
            let outbound = outbound.clone();
            tokio::spawn(async move {
                handler.run_monitor(journal, outbound).await;
            });
        }
    }
}

struct PrivilegedReleaseFacts {
    job_id: String,
    env_dir: PathBuf,
    cancel_file: PathBuf,
    authorization_request: ReleaseAuthorizationRequest,
}

enum PrivilegedReleaseDescriptorState {
    Valid(Box<DeploymentReleaseTask>),
    Unavailable,
    RetryLater,
}

// release 准入阶段（Agent 本地计算摘要）的稳定错误码；此前统一为
// privileged_release_admission_failed，无法区分原因。
const RELEASE_MANIFEST_INVALID: &str = "privileged_release_manifest_invalid";
const RELEASE_ENV_INVALID: &str = "privileged_release_env_invalid";
const RELEASE_ENV_MISMATCH: &str = "privileged_release_env_mismatch";
const RELEASE_CHECKOUT_UNSAFE_ENTRY: &str = "privileged_release_checkout_unsafe_entry";
const RELEASE_CHECKOUT_INVALID: &str = "privileged_release_checkout_invalid";

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
        .map_err(|_| RELEASE_MANIFEST_INVALID.to_owned())?;
    let manifest: PrivilegedArtifactManifest = serde_json::from_slice(
        &fs::read(&manifest_path).map_err(|_| RELEASE_MANIFEST_INVALID.to_owned())?,
    )
    .map_err(|_| RELEASE_MANIFEST_INVALID.to_owned())?;
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
                    .map_err(|_| RELEASE_ENV_INVALID.to_owned())?;
            if !digest.eq_ignore_ascii_case(&item.digest) {
                return Err(RELEASE_ENV_MISMATCH.to_owned());
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
            .map_err(|error| match error {
                deploy_go_agent_executor::release::ReleaseAdmissionError::UnsafeFile => {
                    RELEASE_CHECKOUT_UNSAFE_ENTRY.to_owned()
                }
                _ => RELEASE_CHECKOUT_INVALID.to_owned(),
            })?,
            artifact_manifest_digest: manifest_digest,
            artifacts,
            env_files,
            cancel_file: cancel_file.to_string_lossy().into_owned(),
            secret_environment: None,
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

fn release_executor_error_code(code: &str) -> &'static str {
    const ALLOWED: &[&str] = &[
        "incompatible_version",
        "invalid_sequence_or_version",
        "release_request_invalid",
        "release_authorization_invalid",
        "release_authorization_binding_mismatch",
        "release_authorization_replayed",
        "release_path_escape",
        "release_unsafe_file",
        "release_digest_mismatch",
        "release_job_conflict",
        "release_job_not_found",
        "release_storage_unavailable",
        "release_storage_limit_exceeded",
        "release_storage_low_disk",
        "release_spawn_failed",
        "release_recovery_blocked",
    ];
    ALLOWED
        .iter()
        .copied()
        .find(|allowed| *allowed == code)
        .unwrap_or("executor_rejected_request")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReleaseStartFailure {
    Rejected(&'static str),
    NotSent(&'static str),
    Uncertain(&'static str),
}

impl ReleaseStartFailure {
    fn error_code(self) -> &'static str {
        match self {
            Self::Rejected(code) | Self::NotSent(code) | Self::Uncertain(code) => code,
        }
    }
}

async fn request_privileged_release_start(
    client: &ExecutorClient,
    request: deploy_go_agent_executor::protocol::ReleaseStartRequest,
    expected_job_id: &str,
) -> Result<(), ReleaseStartFailure> {
    let result = client
        .request(deploy_go_agent_executor::protocol::Request::ReleaseStart(
            request,
        ))
        .await;
    if let Err(error) = &result {
        tracing::warn!(
            job_id = expected_job_id,
            error_code = error.error_code(),
            expected_executor_protocol_version = deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
            request_frame_bytes = error.request_frame_bytes().map(|value| value as u64),
            response_frame_bytes = error.response_frame_bytes().map(|value| value as u64),
            frame_limit_bytes = error.frame_limit_bytes().map(|value| value as u64),
            io_kind = ?error.io_kind(),
            "privileged release executor IPC request failed"
        );
    }
    match &result {
        Ok(deploy_go_agent_executor::protocol::Response::ReleaseStarted(response)) => {
            if let Some(code) = release_started_response_error_code(response, expected_job_id) {
                tracing::warn!(
                    job_id = expected_job_id,
                    error_code = code,
                    expected_executor_protocol_version =
                        deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
                    received_executor_protocol_version = response.version,
                    response_job_id_matches = response.job_id == expected_job_id,
                    "privileged release start acknowledgment did not match request"
                );
            }
        }
        Ok(deploy_go_agent_executor::protocol::Response::Error(response)) => {
            tracing::warn!(
                job_id = expected_job_id,
                error_code = release_executor_error_code(&response.code),
                expected_executor_protocol_version =
                    deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
                received_executor_protocol_version = response.version,
                "privileged release executor returned a start error"
            );
        }
        _ => {}
    }
    classify_release_start_result(result, expected_job_id)
}

fn classify_release_start_result(
    result: Result<deploy_go_agent_executor::protocol::Response, ExecutorClientError>,
    expected_job_id: &str,
) -> Result<(), ReleaseStartFailure> {
    use deploy_go_agent_executor::protocol::{PROTOCOL_VERSION, Response};

    match result {
        Ok(Response::ReleaseStarted(response)) => {
            if let Some(code) = release_started_response_error_code(&response, expected_job_id) {
                Err(ReleaseStartFailure::Uncertain(code))
            } else {
                Ok(())
            }
        }
        Ok(Response::Error(error)) => {
            let code = if error.version == PROTOCOL_VERSION {
                release_executor_error_code(&error.code)
            } else {
                "executor_response_version_mismatch"
            };
            if matches!(
                error.code.as_str(),
                "incompatible_version"
                    | "invalid_sequence_or_version"
                    | "release_request_invalid"
                    | "release_authorization_invalid"
                    | "release_authorization_binding_mismatch"
                    | "release_path_escape"
                    | "release_unsafe_file"
                    | "release_digest_mismatch"
                    | "release_storage_limit_exceeded"
                    | "release_storage_low_disk"
            ) && error.version == PROTOCOL_VERSION
            {
                Err(ReleaseStartFailure::Rejected(code))
            } else {
                Err(ReleaseStartFailure::Uncertain(code))
            }
        }
        Ok(_) => Err(ReleaseStartFailure::Uncertain(
            "executor_response_unexpected",
        )),
        Err(error) if error.request_may_have_been_processed() => {
            Err(ReleaseStartFailure::Uncertain(error.error_code()))
        }
        Err(error) => Err(ReleaseStartFailure::NotSent(error.error_code())),
    }
}

async fn request_privileged_release_cancel(
    client: &ExecutorClient,
    job_id: &str,
    payload_digest: &str,
) {
    let _ = client
        .request(deploy_go_agent_executor::protocol::Request::ReleaseCancel(
            deploy_go_agent_executor::protocol::ReleaseCancelRequest {
                version: deploy_go_agent_executor::protocol::PROTOCOL_VERSION,
                job_id: job_id.to_owned(),
                task_payload_digest: payload_digest.to_owned(),
                reason: "task_canceled_during_release_start".to_owned(),
            },
        ))
        .await;
}

fn release_started_response_error_code(
    response: &deploy_go_agent_executor::protocol::ReleaseStartedResponse,
    expected_job_id: &str,
) -> Option<&'static str> {
    if response.version != deploy_go_agent_executor::protocol::PROTOCOL_VERSION {
        Some("executor_response_version_mismatch")
    } else if response.job_id != expected_job_id {
        Some("executor_response_job_mismatch")
    } else {
        None
    }
}

async fn monitor_privileged_release(
    client: Arc<ExecutorClient>,
    executor: Arc<Executor>,
    event_lock: Arc<Mutex<()>>,
    task_id: String,
    payload_digest: String,
    task: Option<DeploymentReleaseTask>,
    outbound: mpsc::Sender<Message>,
) {
    let job_id = format!("release_{task_id}");
    let mut consecutive_output_errors = 0_u32;
    let mut consecutive_status_errors = 0_u32;
    let mut consecutive_local_errors = 0_u32;
    let mut consecutive_terminal_sequence_gaps = 0_u32;
    'monitor: loop {
        if outbound.is_closed() {
            return;
        }
        let task_journal = match executor.load(&task_id) {
            Ok(journal) => journal,
            Err(error) if unrecoverable_journal_error(&error) => {
                log_privileged_release_diagnostic(
                    &task_id,
                    &job_id,
                    &task,
                    None,
                    "journal_read",
                    "journal_unavailable",
                    None,
                    None,
                    "fail_closed_unknown",
                    true,
                );
                return;
            }
            Err(_) => {
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    None,
                    &outbound,
                    &mut consecutive_local_errors,
                    "journal_read",
                    "local_persistence",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue;
            }
        };
        if terminal(&task_journal.state) {
            let mut terminal_journal = task_journal;
            if send_result(&executor, &event_lock, &outbound, &mut terminal_journal)
                .await
                .is_ok()
            {
                return;
            }
            if !retry_privileged_release_monitor(
                &task_id,
                &job_id,
                &task,
                Some(&terminal_journal),
                &outbound,
                &mut consecutive_local_errors,
                "result_delivery",
                "outbox_or_delivery",
                None,
                None,
                "retry_durable_result",
            )
            .await
            {
                return;
            }
            continue;
        }
        if task_journal.payload_digest != payload_digest
            || task_journal.transfer_phase != Some(crate::journal::TransferPhase::PrivilegedRelease)
        {
            log_privileged_release_diagnostic(
                &task_id,
                &job_id,
                &task,
                Some(&task_journal),
                "journal_validate",
                "identity_or_phase_conflict",
                None,
                None,
                "fail_closed_unknown",
                true,
            );
            return;
        }
        if executor.is_cancel_requested(&task_id) {
            // admission 完成前的 cancel 可能返回 not-found；持续对同一 job 重发直到终态。
            request_privileged_release_cancel(&client, &job_id, &payload_digest).await;
        }
        let after_sequence = task_journal.external_output_sequence;
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
        let batch = match output {
            Ok(deploy_go_agent_executor::protocol::Response::ReleaseOutput(batch))
                if batch.version == deploy_go_agent_executor::protocol::PROTOCOL_VERSION
                    && batch.job_id == job_id =>
            {
                batch
            }
            Ok(deploy_go_agent_executor::protocol::Response::Error(error))
                if error.version == deploy_go_agent_executor::protocol::PROTOCOL_VERSION
                    && error.code == "release_job_conflict" =>
            {
                if complete_privileged_release_failure(
                    &executor,
                    &event_lock,
                    &outbound,
                    &task_id,
                    "release_job_conflict",
                )
                .await
                .is_ok()
                {
                    return;
                }
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&task_journal),
                    &outbound,
                    &mut consecutive_local_errors,
                    "result_persistence",
                    "local_persistence",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue;
            }
            Ok(deploy_go_agent_executor::protocol::Response::ReleaseOutput(_))
            | Ok(deploy_go_agent_executor::protocol::Response::ReleaseStatus(_))
            | Ok(deploy_go_agent_executor::protocol::Response::ReleaseExited(_)) => {
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&task_journal),
                    &outbound,
                    &mut consecutive_output_errors,
                    "output_response",
                    "executor_response_mismatch",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue;
            }
            result => {
                let error_class = if result.is_err() {
                    "executor_ipc"
                } else {
                    "executor_rejected_or_unexpected"
                };
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&task_journal),
                    &outbound,
                    &mut consecutive_output_errors,
                    "output_read",
                    error_class,
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue;
            }
        };
        consecutive_output_errors = 0;
        // 不能在这里清零 consecutive_local_errors：后续的帧落盘、journal 写入、事件重建失败都会
        // `continue` 回到循环开头，提前清零会让本地存储故障的退避永远停在第一档并反复刷日志。
        // 整轮（含交付）成功之后才清零。
        let mut journal = match executor.load(&task_id) {
            Ok(journal) => journal,
            Err(_) => {
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&task_journal),
                    &outbound,
                    &mut consecutive_local_errors,
                    "journal_read",
                    "local_persistence",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue;
            }
        };
        for frame in batch.frames {
            if frame.sequence != journal.external_output_sequence.saturating_add(1) {
                let expected = journal.external_output_sequence.saturating_add(1);
                log_privileged_release_diagnostic(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    "output_sequence",
                    "sequence_conflict",
                    None,
                    Some(frame.sequence),
                    "report_interrupted",
                    true,
                );
                tracing::warn!(
                    task_id,
                    expected_sequence = expected,
                    "release output sequence is not contiguous"
                );
                if complete_privileged_release_interrupted(
                    &executor,
                    &event_lock,
                    &outbound,
                    &task_id,
                    "release_output_sequence_conflict",
                )
                .await
                .is_ok()
                {
                    return;
                }
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    &outbound,
                    &mut consecutive_local_errors,
                    "result_persistence",
                    "local_persistence",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue 'monitor;
            }
            let stream = match frame.stream {
                deploy_go_agent_executor::protocol::ReleaseOutputStream::Stdout => {
                    OutputStream::Stdout
                }
                deploy_go_agent_executor::protocol::ReleaseOutputStream::Stderr => {
                    OutputStream::Stderr
                }
            };
            if let Err(error) =
                executor.persist_external_output(&task_id, frame.sequence, stream, &frame.data)
            {
                if transient_monitor_storage_error(&error) {
                    if !retry_privileged_release_monitor(
                        &task_id,
                        &job_id,
                        &task,
                        Some(&journal),
                        &outbound,
                        &mut consecutive_local_errors,
                        "output_persist",
                        "local_persistence",
                        None,
                        None,
                        "retry_same_job_from_durable_sequence",
                    )
                    .await
                    {
                        return;
                    }
                    continue 'monitor;
                }
                log_privileged_release_diagnostic(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    "output_persist",
                    "output_integrity",
                    None,
                    Some(frame.sequence),
                    "report_interrupted",
                    true,
                );
                if complete_privileged_release_interrupted(
                    &executor,
                    &event_lock,
                    &outbound,
                    &task_id,
                    "release_output_persistence_failed",
                )
                .await
                .is_ok()
                {
                    return;
                }
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    &outbound,
                    &mut consecutive_local_errors,
                    "result_persistence",
                    "local_persistence",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue 'monitor;
            }
            journal.external_output_sequence = frame.sequence;
        }
        let output_incomplete = batch.truncated;
        if executor.store_journal(&journal).is_err() {
            if !retry_privileged_release_monitor(
                &task_id,
                &job_id,
                &task,
                Some(&journal),
                &outbound,
                &mut consecutive_local_errors,
                "journal_write",
                "local_persistence",
                None,
                None,
                "retry_same_job_from_durable_sequence",
            )
            .await
            {
                return;
            }
            continue;
        }
        if let Some(release_task) = task.as_ref()
            && let Err(error) =
                rebuild_privileged_events(&executor.task_dir(&task_id), release_task, None)
        {
            if permanent_event_data_error(&error) {
                log_privileged_release_diagnostic(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    "event_rebuild",
                    "event_data_integrity",
                    None,
                    None,
                    "report_interrupted",
                    true,
                );
                if complete_privileged_release_interrupted(
                    &executor,
                    &event_lock,
                    &outbound,
                    &task_id,
                    "release_event_data_invalid",
                )
                .await
                .is_ok()
                {
                    return;
                }
            }
            if !retry_privileged_release_monitor(
                &task_id,
                &job_id,
                &task,
                Some(&journal),
                &outbound,
                &mut consecutive_local_errors,
                "event_rebuild",
                if permanent_event_data_error(&error) {
                    "local_persistence"
                } else {
                    "local_io"
                },
                None,
                None,
                "retry_same_job",
            )
            .await
            {
                return;
            }
            continue;
        }
        if drain_outputs(&executor, &event_lock, &outbound, &mut journal)
            .await
            .is_err()
            || drain_events(&executor, &event_lock, &outbound, &mut journal)
                .await
                .is_err()
        {
            if !retry_privileged_release_monitor(
                &task_id,
                &job_id,
                &task,
                Some(&journal),
                &outbound,
                &mut consecutive_local_errors,
                "event_outbox_delivery",
                "local_delivery_or_outbox",
                None,
                None,
                "retry_durable_output_and_events",
            )
            .await
            {
                return;
            }
            continue;
        }
        consecutive_local_errors = 0;
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
            Ok(deploy_go_agent_executor::protocol::Response::ReleaseStatus(status))
                if status.version == deploy_go_agent_executor::protocol::PROTOCOL_VERSION
                    && status.job_id == job_id =>
            {
                consecutive_status_errors = 0;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Ok(deploy_go_agent_executor::protocol::Response::ReleaseExited(exited))
                if exited.version == deploy_go_agent_executor::protocol::PROTOCOL_VERSION
                    && exited.job_id == job_id =>
            {
                if exited.last_sequence > journal.external_output_sequence {
                    if journal.external_output_sequence > after_sequence {
                        // 本轮读到了新帧：终态 job 的剩余帧都已落盘，立即读取下一页。`truncated`
                        // 是 job 级标志，终态后每一页都为 true，不能据此判定本页之后没有帧；
                        // 仍在前进的多页输出也不应退避（否则每页最长等待 5 秒）。
                        consecutive_terminal_sequence_gaps = 0;
                        continue 'monitor;
                    }
                    if output_incomplete {
                        // executor 已终态且声明仍有帧，但本轮没有新帧且输出已被截断：
                        // 终态之后不会再补帧，属于永久完整性冲突，不能靠重试解决。
                        log_privileged_release_diagnostic(
                            &task_id,
                            &job_id,
                            &task,
                            Some(&journal),
                            "terminal_output_truncated",
                            "output_integrity",
                            Some("terminal"),
                            Some(exited.last_sequence),
                            "report_interrupted",
                            true,
                        );
                        if complete_privileged_release_interrupted(
                            &executor,
                            &event_lock,
                            &outbound,
                            &task_id,
                            "release_output_truncated_unexpectedly",
                        )
                        .await
                        .is_ok()
                        {
                            return;
                        }
                        if !retry_privileged_release_monitor(
                            &task_id,
                            &job_id,
                            &task,
                            Some(&journal),
                            &outbound,
                            &mut consecutive_local_errors,
                            "result_persistence",
                            "local_persistence",
                            Some("terminal"),
                            Some(exited.last_sequence),
                            "retry_same_job_terminal",
                        )
                        .await
                        {
                            return;
                        }
                        continue 'monitor;
                    }
                    if consecutive_terminal_sequence_gaps >= TERMINAL_OUTPUT_STALL_LIMIT {
                        // 终态 job 的输出不会再增长：连续多轮既无新帧又落后于宣告的最后序号，
                        // 属于永久序号冲突，按非成功中断收敛，不再无限等待。
                        log_privileged_release_diagnostic(
                            &task_id,
                            &job_id,
                            &task,
                            Some(&journal),
                            "terminal_output_sequence",
                            "sequence_stalled",
                            Some("terminal"),
                            Some(exited.last_sequence),
                            "report_interrupted",
                            true,
                        );
                        if complete_privileged_release_interrupted(
                            &executor,
                            &event_lock,
                            &outbound,
                            &task_id,
                            "release_output_sequence_conflict",
                        )
                        .await
                        .is_ok()
                        {
                            return;
                        }
                        if !retry_privileged_release_monitor(
                            &task_id,
                            &job_id,
                            &task,
                            Some(&journal),
                            &outbound,
                            &mut consecutive_local_errors,
                            "result_persistence",
                            "local_persistence",
                            Some("terminal"),
                            Some(exited.last_sequence),
                            "retry_same_job_terminal",
                        )
                        .await
                        {
                            return;
                        }
                        continue 'monitor;
                    }
                    // 终态先于最后一轮输出到达的竞态窗口：退避后重读同一 job 的输出，
                    // 避免在没有诊断和间隔的情况下空转。
                    if !retry_privileged_release_monitor(
                        &task_id,
                        &job_id,
                        &task,
                        Some(&journal),
                        &outbound,
                        &mut consecutive_terminal_sequence_gaps,
                        "terminal_output_sequence",
                        "sequence_not_caught_up",
                        Some("terminal"),
                        Some(exited.last_sequence),
                        "retry_same_job_terminal",
                    )
                    .await
                    {
                        return;
                    }
                    continue 'monitor;
                }
                consecutive_terminal_sequence_gaps = 0;
                if exited.last_sequence < journal.external_output_sequence {
                    log_privileged_release_diagnostic(
                        &task_id,
                        &job_id,
                        &task,
                        Some(&journal),
                        "terminal_output_sequence",
                        "sequence_conflict",
                        Some("terminal"),
                        Some(exited.last_sequence),
                        "report_interrupted",
                        true,
                    );
                    if complete_privileged_release_interrupted(
                        &executor,
                        &event_lock,
                        &outbound,
                        &task_id,
                        "release_output_sequence_conflict",
                    )
                    .await
                    .is_ok()
                    {
                        return;
                    }
                    if !retry_privileged_release_monitor(
                        &task_id,
                        &job_id,
                        &task,
                        Some(&journal),
                        &outbound,
                        &mut consecutive_local_errors,
                        "result_persistence",
                        "local_persistence",
                        Some("terminal"),
                        Some(exited.last_sequence),
                        "retry_same_job_terminal",
                    )
                    .await
                    {
                        return;
                    }
                    continue 'monitor;
                }
                let output_limit_exceeded = output_incomplete
                    && exited.state == deploy_go_agent_executor::protocol::ReleaseJobState::Failed
                    && exited.reason == "output_limit_exceeded";
                if output_incomplete && !output_limit_exceeded {
                    log_privileged_release_diagnostic(
                        &task_id,
                        &job_id,
                        &task,
                        Some(&journal),
                        "terminal_output_truncated",
                        "output_integrity",
                        Some("terminal"),
                        Some(exited.last_sequence),
                        "report_interrupted",
                        true,
                    );
                    if complete_privileged_release_interrupted(
                        &executor,
                        &event_lock,
                        &outbound,
                        &task_id,
                        "release_output_truncated_unexpectedly",
                    )
                    .await
                    .is_ok()
                    {
                        return;
                    }
                    if !retry_privileged_release_monitor(
                        &task_id,
                        &job_id,
                        &task,
                        Some(&journal),
                        &outbound,
                        &mut consecutive_local_errors,
                        "result_persistence",
                        "local_persistence",
                        Some("terminal"),
                        Some(exited.last_sequence),
                        "retry_same_job_terminal",
                    )
                    .await
                    {
                        return;
                    }
                    continue 'monitor;
                }
                let exit_ok =
                    exited.state == deploy_go_agent_executor::protocol::ReleaseJobState::Succeeded;
                let protocol_error = if let Some(release_task) = task.as_ref() {
                    match rebuild_privileged_events(
                        &executor.task_dir(&task_id),
                        release_task,
                        Some(exit_ok),
                    ) {
                        Ok(protocol_error) => protocol_error,
                        Err(error) => {
                            if permanent_event_data_error(&error) {
                                // 事件数据永久损坏（如超长行）：job 已终态，重试不会改变结果，
                                // 以非成功的中断收敛，而不是无限等待。
                                log_privileged_release_diagnostic(
                                    &task_id,
                                    &job_id,
                                    &task,
                                    Some(&journal),
                                    "terminal_event_rebuild",
                                    "event_data_integrity",
                                    Some("terminal"),
                                    Some(exited.last_sequence),
                                    "report_interrupted",
                                    true,
                                );
                                if complete_privileged_release_interrupted(
                                    &executor,
                                    &event_lock,
                                    &outbound,
                                    &task_id,
                                    "release_event_data_invalid",
                                )
                                .await
                                .is_ok()
                                {
                                    return;
                                }
                            }
                            if !retry_privileged_release_monitor(
                                &task_id,
                                &job_id,
                                &task,
                                Some(&journal),
                                &outbound,
                                &mut consecutive_local_errors,
                                "terminal_event_rebuild",
                                "local_io",
                                Some("terminal"),
                                Some(exited.last_sequence),
                                "retry_same_job_terminal",
                            )
                            .await
                            {
                                return;
                            }
                            continue;
                        }
                    }
                } else {
                    Some("release_task_descriptor_unavailable".to_owned())
                };
                let (state, error_code) = if task.is_none() {
                    (
                        JournalState::Interrupted,
                        Some("release_task_descriptor_unavailable".to_owned()),
                    )
                } else {
                    match exited.state {
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
                    }
                };
                let mut completed = match executor.complete_task_with_exit_code(
                    &task_id,
                    state,
                    error_code,
                    None,
                    exited.exit_code,
                ) {
                    Ok(completed) => completed,
                    Err(_) => {
                        if !retry_privileged_release_monitor(
                            &task_id,
                            &job_id,
                            &task,
                            Some(&journal),
                            &outbound,
                            &mut consecutive_local_errors,
                            "terminal_persist",
                            "local_persistence",
                            Some("terminal"),
                            Some(exited.last_sequence),
                            "retry_same_job_terminal",
                        )
                        .await
                        {
                            return;
                        }
                        continue;
                    }
                };
                if send_result(&executor, &event_lock, &outbound, &mut completed)
                    .await
                    .is_ok()
                {
                    return;
                }
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&completed),
                    &outbound,
                    &mut consecutive_local_errors,
                    "result_delivery",
                    "outbox_or_delivery",
                    Some("terminal"),
                    Some(exited.last_sequence),
                    "retry_durable_result",
                )
                .await
                {
                    return;
                }
                continue;
            }
            Ok(deploy_go_agent_executor::protocol::Response::Error(error))
                if error.version == deploy_go_agent_executor::protocol::PROTOCOL_VERSION
                    && error.code == "release_job_conflict" =>
            {
                if complete_privileged_release_failure(
                    &executor,
                    &event_lock,
                    &outbound,
                    &task_id,
                    "release_job_conflict",
                )
                .await
                .is_ok()
                {
                    return;
                }
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    &outbound,
                    &mut consecutive_local_errors,
                    "result_persistence",
                    "local_persistence",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue;
            }
            Ok(deploy_go_agent_executor::protocol::Response::ReleaseStatus(_))
            | Ok(deploy_go_agent_executor::protocol::Response::ReleaseExited(_))
            | Ok(deploy_go_agent_executor::protocol::Response::ReleaseOutput(_)) => {
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    &outbound,
                    &mut consecutive_status_errors,
                    "status_response",
                    "executor_response_mismatch",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
                continue;
            }
            Ok(deploy_go_agent_executor::protocol::Response::Error(_)) => {
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    &outbound,
                    &mut consecutive_status_errors,
                    "status_read",
                    "executor_rejected",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
            }
            Err(_) => {
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    &outbound,
                    &mut consecutive_status_errors,
                    "status_read",
                    "executor_ipc",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
            }
            Ok(_) => {
                if !retry_privileged_release_monitor(
                    &task_id,
                    &job_id,
                    &task,
                    Some(&journal),
                    &outbound,
                    &mut consecutive_status_errors,
                    "status_response",
                    "executor_response_invalid",
                    None,
                    None,
                    "retry_same_job",
                )
                .await
                {
                    return;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn log_privileged_release_diagnostic(
    task_id: &str,
    job_id: &str,
    task: &Option<DeploymentReleaseTask>,
    journal: Option<&TaskJournal>,
    failure_stage: &'static str,
    error_class: &'static str,
    executor_terminal_state: Option<&'static str>,
    executor_last_sequence: Option<u64>,
    recovery_action: &'static str,
    is_error: bool,
) {
    let task = task.as_ref();
    let deployment_id = journal
        .and_then(|journal| journal.deployment_id.as_deref())
        .or_else(|| task.map(|task| task.deployment_id.as_str()))
        .unwrap_or("unavailable");
    let target_run_id = journal
        .and_then(|journal| journal.target_run_id.as_deref())
        .or_else(|| {
            task?
                .privileged_context
                .as_ref()
                .map(|context| context.target_run_id.as_str())
        })
        .unwrap_or("unavailable");
    let persisted_sequence = journal.map(|journal| journal.external_output_sequence);
    if is_error {
        tracing::error!(
            diagnostic_scope = "privileged_release",
            deployment_id,
            target_run_id,
            task_id,
            job_id,
            failure_stage,
            error_class,
            agent_persisted_output_sequence = ?persisted_sequence,
            executor_terminal_state = ?executor_terminal_state,
            executor_last_sequence = ?executor_last_sequence,
            recovery_action,
            "privileged release monitor requires recovery"
        );
    } else {
        tracing::warn!(
            diagnostic_scope = "privileged_release",
            deployment_id,
            target_run_id,
            task_id,
            job_id,
            failure_stage,
            error_class,
            agent_persisted_output_sequence = ?persisted_sequence,
            executor_terminal_state = ?executor_terminal_state,
            executor_last_sequence = ?executor_last_sequence,
            recovery_action,
            "privileged release monitor will retry"
        );
    }
}

#[allow(clippy::too_many_arguments)]
async fn retry_privileged_release_monitor(
    task_id: &str,
    job_id: &str,
    task: &Option<DeploymentReleaseTask>,
    journal: Option<&TaskJournal>,
    outbound: &mpsc::Sender<Message>,
    consecutive_errors: &mut u32,
    failure_stage: &'static str,
    error_class: &'static str,
    executor_terminal_state: Option<&'static str>,
    executor_last_sequence: Option<u64>,
    recovery_action: &'static str,
) -> bool {
    if outbound.is_closed() {
        return false;
    }
    *consecutive_errors = consecutive_errors.saturating_add(1);
    if *consecutive_errors == 1 || consecutive_errors.is_multiple_of(40) {
        log_privileged_release_diagnostic(
            task_id,
            job_id,
            task,
            journal,
            failure_stage,
            error_class,
            executor_terminal_state,
            executor_last_sequence,
            recovery_action,
            false,
        );
    }
    tokio::time::sleep(monitor_retry_sleep(*consecutive_errors)).await;
    !outbound.is_closed()
}

fn transient_monitor_storage_error(error: &crate::executor::ExecuteError) -> bool {
    matches!(
        error,
        crate::executor::ExecuteError::Io(_)
            | crate::executor::ExecuteError::Journal(JournalError::Io(_))
    )
}

/// 事件重建的永久数据错误：`InvalidData`（解析失败）与 `Other`（如超长事件行的完整性违规）。
/// 真实的系统 I/O 错误（ENOSPC、EACCES、EIO 等）不属于这两类，仍按暂时错误退避重试。
fn permanent_event_data_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::InvalidData | io::ErrorKind::Other
    )
}

fn unrecoverable_journal_error(error: &crate::executor::ExecuteError) -> bool {
    matches!(
        error,
        crate::executor::ExecuteError::Journal(
            JournalError::Missing | JournalError::InvalidJournal | JournalError::InvalidIdentity
        )
    )
}

fn monitor_retry_delay(consecutive_errors: u32) -> Duration {
    let exponent = consecutive_errors.saturating_sub(1).min(5);
    let millis = 250_u64.saturating_mul(1_u64 << exponent).min(5_000);
    Duration::from_millis(millis)
}

/// 实际等待时长。单元测试构建把退避压缩 25 倍，避免故障注入用例累计等待数十秒；
/// 集成测试和生产使用 `monitor_retry_delay` 的原值。
fn monitor_retry_sleep(consecutive_errors: u32) -> Duration {
    let delay = monitor_retry_delay(consecutive_errors);
    if cfg!(test) { delay / 25 } else { delay }
}

/// 终态 job 的输出不会再增长：连续这么多轮既无新帧又落后于 executor 宣告的最后序号时，
/// 判定为永久序号冲突。退避合计约 13 秒，足以覆盖"终态先于最后一批输出可见"的竞态。
const TERMINAL_OUTPUT_STALL_LIMIT: u32 = 6;

async fn complete_privileged_release_failure(
    executor: &Executor,
    event_lock: &Arc<Mutex<()>>,
    outbound: &mpsc::Sender<Message>,
    task_id: &str,
    error_code: &str,
) -> Result<(), ()> {
    let canceled = executor.is_cancel_requested(task_id);
    let mut failed = executor
        .complete_task_with_exit_code(
            task_id,
            if canceled {
                JournalState::Canceled
            } else {
                JournalState::Failed
            },
            (!canceled).then(|| error_code.to_owned()),
            None,
            None,
        )
        .map_err(|_| ())?;
    send_result(executor, event_lock, outbound, &mut failed).await
}

async fn complete_privileged_release_interrupted(
    executor: &Executor,
    event_lock: &Arc<Mutex<()>>,
    outbound: &mpsc::Sender<Message>,
    task_id: &str,
    error_code: &str,
) -> Result<(), ()> {
    let mut interrupted = executor
        .complete_task_with_exit_code(
            task_id,
            JournalState::Interrupted,
            Some(error_code.to_owned()),
            None,
            None,
        )
        .map_err(|_| ())?;
    send_result(executor, event_lock, outbound, &mut interrupted).await
}

#[cfg(test)]
mod release_executor_error_code_tests {
    use super::{
        ReleaseStartFailure, classify_release_start_result, monitor_retry_delay,
        release_executor_error_code, release_started_response_error_code,
    };
    use crate::executor_client::ExecutorClientError;
    use deploy_go_agent_executor::protocol::{
        ErrorResponse, PROTOCOL_VERSION, ReleaseJobState, ReleaseStartedResponse, Response,
    };
    use std::{io, time::Duration};

    #[test]
    fn preserves_known_executor_errors_and_hides_unknown_values() {
        assert_eq!(
            release_executor_error_code("release_storage_low_disk"),
            "release_storage_low_disk"
        );
        assert_eq!(
            release_executor_error_code("token=secret-value"),
            "executor_rejected_request"
        );
    }

    #[test]
    fn detects_release_start_version_and_job_id_mismatches() {
        let response = ReleaseStartedResponse {
            version: PROTOCOL_VERSION,
            job_id: "release_expected".into(),
            state: ReleaseJobState::Running,
        };
        assert_eq!(
            release_started_response_error_code(&response, "release_expected"),
            None
        );
        assert_eq!(
            release_started_response_error_code(&response, "release_other"),
            Some("executor_response_job_mismatch")
        );
        let old_version = ReleaseStartedResponse {
            version: PROTOCOL_VERSION.saturating_sub(1),
            ..response
        };
        assert_eq!(
            release_started_response_error_code(&old_version, "release_expected"),
            Some("executor_response_version_mismatch")
        );
    }

    #[test]
    fn distinguishes_definite_rejections_from_start_outcome_uncertainty() {
        let rejected = classify_release_start_result(
            Ok(Response::Error(ErrorResponse {
                version: PROTOCOL_VERSION,
                code: "release_digest_mismatch".into(),
            })),
            "release_expected",
        );
        assert_eq!(
            rejected,
            Err(ReleaseStartFailure::Rejected("release_digest_mismatch"))
        );

        for code in ["release_spawn_failed", "release_storage_unavailable"] {
            assert_eq!(
                classify_release_start_result(
                    Ok(Response::Error(ErrorResponse {
                        version: PROTOCOL_VERSION,
                        code: code.into(),
                    })),
                    "release_expected",
                ),
                Err(ReleaseStartFailure::Uncertain(code))
            );
        }

        assert_eq!(
            classify_release_start_result(
                Err(ExecutorClientError::Unavailable {
                    io_kind: io::ErrorKind::NotFound,
                }),
                "release_expected",
            ),
            Err(ReleaseStartFailure::NotSent("executor_unavailable"))
        );
    }

    #[test]
    fn monitor_retry_delay_grows_to_a_bounded_interval() {
        assert_eq!(monitor_retry_delay(1), Duration::from_millis(250));
        assert_eq!(monitor_retry_delay(2), Duration::from_millis(500));
        assert_eq!(monitor_retry_delay(5), Duration::from_secs(4));
        assert_eq!(monitor_retry_delay(6), Duration::from_secs(5));
        assert_eq!(monitor_retry_delay(u32::MAX), Duration::from_secs(5));
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
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let temporary = task_dir.join("events.jsonl.rebuild");
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&temporary)?;
    let mut encoded = io::BufWriter::new(file);
    serde_json::to_writer(&mut encoded, &crate::deploy_events::started_event(&context))?;
    encoded.write_all(b"\n")?;
    let mut reader = io::BufReader::new(crate::log_delivery::LogReader::new(task_dir, "stdout"));
    loop {
        let mut line = Vec::new();
        reader
            .by_ref()
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut line)?;
        if line.len() > 1024 * 1024 {
            return Err(io::Error::other("deploy event line exceeds limit"));
        }
        if line.is_empty() {
            break;
        }
        if !line.ends_with(b"\n") && terminal_exit_ok.is_none() {
            break;
        }
        if line.ends_with(b"\n") {
            line.pop();
        }
        if line.is_empty() {
            continue;
        }
        let line = String::from_utf8_lossy(&line);
        match crate::deploy_events::process_line(&line, &context, &mut state) {
            Ok(Some(event)) => {
                serde_json::to_writer(&mut encoded, &event)?;
                encoded.write_all(b"\n")?;
            }
            Ok(None) => {}
            Err(error) => state.violations.push(error.to_string()),
        }
    }
    let mut protocol_error = None;
    if let Some(exit_ok) = terminal_exit_ok {
        let (event, error) = crate::deploy_events::finished_event(&context, &state, exit_ok);
        serde_json::to_writer(&mut encoded, &event)?;
        encoded.write_all(b"\n")?;
        protocol_error = error;
    }
    encoded.flush()?;
    encoded.get_ref().sync_all()?;
    fs::rename(temporary, task_dir.join("events.jsonl"))?;
    Ok(protocol_error)
}

#[async_trait]
impl MessageHandler for TaskHandler {
    fn negotiated_protocol(&self, version: u16) {
        self.executor.enable_reliable_logs(version >= 18);
    }
    async fn handle(
        &self,
        envelope: Envelope,
        outbound: mpsc::Sender<Message>,
    ) -> Result<(), ConnectionError> {
        match envelope.message {
            Message::TaskEventReceipt(receipt) => {
                let sender = self.receipt_sender.get_or_init(|| {
                    let (sender, mut receiver) = mpsc::channel::<deploy_go_agent_protocol::TaskEventReceipt>(4096);
                    let executor = self.executor.clone();
                    let lock = self.event_lock.clone();
                    tokio::spawn(async move {
                        while let Some(receipt) = receiver.recv().await {
                            let _guard = lock.lock().await;
                            if let Ok(journal) = executor.load(&receipt.task_id)
                                && executor.store_journal(&journal).is_ok()
                                && let Err(error) = crate::log_delivery::acknowledge(&executor.task_dir(&receipt.task_id), &journal, &receipt)
                            {
                                tracing::warn!(task_id = %receipt.task_id, %error, "任务日志回执未接受");
                            }
                        }
                    });
                    sender
                });
                // 接收循环不能等待发送方持有的锁；满队列断线后通过 outbox 安全重放。
                sender
                    .try_send(receipt)
                    .map_err(|_| ConnectionError::InvalidMessage)
            }
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
            Message::SecretEnvironmentLeaseResponse(response) => {
                let broker = Arc::clone(&self.secret_lease);
                tokio::spawn(async move {
                    broker.resolve_environment(response).await;
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
            Message::AgentUpgradeCommand(command) => {
                let handler = self.clone();
                tokio::spawn(async move {
                    let accepted = handler
                        .agent_upgrade_client
                        .as_ref()
                        .zip(handler.agent_upgrade_control_url.as_ref())
                        .zip(handler.agent_upgrade_data_dir.as_ref())
                        .is_some();
                    let _ = outbound
                        .send(Message::AgentUpgradeAck(deploy_go_agent_protocol::AgentUpgradeAck {
                            job_id: command.job_id.clone(),
                            accepted,
                            error_code: (!accepted).then_some(deploy_go_agent_protocol::AgentUpgradeErrorCode::UpgradeUnsupportedProtocol),
                        }))
                        .await;
                    if !accepted {
                        return;
                    }
                    let client = handler
                        .agent_upgrade_client
                        .as_ref()
                        .expect("已检查升级 client");
                    let control_url = handler
                        .agent_upgrade_control_url
                        .as_ref()
                        .expect("已检查控制面 URL");
                    let data_dir = handler
                        .agent_upgrade_data_dir
                        .as_ref()
                        .expect("已检查升级目录");
                    let executor = handler
                        .privileged_release_executor
                        .as_ref()
                        .map(|client| (**client).clone())
                        .unwrap_or_else(|| {
                            ExecutorClient::new(
                                crate::executor_client::DEFAULT_EXECUTOR_SOCKET_PATH.into(),
                            )
                        });
                    let result = agent_upgrade::stage_upgrade(
                        &command,
                        control_url,
                        data_dir,
                        client,
                        &executor,
                        &outbound,
                    )
                    .await;
                    if let Err(error_code) = result {
                        if let Ok(Some(mut state)) =
                            agent_upgrade::UpgradeStateStore::new(data_dir.clone()).load()
                            && state.job_id == command.job_id
                        {
                            state.phase = "failed".to_owned();
                            state.updated_at = chrono::Utc::now().to_rfc3339();
                            let _ = agent_upgrade::save_job_state(data_dir, &state);
                        }
                        let _ = outbound
                            .send(Message::AgentUpgradeReport(
                                deploy_go_agent_protocol::AgentUpgradeReport {
                                    job_id: command.job_id,
                                    status:
                                        deploy_go_agent_protocol::AgentUpgradeReportStatus::Failed,
                                    target_version: command.target_version,
                                    manifest_digest: command.manifest_digest,
                                    error_code: Some(error_code),
                                    error_summary: Some("Agent 升级执行失败".to_owned()),
                                },
                            ))
                            .await;
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

    async fn on_connected(
        &self,
        _connection_generation: u64,
        outbound: mpsc::Sender<Message>,
    ) -> Result<(), ConnectionError> {
        let executor = self.executor.clone();
        let replay_outbound = outbound.clone();
        let lock = self.event_lock.clone();
        tokio::spawn(async move {
            let guard = lock.lock().await;
            let mut unfinished_terminal = Vec::new();
            for id in executor.delivery_task_ids().unwrap_or_default() {
                let dir = executor.task_dir(&id);
                if let Ok(journal) = executor.load(&id)
                    && terminal(&journal.state)
                    && journal.result_sequence.is_none()
                {
                    unfinished_terminal.push(journal);
                }
                if let Ok(paths) = crate::log_delivery::pending_messages(&dir) {
                    for path in paths {
                        if let Ok(message) = crate::log_delivery::read_pending(&path)
                            && replay_outbound.send(message).await.is_err()
                        {
                            return;
                        }
                    }
                }
            }
            drop(guard);
            for mut journal in unfinished_terminal {
                if drain_outputs(&executor, &lock, &replay_outbound, &mut journal)
                    .await
                    .is_err()
                    || drain_events(&executor, &lock, &replay_outbound, &mut journal)
                        .await
                        .is_err()
                {
                    continue;
                }
                let _ = send_result(&executor, &lock, &replay_outbound, &mut journal).await;
            }
        });
        let Some(data_dir) = self.agent_upgrade_data_dir.as_ref() else {
            return Ok(());
        };
        let mut state = match agent_upgrade::UpgradeStateStore::new(data_dir.clone()).load() {
            Ok(Some(state)) => state,
            Ok(None) => return Ok(()),
            Err(_) => return Ok(()),
        };
        if !matches!(
            state.phase.as_str(),
            "installing" | "reconnecting" | "succeeded"
        ) {
            return Ok(());
        }
        if state.target_version != env!("CARGO_PKG_VERSION") {
            return Ok(());
        }
        state.phase = "succeeded".to_owned();
        state.updated_at = chrono::Utc::now().to_rfc3339();
        let _ = agent_upgrade::save_job_state(data_dir, &state);
        let _ = outbound
            .send(Message::AgentUpgradeReport(
                deploy_go_agent_protocol::AgentUpgradeReport {
                    job_id: state.job_id,
                    status: deploy_go_agent_protocol::AgentUpgradeReportStatus::Succeeded,
                    target_version: state.target_version,
                    manifest_digest: state.manifest_digest,
                    error_code: None,
                    error_summary: None,
                },
            ))
            .await;
        Ok(())
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
                if drain_outputs(&executor, &event_lock, &outbound, &mut current)
                    .await
                    .is_err()
                    || drain_events(&executor, &event_lock, &outbound, &mut current)
                        .await
                        .is_err()
                {
                    return;
                }
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
    let message = Message::TaskState(TaskState {
        task_id: journal.task_id.clone(),
        sequence,
        state: state.clone(),
    });
    journal.last_sequence = sequence;
    journal.state = match state {
        TaskLifecycleState::Running => JournalState::Running,
        TaskLifecycleState::Accepted => JournalState::Accepted,
        TaskLifecycleState::Canceling => journal.state.clone(),
    };
    deliver(executor, outbound, journal, message).await
}

async fn emit_transfer_line(
    executor: &Executor,
    event_lock: &Mutex<()>,
    outbound: &mpsc::Sender<Message>,
    task_id: &str,
    text: impl Into<String>,
) -> Result<(), ()> {
    let _guard = event_lock.lock().await;
    let mut journal = executor.load(task_id).map_err(|_| ())?;
    if terminal(&journal.state) || journal.result_sequence.is_some() {
        return Ok(());
    }
    let mut line = text.into();
    if !line.ends_with('\n') {
        line.push('\n');
    }
    let sequence = journal.last_sequence + 1;
    let message = Message::TaskOutput(TaskOutput {
        task_id: task_id.to_owned(),
        sequence,
        stream: OutputStream::Stdout,
        text: line,
    });
    journal.last_sequence = sequence;
    deliver(executor, outbound, &journal, message).await
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
    let file = match fs::File::open(executor.task_dir(&journal.task_id).join("events.jsonl")) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(()),
    };
    let mut sent = journal.events_sent;
    let legacy_offset = if sent == 0 { journal.events_offset } else { 0 };
    let mut reader = io::BufReader::new(file);
    let mut cursor = 0_u64;
    loop {
        let mut chunk = Vec::new();
        let start = cursor;
        reader
            .by_ref()
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut chunk)
            .map_err(|_| ())?;
        if chunk.len() > 1024 * 1024 {
            return Err(());
        }
        if !chunk.ends_with(b"\n") {
            break;
        }
        cursor += chunk.len() as u64;
        if chunk.len() <= 1 {
            continue;
        }
        if start < legacy_offset {
            journal.events_sent += 1;
            continue;
        }
        if sent > 0 {
            sent -= 1;
            continue;
        }
        let line = &chunk[..chunk.len() - 1];
        let message = match serde_json::from_slice::<DeployEvent>(line) {
            Ok(event) => {
                let sequence = journal.last_sequence + 1;
                let message = Message::TaskProgress(TaskProgress {
                    task_id: journal.task_id.clone(),
                    sequence,
                    event,
                });
                journal.last_sequence = sequence;
                Some(message)
            }
            Err(_) => {
                tracing::warn!(
                    task_id = %journal.task_id,
                    "skipping malformed deploy event line"
                );
                None
            }
        };
        journal.events_sent += 1;
        journal.events_offset = cursor;
        if let Some(message) = message {
            deliver(executor, outbound, journal, message).await?;
        } else {
            executor.store_journal(journal).map_err(|_| ())?;
        }
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
        OutputStream::Stdout => ("stdout", journal.stdout_offset),
        OutputStream::Stderr => ("stderr", journal.stderr_offset),
    };
    loop {
        let chunk = crate::log_delivery::read_chunk(
            &executor.task_dir(&journal.task_id),
            filename,
            offset,
            OUTPUT_CHUNK_BYTES,
        )
        .map_err(|_| ())?;
        if chunk.is_empty() {
            break;
        }
        let sequence = journal.last_sequence + 1;
        let message = Message::TaskOutput(TaskOutput {
            task_id: journal.task_id.clone(),
            sequence,
            stream: stream.clone(),
            text: String::from_utf8_lossy(&chunk).into_owned(),
        });
        journal.last_sequence = sequence;
        offset += chunk.len() as u64;
        match stream {
            OutputStream::Stdout => journal.stdout_offset = offset,
            OutputStream::Stderr => journal.stderr_offset = offset,
        }
        deliver(executor, outbound, journal, message).await?;
    }
    Ok(())
}

async fn send_result(
    executor: &Executor,
    event_lock: &Mutex<()>,
    outbound: &mpsc::Sender<Message>,
    journal: &mut TaskJournal,
) -> Result<(), ()> {
    if executor
        .load(&journal.task_id)
        .map_err(|_| ())?
        .result_sequence
        .is_none()
    {
        drain_outputs(executor, event_lock, outbound, journal).await?;
        drain_events(executor, event_lock, outbound, journal).await?;
    }
    let _guard = event_lock.lock().await;
    *journal = executor.load(&journal.task_id).map_err(|_| ())?;
    if journal.result_sequence.is_some() {
        let dir = executor.task_dir(&journal.task_id);
        if crate::log_delivery::enabled(&dir) {
            for path in crate::log_delivery::pending_messages(&dir).map_err(|_| ())? {
                outbound
                    .send(crate::log_delivery::read_pending(&path).map_err(|_| ())?)
                    .await
                    .map_err(|_| ())?;
            }
            return Ok(());
        }
        return resend_result(outbound, journal).await;
    }
    let sequence = journal.last_sequence + 1;
    let result = result_for(journal, sequence);
    journal.last_sequence = sequence;
    journal.result_sequence = Some(sequence);
    deliver(executor, outbound, journal, Message::TaskResult(result)).await?;
    if terminal(&journal.state) {
        // 终态已发送后立即回收 checkout/staging；任务 journal 按保留期存续，
        // 用于断线后的结果重放与诊断。
        let cleanup_executor = executor.clone();
        let cleanup_task_id = journal.task_id.clone();
        drop(tokio::task::spawn_blocking(move || {
            cleanup_executor.cleanup_terminal_sources(&cleanup_task_id);
        }));
    }
    Ok(())
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

async fn deliver(
    executor: &Executor,
    outbound: &mpsc::Sender<Message>,
    journal: &TaskJournal,
    message: Message,
) -> Result<(), ()> {
    crate::log_delivery::persist(&executor.task_dir(&journal.task_id), &message, journal).map_err(|error| {
        tracing::error!(task_id = %journal.task_id, %error, "日志持久化失败，停止转发并保留原始日志");
    })?;
    executor.store_journal(journal).map_err(|_| ())?;
    outbound.send(message).await.map_err(|_| ())
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
        summary: journal
            .result_data
            .as_ref()
            .and_then(|data| data.get("artifact_transfer"))
            .map(|detail| {
                format!(
                    "脚本退出码={}，发布物交接未完成；{}",
                    journal
                        .exit_code
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "未知".to_owned()),
                    safe_transfer_summary(detail)
                )
            }),
        data: journal.result_data.clone(),
    }
}

fn safe_transfer_summary(detail: &serde_json::Value) -> String {
    let stage = detail
        .get("stage")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let category = detail
        .get("category")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    // Journal 可能来自旧版本，仅允许输出当前白名单，不能把未知字符串当日志。
    let stage = match stage {
        "upload_init"
        | "upload_chunk"
        | "upload_status"
        | "upload_finalize"
        | "access_prepare"
        | "archive_prepare"
        | "upload_authorization"
        | "configuration"
        | "transfer" => stage,
        _ => "unknown",
    };
    let category = match category {
        "connect" | "timeout" | "body" | "decode" | "http_rejected" | "authorization"
        | "verification" | "unavailable" | "deadline" | "canceled" | "invalid_response" | "io" => {
            category
        }
        _ => "unknown",
    };
    let number = |key| {
        detail
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "未知".to_owned())
    };
    let mut summary = format!(
        "stage={stage} category={category} http_status={} attempts={} elapsed_ms={} confirmed_offset={}",
        number("http_status"),
        number("attempts"),
        number("elapsed_ms"),
        number("confirmed_offset")
    );
    if let Some(cause) = detail.get("cause").filter(|cause| cause.is_object()) {
        // 最多显示一层因果，避免不可信递归结构扩大摘要。
        let mut cause = cause.clone();
        cause.as_object_mut().unwrap().remove("cause");
        summary.push_str(&format!("；原始失败: {}", safe_transfer_summary(&cause)));
    }
    summary
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

fn materialize_platform_checkout(
    artifact_dir: &Path,
    checkout_dir: &Path,
    modules: &[String],
) -> Result<(), ()> {
    let [module] = modules else {
        return Err(());
    };
    let archive_path = artifact_dir.join(module).join("template.tar.gz");
    let archive = fs::File::open(archive_path).map_err(|_| ())?;
    let mut archive = tar::Archive::new(GzDecoder::new(archive));
    let temporary = checkout_dir.with_extension("platform-checkout");
    if temporary.exists() {
        fs::remove_dir_all(&temporary).map_err(|_| ())?;
    }
    fs::create_dir_all(temporary.join("scripts")).map_err(|_| ())?;

    let mut found = std::collections::HashSet::new();
    let mut archive_files = 0_usize;
    let mut archive_bytes = 0_u64;
    for entry in archive.entries().map_err(|_| ())? {
        let mut entry = entry.map_err(|_| ())?;
        archive_files = archive_files.checked_add(1).ok_or(())?;
        archive_bytes = archive_bytes.checked_add(entry.size()).ok_or(())?;
        if archive_files > PLATFORM_CHECKOUT_MAX_ARCHIVE_FILES
            || archive_bytes > PLATFORM_CHECKOUT_MAX_ARCHIVE_BYTES
        {
            return Err(());
        }
        let path = entry.path().map_err(|_| ())?;
        let Some(path) = path.to_str() else {
            return Err(());
        };
        if path.is_empty()
            || Path::new(path).is_absolute()
            || Path::new(path)
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(());
        }
        if !entry.header().entry_type().is_file() {
            return Err(());
        }
        let Some((destination, mode)) = (match path {
            "Makefile" => Some((temporary.join("Makefile"), 0o644)),
            "scripts/release.sh" => Some((temporary.join("scripts/release.sh"), 0o755)),
            "deploy-go.yaml" => Some((temporary.join("deploy-go.yaml"), 0o644)),
            _ => None,
        }) else {
            continue;
        };
        if entry.size() > PLATFORM_CHECKOUT_MAX_FILE_BYTES || !found.insert(path.to_owned()) {
            return Err(());
        }
        let mut output = fs::File::create(&destination).map_err(|_| ())?;
        io::copy(&mut entry, &mut output).map_err(|_| ())?;
        output.sync_all().map_err(|_| ())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&destination, fs::Permissions::from_mode(mode)).map_err(|_| ())?;
        }
    }
    if found
        != std::collections::HashSet::from([
            "Makefile".to_owned(),
            "scripts/release.sh".to_owned(),
            "deploy-go.yaml".to_owned(),
        ])
    {
        return Err(());
    }
    if !checkout_dir.exists() {
        return fs::rename(&temporary, checkout_dir).map_err(|_| ());
    }

    let previous = checkout_dir.with_extension("platform-checkout-previous");
    if previous.exists() {
        fs::remove_dir_all(&previous).map_err(|_| ())?;
    }
    fs::rename(checkout_dir, &previous).map_err(|_| ())?;
    if fs::rename(&temporary, checkout_dir).is_err() {
        let _ = fs::rename(&previous, checkout_dir);
        return Err(());
    }
    fs::remove_dir_all(previous).map_err(|_| ())
}

fn materialize_workspace_checkout(
    artifact_dir: &Path,
    checkout_dir: &Path,
    limits: &crate::staging::StagingLimits,
) -> Result<(), ()> {
    let archive_path = artifact_dir.join(RESERVED_WORKSPACE_ARTIFACT);
    let file = fs::File::open(&archive_path).map_err(|_| ())?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let temporary = checkout_dir.with_extension("workspace-checkout");
    if temporary.exists() {
        fs::remove_dir_all(&temporary).map_err(|_| ())?;
    }
    fs::create_dir_all(&temporary).map_err(|_| ())?;
    let mut seen = std::collections::HashSet::new();
    let mut files = 0_usize;
    let mut bytes = 0_u64;
    #[cfg(unix)]
    let mut directory_modes = Vec::new();
    for entry in archive.entries().map_err(|_| ())? {
        let mut entry = entry.map_err(|_| ())?;
        files = files.checked_add(1).ok_or(())?;
        bytes = bytes.checked_add(entry.size()).ok_or(())?;
        if files > limits.max_files || bytes > limits.size_limit_bytes {
            return Err(());
        }
        let path = entry.path().map_err(|_| ())?;
        let Some(path) = path.to_str() else {
            return Err(());
        };
        if path.is_empty()
            || Path::new(path).is_absolute()
            || Path::new(path)
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
            || !seen.insert(path.to_owned())
        {
            return Err(());
        }
        let header = entry.header();
        if header.entry_type().is_symlink()
            || (!header.entry_type().is_file() && !header.entry_type().is_dir())
        {
            return Err(());
        }
        let mode = header.mode().map_err(|_| ())?;
        let destination = temporary.join(path);
        if header.entry_type().is_dir() {
            fs::create_dir_all(&destination).map_err(|_| ())?;
            #[cfg(unix)]
            directory_modes.push((destination.clone(), mode & 0o777));
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|_| ())?;
        }
        let mut output = fs::File::create(&destination).map_err(|_| ())?;
        io::copy(&mut entry, &mut output).map_err(|_| ())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&destination, fs::Permissions::from_mode(mode & 0o777))
                .map_err(|_| ())?;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (path, mode) in directory_modes {
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).map_err(|_| ())?;
        }
    }
    if !checkout_dir.exists() {
        return fs::rename(&temporary, checkout_dir).map_err(|_| ());
    }
    let previous = checkout_dir.with_extension("workspace-checkout-previous");
    if previous.exists() {
        fs::remove_dir_all(&previous).map_err(|_| ())?;
    }
    fs::rename(checkout_dir, &previous).map_err(|_| ())?;
    if fs::rename(&temporary, checkout_dir).is_err() {
        let _ = fs::rename(&previous, checkout_dir);
        return Err(());
    }
    fs::remove_dir_all(previous).map_err(|_| ())
}

fn derived_release_task(task: &DeploymentReleaseTask, task_dir: &Path) -> DeploymentReleaseTask {
    let mut derived = task.clone();
    derived.work_root = task_dir.to_string_lossy().into_owned();
    derived.checkout_dir = task_dir.join("checkout").to_string_lossy().into_owned();
    derived.artifact_dir = task_dir.join("staging").to_string_lossy().into_owned();
    derived.cancel_file = task_dir.join("cancel").to_string_lossy().into_owned();
    derived
}

#[cfg(test)]
mod platform_checkout_tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use tar::{Builder, EntryType, Header};

    fn append_file_with_mode<W: io::Write>(
        archive: &mut Builder<W>,
        path: &str,
        content: &[u8],
        mode: u32,
    ) {
        let mut header = Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(mode);
        header.set_cksum();
        archive.append_data(&mut header, path, content).unwrap();
    }

    fn append_file<W: io::Write>(archive: &mut Builder<W>, path: &str, content: &[u8]) {
        append_file_with_mode(archive, path, content, 0o644);
    }

    fn write_template_archive(
        module_dir: &Path,
        entries: &[(&str, &[u8])],
        include_directory: bool,
    ) {
        let archive = fs::File::create(module_dir.join("template.tar.gz")).unwrap();
        let mut archive = Builder::new(GzEncoder::new(archive, Compression::default()));
        for (path, content) in entries {
            append_file(&mut archive, path, content);
        }
        if include_directory {
            let mut header = Header::new_gnu();
            header.set_entry_type(EntryType::Directory);
            header.set_size(0);
            header.set_mode(0o755);
            header.set_cksum();
            archive
                .append_data(&mut header, "unexpected", io::empty())
                .unwrap();
        }
        archive.finish().unwrap();
        archive.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn artifact_checkout_rejects_missing_fixed_files() {
        let directory = tempfile::tempdir().unwrap();
        let artifact_dir = directory.path().join("artifact");
        let module_dir = artifact_dir.join("redis");
        fs::create_dir_all(&module_dir).unwrap();
        write_template_archive(&module_dir, &[("Makefile", b"all:\n\ttrue\n")], false);

        let checkout = directory.path().join("checkout");
        assert!(
            materialize_platform_checkout(&artifact_dir, &checkout, &["redis".into()]).is_err()
        );
        assert!(!checkout.exists());
    }

    #[test]
    fn artifact_checkout_rejects_duplicate_or_nonregular_entries() {
        let directory = tempfile::tempdir().unwrap();
        let artifact_dir = directory.path().join("artifact");
        let module_dir = artifact_dir.join("redis");
        fs::create_dir_all(&module_dir).unwrap();
        let entries = [
            ("Makefile", b"all:\n\ttrue\n".as_slice()),
            ("scripts/release.sh", b"#!/bin/sh\nexit 0\n".as_slice()),
            ("deploy-go.yaml", b"type: redis\n".as_slice()),
            ("Makefile", b"duplicate\n".as_slice()),
        ];
        write_template_archive(&module_dir, &entries, false);

        let checkout = directory.path().join("checkout");
        assert!(
            materialize_platform_checkout(&artifact_dir, &checkout, &["redis".into()]).is_err()
        );
        assert!(!checkout.exists());

        write_template_archive(&module_dir, &entries[..3], true);
        assert!(
            materialize_platform_checkout(&artifact_dir, &checkout, &["redis".into()]).is_err()
        );
        assert!(!checkout.exists());
    }

    #[test]
    fn artifact_checkout_replaces_an_existing_checkout() {
        let directory = tempfile::tempdir().unwrap();
        let artifact_dir = directory.path().join("artifact");
        let module_dir = artifact_dir.join("redis");
        fs::create_dir_all(&module_dir).unwrap();
        let entries = [
            ("Makefile", b"all:\n\ttrue\n".as_slice()),
            ("scripts/release.sh", b"#!/bin/sh\nexit 0\n".as_slice()),
            ("deploy-go.yaml", b"type: redis\n".as_slice()),
        ];
        write_template_archive(&module_dir, &entries, false);

        let checkout = directory.path().join("checkout");
        fs::create_dir_all(&checkout).unwrap();
        fs::write(checkout.join("stale"), b"stale").unwrap();

        materialize_platform_checkout(&artifact_dir, &checkout, &["redis".into()]).unwrap();

        assert!(checkout.join("Makefile").is_file());
        assert!(!checkout.join("stale").exists());
        assert!(
            !checkout
                .with_extension("platform-checkout-previous")
                .exists()
        );
    }

    fn limits() -> crate::staging::StagingLimits {
        crate::staging::StagingLimits {
            size_limit_bytes: 1024 * 1024,
            max_files: 64,
        }
    }

    fn write_workspace_archive(artifact_dir: &Path, entries: &[(&str, &[u8])]) {
        let entries_with_modes: Vec<(&str, &[u8], u32)> = entries
            .iter()
            .map(|(path, content)| (*path, *content, 0o644))
            .collect();
        write_workspace_archive_with_modes(artifact_dir, &entries_with_modes);
    }

    fn write_workspace_archive_with_modes(artifact_dir: &Path, entries: &[(&str, &[u8], u32)]) {
        let archive = fs::File::create(artifact_dir.join("deploy-go-workspace.tar.gz")).unwrap();
        let mut archive = Builder::new(GzEncoder::new(archive, Compression::default()));
        for (path, content, mode) in entries {
            append_file_with_mode(&mut archive, path, content, *mode);
        }
        archive.finish().unwrap();
        archive.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn workspace_checkout_materializes_archive_and_rejects_unsafe_entries() {
        let directory = tempfile::tempdir().unwrap();
        let artifact_dir = directory.path().join("artifact");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_workspace_archive(
            &artifact_dir,
            &[
                ("Makefile", b"all:\n\ttrue\n".as_slice()),
                ("scripts/deploy.sh", b"#!/bin/sh\nexit 0\n".as_slice()),
            ],
        );
        let checkout = directory.path().join("checkout");
        materialize_workspace_checkout(&artifact_dir, &checkout, &limits()).unwrap();
        assert!(checkout.join("Makefile").is_file());
        assert!(checkout.join("scripts/deploy.sh").is_file());

        write_workspace_archive(
            &artifact_dir,
            &[
                ("Makefile", b"all:\n\ttrue\n".as_slice()),
                ("Makefile", b"duplicate\n".as_slice()),
            ],
        );
        fs::remove_dir_all(&checkout).unwrap();
        assert!(materialize_workspace_checkout(&artifact_dir, &checkout, &limits()).is_err());
        assert!(!checkout.exists());
    }

    #[test]
    fn workspace_checkout_restores_file_execute_permissions() {
        let directory = tempfile::tempdir().unwrap();
        let artifact_dir = directory.path().join("artifact");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_workspace_archive_with_modes(
            &artifact_dir,
            &[(
                "scripts/deploy.sh",
                b"#!/bin/sh\nexit 0\n".as_slice(),
                0o755,
            )],
        );
        let checkout = directory.path().join("checkout");
        materialize_workspace_checkout(&artifact_dir, &checkout, &limits()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(checkout.join("scripts/deploy.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o755);
        }
    }

    #[cfg(unix)]
    #[test]
    fn workspace_archive_rejects_hard_links() {
        let directory = tempfile::tempdir().unwrap();
        let workspace = directory.path().join("workspace");
        fs::create_dir_all(workspace.join("scripts")).unwrap();
        fs::write(workspace.join("scripts/deploy.sh"), b"#!/bin/sh\nexit 0\n").unwrap();
        fs::hard_link(
            workspace.join("scripts/deploy.sh"),
            workspace.join("scripts/deploy-link.sh"),
        )
        .unwrap();
        let archive = fs::File::create(directory.path().join("workspace.tar.gz")).unwrap();
        let mut encoder = GzEncoder::new(archive, Compression::default());
        let mut builder = tar::Builder::new(&mut encoder);
        assert!(
            TaskHandler::append_workspace_tree(&mut builder, &workspace, Path::new("")).is_err()
        );
    }

    #[test]
    fn workspace_checkout_replaces_an_existing_checkout() {
        let directory = tempfile::tempdir().unwrap();
        let artifact_dir = directory.path().join("artifact");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_workspace_archive(&artifact_dir, &[("Makefile", b"all:\n\ttrue\n".as_slice())]);
        let checkout = directory.path().join("checkout");
        fs::create_dir_all(&checkout).unwrap();
        fs::write(checkout.join("stale"), b"stale").unwrap();
        materialize_workspace_checkout(&artifact_dir, &checkout, &limits()).unwrap();
        assert!(checkout.join("Makefile").is_file());
        assert!(!checkout.join("stale").exists());
    }
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
        log_delivery_version: None,
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

fn resume_prepare_transfer(journal: &TaskJournal) -> bool {
    journal.result_sequence.is_none()
        && (journal.state == JournalState::Succeeded
            || journal.transfer_phase == Some(crate::journal::TransferPhase::PrepareUpload))
}

fn prepare_transfer_pending(journal: &TaskJournal, executor: &Executor) -> bool {
    if journal.result_sequence.is_some() {
        return false;
    }
    let Ok(bytes) = fs::read(executor.task_dir(&journal.task_id).join("runner-spec.json")) else {
        return false;
    };
    let Ok(spec) = serde_json::from_slice::<crate::runner::RunnerSpec>(&bytes) else {
        return false;
    };
    spec.two_stage
        .as_ref()
        .is_some_and(|two_stage| two_stage.stage == DeploymentStage::Prepare)
}

fn deadline_expired(deadline: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(deadline).map_or(true, |deadline| deadline <= Utc::now())
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

async fn perform_runtime_probe(
    task: &RuntimeProbeTask,
    shared_client: Option<&reqwest::Client>,
) -> Result<serde_json::Value, &'static str> {
    if !task.validate() {
        return Err("invalid_runtime_probe_payload");
    }
    let client = match shared_client {
        Some(client) => client.clone(),
        None => crate::http_client::new_runtime_probe_client(),
    };
    let observed_at = Utc::now().to_rfc3339();
    match task.probe_type {
        RuntimeProbeType::Http => {
            let path = task.path.as_deref().unwrap_or("/");
            let url = url::Url::parse(&format!("http://127.0.0.1:{}{}", task.port, path))
                .map_err(|_| "runtime_probe_url_invalid")?;
            let timeout = Duration::from_millis(u64::from(task.timeout_ms));
            let response = tokio::time::timeout(timeout, client.get(url).timeout(timeout).send())
                .await
                .map_err(|_| "runtime_probe_http_timeout")?
                .map_err(|_| "runtime_probe_http_failed")?;
            let status = response.status().as_u16();
            if Some(status) != task.expected_status {
                return Err("runtime_probe_http_unexpected_status");
            }
            Ok(json!({
                "checked_at": observed_at,
                "http_status": status,
                "port": task.port
            }))
        }
        RuntimeProbeType::Tcp => {
            let timeout = Duration::from_millis(u64::from(task.timeout_ms));
            tokio::time::timeout(
                timeout,
                tokio::net::TcpStream::connect(("127.0.0.1", task.port)),
            )
            .await
            .map_err(|_| "runtime_probe_tcp_timeout")?
            .map_err(|_| "runtime_probe_tcp_connect_failed")?;
            Ok(json!({
                "checked_at": observed_at,
                "port": task.port
            }))
        }
    }
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
mod prepare_transfer_resume_tests {
    use super::{
        Executor, JournalState, PreparedArtifactTransferError, TaskJournal,
        prepare_transfer_pending, result_for, resume_prepare_transfer,
    };
    use crate::journal::TransferPhase;

    fn journal(
        state: JournalState,
        result_sequence: Option<u64>,
        transfer_phase: Option<TransferPhase>,
    ) -> TaskJournal {
        TaskJournal {
            task_id: "task_resume".into(),
            idempotency_key: "idem_resume_0123456789".into(),
            payload_digest: "sha256:0123456789abcdef".into(),
            deployment_id: None,
            target_run_id: None,
            state,
            pid: None,
            process_start_time: None,
            stdout_offset: 0,
            stderr_offset: 0,
            events_offset: 0,
            events_sent: 0,
            last_sequence: 0,
            result_sequence,
            git_lease_id: None,
            exit_code: Some(0),
            error_code: None,
            result_data: None,
            transfer_phase,
            external_output_sequence: 0,
        }
    }

    #[test]
    fn succeeded_without_result_resumes_upload_after_reconnect() {
        let current = journal(JournalState::Succeeded, None, None);
        assert!(resume_prepare_transfer(&current));
    }

    #[test]
    fn succeeded_with_result_replays_terminal_state() {
        let current = journal(JournalState::Succeeded, Some(7), None);
        assert!(!resume_prepare_transfer(&current));
    }

    #[test]
    fn failed_without_result_replays_failure() {
        let current = journal(JournalState::Failed, None, None);
        assert!(!resume_prepare_transfer(&current));
    }

    #[test]
    fn prepare_upload_phase_always_resumes_before_sending_result() {
        let current = journal(
            JournalState::Running,
            None,
            Some(TransferPhase::PrepareUpload),
        );
        assert!(resume_prepare_transfer(&current));
    }

    #[tokio::test]
    async fn transfer_failure_is_durable_and_replays_same_safe_summary_with_exit_zero() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_transfer",
                "idem_transfer_0123456789",
                "sha256:transfer_fixture",
            )
            .await
            .unwrap();
        let mut current = executor.load("task_transfer").unwrap();
        current.state = JournalState::Running;
        current.exit_code = Some(0);
        current.transfer_phase = Some(TransferPhase::PrepareUpload);
        executor.store_journal(&current).unwrap();
        let error = PreparedArtifactTransferError::UploadTimeout;
        let completed = executor
            .complete_task(
                "task_transfer",
                JournalState::Failed,
                Some(error.code().into()),
                Some(error.data(5200)),
            )
            .unwrap();
        let first = result_for(&completed, 7);
        let loaded = executor.load("task_transfer").unwrap();
        assert!(!resume_prepare_transfer(&loaded));
        let replayed = result_for(&loaded, 7);
        assert_eq!(first, replayed);
        assert_eq!(first.exit_code, Some(0));
        assert_eq!(
            first.error_code.as_deref(),
            Some("artifact_transfer_timeout")
        );
        assert!(first.summary.unwrap().contains("category=timeout"));
        assert_eq!(first.data.unwrap()["transfer_elapsed_ms"], 5200);
    }

    #[tokio::test]
    async fn prepare_runner_spec_marks_unfinished_task_for_redispatch() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_prepare_pending",
                "idem_prepare_pending_012345",
                "sha256:prepare_pending",
            )
            .await
            .unwrap();
        let mut journal = executor.load("task_prepare_pending").unwrap();
        journal.state = JournalState::Succeeded;
        executor.store_journal(&journal).unwrap();
        std::fs::write(
            executor
                .task_dir("task_prepare_pending")
                .join("runner-spec.json"),
            serde_json::to_vec(&serde_json::json!({
                "deployment_id": "deployment_prepare_pending",
                "script_path": "make",
                "argument_tokens": [],
                "environment_file_references": [],
                "timeout_seconds": 60,
                "log_budget_bytes": 1024,
                "two_stage": {
                    "stage": "prepare",
                    "checkout_dir": "/tmp/checkout",
                    "work_root": "/tmp/work",
                    "repository_url": null,
                    "commit_sha": "abc",
                    "credential_file": null,
                    "environment": "test",
                    "release_version": "1",
                    "target_code": null,
                    "modules": ["api"],
                    "artifact_dir": "/tmp/out",
                    "staging_size_limit_bytes": 1024,
                    "staging_max_files": 10,
                    "git_lease_id": null
                }
            }))
            .unwrap(),
        )
        .unwrap();

        assert!(prepare_transfer_pending(
            &executor.load("task_prepare_pending").unwrap(),
            &executor,
        ));
    }
}

#[cfg(test)]
mod transfer_output_tests {
    use super::*;

    #[tokio::test]
    async fn receipt_does_not_block_connection_while_replay_holds_event_lock() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor.enable_reliable_logs(true);
        let mut journal = executor
            .create_task(
                "task_receipt",
                "idem_receipt_0123456789",
                "sha256:receipt_0123456789",
            )
            .await
            .unwrap();
        journal.last_sequence = 1;
        let message = Message::TaskOutput(TaskOutput {
            task_id: journal.task_id.clone(),
            sequence: 1,
            stream: OutputStream::Stdout,
            text: "line".into(),
        });
        crate::log_delivery::persist(&executor.task_dir(&journal.task_id), &message, &journal)
            .unwrap();
        let receipt = deploy_go_agent_protocol::TaskEventReceipt {
            task_id: journal.task_id.clone(),
            payload_digest: journal.payload_digest.clone(),
            sequence: 1,
            message_digest: crate::log_delivery::message_digest(&message).unwrap(),
        };
        let handler = TaskHandler::new(executor);
        let guard = handler.event_lock.lock().await;
        let (sender, _receiver) = mpsc::channel(1);
        let envelope = Envelope {
            protocol_version: 18,
            message_id: "receipt".into(),
            sent_at: Utc::now().to_rfc3339(),
            message: Message::TaskEventReceipt(receipt),
        };
        tokio::time::timeout(Duration::from_millis(250), handler.handle(envelope, sender))
            .await
            .unwrap()
            .unwrap();
        drop(guard);
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if crate::log_delivery::pending_messages(
                    &handler.executor.task_dir(&journal.task_id),
                )
                .unwrap()
                .is_empty()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            handler
                .executor
                .load(&journal.task_id)
                .unwrap()
                .last_sequence,
            1
        );
    }

    #[tokio::test]
    async fn transfer_lines_are_streamed_without_replaying_stdout_log() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_transfer_lines",
                "idem_transfer_lines_0123456789",
                "sha256:transfer_lines",
                crate::journal::TransferPhase::ReleaseDownload,
            )
            .await
            .unwrap();
        let (outbound, mut received) = mpsc::channel(8);
        emit_transfer_line(
            &executor,
            &Mutex::new(()),
            &outbound,
            "task_transfer_lines",
            "开始下载",
        )
        .await
        .unwrap();
        emit_transfer_line(
            &executor,
            &Mutex::new(()),
            &outbound,
            "task_transfer_lines",
            "下载完成",
        )
        .await
        .unwrap();

        let first = received.recv().await.unwrap();
        let Message::TaskOutput(first) = first else {
            panic!("expected task output, got {first:?}");
        };
        assert_eq!(first.task_id, "task_transfer_lines");
        assert_eq!(first.sequence, 1);
        assert_eq!(first.text, "开始下载\n");
        let second = received.recv().await.unwrap();
        let Message::TaskOutput(second) = second else {
            panic!("expected task output, got {second:?}");
        };
        assert_eq!(second.sequence, 2);
        assert_eq!(second.text, "下载完成\n");

        let journal = executor.load("task_transfer_lines").unwrap();
        assert_eq!(journal.last_sequence, 2);
        assert!(
            !executor
                .task_dir("task_transfer_lines")
                .join("stdout.log")
                .exists()
        );
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
    use std::sync::atomic::{AtomicUsize, Ordering};
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
            checkout_mode: ReleaseCheckoutMode::Git,
            source_materialization: None,
            secret_environment: None,
        }
    }

    fn facts_fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        // macOS 的临时目录经过 /var -> /private/var 符号链接，准入校验会拒绝根路径本身。
        let root = directory.path().canonicalize().unwrap();
        let checkout = root.join("work/checkout");
        let artifact = root.join("work/artifact");
        let env = root.join("work/env");
        for path in [&checkout, &artifact, &env] {
            std::fs::create_dir_all(path).unwrap();
        }
        std::fs::write(checkout.join("Makefile"), b"deploy-go-release:\n").unwrap();
        std::fs::write(
            artifact.join(deploy_go_agent_executor::release::ARTIFACT_MANIFEST),
            br#"{"artifacts":[]}"#,
        )
        .unwrap();
        (directory, checkout, artifact, env)
    }

    fn facts_context() -> deploy_go_agent_protocol::PrivilegedReleaseContext {
        deploy_go_agent_protocol::PrivilegedReleaseContext {
            target_run_id: "run".into(),
            target_id: "target".into(),
            node_id: "node".into(),
            agent_id: "agent".into(),
            snapshot_hash: "snapshot".into(),
        }
    }

    #[test]
    fn release_facts_use_specific_codes_for_each_admission_failure() {
        let (_guard, checkout, artifact, env) = facts_fixture();
        let task = bridge_task();
        let call = || {
            privileged_release_facts("task", &task, &facts_context(), &checkout, &artifact, &env)
                .map(|_| ())
        };
        assert_eq!(call(), Ok(()), "干净的 checkout 与合法 manifest 必须通过");

        // 业务仓库提交了符号链接：这是 2026-10-13 现场失败的原因。
        std::os::unix::fs::symlink("../elsewhere", checkout.join("link")).unwrap();
        assert_eq!(call(), Err(RELEASE_CHECKOUT_UNSAFE_ENTRY.to_owned()));
        std::fs::remove_file(checkout.join("link")).unwrap();

        std::fs::write(
            artifact.join(deploy_go_agent_executor::release::ARTIFACT_MANIFEST),
            b"not json",
        )
        .unwrap();
        assert_eq!(call(), Err(RELEASE_MANIFEST_INVALID.to_owned()));
    }

    #[tokio::test]
    async fn admission_failure_report_names_the_rejected_entries_in_task_output() {
        let (guard, checkout, _artifact, _env) = facts_fixture();
        std::fs::create_dir_all(checkout.join(".agents/skills")).unwrap();
        std::os::unix::fs::symlink(
            "../../docs",
            checkout.join(".agents/skills/project-test-deploy"),
        )
        .unwrap();
        let executor = Executor::new(guard.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_admission_report",
                "idem_admission_report_01234",
                "sha256:admission_report",
            )
            .await
            .unwrap();
        let handler = TaskHandler::new(executor);
        let (outbound, mut received) = mpsc::channel(16);

        handler
            .report_release_admission_failure(
                "task_admission_report",
                RELEASE_CHECKOUT_UNSAFE_ENTRY,
                &checkout,
                &outbound,
            )
            .await;
        drop(outbound);

        let mut text = String::new();
        while let Some(message) = received.recv().await {
            if let Message::TaskOutput(output) = message {
                text.push_str(&output.text);
            }
        }
        assert!(text.contains("发布准入被拒绝"), "{text}");
        assert!(text.contains("符号链接"), "{text}");
        assert!(
            text.contains(".agents/skills/project-test-deploy"),
            "{text}"
        );
        assert!(
            !text.contains(checkout.to_string_lossy().as_ref()),
            "不得泄露节点上的绝对路径：{text}"
        );
    }

    #[tokio::test]
    async fn release_identity_is_durable_with_privileged_phase() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_release_context",
                "idem_release_context_012345",
                "sha256:release_context",
            )
            .await
            .unwrap();

        let journal = executor
            .begin_privileged_release(
                "task_release_context",
                "deployment_release_context",
                "run_release_context",
            )
            .unwrap();

        assert_eq!(journal.state, JournalState::Running);
        assert_eq!(
            journal.transfer_phase,
            Some(crate::journal::TransferPhase::PrivilegedRelease)
        );
        assert_eq!(
            journal.deployment_id.as_deref(),
            Some("deployment_release_context")
        );
        assert_eq!(
            journal.target_run_id.as_deref(),
            Some("run_release_context")
        );
        assert_eq!(
            executor.load("task_release_context").unwrap(),
            journal,
            "release identity must be durable before executor start"
        );
    }

    #[tokio::test]
    async fn legacy_release_descriptor_backfills_associations_durably() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        let journal = executor
            .create_transfer_task(
                "task_descriptor_backfill",
                "idem_descriptor_backfill_0123",
                "sha256:descriptor_backfill",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let mut task = bridge_task();
        task.privileged_context = Some(deploy_go_agent_protocol::PrivilegedReleaseContext {
            target_run_id: "run_descriptor_backfill".into(),
            target_id: "target".into(),
            node_id: "node".into(),
            agent_id: "agent".into(),
            snapshot_hash: "snapshot".into(),
        });
        std::fs::write(
            executor
                .task_dir(&journal.task_id)
                .join("privileged-release-task.json"),
            serde_json::to_vec(&task).unwrap(),
        )
        .unwrap();
        let handler = TaskHandler::new(executor.clone());

        assert!(matches!(
            handler.load_privileged_release_descriptor(&journal, &journal.task_id),
            PrivilegedReleaseDescriptorState::Valid(_)
        ));
        let recovered = executor.load(&journal.task_id).unwrap();
        assert_eq!(recovered.deployment_id.as_deref(), Some("deployment"));
        assert_eq!(
            recovered.target_run_id.as_deref(),
            Some("run_descriptor_backfill")
        );
    }

    #[tokio::test]
    async fn waiting_monitor_takes_ownership_after_current_owner_exits() {
        use deploy_go_agent_executor::protocol::{
            ReleaseExitedResponse, ReleaseJobState, ReleaseOutputResponse, Request, Response,
            read_request, write_message,
        };
        use tokio::net::UnixListener;

        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_monitor_handoff",
                "idem_monitor_handoff_012345",
                "sha256:monitor_handoff",
            )
            .await
            .unwrap();
        executor
            .begin_privileged_release("task_monitor_handoff", "deployment", "run")
            .unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let (accepted_tx, accepted_rx) = oneshot::channel();
        let mut accepted_rx = accepted_rx;
        let server = tokio::spawn(async move {
            let (mut output, _) = listener.accept().await.unwrap();
            let request = read_request(&mut output, MAX_FRAME_BYTES)
                .await
                .unwrap()
                .unwrap();
            let Request::ReleaseOutput(request) = request else {
                panic!("handoff must observe output, not start release");
            };
            accepted_tx.send(()).unwrap();
            write_message(
                &mut output,
                &Response::ReleaseOutput(ReleaseOutputResponse {
                    version: PROTOCOL_VERSION,
                    job_id: request.job_id,
                    frames: Vec::new(),
                    truncated: false,
                }),
                MAX_FRAME_BYTES,
            )
            .await
            .unwrap();
            let (mut status, _) = listener.accept().await.unwrap();
            let request = read_request(&mut status, MAX_FRAME_BYTES)
                .await
                .unwrap()
                .unwrap();
            let Request::ReleaseStatus(request) = request else {
                panic!("handoff must observe status, not start release");
            };
            write_message(
                &mut status,
                &Response::ReleaseExited(ReleaseExitedResponse {
                    version: PROTOCOL_VERSION,
                    job_id: request.job_id,
                    state: ReleaseJobState::Failed,
                    exit_code: Some(1),
                    reason: "nonzero_exit".into(),
                    last_sequence: 0,
                }),
                MAX_FRAME_BYTES,
            )
            .await
            .unwrap();
        });
        let handler = TaskHandler::new(executor.clone())
            .with_privileged_release_executor(ExecutorClient::new(socket));
        let lock = handler
            .privileged_monitor_lock("task_monitor_handoff")
            .await;
        let current_owner = lock.lock().await;
        let (outbound, mut received) = mpsc::channel(32);
        let waiter = tokio::spawn({
            let handler = handler.clone();
            let outbound = outbound.clone();
            async move {
                handler
                    .run_privileged_release_monitor(
                        "task_monitor_handoff".into(),
                        "sha256:monitor_handoff".into(),
                        Some(bridge_task()),
                        outbound,
                    )
                    .await;
            }
        });

        assert!(
            tokio::time::timeout(Duration::from_millis(30), &mut accepted_rx)
                .await
                .is_err()
        );
        drop(current_owner);
        tokio::time::timeout(Duration::from_secs(2), &mut accepted_rx)
            .await
            .expect("waiting monitor should acquire released ownership")
            .unwrap();
        let mut messages = Vec::new();
        loop {
            let message = tokio::time::timeout(Duration::from_secs(2), received.recv())
                .await
                .expect("monitor handoff result timed out")
                .expect("monitor channel closed before result");
            let terminal = matches!(message, Message::TaskResult(_));
            messages.push(message);
            if terminal {
                break;
            }
        }
        waiter.await.unwrap();
        server.await.unwrap();
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(message, Message::TaskResult(_)))
                .count(),
            1
        );
        assert_eq!(
            executor.load("task_monitor_handoff").unwrap().state,
            JournalState::Failed
        );
    }

    #[tokio::test]
    async fn reconcile_does_not_block_on_an_active_privileged_monitor_owner() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_reconcile_owned",
                "idem_reconcile_owned_01",
                "sha256:abcdef0123456789",
            )
            .await
            .unwrap();
        // 与生产一致：ReleaseStart 之前先把 Running + PrivilegedRelease 与关联标识落盘。
        executor
            .begin_privileged_release("task_reconcile_owned", "deployment", "run")
            .unwrap();
        let handler = TaskHandler::new(executor);
        // 模拟仍在运行的特权 monitor：它会一直持有所有权直到任务终态。
        let owner = handler
            .privileged_monitor_lock("task_reconcile_owned")
            .await;
        let _current_owner = owner.lock().await;
        let (outbound, mut received) = mpsc::channel(8);

        let reconcile = tokio::spawn({
            let handler = handler.clone();
            let outbound = outbound.clone();
            async move {
                handler
                    .reconcile(vec!["task_reconcile_owned".into()], outbound)
                    .await
            }
        });

        let message = tokio::time::timeout(Duration::from_secs(2), received.recv())
            .await
            .expect("reconcile must not wait for an active monitor to finish")
            .expect("reconcile channel closed before report");
        let Message::ReconcileReport(report) = message else {
            panic!("expected reconcile report");
        };
        assert_eq!(report.tasks.len(), 1);
        // 带 transfer_phase 的任务（含特权 release）一律上报 Accepted，由主控幂等重派并以
        // Duplicate ACK 重新挂接 monitor；即使已有 owner 也不能人为降级成 Running 阻止重派。
        assert_eq!(report.tasks[0].state, ReconciledTaskState::Accepted);
        assert!(report.tasks[0].result.is_none());
        reconcile.await.unwrap();
    }

    /// 带完整关联标识的 descriptor（deployment 固定为 "deployment"，target run 由调用方指定）。
    fn write_bridge_descriptor(executor: &Executor, task_id: &str, target_run_id: &str) {
        let mut task = bridge_task();
        task.privileged_context = Some(deploy_go_agent_protocol::PrivilegedReleaseContext {
            target_run_id: target_run_id.into(),
            target_id: "target".into(),
            node_id: "node".into(),
            agent_id: "agent".into(),
            snapshot_hash: "snapshot".into(),
        });
        std::fs::write(
            executor
                .task_dir(task_id)
                .join("privileged-release-task.json"),
            serde_json::to_vec(&task).unwrap(),
        )
        .unwrap();
    }

    /// executor 已终态（非零退出）且从不提供输出帧：ReleaseOutput 恒返回空页，ReleaseStatus 恒
    /// 返回 Exited 并宣告 `last_sequence`。`outputs` 统计输出请求，`starts` 统计本不该出现的
    /// ReleaseStart。
    async fn serve_exited_without_output(
        listener: UnixListener,
        outputs: Arc<AtomicUsize>,
        starts: Arc<AtomicUsize>,
        last_sequence: u64,
    ) {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let Some(request) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap() else {
                continue;
            };
            match request {
                Request::ReleaseOutput(request) => {
                    outputs.fetch_add(1, Ordering::SeqCst);
                    write_message(
                        &mut stream,
                        &Response::ReleaseOutput(ReleaseOutputResponse {
                            version: PROTOCOL_VERSION,
                            job_id: request.job_id,
                            frames: Vec::new(),
                            truncated: false,
                        }),
                        MAX_FRAME_BYTES,
                    )
                    .await
                    .unwrap();
                }
                Request::ReleaseStatus(request) => {
                    write_message(
                        &mut stream,
                        &Response::ReleaseExited(ReleaseExitedResponse {
                            version: PROTOCOL_VERSION,
                            job_id: request.job_id,
                            state: ReleaseJobState::Failed,
                            exit_code: Some(1),
                            reason: "nonzero_exit".into(),
                            last_sequence,
                        }),
                        MAX_FRAME_BYTES,
                    )
                    .await
                    .unwrap();
                }
                Request::ReleaseStart(_) => {
                    starts.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        }
    }

    async fn receive_task_result(
        received: &mut mpsc::Receiver<Message>,
        wait: Duration,
    ) -> deploy_go_agent_protocol::TaskResult {
        tokio::time::timeout(wait, async {
            loop {
                match received.recv().await {
                    Some(Message::TaskResult(result)) => break result,
                    Some(_) => {}
                    None => panic!("发送通道在收到终态前被关闭"),
                }
            }
        })
        .await
        .expect("等待 TaskResult 超时")
    }

    #[tokio::test]
    async fn reconcile_hands_over_to_a_waiting_monitor_after_the_old_owner_exits() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_reconcile_handover",
                "idem_reconcile_handover_01",
                "sha256:reconcile_handover",
            )
            .await
            .unwrap();
        executor
            .begin_privileged_release("task_reconcile_handover", "deployment", "run")
            .unwrap();
        write_bridge_descriptor(&executor, "task_reconcile_handover", "run");
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let outputs = Arc::new(AtomicUsize::new(0));
        let starts = Arc::new(AtomicUsize::new(0));
        let server = tokio::spawn(serve_exited_without_output(
            listener,
            Arc::clone(&outputs),
            Arc::clone(&starts),
            0,
        ));
        let handler = TaskHandler::new(executor.clone())
            .with_privileged_release_executor(ExecutorClient::new(socket));
        // 旧连接上的 monitor 仍是 owner：新连接的 reconcile 既不能等它，也不能因此漏掉接管。
        let lock = handler
            .privileged_monitor_lock("task_reconcile_handover")
            .await;
        let old_owner = lock.lock().await;
        let (outbound, mut received) = mpsc::channel(32);
        handler
            .reconcile(vec!["task_reconcile_handover".into()], outbound)
            .await;
        assert!(matches!(
            received.recv().await,
            Some(Message::ReconcileReport(_))
        ));
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            outputs.load(Ordering::SeqCst),
            0,
            "旧 owner 仍持有所有权时不得并发监控同一个 job"
        );
        drop(old_owner);

        let result = receive_task_result(&mut received, Duration::from_secs(5)).await;
        server.abort();
        assert_eq!(result.status, TaskTerminalStatus::Failed);
        assert_eq!(starts.load(Ordering::SeqCst), 0, "接管不得再次启动 release");
        assert_eq!(
            executor.load("task_reconcile_handover").unwrap().state,
            JournalState::Failed
        );
    }

    #[tokio::test]
    async fn temporarily_unreadable_descriptor_retries_instead_of_abandoning_the_monitor() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_descriptor_retry",
                "idem_descriptor_retry_01",
                "sha256:descriptor_retry",
            )
            .await
            .unwrap();
        executor
            .begin_privileged_release("task_descriptor_retry", "deployment", "run")
            .unwrap();
        // descriptor 路径被目录占用：fs::read 返回 EISDIR（暂时不可读），而不是 NotFound。
        let descriptor = executor
            .task_dir("task_descriptor_retry")
            .join("privileged-release-task.json");
        std::fs::create_dir(&descriptor).unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let outputs = Arc::new(AtomicUsize::new(0));
        let starts = Arc::new(AtomicUsize::new(0));
        let server = tokio::spawn(serve_exited_without_output(
            listener,
            Arc::clone(&outputs),
            Arc::clone(&starts),
            0,
        ));
        let handler = TaskHandler::new(executor.clone())
            .with_privileged_release_executor(ExecutorClient::new(socket));
        let (outbound, mut received) = mpsc::channel(32);
        handler
            .reconcile(vec!["task_descriptor_retry".into()], outbound)
            .await;
        assert!(matches!(
            received.recv().await,
            Some(Message::ReconcileReport(_))
        ));
        // 跨过多个（测试构建中已压缩的）退避周期：monitor 仍在等待 descriptor 恢复，
        // 既没有放弃，也没有在缺少 descriptor 时抢先查询 executor。
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(outputs.load(Ordering::SeqCst), 0);
        std::fs::remove_dir(&descriptor).unwrap();
        write_bridge_descriptor(&executor, "task_descriptor_retry", "run");

        let result = receive_task_result(&mut received, Duration::from_secs(5)).await;
        server.abort();
        assert_eq!(result.status, TaskTerminalStatus::Failed);
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        assert!(outputs.load(Ordering::SeqCst) >= 1);
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
                Some(task),
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
            Some(bridge_task()),
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

    /// 用例关注"大游标 + 历史帧真实在盘"的接管，不是分页用例；跨页由
    /// terminal_status_waits_until_all_paginated_executor_output_is_persisted 覆盖。
    #[tokio::test]
    async fn monitor_resumes_executor_output_from_durable_sequence_699() {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        // 恢复只允许观察原 job：任何 ReleaseStart 都会让计数大于 0。
        let unexpected_starts = Arc::new(AtomicUsize::new(0));
        let server_starts = Arc::clone(&unexpected_starts);
        let server = tokio::spawn(async move {
            for expected_request in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let request = read_request(&mut stream, MAX_FRAME_BYTES)
                    .await
                    .unwrap()
                    .unwrap();
                let response = match request {
                    Request::ReleaseOutput(request) => {
                        assert_eq!(expected_request, 0, "output must be polled only once");
                        assert_eq!(request.after_sequence, 699);
                        Response::ReleaseOutput(ReleaseOutputResponse {
                            version: PROTOCOL_VERSION,
                            job_id: "release_task_resume_699".into(),
                            frames: (700_u64..=721)
                                .map(|sequence| ReleaseOutputFrame {
                                    sequence,
                                    stream: ReleaseOutputStream::Stdout,
                                    // 协议标记属于恢复点之前的历史输出，接管窗口只是续写。
                                    data: format!("recovered-{sequence}\n").into_bytes(),
                                })
                                .collect(),
                            truncated: false,
                        })
                    }
                    Request::ReleaseStatus(_) => Response::ReleaseExited(ReleaseExitedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: "release_task_resume_699".into(),
                        state: ReleaseJobState::Succeeded,
                        exit_code: Some(0),
                        reason: "process_exited".into(),
                        last_sequence: 721,
                    }),
                    Request::ReleaseStart(_) => {
                        server_starts.fetch_add(1, Ordering::SeqCst);
                        continue;
                    }
                    _ => panic!("recovery must only observe the existing job"),
                };
                write_message(&mut stream, &response, MAX_FRAME_BYTES)
                    .await
                    .unwrap();
            }
        });

        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_resume_699",
                "idem_resume_699_0123456789",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let mut journal = executor.load("task_resume_699").unwrap();
        // 先铺好历史帧再推进游标，才符合"崩溃前已持久化"的真实 durable 布局。
        executor
            .seed_external_output_history(
                "task_resume_699",
                699,
                concat!(
                    "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                    "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n"
                )
                .as_bytes(),
            )
            .unwrap();
        journal.external_output_sequence = 699;
        executor.store_journal(&journal).unwrap();
        let (outbound, mut received) = mpsc::channel(64);
        let monitor = tokio::spawn(monitor_privileged_release(
            Arc::new(ExecutorClient::new(socket)),
            Arc::clone(&executor),
            Arc::new(Mutex::new(())),
            "task_resume_699".into(),
            "sha256:abcdef0123456789".into(),
            Some(bridge_task()),
            outbound,
        ));
        // fixture 或状态机一旦回归成无限重试，这里必须失败而不是挂住整个测试进程。
        tokio::time::timeout(Duration::from_secs(30), async {
            monitor.await.unwrap();
            server.await.unwrap();
        })
        .await
        .expect("monitor must converge instead of retrying forever");

        let journal = executor.load("task_resume_699").unwrap();
        assert_eq!(journal.external_output_sequence, 721);
        assert_eq!(journal.state, JournalState::Succeeded);
        assert_eq!(unexpected_starts.load(Ordering::SeqCst), 0);
        let messages: Vec<_> = std::iter::from_fn(|| received.try_recv().ok()).collect();
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(message, Message::TaskResult(_)))
                .count(),
            1
        );
        assert!(messages.iter().any(
            |message| matches!(message, Message::TaskOutput(output) if output.text.contains("recovered-721"))
        ));
    }

    #[derive(Clone, Copy)]
    enum FrameCorruption {
        Missing,
        Unparsable,
        Symlinked,
        Oversized,
        /// 帧文件路径被目录占用：`fs::read` 返回 EISDIR，同样是布局破坏而不是暂时不可读。
        Directory,
    }

    /// durable 帧目录在最终回执 ACK 前不会被回收，因此这些损坏都不是可恢复的临时状态。
    /// 任一形态回归成 transient 重试都会让本用例超时失败。
    #[tokio::test]
    async fn permanent_frame_corruption_converges_to_interrupted_instead_of_retrying_forever() {
        for corruption in [
            FrameCorruption::Missing,
            FrameCorruption::Unparsable,
            FrameCorruption::Symlinked,
            FrameCorruption::Oversized,
            FrameCorruption::Directory,
        ] {
            run_permanent_frame_corruption(corruption).await;
        }
    }

    async fn run_permanent_frame_corruption(corruption: FrameCorruption) {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        // 只提供一轮交互：若 monitor 把永久缺帧当成临时 I/O 退避重试，超时会直接失败。
        let server = tokio::spawn(async move {
            let (mut output, _) = listener.accept().await.unwrap();
            let request = read_request(&mut output, MAX_FRAME_BYTES)
                .await
                .unwrap()
                .unwrap();
            let Request::ReleaseOutput(request) = request else {
                panic!("recovery must query the existing job output");
            };
            assert_eq!(request.after_sequence, 5);
            write_message(
                &mut output,
                &Response::ReleaseOutput(ReleaseOutputResponse {
                    version: PROTOCOL_VERSION,
                    job_id: "release_task_missing_frame".into(),
                    frames: (6_u64..=7)
                        .map(|sequence| ReleaseOutputFrame {
                            sequence,
                            stream: ReleaseOutputStream::Stdout,
                            data: format!("later-{sequence}\n").into_bytes(),
                        })
                        .collect(),
                    truncated: false,
                }),
                MAX_FRAME_BYTES,
            )
            .await
            .unwrap();
        });

        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_missing_frame",
                "idem_missing_frame_0123",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        executor
            .seed_external_output_history(
                "task_missing_frame",
                5,
                concat!(
                    "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                    "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n"
                )
                .as_bytes(),
            )
            .unwrap();
        let frame_path = executor
            .task_dir("task_missing_frame")
            .join("executor-output-frames")
            .join(format!("{0:020}.json", 3));
        match corruption {
            FrameCorruption::Missing => std::fs::remove_file(&frame_path).unwrap(),
            FrameCorruption::Unparsable => std::fs::write(&frame_path, b"{not-json").unwrap(),
            FrameCorruption::Symlinked => {
                std::fs::remove_file(&frame_path).unwrap();
                std::os::unix::fs::symlink("/dev/null", &frame_path).unwrap();
            }
            FrameCorruption::Oversized => {
                std::fs::write(&frame_path, vec![b'x'; 1024 * 1024 + 1]).unwrap()
            }
            FrameCorruption::Directory => {
                std::fs::remove_file(&frame_path).unwrap();
                std::fs::create_dir(&frame_path).unwrap();
            }
        }
        // durable 游标已推进到 5，但历史帧损坏属于永久完整性违规，不是临时不可写。
        let mut journal = executor.load("task_missing_frame").unwrap();
        journal.external_output_sequence = 5;
        executor.store_journal(&journal).unwrap();

        let (outbound, mut received) = mpsc::channel(64);
        let monitor = tokio::spawn(monitor_privileged_release(
            Arc::new(ExecutorClient::new(socket)),
            Arc::clone(&executor),
            Arc::new(Mutex::new(())),
            "task_missing_frame".into(),
            "sha256:abcdef0123456789".into(),
            Some(bridge_task()),
            outbound,
        ));
        tokio::time::timeout(Duration::from_secs(30), async {
            monitor.await.unwrap();
            server.await.unwrap();
        })
        .await
        .expect("permanent frame loss must not be retried as transient storage failure");

        let journal = executor.load("task_missing_frame").unwrap();
        assert_eq!(journal.state, JournalState::Interrupted);
        assert_eq!(
            journal.error_code.as_deref(),
            Some("release_output_persistence_failed")
        );
        let messages: Vec<_> = std::iter::from_fn(|| received.try_recv().ok()).collect();
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(message, Message::TaskResult(_)))
                .count(),
            1
        );
    }

    #[derive(Clone, Copy)]
    enum SequenceConflict {
        NonContiguousFrame,
        TerminalBehindCursor,
        UnexpectedTruncation,
    }

    /// FR-002/SC-003：序号缺口、终态逆序和未预期的截断都必须收敛为非成功中断，
    /// 既不能等待不存在的帧，也不能推断成功。
    #[tokio::test]
    async fn sequence_and_truncation_conflicts_converge_to_interrupted() {
        for scenario in [
            SequenceConflict::NonContiguousFrame,
            SequenceConflict::TerminalBehindCursor,
            SequenceConflict::UnexpectedTruncation,
        ] {
            run_sequence_conflict(scenario).await;
        }
    }

    async fn run_sequence_conflict(conflict: SequenceConflict) {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let job_id = "release_task_sequence_conflict".to_owned();
        let server_job_id = job_id.clone();
        let server = tokio::spawn(async move {
            let (mut output, _) = listener.accept().await.unwrap();
            let request = read_request(&mut output, MAX_FRAME_BYTES)
                .await
                .unwrap()
                .unwrap();
            let Request::ReleaseOutput(request) = request else {
                panic!("recovery must query the existing job output");
            };
            assert_eq!(request.after_sequence, 5);
            let sequences: Vec<u64> = match conflict {
                SequenceConflict::NonContiguousFrame => vec![6, 8],
                _ => vec![6, 7],
            };
            write_message(
                &mut output,
                &Response::ReleaseOutput(ReleaseOutputResponse {
                    version: PROTOCOL_VERSION,
                    job_id: server_job_id.clone(),
                    frames: sequences
                        .iter()
                        .map(|sequence| ReleaseOutputFrame {
                            sequence: *sequence,
                            stream: ReleaseOutputStream::Stdout,
                            data: format!("frame-{sequence}\n").into_bytes(),
                        })
                        .collect(),
                    truncated: matches!(conflict, SequenceConflict::UnexpectedTruncation),
                }),
                MAX_FRAME_BYTES,
            )
            .await
            .unwrap();
            // 序号缺口在输出阶段即中断；其余两种需要 executor 终态才能暴露冲突。
            if matches!(conflict, SequenceConflict::NonContiguousFrame) {
                return;
            }
            let (mut status, _) = listener.accept().await.unwrap();
            assert!(matches!(
                read_request(&mut status, MAX_FRAME_BYTES).await.unwrap(),
                Some(Request::ReleaseStatus(_))
            ));
            write_message(
                &mut status,
                &Response::ReleaseExited(ReleaseExitedResponse {
                    version: PROTOCOL_VERSION,
                    job_id: server_job_id,
                    state: ReleaseJobState::Succeeded,
                    exit_code: Some(0),
                    reason: "process_exited".into(),
                    // 终态宣告的序号落后于本地游标属于逆序冲突；截断场景则序号追平。
                    last_sequence: if matches!(conflict, SequenceConflict::TerminalBehindCursor) {
                        4
                    } else {
                        7
                    },
                }),
                MAX_FRAME_BYTES,
            )
            .await
            .unwrap();
        });

        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_sequence_conflict",
                "idem_sequence_conflict_01",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        executor
            .seed_external_output_history(
                "task_sequence_conflict",
                5,
                concat!(
                    "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                    "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n"
                )
                .as_bytes(),
            )
            .unwrap();
        let mut journal = executor.load("task_sequence_conflict").unwrap();
        journal.external_output_sequence = 5;
        executor.store_journal(&journal).unwrap();

        let (outbound, mut received) = mpsc::channel(64);
        let monitor = tokio::spawn(monitor_privileged_release(
            Arc::new(ExecutorClient::new(socket)),
            Arc::clone(&executor),
            Arc::new(Mutex::new(())),
            "task_sequence_conflict".into(),
            "sha256:abcdef0123456789".into(),
            Some(bridge_task()),
            outbound,
        ));
        tokio::time::timeout(Duration::from_secs(30), async {
            monitor.await.unwrap();
            server.await.unwrap();
        })
        .await
        .expect("sequence or truncation conflicts must converge instead of waiting forever");

        let journal = executor.load("task_sequence_conflict").unwrap();
        assert_eq!(journal.state, JournalState::Interrupted);
        assert_eq!(
            journal.error_code.as_deref(),
            Some(match conflict {
                SequenceConflict::UnexpectedTruncation => "release_output_truncated_unexpectedly",
                _ => "release_output_sequence_conflict",
            })
        );
        let messages: Vec<_> = std::iter::from_fn(|| received.try_recv().ok()).collect();
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(message, Message::TaskResult(_)))
                .count(),
            1
        );
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::TaskResult(result)
                    if result.status == TaskTerminalStatus::Succeeded))
        );
        assert!(
            messages
                .iter()
                .any(|message| matches!(message, Message::TaskResult(result)
                    if result.status == TaskTerminalStatus::Interrupted))
        );
    }

    #[tokio::test]
    async fn output_limit_truncation_converges_to_failed_result_after_readable_frames() {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = tokio::spawn(async move {
            for (first, last) in [(1_u64, 128_u64), (129, 129)] {
                let (mut output, _) = listener.accept().await.unwrap();
                let request = read_request(&mut output, MAX_FRAME_BYTES)
                    .await
                    .unwrap()
                    .unwrap();
                let Request::ReleaseOutput(request) = request else {
                    panic!("expected output polling, not a new release");
                };
                assert_eq!(request.after_sequence, first - 1);
                let frames = (first..=last)
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
                        job_id: "release_task_output_limit".into(),
                        frames,
                        truncated: last == 129,
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
                if last == 128 {
                    write_message(
                        &mut status,
                        &Response::ReleaseStatus(
                            deploy_go_agent_executor::protocol::ReleaseStatusResponse {
                                version: PROTOCOL_VERSION,
                                job_id: "release_task_output_limit".into(),
                                state: ReleaseJobState::Running,
                                last_sequence: 129,
                            },
                        ),
                        MAX_FRAME_BYTES,
                    )
                    .await
                    .unwrap();
                    continue;
                }
                write_message(
                    &mut status,
                    &Response::ReleaseExited(ReleaseExitedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: "release_task_output_limit".into(),
                        state: ReleaseJobState::Failed,
                        exit_code: Some(1),
                        reason: "output_limit_exceeded".into(),
                        last_sequence: 129,
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
                "task_output_limit",
                "idem_output_limit_0123456789",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let (outbound, mut received) = mpsc::channel(32);
        monitor_privileged_release(
            Arc::new(ExecutorClient::new(socket)),
            Arc::clone(&executor),
            Arc::new(Mutex::new(())),
            "task_output_limit".into(),
            "sha256:abcdef0123456789".into(),
            Some(bridge_task()),
            outbound,
        )
        .await;
        server.await.unwrap();

        let journal = executor.load("task_output_limit").unwrap();
        assert_eq!(journal.external_output_sequence, 129);
        assert_eq!(journal.state, JournalState::Failed);
        assert_eq!(journal.error_code.as_deref(), Some("output_limit_exceeded"));
        let messages: Vec<_> = std::iter::from_fn(|| received.try_recv().ok()).collect();
        let results: Vec<_> = messages
            .iter()
            .filter_map(|message| match message {
                Message::TaskResult(result) => Some(result),
                _ => None,
            })
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, TaskTerminalStatus::Failed);
        assert_eq!(
            results[0].error_code.as_deref(),
            Some("output_limit_exceeded")
        );
    }

    #[tokio::test]
    async fn terminal_output_limit_job_reads_every_remaining_page_before_failing() {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let outputs = Arc::new(AtomicUsize::new(0));
        let starts = Arc::new(AtomicUsize::new(0));
        let (server_outputs, server_starts) = (Arc::clone(&outputs), Arc::clone(&starts));
        // 模拟 Agent 重启后才观察到的 job：已因输出超限失败。`truncated` 是 job 级标志，
        // 所以每一页都为 true；共 5 帧、每页最多 2 帧，必须读满 3 页才能追平终态序号。
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let Some(request) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap()
                else {
                    continue;
                };
                match request {
                    Request::ReleaseOutput(request) => {
                        server_outputs.fetch_add(1, Ordering::SeqCst);
                        let frames = (request.after_sequence + 1
                            ..=(request.after_sequence + 2).min(5))
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
                            &mut stream,
                            &Response::ReleaseOutput(ReleaseOutputResponse {
                                version: PROTOCOL_VERSION,
                                job_id: request.job_id,
                                frames,
                                truncated: true,
                            }),
                            MAX_FRAME_BYTES,
                        )
                        .await
                        .unwrap();
                    }
                    Request::ReleaseStatus(request) => {
                        write_message(
                            &mut stream,
                            &Response::ReleaseExited(ReleaseExitedResponse {
                                version: PROTOCOL_VERSION,
                                job_id: request.job_id,
                                state: ReleaseJobState::Failed,
                                exit_code: Some(1),
                                reason: "output_limit_exceeded".into(),
                                last_sequence: 5,
                            }),
                            MAX_FRAME_BYTES,
                        )
                        .await
                        .unwrap();
                    }
                    Request::ReleaseStart(_) => {
                        server_starts.fetch_add(1, Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        });

        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_output_limit_pages",
                "idem_output_limit_pages_0123",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let (outbound, mut received) = mpsc::channel(32);
        tokio::time::timeout(
            Duration::from_secs(10),
            monitor_privileged_release(
                Arc::new(ExecutorClient::new(socket)),
                Arc::clone(&executor),
                Arc::new(Mutex::new(())),
                "task_output_limit_pages".into(),
                "sha256:abcdef0123456789".into(),
                Some(bridge_task()),
                outbound,
            ),
        )
        .await
        .expect("终态的多页超限输出必须读完剩余可读帧并收敛，不能挂起");
        server.abort();

        let journal = executor.load("task_output_limit_pages").unwrap();
        assert_eq!(
            journal.external_output_sequence, 5,
            "剩余可读帧必须全部落盘"
        );
        assert_eq!(journal.state, JournalState::Failed);
        assert_eq!(journal.error_code.as_deref(), Some("output_limit_exceeded"));
        assert_eq!(outputs.load(Ordering::SeqCst), 3);
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        let results: Vec<_> = std::iter::from_fn(|| received.try_recv().ok())
            .filter_map(|message| match message {
                Message::TaskResult(result) => Some(result),
                _ => None,
            })
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, TaskTerminalStatus::Failed);
        assert_eq!(
            results[0].error_code.as_deref(),
            Some("output_limit_exceeded")
        );
    }

    #[tokio::test]
    async fn terminal_job_that_never_serves_declared_frames_converges_to_interrupted() {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let outputs = Arc::new(AtomicUsize::new(0));
        let starts = Arc::new(AtomicUsize::new(0));
        // executor 声称终态且最后序号为 3，却始终返回空页（truncated=false）：
        // 终态之后输出不会再增长，这是永久序号冲突，而不是需要无限等待的竞态。
        let server = tokio::spawn(serve_exited_without_output(
            listener,
            Arc::clone(&outputs),
            Arc::clone(&starts),
            3,
        ));
        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_terminal_stall",
                "idem_terminal_stall_0123456",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let (outbound, mut received) = mpsc::channel(32);
        tokio::time::timeout(
            Duration::from_secs(10),
            monitor_privileged_release(
                Arc::new(ExecutorClient::new(socket)),
                Arc::clone(&executor),
                Arc::new(Mutex::new(())),
                "task_terminal_stall".into(),
                "sha256:abcdef0123456789".into(),
                Some(bridge_task()),
                outbound,
            ),
        )
        .await
        .expect("终态序号长期追不平必须有上限，不能无限等待");
        server.abort();

        let journal = executor.load("task_terminal_stall").unwrap();
        assert_eq!(journal.state, JournalState::Interrupted);
        assert_eq!(
            journal.error_code.as_deref(),
            Some("release_output_sequence_conflict")
        );
        assert_eq!(
            outputs.load(Ordering::SeqCst),
            TERMINAL_OUTPUT_STALL_LIMIT as usize + 1,
            "初次加上限内重试，之后立即收敛"
        );
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        let results: Vec<_> = std::iter::from_fn(|| received.try_recv().ok())
            .filter_map(|message| match message {
                Message::TaskResult(result) => Some(result),
                _ => None,
            })
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, TaskTerminalStatus::Interrupted);
    }

    /// 把 tracing 事件收集到内存，供断言“诊断不含敏感内容”。
    #[derive(Clone, Default)]
    struct CapturedLogs(Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for CapturedLogs {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl CapturedLogs {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }

        fn lines_mentioning(&self, needle: &str) -> String {
            self.text()
                .lines()
                .filter(|line| line.contains(needle))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    /// 进程级只安装一次的捕获订阅者。线程局部订阅者（`set_default`）在整套测试并发运行时会
    /// 因 callsite 兴趣缓存而漏事件，导致用例顺序相关；全局订阅者对所有线程一致生效。
    /// 断言只使用本用例独有的哨兵值和任务标识，不受其它并发用例日志影响。
    fn global_captured_logs() -> CapturedLogs {
        static LOGS: std::sync::OnceLock<CapturedLogs> = std::sync::OnceLock::new();
        LOGS.get_or_init(|| {
            let logs = CapturedLogs::default();
            let writer = logs.clone();
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::WARN)
                .with_ansi(false)
                .with_writer(move || writer.clone())
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("单元测试进程内不应已有其它全局订阅者");
            logs
        })
        .clone()
    }

    /// SC-005 / FR-006：一次完整的“持续停滞 → 中断”收敛过程中，Agent 内部诊断必须带上
    /// deployment / task / job 关联与恢复动作，但不得包含业务输出正文、带凭证的仓库 URL
    /// 或内部绝对路径；对控制面的 TaskResult 也不得携带内部 task/job 标识。
    #[tokio::test]
    async fn monitor_diagnostics_and_results_never_expose_secrets_output_or_internal_paths() {
        let logs = global_captured_logs();

        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let Some(request) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap()
                else {
                    continue;
                };
                match request {
                    Request::ReleaseOutput(request) => {
                        let frames = if request.after_sequence == 0 {
                            vec![ReleaseOutputFrame {
                                sequence: 1,
                                stream: ReleaseOutputStream::Stdout,
                                data: concat!(
                                    "SENTINEL_BUSINESS_OUTPUT password=hunter2\n",
                                    "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.started\"}\n",
                                    "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.succeeded\"}\n"
                                )
                                .as_bytes()
                                .to_vec(),
                            }]
                        } else {
                            Vec::new()
                        };
                        write_message(
                            &mut stream,
                            &Response::ReleaseOutput(ReleaseOutputResponse {
                                version: PROTOCOL_VERSION,
                                job_id: request.job_id,
                                frames,
                                truncated: false,
                            }),
                            MAX_FRAME_BYTES,
                        )
                        .await
                        .unwrap();
                    }
                    Request::ReleaseStatus(request) => {
                        write_message(
                            &mut stream,
                            &Response::ReleaseExited(ReleaseExitedResponse {
                                version: PROTOCOL_VERSION,
                                job_id: request.job_id,
                                state: ReleaseJobState::Failed,
                                exit_code: Some(1),
                                reason: "nonzero_exit".into(),
                                last_sequence: 3,
                            }),
                            MAX_FRAME_BYTES,
                        )
                        .await
                        .unwrap();
                    }
                    _ => {}
                }
            }
        });

        let tasks_root = directory.path().join("tasks");
        let executor = Arc::new(Executor::new(tasks_root.clone()).unwrap());
        executor
            .create_transfer_task(
                "task_diag_leak",
                "idem_diag_leak_0123456789",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let mut task = bridge_task();
        task.repository_url =
            Some("https://deploy:SENTINEL_REPO_PASSWORD@git.example.test/app.git".into());
        let (outbound, mut received) = mpsc::channel(64);
        tokio::time::timeout(
            Duration::from_secs(10),
            monitor_privileged_release(
                Arc::new(ExecutorClient::new(socket)),
                Arc::clone(&executor),
                Arc::new(Mutex::new(())),
                "task_diag_leak".into(),
                "sha256:abcdef0123456789".into(),
                Some(task),
                outbound,
            ),
        )
        .await
        .expect("停滞场景必须收敛");
        server.abort();

        // 诊断确实产出，并带上关联与恢复动作。只看提到本用例任务标识的行，避免并发用例干扰。
        let mine = logs.lines_mentioning("task_diag_leak");
        for expected in [
            "privileged_release",
            "sequence_stalled",
            "report_interrupted",
            "release_task_diag_leak",
            "deployment",
        ] {
            assert!(mine.contains(expected), "诊断缺少 {expected}：\n{mine}");
        }
        // 敏感内容与内部路径一律不得出现。哨兵值和临时目录路径为本用例独有，
        // 因此在全部捕获的日志里查找，防止泄漏的行恰好没有带任务标识。
        let everything = logs.text();
        let task_root = tasks_root.to_string_lossy().into_owned();
        for forbidden in [
            "SENTINEL_BUSINESS_OUTPUT",
            "hunter2",
            "SENTINEL_REPO_PASSWORD",
            task_root.as_str(),
        ] {
            assert!(
                !everything.contains(forbidden),
                "诊断不得包含 {forbidden}：\n{everything}"
            );
        }
        // `/srv/work` 是多个用例共用的 fixture 路径，只检查与本用例相关的行。
        assert!(
            !mine.contains("/srv/work"),
            "诊断不得包含内部路径：\n{mine}"
        );
        // 对控制面的终态结果只带稳定错误码，不暴露内部 task/job 标识。
        let result = std::iter::from_fn(|| received.try_recv().ok())
            .find_map(|message| match message {
                Message::TaskResult(result) => Some(result),
                _ => None,
            })
            .expect("必须产生终态结果");
        assert_eq!(result.status, TaskTerminalStatus::Interrupted);
        assert_eq!(
            result.error_code.as_deref(),
            Some("release_output_sequence_conflict")
        );
        let exposed = format!("{:?} {:?}", result.summary, result.data);
        assert!(!exposed.contains("release_task_diag_leak"), "{exposed}");
        assert!(!exposed.contains("SENTINEL"), "{exposed}");
    }

    /// 复核发现：超长事件行的完整性错误是 `ErrorKind::Other`，曾被当作暂时 I/O 无限重试
    /// （HEAD 在这里是直接停止）。job 在 executor 侧已成功/失败，Agent 却永远发不出终态。
    #[tokio::test]
    async fn oversized_event_line_converges_to_interrupted_instead_of_retrying_forever() {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let Some(request) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap()
                else {
                    continue;
                };
                match request {
                    Request::ReleaseOutput(request) => {
                        // 每页一帧、28KiB 且不含换行：`Vec<u8>` 按数字数组序列化，用 0 字节时每字节只占 2 个字符，
                        // 单帧约 56KB，仍小于 IPC 64KiB 上限；累计超过 1MiB 后形成无法解析的超长事件行。
                        // 帧数尽量少：每轮落盘与事件重建都随累积数据量增长，轮次多了整套并发时会超时。
                        let frames = if request.after_sequence < 40 {
                            vec![ReleaseOutputFrame {
                                sequence: request.after_sequence + 1,
                                stream: ReleaseOutputStream::Stdout,
                                data: vec![0_u8; 28 * 1024],
                            }]
                        } else {
                            Vec::new()
                        };
                        write_message(
                            &mut stream,
                            &Response::ReleaseOutput(ReleaseOutputResponse {
                                version: PROTOCOL_VERSION,
                                job_id: request.job_id,
                                frames,
                                truncated: false,
                            }),
                            MAX_FRAME_BYTES,
                        )
                        .await
                        .unwrap();
                    }
                    Request::ReleaseStatus(request) => {
                        write_message(
                            &mut stream,
                            &Response::ReleaseExited(ReleaseExitedResponse {
                                version: PROTOCOL_VERSION,
                                job_id: request.job_id,
                                state: ReleaseJobState::Failed,
                                exit_code: Some(1),
                                reason: "nonzero_exit".into(),
                                last_sequence: 40,
                            }),
                            MAX_FRAME_BYTES,
                        )
                        .await
                        .unwrap();
                    }
                    _ => {}
                }
            }
        });
        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_oversized_line",
                "idem_oversized_line_0123456",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        // 90 轮每轮都会投递输出消息，且测试期间无人消费：通道必须足够大，避免背压阻塞 monitor。
        let (outbound, mut received) = mpsc::channel(4096);
        tokio::time::timeout(
            Duration::from_secs(40),
            monitor_privileged_release(
                Arc::new(ExecutorClient::new(socket)),
                Arc::clone(&executor),
                Arc::new(Mutex::new(())),
                "task_oversized_line".into(),
                "sha256:abcdef0123456789".into(),
                Some(bridge_task()),
                outbound,
            ),
        )
        .await
        .expect("事件数据永久损坏必须收敛，不能无限重试");
        server.abort();

        let journal = executor.load("task_oversized_line").unwrap();
        assert_eq!(journal.state, JournalState::Interrupted);
        assert_eq!(
            journal.error_code.as_deref(),
            Some("release_event_data_invalid")
        );
        let results: Vec<_> = std::iter::from_fn(|| received.try_recv().ok())
            .filter_map(|message| match message {
                Message::TaskResult(result) => Some(result),
                _ => None,
            })
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, TaskTerminalStatus::Interrupted);
    }

    /// 复核发现：`consecutive_local_errors` 曾在每轮 ReleaseOutput 成功后清零，本地存储持续
    /// 故障时退避永远停在第一档并反复刷日志。用只读的帧目录制造持续的本地写失败，
    /// 在固定窗口内统计对 executor 的请求次数：指数退避只有个位数，热重试会是几十次。
    #[tokio::test]
    async fn persistent_local_failure_backs_off_instead_of_retrying_at_the_first_delay() {
        use std::os::unix::fs::PermissionsExt;
        struct RestoreMode(std::path::PathBuf);
        impl Drop for RestoreMode {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
            }
        }

        let directory = tempfile::tempdir().unwrap();
        // 只读目录权限对 root 不生效：探测后跳过，而不是误报。
        let probe = directory.path().join("probe");
        std::fs::create_dir(&probe).unwrap();
        let _probe_restore = RestoreMode(probe.clone());
        std::fs::set_permissions(&probe, std::fs::Permissions::from_mode(0o500)).unwrap();
        if std::fs::File::create(probe.join("x")).is_ok() {
            eprintln!("跳过：当前身份（root）不受目录只读权限约束，无法注入本地写失败");
            return;
        }

        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let outputs = Arc::new(AtomicUsize::new(0));
        let server_outputs = Arc::clone(&outputs);
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let Some(request) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap()
                else {
                    continue;
                };
                if let Request::ReleaseOutput(request) = request {
                    server_outputs.fetch_add(1, Ordering::SeqCst);
                    write_message(
                        &mut stream,
                        &Response::ReleaseOutput(ReleaseOutputResponse {
                            version: PROTOCOL_VERSION,
                            job_id: request.job_id,
                            frames: vec![ReleaseOutputFrame {
                                sequence: request.after_sequence + 1,
                                stream: ReleaseOutputStream::Stdout,
                                data: b"frame\n".to_vec(),
                            }],
                            truncated: false,
                        }),
                        MAX_FRAME_BYTES,
                    )
                    .await
                    .unwrap();
                }
            }
        });
        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_local_backoff",
                "idem_local_backoff_0123456",
                "sha256:abcdef0123456789",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let frames = executor
            .task_dir("task_local_backoff")
            .join("executor-output-frames");
        std::fs::create_dir_all(&frames).unwrap();
        let _frames_restore = RestoreMode(frames.clone());
        std::fs::set_permissions(&frames, std::fs::Permissions::from_mode(0o500)).unwrap();

        let (outbound, _received) = mpsc::channel(8);
        let monitor = tokio::spawn(monitor_privileged_release(
            Arc::new(ExecutorClient::new(socket)),
            Arc::clone(&executor),
            Arc::new(Mutex::new(())),
            "task_local_backoff".into(),
            "sha256:abcdef0123456789".into(),
            Some(bridge_task()),
            outbound,
        ));
        tokio::time::sleep(Duration::from_millis(700)).await;
        let requests = outputs.load(Ordering::SeqCst);
        monitor.abort();
        server.abort();

        assert!(
            (2..20).contains(&requests),
            "持续本地故障应指数退避（窗口内个位数请求），实际请求 {requests} 次"
        );
    }

    #[tokio::test]
    async fn release_authorization_timeout_cleans_pending_waiter() {
        let directory = tempfile::tempdir().unwrap();
        let handler = TaskHandler::new(Executor::new(directory.path().join("tasks")).unwrap());
        let (outbound, mut received) = mpsc::channel(1);
        let request = ReleaseAuthorizationRequest {
            task_id: "task_auth_timeout".into(),
            authorization_id: "release_auth_timeout_001".into(),
            target_run_id: "run".into(),
            target_id: "target".into(),
            snapshot_hash: "sha256:0123456789abcdef".into(),
            checkout_tree_digest: "sha256:0123456789abcdef".into(),
            artifact_manifest_digest: "sha256:0123456789abcdef".into(),
            artifacts: Vec::new(),
            env_files: Vec::new(),
            cancel_file: "/srv/work/cancel".into(),
            secret_environment: None,
        };
        tokio::time::pause();
        let expected_authorization_id = request.authorization_id.clone();
        let waiter = tokio::spawn({
            let handler = handler.clone();
            let outbound = outbound.clone();
            async move {
                handler
                    .request_release_authorization(request.clone(), &outbound)
                    .await
            }
        });
        assert!(matches!(
            received.recv().await,
            Some(Message::ReleaseAuthorizationRequest(value)) if value.authorization_id == expected_authorization_id
        ));
        tokio::time::advance(Duration::from_secs(11)).await;
        assert_eq!(
            waiter.await.unwrap().unwrap_err(),
            "release_authorization_timeout"
        );
        assert!(handler.release_authorizations.lock().await.is_empty());
    }

    #[tokio::test]
    async fn drain_events_skips_by_count_when_rebuilt_byte_offset_is_stale() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_event_drift",
                "idem_event_drift_0123456789",
                "sha256:0123456789abcdef",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let context = crate::deploy_events::DeployEventContext {
            deploy_id: "deployment".into(),
            stage: deploy_go_agent_protocol::DeploymentStage::Release,
            environment: deploy_go_agent_protocol::Environment::Test,
            release_version: "release-1".into(),
            target: Some("test".into()),
        };
        let mut encoded = Vec::new();
        for _ in 0..3 {
            serde_json::to_writer(&mut encoded, &crate::deploy_events::started_event(&context))
                .unwrap();
            encoded.push(b'\n');
        }
        fs::write(
            executor.task_dir("task_event_drift").join("events.jsonl"),
            &encoded,
        )
        .unwrap();

        let mut journal = executor.load("task_event_drift").unwrap();
        journal.events_sent = 1;
        journal.events_offset = 7;
        executor.store_journal(&journal).unwrap();
        let (outbound, mut received) = mpsc::channel(16);
        drain_events(
            &executor,
            &Arc::new(Mutex::new(())),
            &outbound,
            &mut journal,
        )
        .await
        .unwrap();
        let mut sent_events = Vec::new();
        while let Ok(message) = received.try_recv() {
            if let Message::TaskProgress(progress) = message {
                sent_events.push(progress.event.event);
            }
        }
        assert_eq!(sent_events.len(), 2);
        let stored = executor.load("task_event_drift").unwrap();
        assert_eq!(stored.events_sent, 3);
        assert_eq!(stored.events_offset, encoded.len() as u64);
    }

    #[tokio::test]
    async fn drain_events_migrates_legacy_byte_offset_to_event_count() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Arc::new(Executor::new(directory.path().join("tasks")).unwrap());
        executor
            .create_transfer_task(
                "task_event_legacy",
                "idem_event_legacy_0123456789",
                "sha256:0123456789abcdef",
                crate::journal::TransferPhase::PrivilegedRelease,
            )
            .await
            .unwrap();
        let context = crate::deploy_events::DeployEventContext {
            deploy_id: "deployment".into(),
            stage: deploy_go_agent_protocol::DeploymentStage::Release,
            environment: deploy_go_agent_protocol::Environment::Test,
            release_version: "release-1".into(),
            target: Some("test".into()),
        };
        let mut encoded = Vec::new();
        for _ in 0..3 {
            serde_json::to_writer(&mut encoded, &crate::deploy_events::started_event(&context))
                .unwrap();
            encoded.push(b'\n');
        }
        fs::write(
            executor.task_dir("task_event_legacy").join("events.jsonl"),
            &encoded,
        )
        .unwrap();
        // 旧偏移落在第二行行首之后，模拟整文件重写导致的行首漂移。
        let stale_offset = encoded.iter().position(|byte| *byte == b'\n').unwrap() as u64 + 2;

        let mut journal = executor.load("task_event_legacy").unwrap();
        journal.events_offset = stale_offset;
        executor.store_journal(&journal).unwrap();
        let (outbound, mut received) = mpsc::channel(16);
        drain_events(
            &executor,
            &Arc::new(Mutex::new(())),
            &outbound,
            &mut journal,
        )
        .await
        .unwrap();
        let mut sent_events = Vec::new();
        while let Ok(message) = received.try_recv() {
            if let Message::TaskProgress(progress) = message {
                sent_events.push(progress.event.event);
            }
        }
        assert_eq!(sent_events.len(), 1);
        let stored = executor.load("task_event_legacy").unwrap();
        assert_eq!(stored.events_sent, 3);
        assert_eq!(stored.events_offset, encoded.len() as u64);
    }
}

#[cfg(test)]
mod runtime_probe_tests {
    use super::*;

    async fn ok_health() -> &'static str {
        "ok"
    }

    async fn unhealthy() -> axum::http::StatusCode {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    }

    async fn serve(router: axum::Router) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        port
    }

    #[tokio::test]
    async fn http_and_tcp_probe_succeed_on_local_listener() {
        let port = serve(
            axum::Router::new().route("/healthz", axum::routing::get(ok_health).post(ok_health)),
        )
        .await;
        let http = RuntimeProbeTask {
            runtime_status_id: "runtime_status_probe_http".into(),
            probe_type: RuntimeProbeType::Http,
            port,
            path: Some("/healthz".into()),
            expected_status: Some(200),
            timeout_ms: 5000,
        };
        let shared_client = crate::http_client::new_runtime_probe_client();
        let result = perform_runtime_probe(&http, Some(&shared_client))
            .await
            .unwrap();
        assert_eq!(result["http_status"], 200);

        let tcp = RuntimeProbeTask {
            runtime_status_id: "runtime_status_probe_tcp".into(),
            probe_type: RuntimeProbeType::Tcp,
            port,
            path: None,
            expected_status: None,
            timeout_ms: 5000,
        };
        assert!(perform_runtime_probe(&tcp, None).await.is_ok());
    }

    #[tokio::test]
    async fn http_probe_rejects_unexpected_status() {
        let port =
            serve(axum::Router::new().route("/healthz", axum::routing::get(unhealthy))).await;
        let task = RuntimeProbeTask {
            runtime_status_id: "runtime_status_probe_status".into(),
            probe_type: RuntimeProbeType::Http,
            port,
            path: Some("/healthz".into()),
            expected_status: Some(200),
            timeout_ms: 5000,
        };
        assert_eq!(
            perform_runtime_probe(&task, None).await.unwrap_err(),
            "runtime_probe_http_unexpected_status"
        );
    }

    #[tokio::test]
    async fn probe_validation_rejects_mismatched_payload_shape() {
        let task = RuntimeProbeTask {
            runtime_status_id: "runtime_status_probe_invalid".into(),
            probe_type: RuntimeProbeType::Tcp,
            port: 8080,
            path: Some("/healthz".into()),
            expected_status: Some(200),
            timeout_ms: 1000,
        };
        assert!(!task.validate());
        assert_eq!(
            perform_runtime_probe(&task, None).await.unwrap_err(),
            "invalid_runtime_probe_payload"
        );
    }
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod monitor_recovery_tests {
    use super::*;

    #[tokio::test]
    async fn reconnect_reclaims_running_monitor_after_old_outbound_closes() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_reconnect_monitor",
                "idem_reconnect_monitor_012345",
                "sha256:reconnect_monitor",
            )
            .await
            .unwrap();
        let mut journal = executor.load("task_reconnect_monitor").unwrap();
        journal.state = JournalState::Running;
        journal.pid = Some(std::process::id());
        journal.process_start_time =
            Some(crate::journal::process_start_time(std::process::id()).unwrap());
        executor.store_journal(&journal).unwrap();
        fs::write(
            executor
                .task_dir("task_reconnect_monitor")
                .join("stdout.log"),
            b"output before reconnect\n",
        )
        .unwrap();

        let handler = TaskHandler::new(executor.clone());
        let (old_outbound, mut old_receiver) = mpsc::channel(1);
        let old_handler = handler.clone();
        tokio::spawn(async move {
            old_handler.run_monitor(journal, old_outbound).await;
        });
        assert!(matches!(
            old_receiver.recv().await,
            Some(Message::TaskOutput(_))
        ));

        let (new_outbound, mut received) = mpsc::channel(16);
        handler
            .reconcile(vec!["task_reconnect_monitor".to_owned()], new_outbound)
            .await;
        assert!(matches!(
            received.recv().await,
            Some(Message::ReconcileReport(report))
                if report.tasks[0].state == ReconciledTaskState::Running
        ));

        fs::write(
            executor
                .task_dir("task_reconnect_monitor")
                .join("completion.json"),
            r#"{"exit_code":0,"error_code":null}"#,
        )
        .unwrap();
        drop(old_receiver);

        let result = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some(Message::TaskResult(result)) = received.recv().await {
                    break result;
                }
            }
        })
        .await
        .expect("reconnected monitor must consume completion");
        assert_eq!(result.status, TaskTerminalStatus::Succeeded);
        assert_eq!(
            executor.load("task_reconnect_monitor").unwrap().state,
            JournalState::Succeeded
        );
    }

    #[tokio::test]
    async fn reconnect_redispatches_finished_prepare_without_result() {
        let directory = tempfile::tempdir().unwrap();
        let executor = Executor::new(directory.path().join("tasks")).unwrap();
        executor
            .create_task(
                "task_prepare_reconnect",
                "idem_prepare_reconnect_012345",
                "sha256:prepare_reconnect",
            )
            .await
            .unwrap();
        let mut journal = executor.load("task_prepare_reconnect").unwrap();
        journal.state = JournalState::Succeeded;
        journal.exit_code = Some(0);
        executor.store_journal(&journal).unwrap();
        fs::write(
            executor
                .task_dir("task_prepare_reconnect")
                .join("completion.json"),
            r#"{"exit_code":0,"error_code":null}"#,
        )
        .unwrap();
        fs::write(
            executor
                .task_dir("task_prepare_reconnect")
                .join("runner-spec.json"),
            serde_json::to_vec(&serde_json::json!({
                "deployment_id": "deployment_prepare_reconnect",
                "script_path": "make",
                "argument_tokens": [],
                "environment_file_references": [],
                "timeout_seconds": 60,
                "log_budget_bytes": 1024,
                "two_stage": {
                    "stage": "prepare",
                    "checkout_dir": "/tmp/checkout",
                    "work_root": "/tmp/work",
                    "repository_url": null,
                    "commit_sha": "abc",
                    "credential_file": null,
                    "environment": "test",
                    "release_version": "1",
                    "target_code": null,
                    "modules": ["api"],
                    "artifact_dir": "/tmp/out",
                    "staging_size_limit_bytes": 1024,
                    "staging_max_files": 10,
                    "git_lease_id": null
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let handler = TaskHandler::new(executor.clone());
        let (outbound, mut received) = mpsc::channel(16);
        handler
            .reconcile(vec!["task_prepare_reconnect".to_owned()], outbound)
            .await;
        assert!(matches!(
            received.recv().await,
            Some(Message::ReconcileReport(report))
                if report.tasks[0].state == ReconciledTaskState::Accepted
                    && report.tasks[0].result.is_none()
        ));
    }
}
