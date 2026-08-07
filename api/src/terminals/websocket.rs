use std::time::{Duration, Instant};

use axum::{
    Router,
    extract::{Extension, Path, State, WebSocketUpgrade, ws::Message as WsMessage},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use deploy_go_agent_protocol::{
    Message, TERMINAL_MAX_COLUMNS, TERMINAL_MAX_FRAME_ENCODED_BYTES, TERMINAL_MAX_INPUT_BYTES,
    TERMINAL_MAX_ROWS, TERMINAL_MIN_COLUMNS, TERMINAL_MIN_ROWS, TerminalBytesEncoding,
    TerminalClose, TerminalCloseReason, TerminalInput, TerminalOpen, TerminalResize,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use ulid::Ulid;

use crate::{
    AppState, RequestId,
    auth::{self, AuthUser},
    error::{ApiError, ApiResult},
};

use super::{
    registry::{ForwardError, RegisterError},
    store,
};

const TERMINAL_PROTOCOL: &str = "deploy-go-terminal.v1";
const OPEN_TIMEOUT: Duration = Duration::from_secs(10);
const AUTHORIZATION_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const MAX_BROWSER_MESSAGE_SIZE: usize = 128 * 1024;
const MAX_INPUT_FRAMES_PER_SECOND: u32 = 100;
const MAX_INPUT_BYTES_PER_SECOND: usize = 256 * 1024;

struct InputRateLimit {
    window_started: Instant,
    frames: u32,
    bytes: usize,
}

impl InputRateLimit {
    fn new() -> Self {
        Self {
            window_started: Instant::now(),
            frames: 0,
            bytes: 0,
        }
    }

    fn accept(&mut self, bytes: usize) -> bool {
        if self.window_started.elapsed() >= Duration::from_secs(1) {
            self.window_started = Instant::now();
            self.frames = 0;
            self.bytes = 0;
        }
        if self.frames >= MAX_INPUT_FRAMES_PER_SECOND
            || self.bytes.saturating_add(bytes) > MAX_INPUT_BYTES_PER_SECOND
        {
            return false;
        }
        self.frames += 1;
        self.bytes += bytes;
        true
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum BrowserMessage {
    Open {
        columns: u16,
        rows: u16,
    },
    Input {
        sequence: u64,
        encoding: TerminalBytesEncoding,
        data: String,
    },
    Resize {
        sequence: u64,
        columns: u16,
        rows: u16,
    },
    Close {
        sequence: u64,
    },
}

pub fn router() -> Router<AppState> {
    Router::new().route("/terminal-sessions/{session_id}/stream", get(upgrade))
}

#[utoipa::path(
    operation_id = "terminals_stream",
    get,
    path = "/api/v1/terminal-sessions/{session_id}/stream",
    params(
        ("session_id" = String, Path),
        ("Origin" = String, Header),
        ("Sec-WebSocket-Protocol" = String, Header, description = "deploy-go-terminal.v1 与 csrf.<token>")
    ),
    responses(
        (status = 101, description = "WebSocket 已升级"),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse),
        (status = 409, body = crate::error::ErrorResponse)
    )
)]
pub(crate) async fn upgrade(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    websocket: WebSocketUpgrade,
) -> ApiResult<Response> {
    auth::verify_origin(&state, &headers, request_id.as_str())?;
    actor.require_administrator(request_id.as_str())?;
    let csrf = websocket_csrf(&headers).ok_or_else(|| ApiError::forbidden(request_id.as_str()))?;
    actor.verify_csrf_token(csrf, request_id.as_str())?;
    let session = store::find_session(state.pool(), &session_id)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    if session.actor_id != actor.id {
        return Err(ApiError::forbidden(request_id.as_str()));
    }
    if session.status != "opening" {
        return Err(ApiError::conflict(
            "terminal_session_not_attachable",
            "终端会话不能再次附着",
            request_id.as_str(),
        ));
    }
    let generation = match state.agent_connections().generation(&session.agent_id) {
        Some(generation) => generation,
        None => {
            if let Some(session) = store::finish_session(
                state.pool(),
                &session_id,
                "interrupted",
                "agent_disconnected",
                None,
            )
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?
            {
                super::registry::record_finished(&state, &session).await?;
            }
            return Err(ApiError::conflict(
                "terminal_agent_offline",
                "节点 Agent 当前离线",
                request_id.as_str(),
            ));
        }
    };
    let attachment_id = format!("attach_{}", Ulid::new());
    let registration = state
        .terminal_connections()
        .register(&session_id, attachment_id, &session.agent_id, generation)
        .map_err(|error| match error {
            RegisterError::AlreadyAttached => ApiError::conflict(
                "terminal_session_attached",
                "终端会话已被附着",
                request_id.as_str(),
            ),
        })?;
    let actor_id = actor.id;
    let auth_session_id = actor.session_id;
    Ok(websocket
        .max_message_size(MAX_BROWSER_MESSAGE_SIZE)
        .max_frame_size(MAX_BROWSER_MESSAGE_SIZE)
        .protocols([TERMINAL_PROTOCOL])
        .on_upgrade(move |socket| {
            run_socket(
                socket,
                state,
                session_id,
                registration,
                actor_id,
                auth_session_id,
            )
        })
        .into_response())
}

async fn run_socket(
    mut socket: axum::extract::ws::WebSocket,
    state: AppState,
    session_id: String,
    mut registration: super::registry::TerminalRegistration,
    actor_id: String,
    auth_session_id: String,
) {
    let attachment_id = registration.attachment_id.clone();
    let mut authorization_check = tokio::time::interval(AUTHORIZATION_CHECK_INTERVAL);
    authorization_check.tick().await;
    let open_timeout = tokio::time::sleep(OPEN_TIMEOUT);
    tokio::pin!(open_timeout);
    let mut agent_opened = false;
    let mut input_rate = InputRateLimit::new();
    loop {
        tokio::select! {
            _ = &mut open_timeout, if !agent_opened => {
                let _ = send_error(&mut socket, "terminal_open_timeout", "终端打开请求超时").await;
                let _ = state.terminal_connections().terminate(&state, &session_id, "open_timeout", "failed", None).await;
                break;
            }
            _ = authorization_check.tick() => {
                if !auth::session_is_active_administrator(&state, &auth_session_id, &actor_id).await {
                    let _ = state.terminal_connections().terminate(&state, &session_id, "authorization_revoked", "interrupted", None).await;
                    break;
                }
            }
            event = registration.receiver.recv() => {
                let Some(event) = event else { break; };
                if matches!(event, super::registry::BrowserEvent::Opened { .. }) {
                    agent_opened = true;
                }
                let terminal = matches!(event, super::registry::BrowserEvent::Exited { .. });
                let Ok(text) = serde_json::to_string(&event) else { break; };
                if socket.send(WsMessage::Text(text.into())).await.is_err() || terminal {
                    break;
                }
            }
            incoming = socket.next() => {
                let message = match incoming {
                    Some(Ok(WsMessage::Text(text))) => serde_json::from_str::<BrowserMessage>(&text).ok(),
                    Some(Ok(WsMessage::Ping(bytes))) => {
                        if socket.send(WsMessage::Pong(bytes)).await.is_err() { break; }
                        continue;
                    }
                    Some(Ok(WsMessage::Pong(_))) => continue,
                    Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(WsMessage::Binary(_))) => None,
                };
                let Some(message) = message else {
                    let _ = send_error(&mut socket, "terminal_message_invalid", "终端消息格式无效").await;
                    let _ = state.terminal_connections().terminate(&state, &session_id, "protocol_error", "failed", None).await;
                    break;
                };
                match handle_browser_message(
                    &state,
                    &session_id,
                    &attachment_id,
                    message,
                    &mut input_rate,
                ).await {
                    Ok(_) => {}
                    Err((code, message)) => {
                        let _ = send_error(&mut socket, code, message).await;
                        let _ = state.terminal_connections().terminate(&state, &session_id, "protocol_error", "failed", None).await;
                        break;
                    }
                }
            }
        }
    }
    let _ = state
        .terminal_connections()
        .terminate_attachment(
            &state,
            &session_id,
            &attachment_id,
            "browser_disconnected",
            "interrupted",
        )
        .await;
    let _ = socket.close().await;
}

