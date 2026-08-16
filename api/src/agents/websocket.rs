use std::{collections::HashMap, sync::Mutex, time::Duration};

use axum::{
    Router,
    extract::{
        Extension, State, WebSocketUpgrade,
        ws::{Message as WsMessage, WebSocket},
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Utc;
use deploy_go_agent_protocol::{
    AgentCapability, AuthRefresh, AuthRefreshed, Envelope, HeartbeatAck, Hello, HelloAck,
    MIN_SUPPORTED_PROTOCOL_VERSION, Message, NODE_TELEMETRY_MAX_BYTES, NodeTelemetry,
    PROTOCOL_VERSION, ProtocolError, ReconcileRequest,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::FromRow;
use tokio::sync::watch;
use ulid::Ulid;

use crate::{AppState, RequestId, audit, error::ApiResult};

use super::auth::{AgentAccessIdentity, authenticate_access, token_hash};

const MAX_MESSAGE_SIZE: usize = 1024 * 1024;
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_INTERVAL_SECONDS: u32 = 15;
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnvelope {
    protocol_version: u16,
    message_id: String,
    sent_at: String,
    message: Value,
}

enum IncomingEnvelope {
    Control(Box<Envelope>),
    Telemetry(Box<NodeTelemetry>),
    DroppedTelemetry,
}

#[derive(Clone)]
struct ActiveConnection {
    generation: i64,
    stop: watch::Sender<bool>,
    outbound: tokio::sync::mpsc::Sender<Message>,
}

#[derive(Default)]
pub struct ConnectionRegistry {
    active: Mutex<HashMap<String, ActiveConnection>>,
}

impl ConnectionRegistry {
    pub(crate) fn is_connected(&self, agent_id: &str) -> bool {
        self.active
            .lock()
            .expect("连接注册表锁未中毒")
            .contains_key(agent_id)
    }

    fn register(
        &self,
        agent_id: &str,
        generation: i64,
        outbound: tokio::sync::mpsc::Sender<Message>,
    ) -> watch::Receiver<bool> {
        let (stop, receiver) = watch::channel(false);
        let mut active = self.active.lock().expect("连接注册表锁未中毒");
        if active
            .get(agent_id)
            .is_some_and(|connection| connection.generation >= generation)
        {
            let _ = stop.send(true);
            return receiver;
        }
        let previous = active.insert(
            agent_id.to_owned(),
            ActiveConnection {
                generation,
                stop,
                outbound,
            },
        );
        drop(active);
        if let Some(previous) = previous {
            let _ = previous.stop.send(true);
        }
        receiver
    }

    pub(crate) fn disconnect(&self, agent_id: &str) {
        if let Some(connection) = self
            .active
            .lock()
            .expect("连接注册表锁未中毒")
            .get(agent_id)
            .cloned()
        {
            let _ = connection.stop.send(true);
        }
    }

    pub(crate) fn disconnect_generation(&self, agent_id: &str, generation: i64) {
        if let Some(connection) = self
            .active
            .lock()
            .expect("连接注册表锁未中毒")
            .get(agent_id)
            .filter(|connection| connection.generation == generation)
            .cloned()
        {
            let _ = connection.stop.send(true);
        }
    }

    pub async fn send(&self, agent_id: &str, message: Message) -> Result<i64, ()> {
        let connection = self
            .active
            .lock()
            .expect("连接注册表锁未中毒")
            .get(agent_id)
            .cloned()
            .ok_or(())?;
        connection.outbound.send(message).await.map_err(|_| ())?;
        Ok(connection.generation)
    }

    pub(crate) fn generation(&self, agent_id: &str) -> Option<i64> {
        self.active
            .lock()
            .expect("连接注册表锁未中毒")
            .get(agent_id)
            .map(|connection| connection.generation)
    }

    pub(crate) fn try_send_generation(
        &self,
        agent_id: &str,
        generation: i64,
        message: Message,
    ) -> Result<(), ()> {
        let connection = self
            .active
            .lock()
            .expect("连接注册表锁未中毒")
            .get(agent_id)
            .filter(|connection| connection.generation == generation)
            .cloned()
            .ok_or(())?;
        connection.outbound.try_send(message).map_err(|_| ())
    }

    fn unregister(&self, agent_id: &str, generation: i64) -> bool {
        let mut active = self.active.lock().expect("连接注册表锁未中毒");
        if active
            .get(agent_id)
            .is_some_and(|connection| connection.generation == generation)
        {
            active.remove(agent_id);
            true
        } else {
            false
        }
    }
}

#[derive(FromRow)]
struct RefreshConfirmation {
    access_id: String,
    agent_id: String,
    family_id: String,
    access_expires_at: String,
    predecessor_id: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/agent/control", get(upgrade))
}

async fn upgrade(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> ApiResult<Response> {
    let identity = authenticate_access(state.pool(), &headers, request_id.as_str()).await?;
    Ok(websocket
        .max_message_size(MAX_MESSAGE_SIZE)
        .max_frame_size(MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| run_connection(socket, state, identity))
        .into_response())
}

async fn run_connection(mut socket: WebSocket, state: AppState, mut identity: AgentAccessIdentity) {
    let hello = match tokio::time::timeout(HELLO_TIMEOUT, receive_envelope(&mut socket)).await {
        Ok(Ok(Some(envelope))) => match envelope.message {
            Message::Hello(hello)
                if envelope.protocol_version == MIN_SUPPORTED_PROTOCOL_VERSION
                    && validate_hello(&hello, &identity.agent_id) =>
            {
                hello
            }
            _ => {
                let _ = send_protocol_error(&mut socket, PROTOCOL_VERSION, "hello_required", None)
                    .await;
                return;
            }
        },
        _ => return,
    };
    let negotiated_version = hello.max_protocol_version.min(PROTOCOL_VERSION);
    let connection_id = format!("conn_{}", Ulid::new());
    let generation = match claim_connection(
        &state,
        &identity,
        &hello,
        &connection_id,
        negotiated_version,
    )
    .await
    {
        Ok(generation) => generation,
        Err(()) => return,
    };
    let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::channel(64);
    let mut takeover =
        state
            .agent_connections()
            .register(&identity.agent_id, generation, outbound_tx);
    if send_envelope(
        &mut socket,
        negotiated_version,
        Message::HelloAck(HelloAck {
            connection_id: connection_id.clone(),
            connection_generation: generation as u64,
            protocol_version: negotiated_version,
            heartbeat_interval_seconds: HEARTBEAT_INTERVAL_SECONDS,
            telemetry_interval_seconds: (negotiated_version >= 12).then_some(30),
        }),
    )
    .await
    .is_err()
    {
        cleanup_connection(&state, &identity.agent_id, generation).await;
        return;
    }
    let task_ids = match super::dispatcher::active_task_ids(&state, &identity.agent_id).await {
        Ok(task_ids) => task_ids,
        Err(_) => {
            cleanup_connection(&state, &identity.agent_id, generation).await;
            return;
        }
    };
    if !task_ids.is_empty()
        && send_envelope(
            &mut socket,
            negotiated_version,
            Message::ReconcileRequest(ReconcileRequest { task_ids }),
        )
        .await
        .is_err()
    {
        cleanup_connection(&state, &identity.agent_id, generation).await;
        return;
    }
    if super::dispatcher::enqueue_pending_env_syncs_for_agent(&state, &identity.agent_id)
        .await
        .is_err()
        || super::dispatcher::dispatch_queued_for_agent(&state, &identity.agent_id)
            .await
            .is_err()
    {
        cleanup_connection(&state, &identity.agent_id, generation).await;
        return;
    }

    let (terminal_tx, mut terminal_rx) = tokio::sync::mpsc::channel(64);
    let terminal_state = state.clone();
    let terminal_agent_id = identity.agent_id.clone();
    tokio::spawn(async move {
        while let Some(message) = terminal_rx.recv().await {
            if terminal_state
                .terminal_connections()
                .handle_agent_message(&terminal_state, &terminal_agent_id, generation, &message)
                .await
                .is_err()
            {
                terminal_state
                    .terminal_connections()
                    .agent_stream_failed(
                        &terminal_state,
                        &terminal_agent_id,
                        generation,
                        "terminal_stream_failed",
                    )
                    .await;
            }
        }
    });

    let (telemetry_tx, mut telemetry_rx) = tokio::sync::mpsc::channel::<NodeTelemetry>(4);
    let telemetry_state = state.clone();
    let telemetry_agent_id = identity.agent_id.clone();
    tokio::spawn(async move {
        while let Some(sample) = telemetry_rx.recv().await {
            if let Err(error) = crate::node_telemetry::store(
                telemetry_state.pool(),
                &telemetry_agent_id,
                generation,
                &sample,
            )
            .await
            {
                tracing::warn!(agent_id = %telemetry_agent_id, error = %error, "节点遥测样本落库失败");
            }
        }
    });

    let mut last_heartbeat = tokio::time::Instant::now();
    let mut last_telemetry_sequence = 0;
    let mut last_telemetry_at: Option<tokio::time::Instant> = None;
    let mut timeout_check = tokio::time::interval(Duration::from_secs(5));
    timeout_check.tick().await;
    loop {
        tokio::select! {
            changed = takeover.changed() => {
                if changed.is_err() || *takeover.borrow() {
                    break;
                }
            }
            _ = timeout_check.tick() => {
                if last_heartbeat.elapsed() > HEARTBEAT_TIMEOUT || access_expired(&identity.expires_at) {
                    break;
                }
            }
            outbound = outbound_rx.recv() => {
                let Some(outbound) = outbound else { break; };
                if send_envelope(&mut socket, negotiated_version, outbound).await.is_err() {
                    break;
                }
            }
            incoming = receive_incoming_envelope(&mut socket, negotiated_version) => {
                let incoming = match incoming {
                    Ok(Some(incoming)) => incoming,
                    _ => break,
                };
                let envelope = match incoming {
                    IncomingEnvelope::DroppedTelemetry => continue,
                    IncomingEnvelope::Telemetry(telemetry) => {
                        if negotiated_version >= 12
                            && telemetry.connection_generation == generation as u64
                            && telemetry.sample_sequence > last_telemetry_sequence
                        {
                            last_telemetry_sequence = telemetry.sample_sequence;
                            let now = tokio::time::Instant::now();
                            let interval_ok = last_telemetry_at
                                .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(10));
                            if interval_ok && state.telemetry_budget().try_acquire() {
                                last_telemetry_at = Some(now);
                                let _ = telemetry_tx.try_send(*telemetry);
                            }
                        }
                        continue;
                    }
                    IncomingEnvelope::Control(envelope) => *envelope,
                };
                match envelope.message {
                    Message::Heartbeat(heartbeat) if heartbeat.connection_generation == generation as u64 => {
                        if record_heartbeat(&state, &identity.agent_id, generation).await.is_err() {
                            break;
                        }
                        last_heartbeat = tokio::time::Instant::now();
                        if send_envelope(&mut socket, negotiated_version, Message::HeartbeatAck(HeartbeatAck {
                            connection_generation: generation as u64,
                            server_time: Utc::now().to_rfc3339(),
                        })).await.is_err() {
                            break;
                        }
                    }
                    Message::AuthRefresh(refresh) => {
                        match confirm_refresh(&state, &identity, &refresh, &connection_id).await {
                            Ok(next) => {
                                identity = next;
                                if send_envelope(&mut socket, negotiated_version, Message::AuthRefreshed(AuthRefreshed {
                                    rotation_id: refresh.rotation_id,
                                    access_expires_at: identity.expires_at.clone(),
                                })).await.is_err() {
                                    break;
                                }
                            }
                            Err(()) => break,
                        }
                    }
                    message => {
                        let terminal = matches!(
                            message,
                            Message::TerminalOpened(_)
                                | Message::TerminalOutput(_)
                                | Message::TerminalExited(_)
                        );
                        let handled = if terminal && negotiated_version < 6 {
                            Ok(false)
                        } else if terminal {
                            if terminal_tx.try_send(message).is_err() {
                                state
                                    .terminal_connections()
                                    .agent_stream_failed(
                                        &state,
                                        &identity.agent_id,
                                        generation,
                                        "terminal_stream_backpressure",
                                    )
                                    .await;
                            }
                            Ok(true)
                        } else {
                            super::dispatcher::handle_agent_message(
                                &state,
                                &identity.agent_id,
                                generation,
                                &message,
                            )
                            .await
                        };
                        if !matches!(handled, Ok(true)) {
                            if send_protocol_error(&mut socket, negotiated_version, "unexpected_message", Some(envelope.message_id)).await.is_err() {
                                break;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    let _ = socket.close().await;
    cleanup_connection(&state, &identity.agent_id, generation).await;
}

fn validate_hello(hello: &Hello, expected_agent_id: &str) -> bool {
    hello.agent_id == expected_agent_id
        && hello.min_protocol_version <= hello.max_protocol_version
        && hello.min_protocol_version <= PROTOCOL_VERSION
        && hello.max_protocol_version >= MIN_SUPPORTED_PROTOCOL_VERSION
        && !hello.agent_version.is_empty()
        && hello.agent_version.len() <= 128
        && !hello.os.is_empty()
        && hello.os.len() <= 128
        && !hello.architecture.is_empty()
        && hello.architecture.len() <= 128
        && hello.capabilities.contains(&AgentCapability::PtyTerminal)
        && hello
            .capabilities
            .contains(&AgentCapability::PrivilegedRelease)
}

async fn claim_connection(
    state: &AppState,
    identity: &AgentAccessIdentity,
    hello: &Hello,
    connection_id: &str,
    negotiated_version: u16,
) -> Result<i64, ()> {
    let mut transaction = state.pool().begin().await.map_err(|_| ())?;
    let now = Utc::now().to_rfc3339();
    let capabilities_json = serde_json::to_string(&hello.capabilities).map_err(|_| ())?;
    let (generation, node_id): (i64, String) = sqlx::query_as(
        "UPDATE agents SET connection_generation=connection_generation+1,agent_version=?,protocol_version=?,os_name=?,architecture=?,capabilities_json=?,last_seen_at=?,updated_at=? WHERE id=? AND revoked_at IS NULL AND archived_at IS NULL RETURNING connection_generation,node_id",
    )
    .bind(&hello.agent_version)
    .bind(i64::from(negotiated_version))
    .bind(&hello.os)
    .bind(&hello.architecture)
    .bind(capabilities_json)
    .bind(&now)
    .bind(&now)
    .bind(&identity.agent_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| ())?;
    let access_updated = sqlx::query(
        "UPDATE agent_access_sessions SET connection_id=? WHERE id=? AND revoked_at IS NULL",
    )
    .bind(connection_id)
    .bind(&identity.access_id)
    .execute(&mut *transaction)
    .await
    .map_err(|_| ())?;
    if access_updated.rows_affected() != 1 {
        return Err(());
    }
    sqlx::query("UPDATE nodes SET status='online',checked_at=?,updated_at=?,version=version+1 WHERE id=? AND status!='disabled'")
        .bind(&now)
        .bind(&now)
        .bind(node_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ())?;
    transaction.commit().await.map_err(|_| ())?;
    Ok(generation)
}

async fn record_heartbeat(state: &AppState, agent_id: &str, generation: i64) -> Result<(), ()> {
    let now = Utc::now().to_rfc3339();
    let mut transaction = state.pool().begin().await.map_err(|_| ())?;
    let node_id: Option<String> = sqlx::query_scalar(
        "UPDATE agents SET last_seen_at=?,updated_at=? WHERE id=? AND connection_generation=? AND revoked_at IS NULL AND archived_at IS NULL RETURNING node_id",
    )
    .bind(&now)
    .bind(&now)
    .bind(agent_id)
    .bind(generation)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| ())?;
    let Some(node_id) = node_id else {
        return Err(());
    };
    sqlx::query("UPDATE nodes SET status='online',checked_at=?,updated_at=?,version=version+1 WHERE id=? AND status!='disabled'")
        .bind(&now)
        .bind(&now)
        .bind(node_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ())?;
    transaction.commit().await.map_err(|_| ())
}

async fn confirm_refresh(
    state: &AppState,
    current: &AgentAccessIdentity,
    refresh: &AuthRefresh,
    connection_id: &str,
) -> Result<AgentAccessIdentity, ()> {
    if refresh.rotation_id.len() > 128 || refresh.access_token.len() > 256 {
        return Err(());
    }
    let now = Utc::now().to_rfc3339();
    let mut transaction = state.pool().begin().await.map_err(|_| ())?;
    let confirmation = sqlx::query_as::<_, RefreshConfirmation>(
        "SELECT access.id AS access_id,access.agent_id,access.family_id,access.expires_at AS access_expires_at,predecessor.id AS predecessor_id FROM agent_access_sessions access JOIN agent_refresh_credentials successor ON successor.id=access.refresh_credential_id JOIN agent_refresh_credentials predecessor ON predecessor.replaced_by_id=successor.id AND predecessor.rotation_id=? JOIN agent_credential_families family ON family.id=access.family_id WHERE access.token_hash=? AND access.agent_id=? AND access.family_id=? AND access.revoked_at IS NULL AND access.expires_at>? AND successor.revoked_at IS NULL AND family.revoked_at IS NULL",
    )
    .bind(&refresh.rotation_id)
    .bind(token_hash("access", &refresh.access_token))
    .bind(&current.agent_id)
    .bind(&current.family_id)
    .bind(&now)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| ())?
    .ok_or(())?;
    sqlx::query("UPDATE agent_refresh_credentials SET committed_at=COALESCE(committed_at,?),revoked_at=COALESCE(revoked_at,?) WHERE id=? AND revoked_at IS NULL")
        .bind(&now)
        .bind(&now)
        .bind(&confirmation.predecessor_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ())?;
    sqlx::query("UPDATE agent_access_sessions SET connection_id=? WHERE id=?")
        .bind(connection_id)
        .bind(&confirmation.access_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ())?;
    sqlx::query("UPDATE agent_access_sessions SET revoked_at=COALESCE(revoked_at,?) WHERE family_id=? AND id!=?")
        .bind(&now)
        .bind(&confirmation.family_id)
        .bind(&confirmation.access_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ())?;
    audit::record(
        &mut transaction,
        None,
        "agent.credential_rotated",
        "agent",
        &confirmation.agent_id,
        "agent_websocket",
        json!({"rotation_id":refresh.rotation_id}),
    )
    .await
    .map_err(|_| ())?;
    transaction.commit().await.map_err(|_| ())?;
    Ok(AgentAccessIdentity {
        access_id: confirmation.access_id,
        agent_id: confirmation.agent_id,
        family_id: confirmation.family_id,
        expires_at: confirmation.access_expires_at,
    })
}

fn access_expired(expires_at: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(expires_at)
        .map(|expires_at| expires_at <= Utc::now())
        .unwrap_or(true)
}

async fn cleanup_connection(state: &AppState, agent_id: &str, generation: i64) {
    state
        .terminal_connections()
        .agent_disconnected(state, agent_id, generation)
        .await;
    if !state.agent_connections().unregister(agent_id, generation) {
        return;
    }
    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query("UPDATE nodes SET status='offline',updated_at=?,version=version+1 WHERE id=(SELECT node_id FROM agents WHERE id=? AND connection_generation=?) AND status!='disabled'")
        .bind(now)
        .bind(agent_id)
        .bind(generation)
        .execute(state.pool())
        .await;
}

pub async fn reset_online_state(pool: &sqlx::SqlitePool) -> sqlx::Result<u64> {
    let result = sqlx::query("UPDATE nodes SET status='offline',updated_at=?,version=version+1 WHERE status='online' AND EXISTS(SELECT 1 FROM agents WHERE agents.node_id=nodes.id)")
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

async fn receive_envelope(socket: &mut WebSocket) -> Result<Option<Envelope>, ()> {
    receive_envelope_for_version(socket, None).await
}

async fn receive_incoming_envelope(
    socket: &mut WebSocket,
    expected_version: u16,
) -> Result<Option<IncomingEnvelope>, ()> {
    loop {
        let Some(message) = socket.next().await else {
            return Ok(None);
        };
        match message.map_err(|_| ())? {
            WsMessage::Text(text) => {
                let raw = serde_json::from_str::<RawEnvelope>(&text).map_err(|_| ())?;
                if raw.protocol_version != expected_version {
                    return Err(());
                }
                let is_telemetry =
                    raw.message.get("type").and_then(Value::as_str) == Some("node_telemetry");
                if is_telemetry {
                    if text.len() > NODE_TELEMETRY_MAX_BYTES {
                        return Ok(Some(IncomingEnvelope::DroppedTelemetry));
                    }
                    return Ok(Some(match serde_json::from_value::<Message>(raw.message) {
                        Ok(Message::NodeTelemetry(telemetry)) if telemetry.validate().is_ok() => {
                            IncomingEnvelope::Telemetry(Box::new(telemetry))
                        }
                        _ => IncomingEnvelope::DroppedTelemetry,
                    }));
                }
                let envelope = serde_json::from_value::<Envelope>(json!({
                    "protocol_version": raw.protocol_version,
                    "message_id": raw.message_id,
                    "sent_at": raw.sent_at,
                    "message": raw.message,
                }))
                .map_err(|_| ())?;
                return Ok(Some(IncomingEnvelope::Control(Box::new(envelope))));
            }
            WsMessage::Ping(bytes) => socket.send(WsMessage::Pong(bytes)).await.map_err(|_| ())?,
            WsMessage::Close(_) => return Ok(None),
            WsMessage::Binary(_) => return Err(()),
            WsMessage::Pong(_) => {}
        }
    }
}

async fn receive_envelope_for_version(
    socket: &mut WebSocket,
    expected_version: Option<u16>,
) -> Result<Option<Envelope>, ()> {
    loop {
        let Some(message) = socket.next().await else {
            return Ok(None);
        };
        match message.map_err(|_| ())? {
            WsMessage::Text(text) => {
                let envelope = serde_json::from_str::<Envelope>(&text).map_err(|_| ())?;
                match expected_version {
                    Some(expected) if envelope.protocol_version != expected => return Err(()),
                    None if !(MIN_SUPPORTED_PROTOCOL_VERSION..=PROTOCOL_VERSION)
                        .contains(&envelope.protocol_version) =>
                    {
                        return Err(());
                    }
                    _ => {}
                }
                return Ok(Some(envelope));
            }
            WsMessage::Ping(bytes) => socket.send(WsMessage::Pong(bytes)).await.map_err(|_| ())?,
            WsMessage::Close(_) => return Ok(None),
            WsMessage::Binary(_) => return Err(()),
            WsMessage::Pong(_) => {}
        }
    }
}

async fn send_envelope(
    socket: &mut WebSocket,
    protocol_version: u16,
    message: Message,
) -> Result<(), ()> {
    let envelope = Envelope {
        protocol_version,
        message_id: format!("msg_{}", Ulid::new()),
        sent_at: Utc::now().to_rfc3339(),
        message,
    };
    let text = serde_json::to_string(&envelope).map_err(|_| ())?;
    socket
        .send(WsMessage::Text(text.into()))
        .await
        .map_err(|_| ())
}

async fn send_protocol_error(
    socket: &mut WebSocket,
    protocol_version: u16,
    code: &str,
    related_message_id: Option<String>,
) -> Result<(), ()> {
    send_envelope(
        socket,
        protocol_version,
        Message::ProtocolError(ProtocolError {
            code: code.to_owned(),
            message: "控制协议消息无效".to_owned(),
            related_message_id,
            details: None,
        }),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_connection_replaces_the_previous_generation() {
        let registry = ConnectionRegistry::default();
        let (first_tx, _) = tokio::sync::mpsc::channel(1);
        let (second_tx, _) = tokio::sync::mpsc::channel(1);
        let first = registry.register("agent_01", 1, first_tx);
        let _second = registry.register("agent_01", 2, second_tx);
        assert!(*first.borrow());
        assert!(!registry.unregister("agent_01", 1));
        assert!(registry.unregister("agent_01", 2));
    }

    #[test]
    fn older_connection_cannot_replace_a_newer_generation() {
        let registry = ConnectionRegistry::default();
        let (newer_tx, _) = tokio::sync::mpsc::channel(1);
        let (older_tx, _) = tokio::sync::mpsc::channel(1);
        let newer = registry.register("agent_01", 2, newer_tx);
        let older = registry.register("agent_01", 1, older_tx);
        assert!(*older.borrow());
        assert!(!*newer.borrow());
        assert!(registry.unregister("agent_01", 2));
    }

    #[test]
    fn disconnect_generation_never_stops_a_replacement_connection() {
        let registry = ConnectionRegistry::default();
        let (first_tx, _) = tokio::sync::mpsc::channel(1);
        let (second_tx, _) = tokio::sync::mpsc::channel(1);
        let first = registry.register("agent_01", 1, first_tx);
        let second = registry.register("agent_01", 2, second_tx);

        registry.disconnect_generation("agent_01", 1);
        assert!(*first.borrow());
        assert!(!*second.borrow());

        registry.disconnect_generation("agent_01", 2);
        assert!(*second.borrow());
    }
}
