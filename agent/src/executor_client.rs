use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use deploy_go_agent_executor::protocol::{
    ExecutorCapability, FrameError, MAX_FRAME_BYTES, PROTOCOL_VERSION, ProbeRequest, Request,
    Response, VersionProbeRequest, VersionResponse, read_response, write_message,
};
use tokio::{
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::Mutex,
};

pub const DEFAULT_EXECUTOR_SOCKET_PATH: &str = "/run/deploy-go-agent/executor.sock";

#[derive(Debug, thiserror::Error)]
pub enum ExecutorClientError {
    #[error("executor unavailable")]
    Unavailable { io_kind: io::ErrorKind },
    #[error("executor request frame exceeds limit")]
    RequestTooLarge {
        actual_bytes: usize,
        limit_bytes: usize,
    },
    #[error("executor request could not be encoded")]
    RequestEncodeFailed,
    #[error("executor request write failed")]
    RequestWriteFailed { io_kind: io::ErrorKind },
    #[error("executor response timed out")]
    ResponseTimeout,
    #[error("executor closed before returning a response")]
    ResponseClosed,
    #[error("executor response frame exceeds limit")]
    ResponseTooLarge {
        actual_bytes: usize,
        limit_bytes: usize,
    },
    #[error("executor returned an empty response frame")]
    ResponseEmpty,
    #[error("executor response could not be decoded")]
    ResponseInvalid,
    #[error("executor response frame was truncated")]
    ResponseTruncated,
    #[error("executor response read failed")]
    ResponseReadFailed { io_kind: io::ErrorKind },
}

impl ExecutorClientError {
    pub fn request_may_have_been_processed(&self) -> bool {
        matches!(
            self,
            Self::RequestWriteFailed { .. }
                | Self::ResponseTimeout
                | Self::ResponseClosed
                | Self::ResponseTooLarge { .. }
                | Self::ResponseEmpty
                | Self::ResponseInvalid
                | Self::ResponseTruncated
                | Self::ResponseReadFailed { .. }
        )
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "executor_unavailable",
            Self::RequestTooLarge { .. } => "executor_request_too_large",
            Self::RequestEncodeFailed => "executor_request_encode_failed",
            Self::RequestWriteFailed { .. } => "executor_request_write_failed",
            Self::ResponseTimeout => "executor_response_timeout",
            Self::ResponseClosed => "executor_response_closed",
            Self::ResponseTooLarge { .. } => "executor_response_too_large",
            Self::ResponseEmpty => "executor_response_empty",
            Self::ResponseInvalid => "executor_response_invalid",
            Self::ResponseTruncated => "executor_response_truncated",
            Self::ResponseReadFailed { .. } => "executor_response_read_failed",
        }
    }

    pub fn request_frame_bytes(&self) -> Option<usize> {
        match self {
            Self::RequestTooLarge { actual_bytes, .. } => Some(*actual_bytes),
            _ => None,
        }
    }

    pub fn response_frame_bytes(&self) -> Option<usize> {
        match self {
            Self::ResponseTooLarge { actual_bytes, .. } => Some(*actual_bytes),
            _ => None,
        }
    }

    pub fn frame_limit_bytes(&self) -> Option<usize> {
        match self {
            Self::RequestTooLarge { limit_bytes, .. }
            | Self::ResponseTooLarge { limit_bytes, .. } => Some(*limit_bytes),
            _ => None,
        }
    }

    pub fn io_kind(&self) -> Option<io::ErrorKind> {
        match self {
            Self::Unavailable { io_kind }
            | Self::RequestWriteFailed { io_kind }
            | Self::ResponseReadFailed { io_kind } => Some(*io_kind),
            _ => None,
        }
    }
}

pub struct ExecutorConnection {
    pub reader: Mutex<OwnedReadHalf>,
    writer: Mutex<OwnedWriteHalf>,
}

