use std::{path::PathBuf, sync::Arc};

use base64::{Engine, engine::general_purpose::STANDARD};
use deploy_go_agent_executor::protocol::{
    CloseReason, CloseRequest, InputRequest, MAX_FRAME_BYTES, OpenRequest,
    PROTOCOL_VERSION as EXECUTOR_VERSION, Request, ResizeRequest, Response, read_response,
};
use deploy_go_agent_protocol::{
    Message, TerminalBytesEncoding, TerminalClose, TerminalExitReason, TerminalExited,
    TerminalInput, TerminalOpen, TerminalOpened, TerminalOutput, TerminalResize,
    TerminalSequenceTracker,
};
use tokio::sync::{Mutex, mpsc};

use crate::executor_client::{ExecutorClient, ExecutorConnection};

struct ActiveSession {
    session_id: String,
    inbound: TerminalSequenceTracker,
    last_inbound_sequence: u64,
    connection: Arc<ExecutorConnection>,
    reader_task: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Clone)]
pub struct TerminalBridge {
    executor: ExecutorClient,
    active: Arc<Mutex<Option<ActiveSession>>>,
}

impl TerminalBridge {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            executor: ExecutorClient::new(socket_path),
            active: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn probe(&self) -> bool {
        self.executor.probe().await
    }

    pub async fn handle(&self, message: Message, outbound: mpsc::Sender<Message>) {
        match message {
            Message::TerminalOpen(open) => self.open(open, outbound).await,
            Message::TerminalInput(input) => self.input(input, outbound).await,
            Message::TerminalResize(resize) => self.resize(resize, outbound).await,
            Message::TerminalClose(close) => self.close_request(close).await,
            _ => {}
        }
    }

    async fn open(&self, open: TerminalOpen, outbound: mpsc::Sender<Message>) {
        let mut active = self.active.lock().await;
        if active.is_some() {
            send_exited(
                &outbound,
                &open.session_id,
                TerminalExitReason::ProtocolError,
            )
            .await;
            return;
        }
        let Ok(connection) = self.executor.connect().await else {
            send_exited(
                &outbound,
                &open.session_id,
                TerminalExitReason::ExecutorUnavailable,
            )
            .await;
            return;
        };
        let connection = Arc::new(connection);
        if connection
            .send(&Request::Open(OpenRequest {
                version: EXECUTOR_VERSION,
                session_id: open.session_id.clone(),
                sequence: open.sequence,
                rows: open.rows,
                cols: open.columns,
            }))
            .await
            .is_err()
        {
            send_exited(
                &outbound,
                &open.session_id,
                TerminalExitReason::ExecutorUnavailable,
            )
            .await;
            return;
        }
        *active = Some(ActiveSession {
            session_id: open.session_id.clone(),
            inbound: TerminalSequenceTracker::with_first_sequence(open.session_id, 1),
            last_inbound_sequence: 0,
            connection,
            reader_task: None,
        });
        let session = active.as_ref().unwrap();
        let session_id = session.session_id.clone();
        let reader_connection = Arc::clone(&session.connection);
        let active_state = Arc::clone(&self.active);
        let reader_task = tokio::spawn(async move {
            forward_responses(reader_connection, session_id.clone(), outbound).await;
            let mut active = active_state.lock().await;
            if active
                .as_ref()
                .is_some_and(|current| current.session_id == session_id)
            {
                active.take();
            }
        });
        active.as_mut().unwrap().reader_task = Some(reader_task);
    }

    async fn input(&self, input: TerminalInput, _outbound: mpsc::Sender<Message>) {
        let Ok(data) = STANDARD.decode(input.data.as_bytes()) else {
            self.close_active(CloseReason::ProtocolError, false).await;
            return;
        };
        self.forward(&input.session_id, input.sequence, |session| {
            Request::Input(InputRequest {
                version: EXECUTOR_VERSION,
                session_id: session.into(),
                sequence: input.sequence,
                data,
            })
        })
        .await;
    }

    async fn resize(&self, resize: TerminalResize, _outbound: mpsc::Sender<Message>) {
        self.forward(&resize.session_id, resize.sequence, |session| {
            Request::Resize(ResizeRequest {
                version: EXECUTOR_VERSION,
                session_id: session.into(),
                sequence: resize.sequence,
                rows: resize.rows,
                cols: resize.columns,
            })
        })
        .await;
    }

    async fn close_request(&self, close: TerminalClose) {
        let mut active = self.active.lock().await;
        let Some(session) = active.as_mut() else {
            return;
        };
        if session
            .inbound
            .accept(&close.session_id, close.sequence)
            .is_ok()
        {
            session.last_inbound_sequence = close.sequence;
            let _ = session
                .connection
                .send(&Request::Close(CloseRequest {
                    version: EXECUTOR_VERSION,
                    session_id: close.session_id,
                    sequence: close.sequence,
                    reason: map_close_reason(close.reason),
                }))
                .await;
        } else {
            drop(active);
            self.close_active(CloseReason::ProtocolError, false).await;
        }
    }

