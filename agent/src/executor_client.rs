use std::{
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
    Unavailable,
    #[error("executor protocol error")]
    Protocol,
    #[error("executor frame error")]
    Frame(#[from] FrameError),
}

pub struct ExecutorConnection {
    pub reader: Mutex<OwnedReadHalf>,
    writer: Mutex<OwnedWriteHalf>,
}

impl ExecutorConnection {
    pub async fn connect(path: &Path) -> Result<Self, ExecutorClientError> {
        let stream = UnixStream::connect(path)
            .await
            .map_err(|_| ExecutorClientError::Unavailable)?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
        })
    }

    pub async fn send(&self, request: &Request) -> Result<(), ExecutorClientError> {
        write_message(&mut *self.writer.lock().await, request, MAX_FRAME_BYTES).await?;
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
            Ok(Err(error)) => Err(error.into()),
            _ => Err(ExecutorClientError::Protocol),
        }
    }
}
