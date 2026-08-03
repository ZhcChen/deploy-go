use std::{fs, io, path::Path, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::Utc;
use deploy_go_agent_protocol::{
    Envelope, Message, OutputStream, ReconcileReport, ReconciledTask, ReconciledTaskState,
    SystemInspectTask, TaskAck, TaskAckDisposition, TaskCancel, TaskDispatch, TaskLifecycleState,
    TaskOutput, TaskPayload, TaskResult, TaskState, TaskTerminalStatus,
};
use serde_json::json;
use tokio::sync::{Mutex, mpsc};

use crate::{
    connection::{ConnectionError, MessageHandler},
    executor::{ExecuteError, Executor},
    journal::{JournalState, RecoveryState, TaskJournal},
};

const OUTPUT_CHUNK_BYTES: usize = 32 * 1024;

#[derive(Clone)]
pub struct TaskHandler {
    executor: Arc<Executor>,
    event_lock: Arc<Mutex<()>>,
}

impl TaskHandler {
    pub fn new(executor: Executor) -> Self {
        Self {
            executor: Arc::new(executor),
            event_lock: Arc::new(Mutex::new(())),
        }
    }

    async fn dispatch(&self, dispatch: TaskDispatch, outbound: mpsc::Sender<Message>) {
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
        if let TaskPayload::SystemInspect(task) = &dispatch.task {
            self.inspect(&dispatch, task, outbound).await;
            return;
        }
        let TaskPayload::DeploymentExecute(task) = &dispatch.task else {
            let _ = send_ack(
                &outbound,
                &dispatch,
                TaskAckDisposition::Rejected,
                Some("unsupported_task_type"),
            )
            .await;
            return;
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

    async fn cancel(&self, cancel: TaskCancel, outbound: mpsc::Sender<Message>) {
        let Ok(mut journal) = self.executor.load(&cancel.task_id) else {
            return;
        };
        if terminal(&journal.state) {
            let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut journal).await;
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
        if let Ok(mut completed) = self.executor.cancel(&cancel.task_id).await {
            let _ =
                drain_outputs(&self.executor, &self.event_lock, &outbound, &mut completed).await;
            let _ = send_result(&self.executor, &self.event_lock, &outbound, &mut completed).await;
        }
    }

    async fn reconcile(&self, task_ids: Vec<String>, outbound: mpsc::Sender<Message>) {
        let mut tasks = Vec::with_capacity(task_ids.len());
        for task_id in task_ids {
            let item = match self.executor.recover(&task_id) {
                Ok(state) => reconciled(state),
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
    }
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
        match executor.poll_completion(&journal.task_id) {
            Ok(Some(mut current)) => {
                let _ = drain_outputs(&executor, &event_lock, &outbound, &mut current).await;
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

fn reconciled(state: RecoveryState) -> ReconciledTask {
    let journal = match state {
        RecoveryState::Accepted(journal)
        | RecoveryState::Running(journal)
        | RecoveryState::Terminal(journal)
        | RecoveryState::Interrupted(journal) => journal,
    };
    let state = if terminal(&journal.state) {
        ReconciledTaskState::Terminal
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

fn inspect_system(task: &SystemInspectTask) -> Result<serde_json::Value, &'static str> {
    let work_root = inspect_directory(&task.work_root).map_err(|_| "work_root_inaccessible")?;
    inspect_directory(&task.secrets_root).map_err(|_| "secrets_root_inaccessible")?;
    let filesystem =
        nix::sys::statvfs::statvfs(&work_root).map_err(|_| "disk_inspection_failed")?;
    let disk_available_bytes =
        u64::from(filesystem.blocks_available()).saturating_mul(filesystem.block_size());
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
