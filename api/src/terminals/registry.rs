use std::{collections::HashMap, sync::Mutex};

use axum::http::StatusCode;
use base64::{Engine, engine::general_purpose::STANDARD};
use deploy_go_agent_protocol::{
    Message, TerminalClose, TerminalCloseReason, TerminalExitReason, TerminalExited,
    TerminalOpened, TerminalOutput,
};
use serde::Serialize;
use serde_json::json;
use tokio::sync::mpsc;

use crate::{AppState, audit, error::ApiError};

use super::store::{self, TerminalSessionRecord};

pub const BROWSER_QUEUE_CAPACITY: usize = 64;
pub const MAX_SESSION_INPUT_BYTES: i64 = 8 * 1024 * 1024;
pub const MAX_SESSION_OUTPUT_BYTES: i64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrowserEvent {
    Opened {
        session_id: String,
        sequence: u64,
    },
    Output {
        session_id: String,
        sequence: u64,
        encoding: &'static str,
        data: String,
    },
    Exited {
        session_id: String,
        sequence: u64,
        reason: String,
        exit_code: Option<i32>,
    },
    Error {
        code: &'static str,
        message: &'static str,
    },
}

#[derive(Clone)]
struct ActiveTerminal {
    attachment_id: String,
    agent_id: String,
    generation: i64,
    browser: mpsc::Sender<BrowserEvent>,
    next_agent_sequence: u64,
    next_server_sequence: u64,
    open_sent: bool,
    close_sent: bool,
}

#[derive(Default)]
pub struct TerminalRegistry {
    active: Mutex<HashMap<String, ActiveTerminal>>,
}

