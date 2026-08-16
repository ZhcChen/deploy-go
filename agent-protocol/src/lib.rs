use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const PROTOCOL_VERSION: u16 = 12;
pub const MIN_SUPPORTED_PROTOCOL_VERSION: u16 = 11;
pub const NODE_TELEMETRY_MAX_BYTES: usize = 16 * 1024;
pub const NODE_TELEMETRY_MAX_GPUS: usize = 8;
pub const TERMINAL_MAX_INPUT_BYTES: usize = 12 * 1024;
pub const TERMINAL_MAX_FRAME_ENCODED_BYTES: usize = 16 * 1024;
pub const TERMINAL_MIN_COLUMNS: u16 = 1;
pub const TERMINAL_MAX_COLUMNS: u16 = 500;
pub const TERMINAL_MIN_ROWS: u16 = 1;
pub const TERMINAL_MAX_ROWS: u16 = 1_000;

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
#[allow(clippy::large_enum_variant)] // 线协议保持携带完整任务载荷，受帧上限约束
pub enum Message {
    Hello(Hello),
    HelloAck(HelloAck),
    AuthRefresh(AuthRefresh),
    AuthRefreshed(AuthRefreshed),
    Heartbeat(Heartbeat),
    HeartbeatAck(HeartbeatAck),
    NodeTelemetry(NodeTelemetry),
    TaskDispatch(TaskDispatch),
    TaskAck(TaskAck),
    TaskOutput(TaskOutput),
    TaskProgress(TaskProgress),
    TaskState(TaskState),
    TaskResult(TaskResult),
    TaskCancel(TaskCancel),
    ReconcileRequest(ReconcileRequest),
    ReconcileReport(ReconcileReport),
    SecretLeaseRequest(SecretLeaseRequest),
    SecretLeaseResponse(SecretLeaseResponse),
    ArtifactPrepared(ArtifactPrepared),
    ArtifactUploadAuthorized(ArtifactUploadAuthorized),
    ReleaseAuthorizationRequest(ReleaseAuthorizationRequest),
    ReleaseAuthorizationResponse(ReleaseAuthorizationResponse),
    TerminalOpen(TerminalOpen),
    TerminalOpened(TerminalOpened),
    TerminalInput(TerminalInput),
    TerminalOutput(TerminalOutput),
    TerminalResize(TerminalResize),
    TerminalClose(TerminalClose),
    TerminalExited(TerminalExited),
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<AgentCapability>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    PtyTerminal,
    PrivilegedRelease,
}

impl std::fmt::Display for AgentCapability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::PtyTerminal => "pty_terminal",
            Self::PrivilegedRelease => "privileged_release",
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HelloAck {
    pub connection_id: String,
    pub connection_generation: u64,
    pub protocol_version: u16,
    pub heartbeat_interval_seconds: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry_interval_seconds: Option<u32>,
}

