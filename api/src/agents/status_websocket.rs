use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    extract::{
        Extension, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Utc;
use futures_util::StreamExt;
use serde_json::json;

use crate::{
    AppState, RequestId,
    auth::{self, AuthUser},
    error::ApiResult,
};

const PROTOCOL: &str = "admin-node-status.v1";
const HEARTBEAT: Duration = Duration::from_secs(20);
const EVENT_INTERVAL: Duration = Duration::from_secs(2);
static BOOT_ID: OnceLock<String> = OnceLock::new();
static EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/node-status", get(upgrade))
}

#[utoipa::path(
    operation_id = "admin_node_status_stream",
    get,
    path = "/api/v1/admin/node-status",
    params(
        ("Origin" = String, Header),
        ("Sec-WebSocket-Protocol" = String, Header, description = "admin-node-status.v1 与 csrf.<token>")
    ),
    responses(
        (status = 101, description = "管理员节点状态 WebSocket 已升级"),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse)
    )
)]
pub async fn upgrade(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    websocket: WebSocketUpgrade,
) -> ApiResult<Response> {
    auth::verify_origin(&state, &headers, request_id.as_str())?;
    actor.require_administrator(request_id.as_str())?;
    let csrf = csrf_token(&headers)
        .ok_or_else(|| crate::error::ApiError::forbidden(request_id.as_str()))?;
    actor.verify_csrf_token(csrf, request_id.as_str())?;
    let session_id = actor.session_id.clone();
    let actor_id = actor.id.clone();
    Ok(websocket
        .max_message_size(64 * 1024)
        .max_frame_size(64 * 1024)
        .protocols([PROTOCOL])
        .on_upgrade(move |socket| run_socket(socket, state, session_id, actor_id))
        .into_response())
}

async fn run_socket(mut socket: WebSocket, state: AppState, session_id: String, actor_id: String) {
    let boot_id = BOOT_ID
        .get_or_init(|| format!("boot_{}", ulid::Ulid::new()))
        .clone();
    let _ = socket
        .send(Message::Text(
            json!({
                "type": "hello",
                "boot_id": boot_id,
                "event_sequence": EVENT_SEQUENCE.load(Ordering::Relaxed),
                "server_time": Utc::now().to_rfc3339(),
            })
            .to_string()
            .into(),
        ))
        .await;
    let mut events = tokio::time::interval(EVENT_INTERVAL);
    let mut heartbeat = tokio::time::interval(HEARTBEAT);
    events.tick().await;
    heartbeat.tick().await;
    loop {
        tokio::select! {
            _ = events.tick() => {
                let sequence = EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
                if socket.send(Message::Text(json!({
                    "type": "snapshot_invalidated",
                    "boot_id": boot_id,
                    "event_sequence": sequence,
                    "changed": ["nodes", "agents"],
                }).to_string().into())).await.is_err() { break; }
            }
            _ = heartbeat.tick() => {
                if !auth::session_is_active_administrator(&state, &session_id, &actor_id).await { break; }
                if socket.send(Message::Ping(Vec::new().into())).await.is_err() { break; }
            }
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Pong(_))) | Some(Ok(Message::Ping(_))) => {}
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(Message::Text(_))) | Some(Ok(Message::Binary(_))) => {}
                }
            }
        }
    }
}

fn csrf_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("sec-websocket-protocol")?
        .to_str()
        .ok()?
        .split(',')
        .map(str::trim)
        .find_map(|value| value.strip_prefix("csrf."))
        .filter(|value| !value.is_empty() && value.len() <= 256)
}
