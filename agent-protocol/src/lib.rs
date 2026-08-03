use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MIN_SUPPORTED_PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub protocol_version: u16,
    pub message_id: String,
    pub sent_at: String,
    pub message: Message,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Hello(Hello),
    HelloAck(HelloAck),
    AuthRefresh(AuthRefresh),
    AuthRefreshed(AuthRefreshed),
    Heartbeat(Heartbeat),
    HeartbeatAck(HeartbeatAck),
    TaskDispatch(TaskDispatch),
    TaskAck(TaskAck),
    TaskOutput(TaskOutput),
    TaskState(TaskState),
    TaskResult(TaskResult),
    TaskCancel(TaskCancel),
    ReconcileRequest(ReconcileRequest),
    ReconcileReport(ReconcileReport),
    ProtocolError(ProtocolError),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub agent_id: String,
    pub agent_version: String,
    pub min_protocol_version: u16,
    pub max_protocol_version: u16,
    pub os: String,
    pub architecture: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HelloAck {
    pub connection_id: String,
    pub connection_generation: u64,
    pub protocol_version: u16,
    pub heartbeat_interval_seconds: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthRefresh {
    pub access_token: String,
    pub rotation_id: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthRefreshed {
    pub rotation_id: String,
    pub access_expires_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Heartbeat {
    pub connection_generation: u64,
    pub active_task_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeartbeatAck {
    pub connection_generation: u64,
    pub server_time: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskDispatch {
    pub task_id: String,
    pub idempotency_key: String,
    pub deadline_at: String,
    pub payload_digest: String,
    pub task: TaskPayload,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum TaskPayload {
    SystemInspect(SystemInspectTask),
    DeploymentExecute(DeploymentExecuteTask),
    HealthDiagnose(HealthDiagnoseTask),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SystemInspectTask {
    pub work_root: String,
    pub secrets_root: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentExecuteTask {
    pub deployment_id: String,
    pub work_root: String,
    pub script_path: String,
    pub argument_tokens: Vec<String>,
    pub environment_file_references: Vec<EnvironmentFileReference>,
    pub timeout_seconds: u32,
    pub wrapper_version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentFileReference {
    pub environment_key: String,
    pub file_path: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HealthDiagnoseTask {
    pub checks: Vec<DiagnosticCheck>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCheck {
    Disk,
    WorkRoot,
    SecretsRoot,
    Clock,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskAck {
    pub task_id: String,
    pub payload_digest: String,
    pub disposition: TaskAckDisposition,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAckDisposition {
    Accepted,
    Duplicate,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskOutput {
    pub task_id: String,
    pub sequence: u64,
    pub stream: OutputStream,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskState {
    pub task_id: String,
    pub sequence: u64,
    pub state: TaskLifecycleState,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskLifecycleState {
    Accepted,
    Running,
    Canceling,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskResult {
    pub task_id: String,
    pub sequence: u64,
    pub status: TaskTerminalStatus,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskTerminalStatus {
    Succeeded,
    Failed,
    Canceled,
    Interrupted,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCancel {
    pub task_id: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcileRequest {
    pub task_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcileReport {
    pub tasks: Vec<ReconciledTask>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciledTask {
    pub task_id: String,
    pub payload_digest: String,
    pub state: ReconciledTaskState,
    pub last_sequence: u64,
    pub result: Option<TaskResult>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciledTaskState {
    Accepted,
    Running,
    Terminal,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
    pub related_message_id: Option<String>,
    pub details: Option<Map<String, Value>>,
}

impl Envelope {
    pub fn validate_version(&self) -> Result<(), VersionError> {
        if self.protocol_version == PROTOCOL_VERSION {
            Ok(())
        } else {
            Err(VersionError {
                received: self.protocol_version,
                minimum: MIN_SUPPORTED_PROTOCOL_VERSION,
                maximum: PROTOCOL_VERSION,
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VersionError {
    pub received: u16,
    pub minimum: u16,
    pub maximum: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_task_dispatch() {
        let envelope = Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: "msg_01".into(),
            sent_at: "2026-08-03T03:00:00Z".into(),
            message: Message::TaskDispatch(TaskDispatch {
                task_id: "task_01".into(),
                idempotency_key: "idem-0123456789abcdef".into(),
                deadline_at: "2026-08-03T03:10:00Z".into(),
                payload_digest: "sha256:abc".into(),
                task: TaskPayload::DeploymentExecute(DeploymentExecuteTask {
                    deployment_id: "dep_01".into(),
                    work_root: "/srv/example".into(),
                    script_path: "/srv/example/deploy.sh".into(),
                    argument_tokens: vec!["--environment".into(), "production".into()],
                    environment_file_references: vec![EnvironmentFileReference {
                        environment_key: "APP_ENV_FILE".into(),
                        file_path: "/srv/example/secrets/app.env".into(),
                    }],
                    timeout_seconds: 600,
                    wrapper_version: "1".into(),
                }),
            }),
        };
        let json = serde_json::to_string(&envelope).unwrap();
        assert_eq!(serde_json::from_str::<Envelope>(&json).unwrap(), envelope);
    }

    #[test]
    fn rejects_unknown_message_and_payload_fields() {
        let unknown_message = r#"{"protocol_version":1,"message_id":"msg","sent_at":"2026-08-03T03:00:00Z","message":{"type":"shell","command":"id"}}"#;
        assert!(serde_json::from_str::<Envelope>(unknown_message).is_err());

        let shell_field = r#"{"protocol_version":1,"message_id":"msg","sent_at":"2026-08-03T03:00:00Z","message":{"type":"task_dispatch","task_id":"task","idempotency_key":"idem-0123456789abcdef","deadline_at":"2026-08-03T03:10:00Z","payload_digest":"sha256:abc","task":{"kind":"deployment_execute","payload":{"deployment_id":"dep","work_root":"/srv/app","script_path":"/srv/app/deploy.sh","argument_tokens":[],"environment_file_references":[],"timeout_seconds":60,"wrapper_version":"1","shell_command":"id"}}}}"#;
        assert!(serde_json::from_str::<Envelope>(shell_field).is_err());
    }

    #[test]
    fn rejects_unsupported_protocol_version() {
        let envelope = Envelope {
            protocol_version: PROTOCOL_VERSION + 1,
            message_id: "msg".into(),
            sent_at: "2026-08-03T03:00:00Z".into(),
            message: Message::Heartbeat(Heartbeat {
                connection_generation: 1,
                active_task_ids: vec![],
            }),
        };
        assert_eq!(envelope.validate_version().unwrap_err().received, 2);
    }
}