async fn handle_browser_message(
    state: &AppState,
    session_id: &str,
    attachment_id: &str,
    message: BrowserMessage,
    input_rate: &mut InputRateLimit,
) -> Result<bool, (&'static str, &'static str)> {
    match message {
        BrowserMessage::Open { columns, rows } => {
            validate_size(columns, rows)?;
            let (agent_id, generation) = state
                .terminal_connections()
                .prepare_open(session_id, attachment_id)
                .map_err(forward_error)?;
            state
                .agent_connections()
                .try_send_generation(
                    &agent_id,
                    generation,
                    Message::TerminalOpen(TerminalOpen {
                        session_id: session_id.to_owned(),
                        sequence: 0,
                        columns,
                        rows,
                    }),
                )
                .map_err(|_| ("terminal_agent_unavailable", "Agent 终端通道不可用"))?;
            Ok(true)
        }
        BrowserMessage::Input {
            sequence,
            encoding,
            data,
        } => {
            if sequence == 0 || data.is_empty() || data.len() > TERMINAL_MAX_FRAME_ENCODED_BYTES {
                return Err(("terminal_input_invalid", "终端输入无效"));
            }
            let bytes = STANDARD
                .decode(&data)
                .map_err(|_| ("terminal_input_invalid", "终端输入编码无效"))?;
            if bytes.len() > TERMINAL_MAX_INPUT_BYTES {
                return Err(("terminal_input_invalid", "终端输入超过单帧限制"));
            }
            if !input_rate.accept(bytes.len()) {
                return Err(("terminal_input_rate_limited", "终端输入速率超过限制"));
            }
            let (agent_id, generation) = state
                .terminal_connections()
                .prepare_client_frame(session_id, attachment_id, sequence)
                .map_err(forward_error)?;
            let accepted = store::add_input_bytes(
                state.pool(),
                session_id,
                i64::try_from(bytes.len()).unwrap_or(i64::MAX),
                super::registry::MAX_SESSION_INPUT_BYTES,
            )
            .await
            .map_err(|_| ("terminal_state_failed", "终端状态更新失败"))?;
            if !accepted {
                return Err(("terminal_input_limit", "终端输入已超过会话限制"));
            }
            state
                .agent_connections()
                .try_send_generation(
                    &agent_id,
                    generation,
                    Message::TerminalInput(TerminalInput {
                        session_id: session_id.to_owned(),
                        sequence,
                        encoding,
                        data,
                    }),
                )
                .map_err(|_| ("terminal_agent_unavailable", "Agent 终端通道不可用"))?;
            Ok(false)
        }
        BrowserMessage::Resize {
            sequence,
            columns,
            rows,
        } => {
            validate_size(columns, rows)?;
            let (agent_id, generation) = state
                .terminal_connections()
                .prepare_client_frame(session_id, attachment_id, sequence)
                .map_err(forward_error)?;
            state
                .agent_connections()
                .try_send_generation(
                    &agent_id,
                    generation,
                    Message::TerminalResize(TerminalResize {
                        session_id: session_id.to_owned(),
                        sequence,
                        columns,
                        rows,
                    }),
                )
                .map_err(|_| ("terminal_agent_unavailable", "Agent 终端通道不可用"))?;
            Ok(false)
        }
        BrowserMessage::Close { sequence } => {
            let (agent_id, generation) = state
                .terminal_connections()
                .prepare_client_close(session_id, attachment_id, sequence)
                .map_err(forward_error)?;
            store::request_close(state.pool(), session_id, "administrator_closed")
                .await
                .map_err(|_| ("terminal_state_failed", "终端状态更新失败"))?;
            state
                .agent_connections()
                .try_send_generation(
                    &agent_id,
                    generation,
                    Message::TerminalClose(TerminalClose {
                        session_id: session_id.to_owned(),
                        sequence,
                        reason: TerminalCloseReason::AdministratorRequest,
                    }),
                )
                .map_err(|_| ("terminal_agent_unavailable", "Agent 终端通道不可用"))?;
            Ok(false)
        }
    }
}

