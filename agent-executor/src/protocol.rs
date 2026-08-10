use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const PROTOCOL_VERSION: u16 = 2;
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Request {
    Probe(ProbeRequest),
    Open(OpenRequest),
    Input(InputRequest),
    Resize(ResizeRequest),
    Close(CloseRequest),
    ReleaseStart(ReleaseStartRequest),
    ReleaseStatus(ReleaseStatusRequest),
    ReleaseOutput(ReleaseOutputRequest),
    ReleaseCancel(ReleaseCancelRequest),
    SelfTest(SelfTestRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Response {
    Healthy(HealthyResponse),
    Opened(OpenedResponse),
    Output(OutputResponse),
    Exited(ExitedResponse),
    ReleaseStarted(ReleaseStartedResponse),
    ReleaseStatus(ReleaseStatusResponse),
    ReleaseOutput(ReleaseOutputResponse),
    ReleaseExited(ReleaseExitedResponse),
    SelfTestResult(SelfTestResponse),
    Error(ErrorResponse),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeRequest {
    pub version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfTestRequest {
    pub version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfTestResponse {
    pub version: u16,
    pub succeeded: bool,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthyResponse {
    pub version: u16,
    pub capabilities: Vec<ExecutorCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorCapability {
    PtyTerminal,
    DeploymentRelease,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenedResponse {
    pub version: u16,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputResponse {
    pub version: u16,
    pub session_id: String,
    pub sequence: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitedResponse {
    pub version: u16,
    pub session_id: String,
    pub reason: String,
    pub exit_code: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    pub version: u16,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRequest {
    pub version: u16,
    pub session_id: String,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub connection_generation: i64,
    pub capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputRequest {
    pub version: u16,
    pub session_id: String,
    pub sequence: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResizeRequest {
    pub version: u16,
    pub session_id: String,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloseRequest {
    pub version: u16,
    pub session_id: String,
    pub sequence: u64,
    pub reason: CloseReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseStartRequest {
    pub version: u16,
    pub job_id: String,
    pub authorization: String,
    pub deployment_id: String,
    pub target_run_id: String,
    pub target_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub snapshot_hash: String,
    pub commit_sha: String,
    pub checkout_dir: String,
    pub artifact_dir: String,
    pub env_dir: String,
    pub cancel_file: String,
    pub environment: String,
    pub release_version: String,
    pub modules: Vec<String>,
    pub target_code: String,
    pub task_payload_digest: String,
    pub deadline_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseStatusRequest {
    pub version: u16,
    pub job_id: String,
    pub task_payload_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseOutputRequest {
    pub version: u16,
    pub job_id: String,
    pub task_payload_digest: String,
    pub after_sequence: u64,
    pub max_frames: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseCancelRequest {
    pub version: u16,
    pub job_id: String,
    pub task_payload_digest: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseStartedResponse {
    pub version: u16,
    pub job_id: String,
    pub state: ReleaseJobState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseStatusResponse {
    pub version: u16,
    pub job_id: String,
    pub state: ReleaseJobState,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseOutputResponse {
    pub version: u16,
    pub job_id: String,
    pub frames: Vec<ReleaseOutputFrame>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseOutputFrame {
    pub sequence: u64,
    pub stream: ReleaseOutputStream,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseExitedResponse {
    pub version: u16,
    pub job_id: String,
    pub state: ReleaseJobState,
    pub exit_code: Option<i32>,
    pub reason: String,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseJobState {
    Sealing,
    Running,
    Canceling,
    Succeeded,
    Failed,
    Canceled,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    AdministratorRequest,
    BrowserDisconnected,
    AuthorizationRevoked,
    IdleTimeout,
    LifetimeExceeded,
    ProtocolError,
    PeerDisconnected,
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame exceeds configured limit")]
    TooLarge,
    #[error("empty frame")]
    Empty,
    #[error("invalid protocol message")]
    Invalid(#[from] serde_json::Error),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
}

pub async fn read_request<R: AsyncRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<Request>, FrameError> {
    let length = match reader.read_u32().await {
        Ok(value) => value as usize,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if length == 0 {
        return Err(FrameError::Empty);
    }
    if length > max_bytes.min(MAX_FRAME_BYTES) {
        return Err(FrameError::TooLarge);
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload).await?;
    Ok(Some(serde_json::from_slice(&payload)?))
}

pub async fn read_response<R: AsyncRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<Response>, FrameError> {
    read_frame(reader, max_bytes).await
}

async fn read_frame<R: AsyncRead + Unpin, T: for<'de> Deserialize<'de>>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<T>, FrameError> {
    let length = match reader.read_u32().await {
        Ok(value) => value as usize,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if length == 0 {
        return Err(FrameError::Empty);
    }
    if length > max_bytes.min(MAX_FRAME_BYTES) {
        return Err(FrameError::TooLarge);
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload).await?;
    Ok(Some(serde_json::from_slice(&payload)?))
}

pub async fn write_message<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    message: &T,
    max_bytes: usize,
) -> Result<(), FrameError> {
    let payload = serde_json::to_vec(message)?;
    if payload.is_empty() {
        return Err(FrameError::Empty);
    }
    if payload.len() > max_bytes.min(MAX_FRAME_BYTES) {
        return Err(FrameError::TooLarge);
    }
    writer.write_u32(payload.len() as u32).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

pub fn validate_dimensions(rows: u16, cols: u16) -> bool {
    (2..=300).contains(&rows) && (2..=500).contains(&cols)
}

pub fn validate_request_sequence(request: &Request, previous: Option<u64>) -> bool {
    let sequence = match request {
        Request::Probe(_) => return previous.is_none(),
        Request::Open(value) => value.sequence,
        Request::Input(value) => value.sequence,
        Request::Resize(value) => value.sequence,
        Request::Close(value) => value.sequence,
        Request::ReleaseStart(_)
        | Request::ReleaseStatus(_)
        | Request::ReleaseOutput(_)
        | Request::ReleaseCancel(_)
        | Request::SelfTest(_) => return previous.is_none(),
    };
    match (request, previous) {
        (Request::Open(_), None) => sequence == 0,
        (Request::Open(_), Some(_)) => false,
        (_, Some(last)) => sequence == last + 1,
        (_, None) => false,
    }
}