impl HelloAck {
    pub fn validate_for_envelope_version(&self, envelope_version: u16) -> bool {
        envelope_version == self.protocol_version
            && (MIN_SUPPORTED_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&self.protocol_version)
            && (5..=300).contains(&self.heartbeat_interval_seconds)
            && match self.protocol_version {
                12 => self
                    .telemetry_interval_seconds
                    .is_some_and(|interval| (10..=300).contains(&interval)),
                11 => self.telemetry_interval_seconds.is_none(),
                _ => false,
            }
    }
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
pub struct NodeTelemetry {
    pub connection_generation: u64,
    pub sample_sequence: u64,
    pub captured_at: String,
    pub snapshot: NodeTelemetrySnapshot,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeTelemetrySnapshot {
    pub cpu: CpuTelemetry,
    pub memory: MemoryTelemetry,
    pub work_root_disk: DiskTelemetry,
    pub disk_io: DiskIoTelemetry,
    pub network: NetworkTelemetry,
    pub gpu_status: TelemetryMetricStatus,
    pub gpu_reason: Option<TelemetryMetricReason>,
    pub gpus: Vec<GpuTelemetry>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryMetricStatus {
    Available,
    WarmingUp,
    Unsupported,
    CollectionError,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryMetricReason {
    HardwareNotPresent,
    UnsupportedPlatform,
    BackendUnavailable,
    PermissionDenied,
    Timeout,
    ParseError,
    SourceUnavailable,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CpuTelemetry {
    pub status: TelemetryMetricStatus,
    pub usage_percent: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryTelemetry {
    pub status: TelemetryMetricStatus,
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub usage_percent: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiskTelemetry {
    pub status: TelemetryMetricStatus,
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub usage_percent: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiskIoTelemetry {
    pub status: TelemetryMetricStatus,
    pub read_bytes_per_second: Option<f64>,
    pub write_bytes_per_second: Option<f64>,
    pub busy_percent: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkTelemetry {
    pub status: TelemetryMetricStatus,
    pub receive_bytes_per_second: Option<f64>,
    pub transmit_bytes_per_second: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GpuTelemetry {
    pub index: u8,
    pub status: TelemetryMetricStatus,
    pub model: Option<String>,
    pub utilization_percent: Option<f64>,
    pub memory_total_bytes: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub temperature_celsius: Option<f64>,
}

impl NodeTelemetry {
    pub fn validate(&self) -> Result<(), TelemetryValidationError> {
        if self.connection_generation == 0
            || self.sample_sequence == 0
            || self.snapshot.gpus.len() > NODE_TELEMETRY_MAX_GPUS
            || !valid_percent(self.snapshot.cpu.usage_percent)
            || !valid_capacity(
                self.snapshot.memory.total_bytes,
                self.snapshot.memory.used_bytes,
                self.snapshot.memory.usage_percent,
            )
            || !valid_capacity(
                self.snapshot.work_root_disk.total_bytes,
                self.snapshot.work_root_disk.used_bytes,
                self.snapshot.work_root_disk.usage_percent,
            )
            || !valid_nonnegative(self.snapshot.disk_io.read_bytes_per_second)
            || !valid_nonnegative(self.snapshot.disk_io.write_bytes_per_second)
            || !valid_percent(self.snapshot.disk_io.busy_percent)
            || !valid_nonnegative(self.snapshot.network.receive_bytes_per_second)
            || !valid_nonnegative(self.snapshot.network.transmit_bytes_per_second)
            || !status_matches(
                self.snapshot.cpu.status,
                [self.snapshot.cpu.usage_percent.is_some()],
            )
            || !status_matches(
                self.snapshot.memory.status,
                [
                    self.snapshot.memory.total_bytes.is_some(),
                    self.snapshot.memory.used_bytes.is_some(),
                    self.snapshot.memory.usage_percent.is_some(),
                ],
            )
            || !status_matches(
                self.snapshot.work_root_disk.status,
                [
                    self.snapshot.work_root_disk.total_bytes.is_some(),
                    self.snapshot.work_root_disk.used_bytes.is_some(),
                    self.snapshot.work_root_disk.usage_percent.is_some(),
                ],
            )
            || !status_matches(
                self.snapshot.disk_io.status,
                [
                    self.snapshot.disk_io.read_bytes_per_second.is_some(),
                    self.snapshot.disk_io.write_bytes_per_second.is_some(),
                    self.snapshot.disk_io.busy_percent.is_some(),
                ],
            )
            || !status_matches(
                self.snapshot.network.status,
                [
                    self.snapshot.network.receive_bytes_per_second.is_some(),
                    self.snapshot.network.transmit_bytes_per_second.is_some(),
                ],
            )
            || !gpu_status_matches(self.snapshot.gpu_status, &self.snapshot.gpus)
            || !gpu_reason_matches(self.snapshot.gpu_status, self.snapshot.gpu_reason)
        {
            return Err(TelemetryValidationError);
        }

        let mut gpu_indexes = [false; NODE_TELEMETRY_MAX_GPUS];
        for gpu in &self.snapshot.gpus {
            let index = usize::from(gpu.index);
            if index >= NODE_TELEMETRY_MAX_GPUS
                || std::mem::replace(&mut gpu_indexes[index], true)
                || gpu.model.as_ref().is_some_and(|model| {
                    model.is_empty() || model.len() > 128 || model.chars().any(char::is_control)
                })
                || !valid_percent(gpu.utilization_percent)
                || !valid_capacity(gpu.memory_total_bytes, gpu.memory_used_bytes, None)
                || gpu
                    .temperature_celsius
                    .is_some_and(|value| !value.is_finite() || !(-100.0..=300.0).contains(&value))
                || !status_matches(
                    gpu.status,
                    [
                        gpu.model.is_some(),
                        gpu.utilization_percent.is_some(),
                        gpu.memory_total_bytes.is_some(),
                        gpu.memory_used_bytes.is_some(),
                        gpu.temperature_celsius.is_some(),
                    ],
                )
            {
                return Err(TelemetryValidationError);
            }
        }
        Ok(())
    }
}

fn gpu_reason_matches(
    status: TelemetryMetricStatus,
    reason: Option<TelemetryMetricReason>,
) -> bool {
    match status {
        TelemetryMetricStatus::Available => reason.is_none(),
        TelemetryMetricStatus::Unsupported => matches!(
            reason,
            Some(
                TelemetryMetricReason::HardwareNotPresent
                    | TelemetryMetricReason::UnsupportedPlatform
            )
        ),
        TelemetryMetricStatus::CollectionError => matches!(
            reason,
            Some(
                TelemetryMetricReason::BackendUnavailable
                    | TelemetryMetricReason::PermissionDenied
                    | TelemetryMetricReason::Timeout
                    | TelemetryMetricReason::ParseError
                    | TelemetryMetricReason::SourceUnavailable
            )
        ),
        TelemetryMetricStatus::WarmingUp => reason.is_none(),
    }
}

fn status_matches<const N: usize>(status: TelemetryMetricStatus, values: [bool; N]) -> bool {
    match status {
        TelemetryMetricStatus::Available => values.into_iter().all(std::convert::identity),
        TelemetryMetricStatus::WarmingUp
        | TelemetryMetricStatus::Unsupported
        | TelemetryMetricStatus::CollectionError => values.into_iter().all(|present| !present),
    }
}

fn gpu_status_matches(status: TelemetryMetricStatus, gpus: &[GpuTelemetry]) -> bool {
    match status {
        TelemetryMetricStatus::Available => !gpus.is_empty(),
        TelemetryMetricStatus::WarmingUp
        | TelemetryMetricStatus::Unsupported
        | TelemetryMetricStatus::CollectionError => gpus.is_empty(),
    }
}

fn valid_percent(value: Option<f64>) -> bool {
    value.is_none_or(|value| value.is_finite() && (0.0..=100.0).contains(&value))
}

fn valid_nonnegative(value: Option<f64>) -> bool {
    value.is_none_or(|value| value.is_finite() && value >= 0.0)
}

fn valid_capacity(total: Option<u64>, used: Option<u64>, percent: Option<f64>) -> bool {
    valid_percent(percent) && !matches!((total, used), (Some(total), Some(used)) if used > total)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TelemetryValidationError;

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
#[allow(clippy::large_enum_variant)] // 线协议保持携带完整任务载荷，受帧上限约束
pub enum TaskPayload {
    SystemInspect(SystemInspectTask),
    DeploymentExecute(DeploymentExecuteTask),
    HealthDiagnose(HealthDiagnoseTask),
    GitRefsQuery(GitRefsQueryTask),
    DeploymentPrepare(DeploymentPrepareTask),
    DeploymentRelease(DeploymentReleaseTask),
    EnvSync(EnvSyncTask),
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
pub struct GitRefsQueryTask {
    pub refs_query_id: String,
    pub repository_url: String,
    pub git_credential_lease_id: Option<String>,
    pub timeout_seconds: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentPrepareTask {
    pub deployment_id: String,
    pub source_policy: SourcePolicy,
    pub repository_url: String,
    pub commit_sha: String,
    pub checkout_dir: String,
    pub work_root: String,
    pub output_dir: String,
    pub environment: Environment,
    pub release_version: String,
    pub modules: Vec<String>,
    pub make_target: MakeTarget,
    pub git_credential_lease_id: Option<String>,
    pub timeout_seconds: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_upload: Option<ArtifactUploadRequest>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentReleaseTask {
    pub deployment_id: String,
    pub target_code: String,
    pub work_root: String,
    pub checkout_dir: String,
    pub artifact_dir: String,
    pub environment: Environment,
    pub release_version: String,
    pub commit_sha: String,
    pub modules: Vec<String>,
    pub make_target: MakeTarget,
    pub timeout_seconds: u32,
    pub cancel_file: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub privileged: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privileged_context: Option<PrivilegedReleaseContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_download: Option<ArtifactDownloadRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_credential_lease_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_env: Vec<RequiredEnvVersion>,
    #[serde(default, skip_serializing_if = "ReleaseCheckoutMode::is_git")]
    pub checkout_mode: ReleaseCheckoutMode,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseCheckoutMode {
    #[default]
    Git,
    Artifact,
}

impl ReleaseCheckoutMode {
    pub const fn is_git(value: &Self) -> bool {
        matches!(value, Self::Git)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrivilegedReleaseContext {
    pub target_run_id: String,
    pub target_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub snapshot_hash: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredEnvVersion {
    pub file_name: String,
    pub env_version: u64,
    pub digest: String,
    pub action: EnvSyncAction,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvSyncTask {
    pub env_sync_id: String,
    pub application_slug: String,
    pub file_name: String,
    pub env_version: u64,
    pub digest: String,
    pub lease_id: String,
    pub action: EnvSyncAction,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvSyncAction {
    Write,
    Delete,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactUploadRequest {
    pub authorization_id: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDownloadRequest {
    pub target_run_id: String,
    pub lease_id: String,
    pub archive_digest: String,
    pub manifest_digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPrepared {
    pub task_id: String,
    pub authorization_id: String,
    pub deployment_id: String,
    pub manifest_json: String,
    pub manifest_digest: String,
    pub total_size: u64,
    pub file_count: u32,
    pub archive_size: u64,
    pub archive_digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactUploadAuthorized {
    pub task_id: String,
    pub authorization_id: String,
    pub lease_id: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseFileDigest {
    pub relative_path: String,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseAuthorizationRequest {
    pub task_id: String,
    pub authorization_id: String,
    pub target_run_id: String,
    pub target_id: String,
    pub snapshot_hash: String,
    pub checkout_tree_digest: String,
    pub artifact_manifest_digest: String,
    pub artifacts: Vec<ReleaseFileDigest>,
    pub env_files: Vec<ReleaseFileDigest>,
    pub cancel_file: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseAuthorizationResponse {
    pub task_id: String,
    pub authorization_id: String,
    pub authorization: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalOpen {
    pub session_id: String,
    #[serde(deserialize_with = "deserialize_terminal_open_sequence")]
    pub sequence: u64,
    #[serde(deserialize_with = "deserialize_terminal_columns")]
    pub columns: u16,
    #[serde(deserialize_with = "deserialize_terminal_rows")]
    pub rows: u16,
    pub connection_generation: i64,
    pub capability: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalOpened {
    pub session_id: String,
    #[serde(deserialize_with = "deserialize_terminal_sequence")]
    pub sequence: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalInput {
    pub session_id: String,
    #[serde(deserialize_with = "deserialize_terminal_sequence")]
    pub sequence: u64,
    pub encoding: TerminalBytesEncoding,
    #[serde(deserialize_with = "deserialize_terminal_frame")]
    pub data: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalOutput {
    pub session_id: String,
    #[serde(deserialize_with = "deserialize_terminal_sequence")]
    pub sequence: u64,
    pub encoding: TerminalBytesEncoding,
    #[serde(deserialize_with = "deserialize_terminal_frame")]
    pub data: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalResize {
    pub session_id: String,
    #[serde(deserialize_with = "deserialize_terminal_sequence")]
    pub sequence: u64,
    #[serde(deserialize_with = "deserialize_terminal_columns")]
    pub columns: u16,
    #[serde(deserialize_with = "deserialize_terminal_rows")]
    pub rows: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalClose {
    pub session_id: String,
    #[serde(deserialize_with = "deserialize_terminal_sequence")]
    pub sequence: u64,
    pub reason: TerminalCloseReason,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalExited {
    pub session_id: String,
    #[serde(deserialize_with = "deserialize_terminal_sequence")]
    pub sequence: u64,
    pub reason: TerminalExitReason,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalBytesEncoding {
    Base64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalCloseReason {
    AdministratorRequest,
    BrowserDisconnected,
    AuthorizationRevoked,
    IdleTimeout,
    LifetimeExceeded,
    ProtocolError,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalExitReason {
    ProcessExited,
    AdministratorRequest,
    PeerDisconnected,
    AuthorizationRevoked,
    IdleTimeout,
    LifetimeExceeded,
    OutputLimitExceeded,
    ProtocolError,
    ExecutorUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageDirection {
    ServerToAgent,
    AgentToServer,
    Bidirectional,
}

impl Message {
    pub fn direction(&self) -> MessageDirection {
        match self {
            Self::TerminalOpen(_)
            | Self::TerminalInput(_)
            | Self::TerminalResize(_)
            | Self::TerminalClose(_) => MessageDirection::ServerToAgent,
            Self::TerminalOpened(_) | Self::TerminalOutput(_) | Self::TerminalExited(_) => {
                MessageDirection::AgentToServer
            }
            Self::ReleaseAuthorizationRequest(_) => MessageDirection::AgentToServer,
            Self::ReleaseAuthorizationResponse(_) => MessageDirection::ServerToAgent,
            Self::NodeTelemetry(_) => MessageDirection::AgentToServer,
            _ => MessageDirection::Bidirectional,
        }
    }

    pub fn validate_direction(
        &self,
        expected: MessageDirection,
    ) -> Result<(), MessageDirectionError> {
        let actual = self.direction();
        if actual == MessageDirection::Bidirectional || actual == expected {
            Ok(())
        } else {
            Err(MessageDirectionError { expected, actual })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageDirectionError {
    pub expected: MessageDirection,
    pub actual: MessageDirection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalSequenceTracker {
    session_id: String,
    next_sequence: u64,
}

impl TerminalSequenceTracker {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self::with_first_sequence(session_id, 0)
    }

    pub fn with_first_sequence(session_id: impl Into<String>, first_sequence: u64) -> Self {
        Self {
            session_id: session_id.into(),
            next_sequence: first_sequence,
        }
    }

    pub fn accept(&mut self, session_id: &str, sequence: u64) -> Result<(), TerminalSequenceError> {
        if session_id != self.session_id {
            return Err(TerminalSequenceError::WrongSession);
        }
        if sequence != self.next_sequence {
            return Err(TerminalSequenceError::Unexpected {
                expected: self.next_sequence,
                received: sequence,
            });
        }
        self.next_sequence = sequence.saturating_add(1);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalSequenceError {
    WrongSession,
    Unexpected { expected: u64, received: u64 },
}

fn deserialize_terminal_open_sequence<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u64::deserialize(deserializer)?;
    if value == 0 {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "terminal open sequence must be zero",
        ))
    }
}

fn deserialize_terminal_sequence<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u64::deserialize(deserializer)?;
    if value > 0 {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "terminal message sequence must be positive",
        ))
    }
}

fn deserialize_terminal_columns<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_bounded_u16(
        deserializer,
        TERMINAL_MIN_COLUMNS,
        TERMINAL_MAX_COLUMNS,
        "terminal columns",
    )
}

fn deserialize_terminal_rows<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_bounded_u16(
        deserializer,
        TERMINAL_MIN_ROWS,
        TERMINAL_MAX_ROWS,
        "terminal rows",
    )
}

fn deserialize_bounded_u16<'de, D>(
    deserializer: D,
    minimum: u16,
    maximum: u16,
    label: &str,
) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u16::deserialize(deserializer)?;
    if (minimum..=maximum).contains(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format_args!(
            "{label} must be between {minimum} and {maximum}"
        )))
    }
}

fn deserialize_terminal_frame<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.is_empty() || value.len() > TERMINAL_MAX_FRAME_ENCODED_BYTES {
        return Err(serde::de::Error::custom(
            "terminal frame size is outside the allowed range",
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
    {
        return Err(serde::de::Error::custom(
            "terminal frame is not base64 encoded",
        ));
    }
    Ok(value)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePolicy {
    Branch,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MakeTarget {
    DeployGoPrepare,
    DeployGoRelease,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Dev,
    Test,
    Staging,
    #[serde(rename = "prod")]
    Production,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStage {
    Prepare,
    Release,
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
#[serde(deny_unknown_fields)]
pub struct TaskProgress {
    pub task_id: String,
    pub sequence: u64,
    pub event: DeployEvent,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeployEvent {
    pub deploy_id: String,
    pub stage: DeploymentStage,
    pub event: DeployEventName,
    pub timestamp: String,
    pub status: DeployEventStatus,
    pub environment: Environment,
    pub release_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_switched: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum DeployEventName {
    #[serde(rename = "deploy.started")]
    DeployStarted,
    #[serde(rename = "deploy.finished")]
    DeployFinished,
    #[serde(rename = "deploy.preflight.started")]
    PreflightStarted,
    #[serde(rename = "deploy.preflight.succeeded")]
    PreflightSucceeded,
    #[serde(rename = "deploy.preflight.failed")]
    PreflightFailed,
    #[serde(rename = "deploy.module.started")]
    ModuleStarted,
    #[serde(rename = "deploy.module.succeeded")]
    ModuleSucceeded,
    #[serde(rename = "deploy.module.failed")]
    ModuleFailed,
    #[serde(rename = "deploy.step.started")]
    StepStarted,
    #[serde(rename = "deploy.step.succeeded")]
    StepSucceeded,
    #[serde(rename = "deploy.step.failed")]
    StepFailed,
    #[serde(rename = "deploy.verification.started")]
    VerificationStarted,
    #[serde(rename = "deploy.verification.succeeded")]
    VerificationSucceeded,
    #[serde(rename = "deploy.verification.failed")]
    VerificationFailed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployEventStatus {
    Started,
    Succeeded,
    Failed,
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
#[serde(deny_unknown_fields)]
pub struct SecretLeaseRequest {
    pub task_id: String,
    pub lease_id: String,
    pub payload_digest: String,
    pub purpose: SecretLeasePurpose,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretLeasePurpose {
    GitCredential,
    ApplicationEnv,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecretLeaseResponse {
    pub lease_id: String,
    pub private_key: String,
    pub expires_at: String,
    pub error_code: Option<String>,
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
        if (MIN_SUPPORTED_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&self.protocol_version) {
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
        assert_eq!(
            envelope.validate_version().unwrap_err().received,
            PROTOCOL_VERSION + 1
        );
    }

    #[test]
    fn round_trips_two_stage_task_payloads() {
        let refs = TaskPayload::GitRefsQuery(GitRefsQueryTask {
            refs_query_id: "refs_01".into(),
            repository_url: "git@git.example.test:deploy-go/example.git".into(),
            git_credential_lease_id: Some("lease_01".into()),
            timeout_seconds: 60,
        });
        let prepare = TaskPayload::DeploymentPrepare(DeploymentPrepareTask {
            deployment_id: "dep_01".into(),
            source_policy: SourcePolicy::Branch,
            repository_url: "git@git.example.test:deploy-go/example.git".into(),
            commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            checkout_dir: "/srv/tasks/task_01/checkout".into(),
            work_root: "/srv/tasks/task_01".into(),
            output_dir: "/srv/tasks/task_01/staging".into(),
            environment: Environment::Staging,
            release_version: "20260806183000".into(),
            modules: vec!["api".into(), "web".into()],
            make_target: MakeTarget::DeployGoPrepare,
            git_credential_lease_id: Some("lease_01".into()),
            timeout_seconds: 900,
            artifact_upload: None,
        });
        let release = TaskPayload::DeploymentRelease(DeploymentReleaseTask {
            deployment_id: "dep_01".into(),
            target_code: "qfy-test".into(),
            work_root: "/srv/tasks/task_02".into(),
            checkout_dir: "/srv/tasks/task_02/checkout".into(),
            artifact_dir: "/srv/tasks/task_02/staging".into(),
            environment: Environment::Production,
            release_version: "20260806183000".into(),
            commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            modules: vec!["api".into()],
            make_target: MakeTarget::DeployGoRelease,
            timeout_seconds: 900,
            cancel_file: "/srv/tasks/task_02/cancel".into(),
            privileged: false,
            privileged_context: None,
            artifact_download: None,
            repository_url: None,
            git_credential_lease_id: None,
            application_slug: Some("voucher-production".into()),
            required_env: vec![RequiredEnvVersion {
                file_name: "api.env".into(),
                env_version: 2,
                digest: "a".repeat(64),
                action: EnvSyncAction::Write,
            }],
            checkout_mode: ReleaseCheckoutMode::Git,
        });
        let env_sync = TaskPayload::EnvSync(EnvSyncTask {
            env_sync_id: "envsync_01".into(),
            application_slug: "voucher-production".into(),
            file_name: "api.env".into(),
            env_version: 2,
            digest: "a".repeat(64),
            lease_id: "envlease_01".into(),
            action: EnvSyncAction::Write,
        });
        for task in [refs, prepare, release, env_sync] {
            let envelope = Envelope {
                protocol_version: PROTOCOL_VERSION,
                message_id: "msg_01".into(),
                sent_at: "2026-08-06T03:00:00Z".into(),
                message: Message::TaskDispatch(TaskDispatch {
                    task_id: "task_01".into(),
                    idempotency_key: "idem-0123456789abcdef".into(),
                    deadline_at: "2026-08-06T03:10:00Z".into(),
                    payload_digest: "sha256:abc".into(),
                    task,
                }),
            };
            let json = serde_json::to_string(&envelope).unwrap();
            assert_eq!(serde_json::from_str::<Envelope>(&json).unwrap(), envelope);
        }
    }

    #[test]
    fn round_trips_progress_and_secret_lease_messages() {
        let progress = Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: "msg_02".into(),
            sent_at: "2026-08-06T03:00:00Z".into(),
            message: Message::TaskProgress(TaskProgress {
                task_id: "task_01".into(),
                sequence: 1,
                event: DeployEvent {
                    deploy_id: "dep_01".into(),
                    stage: DeploymentStage::Prepare,
                    event: DeployEventName::ModuleStarted,
                    timestamp: "2026-08-06T03:00:01Z".into(),
                    status: DeployEventStatus::Started,
                    environment: Environment::Staging,
                    release_version: "20260806183000".into(),
                    target: None,
                    module: Some("api".into()),
                    module_name: Some("API".into()),
                    step_id: None,
                    step: None,
                    message: Some("build api".into()),
                    failure_stage: None,
                    recovery_hint: None,
                    candidate_release: None,
                    current_release: None,
                    current_switched: None,
                },
            }),
        };
        let lease_request = Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: "msg_03".into(),
            sent_at: "2026-08-06T03:00:00Z".into(),
            message: Message::SecretLeaseRequest(SecretLeaseRequest {
                task_id: "task_01".into(),
                lease_id: "lease_01".into(),
                payload_digest: "sha256:abc".into(),
                purpose: SecretLeasePurpose::GitCredential,
            }),
        };
        let lease_response = Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: "msg_04".into(),
            sent_at: "2026-08-06T03:00:01Z".into(),
            message: Message::SecretLeaseResponse(SecretLeaseResponse {
                lease_id: "lease_01".into(),
                private_key: "opaque-private-key".into(),
                expires_at: "2026-08-06T03:05:00Z".into(),
                error_code: None,
            }),
        };
        for envelope in [progress, lease_request, lease_response] {
            let json = serde_json::to_string(&envelope).unwrap();
            assert_eq!(serde_json::from_str::<Envelope>(&json).unwrap(), envelope);
        }
    }

    #[test]
    fn rejects_inline_secrets_arbitrary_targets_and_unknown_fields() {
        let with_private_key = r#"{"protocol_version":2,"message_id":"msg","sent_at":"2026-08-06T03:00:00Z","message":{"type":"task_dispatch","task_id":"task","idempotency_key":"idem-0123456789abcdef","deadline_at":"2026-08-06T03:10:00Z","payload_digest":"sha256:abc","task":{"kind":"deployment_prepare","payload":{"deployment_id":"dep","source_policy":"branch","repository_url":"git@git.example.test:deploy-go/example.git","commit_sha":"0123456789abcdef0123456789abcdef01234567","checkout_dir":"/srv/tasks/task/checkout","work_root":"/srv/tasks/task","output_dir":"/srv/tasks/task/staging","environment":"staging","release_version":"20260806183000","modules":["api"],"make_target":"deploy_go_prepare","git_credential_lease_id":null,"timeout_seconds":900,"private_key":"BEGIN PRIVATE KEY"}}}}"#;
        assert!(serde_json::from_str::<Envelope>(with_private_key).is_err());

        let arbitrary_target = r#"{"protocol_version":2,"message_id":"msg","sent_at":"2026-08-06T03:00:00Z","message":{"type":"task_dispatch","task_id":"task","idempotency_key":"idem-0123456789abcdef","deadline_at":"2026-08-06T03:10:00Z","payload_digest":"sha256:abc","task":{"kind":"deployment_release","payload":{"deployment_id":"dep","target_code":"qfy-test","work_root":"/srv/tasks/task","checkout_dir":"/srv/tasks/task/checkout","artifact_dir":"/srv/tasks/task/staging","environment":"prod","release_version":"20260806183000","commit_sha":"0123456789abcdef0123456789abcdef01234567","modules":["api"],"make_target":"deploy","timeout_seconds":900,"cancel_file":"/srv/tasks/task/cancel"}}}}"#;
        assert!(serde_json::from_str::<Envelope>(arbitrary_target).is_err());
    }
}