fn websocket_csrf(headers: &HeaderMap) -> Option<&str> {
    let protocols = headers
        .get("sec-websocket-protocol")?
        .to_str()
        .ok()?
        .split(',')
        .map(str::trim)
        .collect::<Vec<_>>();
    if !protocols.contains(&TERMINAL_PROTOCOL) {
        return None;
    }
    protocols
        .into_iter()
        .find_map(|protocol| protocol.strip_prefix("csrf."))
        .filter(|token| !token.is_empty() && token.len() <= 256)
}

fn validate_size(columns: u16, rows: u16) -> Result<(), (&'static str, &'static str)> {
    if !(TERMINAL_MIN_COLUMNS..=TERMINAL_MAX_COLUMNS).contains(&columns)
        || !(TERMINAL_MIN_ROWS..=TERMINAL_MAX_ROWS).contains(&rows)
    {
        Err(("terminal_size_invalid", "终端尺寸无效"))
    } else {
        Ok(())
    }
}

fn forward_error(error: ForwardError) -> (&'static str, &'static str) {
    match error {
        ForwardError::Missing => ("terminal_session_stale", "终端会话已失效"),
        ForwardError::AlreadyOpened => ("terminal_already_opened", "终端已经打开"),
        ForwardError::NotOpened => ("terminal_not_opened", "终端尚未打开"),
        ForwardError::Closing => ("terminal_closing", "终端正在关闭"),
        ForwardError::WrongSequence => ("terminal_sequence_invalid", "终端消息序号无效"),
    }
}

async fn send_error(
    socket: &mut axum::extract::ws::WebSocket,
    code: &'static str,
    message: &'static str,
) -> Result<(), ()> {
    let text = serde_json::to_string(&super::registry::BrowserEvent::Error { code, message })
        .map_err(|_| ())?;
    socket
        .send(WsMessage::Text(text.into()))
        .await
        .map_err(|_| ())
}