pub struct TerminalRegistration {
    pub attachment_id: String,
    pub receiver: mpsc::Receiver<BrowserEvent>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RegisterError {
    AlreadyAttached,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ForwardError {
    Missing,
    AlreadyOpened,
    NotOpened,
    Closing,
    WrongSequence,
}

impl TerminalRegistry {
    pub fn register(
        &self,
        session_id: &str,
        attachment_id: String,
        agent_id: &str,
        generation: i64,
    ) -> Result<TerminalRegistration, RegisterError> {
        let (browser, receiver) = mpsc::channel(BROWSER_QUEUE_CAPACITY);
        let mut active = self.active.lock().expect("终端注册表锁未中毒");
        if active.contains_key(session_id) {
            return Err(RegisterError::AlreadyAttached);
        }
        active.insert(
            session_id.to_owned(),
            ActiveTerminal {
                attachment_id: attachment_id.clone(),
                agent_id: agent_id.to_owned(),
                generation,
                browser,
                next_agent_sequence: 1,
                next_server_sequence: 1,
                open_sent: false,
                close_sent: false,
            },
        );
        Ok(TerminalRegistration {
            attachment_id,
            receiver,
        })
    }

    pub fn prepare_open(
        &self,
        session_id: &str,
        attachment_id: &str,
    ) -> Result<(String, i64), ForwardError> {
        let mut active = self.active.lock().expect("终端注册表锁未中毒");
        let entry = active.get_mut(session_id).ok_or(ForwardError::Missing)?;
        if entry.attachment_id != attachment_id {
            return Err(ForwardError::Missing);
        }
        if entry.open_sent {
            return Err(ForwardError::AlreadyOpened);
        }
        entry.open_sent = true;
        Ok((entry.agent_id.clone(), entry.generation))
    }

    pub fn prepare_client_frame(
        &self,
        session_id: &str,
        attachment_id: &str,
        sequence: u64,
    ) -> Result<(String, i64), ForwardError> {
        let mut active = self.active.lock().expect("终端注册表锁未中毒");
        let entry = active.get_mut(session_id).ok_or(ForwardError::Missing)?;
        if entry.attachment_id != attachment_id {
            return Err(ForwardError::Missing);
        }
        if !entry.open_sent {
            return Err(ForwardError::NotOpened);
        }
        if entry.close_sent {
            return Err(ForwardError::Closing);
        }
        if entry.next_server_sequence != sequence {
            return Err(ForwardError::WrongSequence);
        }
        entry.next_server_sequence = entry.next_server_sequence.saturating_add(1);
        Ok((entry.agent_id.clone(), entry.generation))
    }

    pub fn prepare_client_close(
        &self,
        session_id: &str,
        attachment_id: &str,
        sequence: u64,
    ) -> Result<(String, i64), ForwardError> {
        let mut active = self.active.lock().expect("终端注册表锁未中毒");
        let entry = active.get_mut(session_id).ok_or(ForwardError::Missing)?;
        if entry.attachment_id != attachment_id {
            return Err(ForwardError::Missing);
        }
        if !entry.open_sent {
            return Err(ForwardError::NotOpened);
        }
        if entry.close_sent {
            return Err(ForwardError::Closing);
        }
        if entry.next_server_sequence != sequence {
            return Err(ForwardError::WrongSequence);
        }
        entry.next_server_sequence = entry.next_server_sequence.saturating_add(1);
        entry.close_sent = true;
        Ok((entry.agent_id.clone(), entry.generation))
    }

    pub async fn handle_agent_message(
        &self,
        state: &AppState,
        agent_id: &str,
        generation: i64,
        message: &Message,
    ) -> Result<bool, ApiError> {
        match message {
            Message::TerminalOpened(opened) => {
                self.handle_opened(state, agent_id, generation, opened)
                    .await?;
                Ok(true)
            }
            Message::TerminalOutput(output) => {
                self.handle_output(state, agent_id, generation, output)
                    .await?;
                Ok(true)
            }
            Message::TerminalExited(exited) => {
                self.handle_exited(state, agent_id, generation, exited)
                    .await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn handle_opened(
        &self,
        state: &AppState,
        agent_id: &str,
        generation: i64,
        opened: &TerminalOpened,
    ) -> Result<(), ApiError> {
        let browser = match self.accept_agent_sequence(
            &opened.session_id,
            agent_id,
            generation,
            opened.sequence,
        ) {
            Ok(Some(browser)) => browser,
            Ok(None) => return Ok(()),
            Err(()) => {
                self.terminate(state, &opened.session_id, "protocol_error", "failed", None)
                    .await?;
                return Ok(());
            }
        };
        if !store::mark_opened(state.pool(), &opened.session_id)
            .await
            .map_err(|_| ApiError::internal("agent_terminal"))?
        {
            self.terminate(
                state,
                &opened.session_id,
                "terminal_session_stale",
                "interrupted",
                None,
            )
            .await?;
            return Ok(());
        }
        if browser
            .try_send(BrowserEvent::Opened {
                session_id: opened.session_id.clone(),
                sequence: opened.sequence,
            })
            .is_err()
        {
            self.terminate(
                state,
                &opened.session_id,
                "browser_backpressure",
                "interrupted",
                None,
            )
            .await?;
        }
        Ok(())
    }

    async fn handle_output(
        &self,
        state: &AppState,
        agent_id: &str,
        generation: i64,
        output: &TerminalOutput,
    ) -> Result<(), ApiError> {
        let browser = match self.accept_agent_sequence(
            &output.session_id,
            agent_id,
            generation,
            output.sequence,
        ) {
            Ok(Some(browser)) => browser,
            Ok(None) => return Ok(()),
            Err(()) => {
                self.terminate(state, &output.session_id, "protocol_error", "failed", None)
                    .await?;
                return Ok(());
            }
        };
        let bytes = STANDARD.decode(&output.data).map_err(|_| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "terminal_output_invalid",
                "Agent 终端输出编码无效",
                "agent_terminal",
            )
        })?;
        let accepted = store::add_output_bytes(
            state.pool(),
            &output.session_id,
            i64::try_from(bytes.len()).unwrap_or(i64::MAX),
            MAX_SESSION_OUTPUT_BYTES,
        )
        .await
        .map_err(|_| ApiError::internal("agent_terminal"))?;
        if !accepted {
            self.terminate(
                state,
                &output.session_id,
                "output_limit_exceeded",
                "failed",
                None,
            )
            .await?;
            return Ok(());
        }
        if browser
            .try_send(BrowserEvent::Output {
                session_id: output.session_id.clone(),
                sequence: output.sequence,
                encoding: "base64",
                data: output.data.clone(),
            })
            .is_err()
        {
            self.terminate(
                state,
                &output.session_id,
                "browser_backpressure",
                "interrupted",
                None,
            )
            .await?;
        }
        Ok(())
    }

    async fn handle_exited(
        &self,
        state: &AppState,
        agent_id: &str,
        generation: i64,
        exited: &TerminalExited,
    ) -> Result<(), ApiError> {
        let browser = match self.accept_agent_sequence(
            &exited.session_id,
            agent_id,
            generation,
            exited.sequence,
        ) {
            Ok(Some(browser)) => browser,
            Ok(None) => return Ok(()),
            Err(()) => {
                self.terminate(state, &exited.session_id, "protocol_error", "failed", None)
                    .await?;
                return Ok(());
            }
        };
        self.remove_session(&exited.session_id);
        let reason = exit_reason(exited.reason);
        let status = exit_status(exited.reason);
        let session = store::finish_session(
            state.pool(),
            &exited.session_id,
            status,
            reason,
            exited.exit_code,
        )
        .await
        .map_err(|_| ApiError::internal("agent_terminal"))?;
        if let Some(session) = session {
            record_finished(state, &session).await?;
        }
        let _ = browser.try_send(BrowserEvent::Exited {
            session_id: exited.session_id.clone(),
            sequence: exited.sequence,
            reason: reason.to_owned(),
            exit_code: exited.exit_code,
        });
        Ok(())
    }

    fn accept_agent_sequence(
        &self,
        session_id: &str,
        agent_id: &str,
        generation: i64,
        sequence: u64,
    ) -> Result<Option<mpsc::Sender<BrowserEvent>>, ()> {
        let mut active = self.active.lock().expect("终端注册表锁未中毒");
        let Some(entry) = active.get_mut(session_id) else {
            return Ok(None);
        };
        if entry.agent_id != agent_id || entry.generation != generation {
            return Ok(None);
        }
        if !entry.open_sent || entry.next_agent_sequence != sequence {
            return Err(());
        }
        entry.next_agent_sequence = entry.next_agent_sequence.saturating_add(1);
        Ok(Some(entry.browser.clone()))
    }

    pub async fn terminate(
        &self,
        state: &AppState,
        session_id: &str,
        reason: &str,
        status: &str,
        exit_code: Option<i32>,
    ) -> Result<(), ApiError> {
        let Some(entry) = self.remove_session(session_id) else {
            return Ok(());
        };
        let close_delivered = state
            .agent_connections()
            .try_send_generation(
                &entry.agent_id,
                entry.generation,
                Message::TerminalClose(TerminalClose {
                    session_id: session_id.to_owned(),
                    sequence: entry.next_server_sequence,
                    reason: close_reason(reason),
                }),
            )
            .is_ok();
        if !close_delivered {
            // Closing the exact control generation triggers the Agent guard, which closes
            // its executor bridge even when the bounded outbound queue cannot accept close.
            state
                .agent_connections()
                .disconnect_generation(&entry.agent_id, entry.generation);
        }
        let session = store::finish_session(state.pool(), session_id, status, reason, exit_code)
            .await
            .map_err(|_| ApiError::internal("terminal_cleanup"))?;
        if let Some(session) = session {
            record_finished(state, &session).await?;
        }
        let _ = entry.browser.try_send(BrowserEvent::Exited {
            session_id: session_id.to_owned(),
            sequence: entry.next_agent_sequence,
            reason: reason.to_owned(),
            exit_code,
        });
        Ok(())
    }

    pub async fn terminate_attachment(
        &self,
        state: &AppState,
        session_id: &str,
        attachment_id: &str,
        reason: &str,
        status: &str,
    ) -> Result<(), ApiError> {
        let matches = self
            .active
            .lock()
            .expect("终端注册表锁未中毒")
            .get(session_id)
            .is_some_and(|entry| entry.attachment_id == attachment_id);
        if matches {
            self.terminate(state, session_id, reason, status, None)
                .await?;
        }
        Ok(())
    }

    pub async fn agent_disconnected(&self, state: &AppState, agent_id: &str, generation: i64) {
        let session_ids = {
            let active = self.active.lock().expect("终端注册表锁未中毒");
            active
                .iter()
                .filter(|(_, entry)| entry.agent_id == agent_id && entry.generation == generation)
                .map(|(session_id, _)| session_id.clone())
                .collect::<Vec<_>>()
        };
        for session_id in session_ids {
            let _ = self
                .terminate(
                    state,
                    &session_id,
                    "agent_disconnected",
                    "interrupted",
                    None,
                )
                .await;
        }
    }

    pub async fn agent_stream_failed(
        &self,
        state: &AppState,
        agent_id: &str,
        generation: i64,
        reason: &str,
    ) {
        let session_ids = {
            let active = self.active.lock().expect("终端注册表锁未中毒");
            active
                .iter()
                .filter(|(_, entry)| entry.agent_id == agent_id && entry.generation == generation)
                .map(|(session_id, _)| session_id.clone())
                .collect::<Vec<_>>()
        };
        for session_id in session_ids {
            let _ = self
                .terminate(state, &session_id, reason, "failed", None)
                .await;
        }
    }

    pub async fn request_administrator_close(
        &self,
        state: &AppState,
        session_id: &str,
    ) -> Result<bool, ApiError> {
        let outbound = {
            let mut active = self.active.lock().expect("终端注册表锁未中毒");
            active.get_mut(session_id).and_then(|entry| {
                if entry.close_sent {
                    return None;
                }
                let sequence = entry.next_server_sequence;
                entry.next_server_sequence = entry.next_server_sequence.saturating_add(1);
                entry.close_sent = true;
                Some((entry.agent_id.clone(), entry.generation, sequence))
            })
        };
        let Some((agent_id, generation, sequence)) = outbound else {
            return Ok(false);
        };
        store::request_close(state.pool(), session_id, "administrator_closed")
            .await
            .map_err(|_| ApiError::internal("terminal_close"))?;
        if state
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
            .is_err()
        {
            self.terminate(state, session_id, "agent_disconnected", "interrupted", None)
                .await?;
        }
        Ok(true)
    }

    pub async fn authorization_revoked_for_agent(
        &self,
        state: &AppState,
        agent_id: &str,
        reason: &str,
    ) {
        let session_ids = self.session_ids_for(|entry| entry.agent_id == agent_id);
        for session_id in session_ids {
            let _ = self
                .terminate(state, &session_id, reason, "interrupted", None)
                .await;
        }
    }

    pub async fn authorization_revoked_for_node(
        &self,
        state: &AppState,
        node_id: &str,
        reason: &str,
    ) {
        let agent_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM agents WHERE node_id=?")
            .bind(node_id)
            .fetch_all(state.pool())
            .await
            .unwrap_or_default();
        for agent_id in agent_ids {
            self.authorization_revoked_for_agent(state, &agent_id, reason)
                .await;
        }
    }

    fn session_ids_for(&self, predicate: impl Fn(&ActiveTerminal) -> bool) -> Vec<String> {
        let active = self.active.lock().expect("终端注册表锁未中毒");
        active
            .iter()
            .filter(|(_, entry)| predicate(entry))
            .map(|(session_id, _)| session_id.clone())
            .collect()
    }

    fn remove_session(&self, session_id: &str) -> Option<ActiveTerminal> {
        self.active
            .lock()
            .expect("终端注册表锁未中毒")
            .remove(session_id)
    }

    #[cfg(test)]
    fn contains(&self, session_id: &str) -> bool {
        self.active
            .lock()
            .expect("终端注册表锁未中毒")
            .contains_key(session_id)
    }
}

pub(crate) async fn record_finished(
    state: &AppState,
    session: &TerminalSessionRecord,
) -> Result<(), ApiError> {
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal("terminal_audit"))?;
    audit::record(
        &mut transaction,
        Some(&session.actor_id),
        "terminal.session.finished",
        "terminal_session",
        &session.id,
        &session.request_id,
        json!({
            "node_id":session.node_id,
            "agent_id":session.agent_id,
            "started_at":session.started_at,
            "opened_at":session.opened_at,
            "finished_at":session.finished_at,
            "exit_reason":session.exit_reason,
            "exit_code":session.exit_code,
            "input_bytes":session.input_bytes,
            "output_bytes":session.output_bytes,
        }),
    )
    .await
    .map_err(|_| ApiError::internal("terminal_audit"))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("terminal_audit"))
}

fn exit_reason(reason: TerminalExitReason) -> &'static str {
    match reason {
        TerminalExitReason::ProcessExited => "process_exited",
        TerminalExitReason::AdministratorRequest => "administrator_request",
        TerminalExitReason::PeerDisconnected => "peer_disconnected",
        TerminalExitReason::AuthorizationRevoked => "authorization_revoked",
        TerminalExitReason::IdleTimeout => "idle_timeout",
        TerminalExitReason::LifetimeExceeded => "lifetime_exceeded",
        TerminalExitReason::OutputLimitExceeded => "output_limit_exceeded",
        TerminalExitReason::ProtocolError => "protocol_error",
        TerminalExitReason::ExecutorUnavailable => "executor_unavailable",
    }
}