impl ExecutorConnection {
    pub async fn connect(path: &Path) -> Result<Self, ExecutorClientError> {
        let stream =
            UnixStream::connect(path)
                .await
                .map_err(|error| ExecutorClientError::Unavailable {
                    io_kind: error.kind(),
                })?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
        })
    }

    pub async fn send(&self, request: &Request) -> Result<(), ExecutorClientError> {
        write_message(&mut *self.writer.lock().await, request, MAX_FRAME_BYTES)
            .await
            .map_err(map_request_error)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ExecutorClient {
    socket_path: PathBuf,
}

impl ExecutorClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub async fn connect(&self) -> Result<ExecutorConnection, ExecutorClientError> {
        ExecutorConnection::connect(&self.socket_path).await
    }

    pub async fn probe(&self) -> bool {
        self.probe_capabilities()
            .await
            .is_some_and(|capabilities| capabilities.contains(&ExecutorCapability::PtyTerminal))
    }

    pub async fn probe_capabilities(&self) -> Option<Vec<ExecutorCapability>> {
        let Ok(connection) = self.connect().await else {
            return None;
        };
        if connection
            .send(&Request::Probe(ProbeRequest {
                version: PROTOCOL_VERSION,
            }))
            .await
            .is_err()
        {
            return None;
        }
        match tokio::time::timeout(
            std::time::Duration::from_secs(1),
            read_response(&mut *connection.reader.lock().await, MAX_FRAME_BYTES),
        )
        .await
        {
            Ok(Ok(Some(Response::Healthy(response)))) if response.version == PROTOCOL_VERSION => {
                Some(response.capabilities)
            }
            _ => None,
        }
    }

    pub async fn probe_version(&self) -> Option<String> {
        let Ok(connection) = self.connect().await else {
            return None;
        };
        if connection
            .send(&Request::VersionProbe(VersionProbeRequest {
                version: PROTOCOL_VERSION,
            }))
            .await
            .is_err()
        {
            return None;
        }
        match tokio::time::timeout(
            std::time::Duration::from_secs(1),
            read_response(&mut *connection.reader.lock().await, MAX_FRAME_BYTES),
        )
        .await
        {
            Ok(Ok(Some(Response::Version(VersionResponse {
                version,
                package_version,
            }))))
                if version == PROTOCOL_VERSION && !package_version.is_empty() =>
            {
                Some(package_version)
            }
            _ => None,
        }
    }

    pub async fn request(&self, request: Request) -> Result<Response, ExecutorClientError> {
        self.request_with_timeout(request, Duration::from_secs(5))
            .await
    }

    pub async fn request_with_timeout(
        &self,
        request: Request,
        timeout: Duration,
    ) -> Result<Response, ExecutorClientError> {
        let connection = self.connect().await?;
        connection.send(&request).await?;
        match tokio::time::timeout(
            timeout,
            read_response(&mut *connection.reader.lock().await, MAX_FRAME_BYTES),
        )
        .await
        {
            Ok(Ok(Some(response))) => Ok(response),
            Ok(Ok(None)) => Err(ExecutorClientError::ResponseClosed),
            Ok(Err(error)) => Err(map_response_error(error)),
            Err(_) => Err(ExecutorClientError::ResponseTimeout),
        }
    }
}

fn map_request_error(error: FrameError) -> ExecutorClientError {
    match error {
        FrameError::TooLarge {
            actual_bytes,
            limit_bytes,
        } => ExecutorClientError::RequestTooLarge {
            actual_bytes,
            limit_bytes,
        },
        FrameError::Io(error) => ExecutorClientError::RequestWriteFailed {
            io_kind: error.kind(),
        },
        FrameError::Empty | FrameError::Invalid(_) => ExecutorClientError::RequestEncodeFailed,
    }
}

fn map_response_error(error: FrameError) -> ExecutorClientError {
    match error {
        FrameError::TooLarge {
            actual_bytes,
            limit_bytes,
        } => ExecutorClientError::ResponseTooLarge {
            actual_bytes,
            limit_bytes,
        },
        FrameError::Empty => ExecutorClientError::ResponseEmpty,
        FrameError::Invalid(_) => ExecutorClientError::ResponseInvalid,
        FrameError::Io(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
            ExecutorClientError::ResponseTruncated
        }
        FrameError::Io(error) => ExecutorClientError::ResponseReadFailed {
            io_kind: error.kind(),
        },
    }
}