    async fn forward<F>(&self, session_id: &str, sequence: u64, request: F)
    where
        F: FnOnce(&str) -> Request,
    {
        let mut active = self.active.lock().await;
        let valid = active.as_mut().is_some_and(|session| {
            if session.inbound.accept(session_id, sequence).is_ok() {
                session.last_inbound_sequence = sequence;
                true
            } else {
                false
            }
        });
        if !valid {
            drop(active);
            self.close_active(CloseReason::ProtocolError, false).await;
            return;
        }
        let session = active.as_ref().unwrap();
        if session
            .connection
            .send(&request(&session.session_id))
            .await
            .is_err()
        {
            drop(active);
            self.close_active(CloseReason::ProtocolError, true).await;
        }
    }

    pub async fn close(&self) {
        self.close_active(CloseReason::PeerDisconnected, true).await;
    }

    async fn close_active(&self, reason: CloseReason, force: bool) {
        let mut active = self.active.lock().await;
        if let Some(session) = active.as_mut() {
            let close_sequence = session.last_inbound_sequence.saturating_add(1);
            let _ = session
                .connection
                .send(&Request::Close(CloseRequest {
                    version: EXECUTOR_VERSION,
                    session_id: session.session_id.clone(),
                    sequence: close_sequence,
                    reason,
                }))
                .await;
        }
        if force
            && let Some(mut session) = active.take()
            && let Some(task) = session.reader_task.take()
        {
            task.abort();
        }
    }
}

async fn forward_responses(
    connection: Arc<ExecutorConnection>,
    session_id: String,
    outbound: mpsc::Sender<Message>,
) {
    let mut sequence = 1_u64;
    let mut reader = connection.reader.lock().await;
    loop {
        let response = read_response(&mut *reader, MAX_FRAME_BYTES).await;
        let message = match response {
            Ok(Some(Response::Opened(value))) if value.session_id == session_id => {
                Message::TerminalOpened(TerminalOpened {
                    session_id: session_id.clone(),
                    sequence,
                })
            }
            Ok(Some(Response::Output(value))) if value.session_id == session_id => {
                Message::TerminalOutput(TerminalOutput {
                    session_id: session_id.clone(),
                    sequence,
                    encoding: TerminalBytesEncoding::Base64,
                    data: STANDARD.encode(value.data),
                })
            }
            Ok(Some(Response::Exited(value))) if value.session_id == session_id => {
                Message::TerminalExited(TerminalExited {
                    session_id: session_id.clone(),
                    sequence,
                    reason: map_reason(&value.reason),
                    exit_code: value.exit_code.map(|code| code as i32),
                })
            }
            Ok(Some(Response::Error(_))) => Message::TerminalExited(TerminalExited {
                session_id: session_id.clone(),
                sequence,
                reason: TerminalExitReason::ProtocolError,
                exit_code: None,
            }),
            _ => Message::TerminalExited(TerminalExited {
                session_id: session_id.clone(),
                sequence,
                reason: TerminalExitReason::ExecutorUnavailable,
                exit_code: None,
            }),
        };
        let terminal = matches!(message, Message::TerminalExited(_));
        if outbound.send(message).await.is_err() || terminal {
            break;
        }
        sequence = sequence.saturating_add(1);
    }
}

fn map_reason(reason: &str) -> TerminalExitReason {
    match reason {
        "process_exited" => TerminalExitReason::ProcessExited,
        "idle_timeout" => TerminalExitReason::IdleTimeout,
        "max_lifetime" => TerminalExitReason::LifetimeExceeded,
        "output_overflow" => TerminalExitReason::OutputLimitExceeded,
        "administrator_request" => TerminalExitReason::AdministratorRequest,
        "browser_disconnected" | "peer_disconnected" => TerminalExitReason::PeerDisconnected,
        "authorization_revoked" => TerminalExitReason::AuthorizationRevoked,
        "lifetime_exceeded" => TerminalExitReason::LifetimeExceeded,
        "protocol_error" => TerminalExitReason::ProtocolError,
        _ => TerminalExitReason::ProtocolError,
    }
}

fn map_close_reason(reason: deploy_go_agent_protocol::TerminalCloseReason) -> CloseReason {
    use deploy_go_agent_protocol::TerminalCloseReason;
    match reason {
        TerminalCloseReason::AdministratorRequest => CloseReason::AdministratorRequest,
        TerminalCloseReason::BrowserDisconnected => CloseReason::BrowserDisconnected,
        TerminalCloseReason::AuthorizationRevoked => CloseReason::AuthorizationRevoked,
        TerminalCloseReason::IdleTimeout => CloseReason::IdleTimeout,
        TerminalCloseReason::LifetimeExceeded => CloseReason::LifetimeExceeded,
        TerminalCloseReason::ProtocolError => CloseReason::ProtocolError,
    }
}

async fn send_exited(
    outbound: &mpsc::Sender<Message>,
    session_id: &str,
    reason: TerminalExitReason,
) {
    let _ = outbound
        .send(Message::TerminalExited(TerminalExited {
            session_id: session_id.into(),
            sequence: 1,
            reason,
            exit_code: None,
        }))
        .await;
}