fn exit_status(reason: TerminalExitReason) -> &'static str {
    match reason {
        TerminalExitReason::ProcessExited | TerminalExitReason::AdministratorRequest => "closed",
        TerminalExitReason::AuthorizationRevoked
        | TerminalExitReason::PeerDisconnected
        | TerminalExitReason::IdleTimeout
        | TerminalExitReason::LifetimeExceeded => "interrupted",
        TerminalExitReason::OutputLimitExceeded
        | TerminalExitReason::ProtocolError
        | TerminalExitReason::ExecutorUnavailable => "failed",
    }
}

fn close_reason(reason: &str) -> TerminalCloseReason {
    match reason {
        "administrator_closed" => TerminalCloseReason::AdministratorRequest,
        "authorization_revoked" | "agent_identity_revoked" => {
            TerminalCloseReason::AuthorizationRevoked
        }
        "idle_timeout" => TerminalCloseReason::IdleTimeout,
        "lifetime_exceeded" => TerminalCloseReason::LifetimeExceeded,
        "browser_disconnected" | "browser_backpressure" => TerminalCloseReason::BrowserDisconnected,
        _ => TerminalCloseReason::ProtocolError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_agent_generation_cannot_advance_or_inject_terminal_output() {
        let registry = TerminalRegistry::default();
        let registration = registry
            .register("term_one", "attach_one".into(), "agent_one", 2)
            .unwrap();
        assert!(
            registry
                .accept_agent_sequence("term_one", "agent_one", 1, 1)
                .unwrap()
                .is_none()
        );
        assert!(registry.contains("term_one"));
        registry.prepare_open("term_one", "attach_one").unwrap();
        assert!(
            registry
                .accept_agent_sequence("term_one", "agent_one", 2, 1)
                .unwrap()
                .is_some()
        );
        drop(registration);
    }

    #[test]
    fn session_allows_only_one_browser_attachment() {
        let registry = TerminalRegistry::default();
        let first = registry
            .register("term_one", "attach_one".into(), "agent_one", 1)
            .unwrap();
        assert!(matches!(
            registry.register("term_one", "attach_two".into(), "agent_one", 1),
            Err(RegisterError::AlreadyAttached)
        ));
        drop(first);
    }

    #[test]
    fn client_close_is_terminal_for_the_browser_input_sequence() {
        let registry = TerminalRegistry::default();
        let registration = registry
            .register("term_one", "attach_one".into(), "agent_one", 1)
            .unwrap();
        registry.prepare_open("term_one", "attach_one").unwrap();
        registry
            .prepare_client_close("term_one", "attach_one", 1)
            .unwrap();
        assert_eq!(
            registry.prepare_client_frame("term_one", "attach_one", 2),
            Err(ForwardError::Closing)
        );
        assert_eq!(
            registry.prepare_client_close("term_one", "attach_one", 2),
            Err(ForwardError::Closing)
        );
        drop(registration);
    }
}
