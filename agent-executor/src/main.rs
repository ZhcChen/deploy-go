use deploy_go_agent_executor::{
    authorization::{AuthorizationError, CapabilityAuthorizer},
    config::{DEFAULT_CONFIG_PATH, ExecutorConfig, LocalConfig, set_owned_permissions},
    peer_auth::{PeerPolicy, credentials, executable_is},
    protocol::{
        ErrorResponse, ExitedResponse, HealthyResponse, OpenedResponse, OutputResponse,
        PROTOCOL_VERSION, ReleaseExitedResponse, ReleaseStartedResponse, ReleaseStatusResponse,
        Request, Response, RuntimeStatusResponse, SelfTestResponse, VersionResponse, read_request,
        validate_request_sequence, write_message,
    },
    pty::PtySession,
    release::ReleaseAdmission,
    release_job::{ReleaseJobError, ReleaseJobManager},
    session_claim::{SessionClaim, SessionRegistry},
};
#[cfg(target_os = "linux")]
use std::sync::Mutex;
use std::{sync::Arc, time::Instant};
use tokio::net::{UnixListener, UnixStream};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    if deploy_go_agent_executor::cgroup::run_launcher_if_requested()? {
        return Ok(());
    }
    tracing_subscriber::fmt().with_env_filter("info").init();
    if unsafe { libc::geteuid() } != 0 {
        anyhow::bail!("executor must run as root");
    }
    let raw = std::fs::read(DEFAULT_CONFIG_PATH)?;
    let local: LocalConfig = serde_json::from_slice(&raw)?;
    let config = Arc::new(ExecutorConfig::from(local));
    config.validate()?;
    #[cfg(target_os = "linux")]
    config.validate_allowed_executable()?;
    let authorizer = Arc::new(CapabilityAuthorizer::new(
        deploy_go_terminal_capability::CapabilityVerifier::from_base64(
            &config.capability_public_key,
        )?,
        config.node_id.clone(),
        config.agent_id.clone(),
        config.capability_replay_dir.clone(),
    ));
    let release_admission = Arc::new(ReleaseAdmission::new(
        deploy_go_release_authorization::ReleaseVerifier::from_base64(&config.release_public_key)?,
        config.release_jobs_dir.clone(),
        config.node_id.clone(),
        config.agent_id.clone(),
    ));
    let release_jobs = Arc::new(
        ReleaseJobManager::new(config.release_jobs_dir.clone()).with_storage_policy(
            config.release_global_storage_bytes,
            config.release_minimum_free_bytes,
            config.release_retention,
        ),
    );
    release_jobs.reconcile_after_restart()?;
    if let Some(parent) = config.socket_path.parent() {
        std::fs::create_dir_all(parent)?;
        set_owned_permissions(parent, config.allowed_gid, 0o750)?;
    }
    match std::fs::remove_file(&config.socket_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let listener = UnixListener::bind(&config.socket_path)?;
    set_owned_permissions(&config.socket_path, config.allowed_gid, 0o660)?;
    let state = Arc::new(SessionRegistry::default());
    let peer_identity = Arc::new(PeerIdentityRegistry::default());
    loop {
        let (stream, _) = listener.accept().await?;
        let config = Arc::clone(&config);
        let state = Arc::clone(&state);
        let authorizer = Arc::clone(&authorizer);
        let peer_identity = Arc::clone(&peer_identity);
        let release_admission = Arc::clone(&release_admission);
        let release_jobs = Arc::clone(&release_jobs);
        tokio::spawn(async move {
            if let Err(error) = serve(
                stream,
                config,
                state,
                peer_identity,
                authorizer,
                release_admission,
                release_jobs,
            )
            .await
            {
                tracing::warn!(error = %error, "executor connection closed");
            }
        });
    }
}

async fn serve(
    mut stream: UnixStream,
    config: Arc<ExecutorConfig>,
    state: Arc<SessionRegistry>,
    peer_identity: Arc<PeerIdentityRegistry>,
    authorizer: Arc<CapabilityAuthorizer>,
    release_admission: Arc<ReleaseAdmission>,
    release_jobs: Arc<ReleaseJobManager>,
) -> anyhow::Result<()> {
    let peer = credentials(&stream)?;
    let policy = PeerPolicy {
        allowed_uid: config.allowed_uid,
        allowed_gid: config.allowed_gid,
    };
    if !policy.authorizes(peer)
        || !executable_is(peer, &config.allowed_executable)
        || !peer_identity.authorize(peer)
    {
        anyhow::bail!("unauthorized local peer");
    }
    // 连接结束后释放 PID 绑定，避免一次性的 doctor/probe/self-test 进程
    // 被仍在运行的 Agent 服务进程永久挡住。
    let _identity_guard = PeerIdentityGuard::new(peer_identity, peer.pid);

    let mut session: Option<PtySession> = None;
    let mut claim: Option<SessionClaim> = None;
    let mut session_id = String::new();
    let mut last_input_sequence = None;
    let mut output_sequence = 0;
    let started = Instant::now();
    let mut last_activity = Instant::now();
    loop {
        let (output_overflowed, pending_output) = if let Some(current) = session.as_ref() {
            let mut pending = Vec::new();
            while let Some(data) = current.recv_output_timeout(std::time::Duration::ZERO) {
                pending.push(data);
            }
            (current.output_overflowed(), pending)
        } else {
            (false, Vec::new())
        };
        if output_overflowed {
            send(
                &mut stream,
                &Response::Exited(ExitedResponse {
                    version: PROTOCOL_VERSION,
                    session_id: session_id.clone(),
                    reason: "output_overflow".into(),
                    exit_code: None,
                }),
                &config,
            )
            .await?;
            break;
        }
        for data in pending_output {
            output_sequence += 1;
            last_activity = Instant::now();
            send(
                &mut stream,
                &Response::Output(OutputResponse {
                    version: PROTOCOL_VERSION,
                    session_id: session_id.clone(),
                    sequence: output_sequence,
                    data,
                }),
                &config,
            )
            .await?;
        }
        let exit_code = match session.as_mut() {
            Some(current) => current.try_wait()?,
            None => None,
        };
        if let Some(exit_code) = exit_code {
            send(
                &mut stream,
                &Response::Exited(ExitedResponse {
                    version: PROTOCOL_VERSION,
                    session_id: session_id.clone(),
                    reason: "process_exited".into(),
                    exit_code: Some(exit_code),
                }),
                &config,
            )
            .await?;
            break;
        }
        if last_activity.elapsed() >= config.idle_timeout {
            send(
                &mut stream,
                &Response::Exited(ExitedResponse {
                    version: PROTOCOL_VERSION,
                    session_id: session_id.clone(),
                    reason: "idle_timeout".into(),
                    exit_code: None,
                }),
                &config,
            )
            .await?;
            break;
        }
        if started.elapsed() >= config.max_lifetime {
            send(
                &mut stream,
                &Response::Exited(ExitedResponse {
                    version: PROTOCOL_VERSION,
                    session_id: session_id.clone(),
                    reason: "max_lifetime".into(),
                    exit_code: None,
                }),
                &config,
            )
            .await?;
            break;
        }

        let request = tokio::time::timeout(
            std::time::Duration::from_millis(10),
            read_request(&mut stream, config.max_frame_bytes),
        )
        .await;
        let request = match request {
            Ok(Ok(Some(request))) => request,
            Ok(Ok(None)) => break,
            Ok(Err(error)) => return Err(error.into()),
            Err(_) => continue,
        };
        if let Request::Probe(request) = request {
            if session.is_some() || request.version != PROTOCOL_VERSION {
                send_error(&mut stream, "incompatible_version", &config).await?;
            } else {
                send(
                    &mut stream,
                    &Response::Healthy(HealthyResponse {
                        version: PROTOCOL_VERSION,
                        capabilities: vec![
                            deploy_go_agent_executor::protocol::ExecutorCapability::PtyTerminal,
                            deploy_go_agent_executor::protocol::ExecutorCapability::DeploymentRelease,
                            deploy_go_agent_executor::protocol::ExecutorCapability::RuntimeStatus,
                        ],
                    }),
                    &config,
                )
                .await?;
            }
            continue;
        }
        if let Request::RuntimeStatus(request) = request {
            if session.is_some()
                || request.version != PROTOCOL_VERSION
                || !deploy_go_agent_executor::runtime_status::validate_request_id(
                    &request.request_id,
                )
            {
                send_error(&mut stream, "incompatible_version", &config).await?;
                continue;
            }
            let timeout =
                std::time::Duration::from_secs(u64::from(request.timeout_seconds.clamp(1, 30)));
            let result = tokio::time::timeout(
                timeout,
                deploy_go_agent_executor::runtime_status::collect(&request.target_code),
            )
            .await;
            let response = match result {
                Ok(Ok(payload)) => RuntimeStatusResponse {
                    version: PROTOCOL_VERSION,
                    request_id: request.request_id.clone(),
                    succeeded: true,
                    payload,
                    error_code: None,
                },
                Ok(Err(error)) => RuntimeStatusResponse {
                    version: PROTOCOL_VERSION,
                    request_id: request.request_id.clone(),
                    succeeded: false,
                    payload: String::new(),
                    error_code: Some(runtime_status_error_code(&error).into()),
                },
                Err(_) => RuntimeStatusResponse {
                    version: PROTOCOL_VERSION,
                    request_id: request.request_id.clone(),
                    succeeded: false,
                    payload: String::new(),
                    error_code: Some("runtime_status_timeout".into()),
                },
            };
            send(&mut stream, &Response::RuntimeStatus(response), &config).await?;
            continue;
        }
        if let Request::VersionProbe(request) = request {
            if session.is_some() || request.version != PROTOCOL_VERSION {
                send_error(&mut stream, "incompatible_version", &config).await?;
            } else {
                send(
                    &mut stream,
                    &Response::Version(VersionResponse {
                        version: PROTOCOL_VERSION,
                        package_version: env!("CARGO_PKG_VERSION").to_owned(),
                    }),
                    &config,
                )
                .await?;
            }
            continue;
        }
        last_activity = Instant::now();
        let (version, sequence) = request_identity(&request);
        let sequence_valid = validate_request_sequence(&request, last_input_sequence);
        if version != PROTOCOL_VERSION || !sequence_valid {
            send_error(&mut stream, "invalid_sequence_or_version", &config).await?;
            break;
        }
        last_input_sequence = Some(sequence);
        match request {
            Request::Open(request) => {
                if session.is_some() {
                    send_error(&mut stream, "session_conflict", &config).await?;
                    continue;
                }
                if let Err(error) = authorizer.authorize(&request, chrono::Utc::now().timestamp()) {
                    let code = match error {
                        AuthorizationError::Replayed => "capability_replayed",
                        AuthorizationError::InvalidCapability => "invalid_capability",
                        AuthorizationError::ReplayStore => "capability_store_unavailable",
                    };
                    send_error(&mut stream, code, &config).await?;
                    break;
                }
                let Some(created_claim) = state.claim(&request.session_id) else {
                    send_error(&mut stream, "session_conflict", &config).await?;
                    continue;
                };
                #[cfg(target_os = "linux")]
                let cgroup =
                    deploy_go_agent_executor::cgroup::SessionCgroup::create(&request.session_id)?
                        .with_cleanup_gate(Arc::clone(&state));
                let created = PtySession::spawn(
                    &config.shell,
                    &config.home,
                    request.rows,
                    request.cols,
                    config.output_buffer_frames,
                    config.close_grace,
                    #[cfg(target_os = "linux")]
                    Some(cgroup),
                    Some(Arc::clone(&state)),
                )?;
                session_id.clone_from(&request.session_id);
                claim = Some(created_claim);
                session = Some(created);
                send(
                    &mut stream,
                    &Response::Opened(OpenedResponse {
                        version: PROTOCOL_VERSION,
                        session_id: session_id.clone(),
                    }),
                    &config,
                )
                .await?;
            }
            Request::Input(request) if request.session_id == session_id => {
                if let Some(current) = session.as_ref() {
                    current.input(&request.data)?;
                }
            }
            Request::Resize(request) if request.session_id == session_id => {
                if let Some(current) = session.as_ref() {
                    current.resize(request.rows, request.cols)?;
                }
            }
            Request::Close(request) if request.session_id == session_id => {
                send(
                    &mut stream,
                    &Response::Exited(ExitedResponse {
                        version: PROTOCOL_VERSION,
                        session_id: session_id.clone(),
                        reason: close_reason(request.reason).into(),
                        exit_code: None,
                    }),
                    &config,
                )
                .await?;
                break;
            }
            Request::ReleaseStart(request) => {
                let state = match release_jobs.status(&request.job_id, &request.task_payload_digest)
                {
                    Ok(state) => state,
                    Err(ReleaseJobError::NotFound) => {
                        if let Err(error) = release_jobs.prepare_admission() {
                            send_release_error(&mut stream, &error, &config).await?;
                            break;
                        }
                        match release_admission.admit(&request, chrono::Utc::now().timestamp()) {
                            Ok(sealed) => match release_jobs.start(sealed, &request.target_code) {
                                Ok(state) => state,
                                Err(error) => {
                                    send_release_error(&mut stream, &error, &config).await?;
                                    break;
                                }
                            },
                            Err(error) => {
                                send_error(&mut stream, release_admission_error(&error), &config)
                                    .await?;
                                break;
                            }
                        }
                    }
                    Err(error) => {
                        send_release_error(&mut stream, &error, &config).await?;
                        break;
                    }
                };
                send(
                    &mut stream,
                    &Response::ReleaseStarted(ReleaseStartedResponse {
                        version: PROTOCOL_VERSION,
                        job_id: state.job_id,
                        state: state.state,
                    }),
                    &config,
                )
                .await?;
                break;
            }
            Request::ReleaseStatus(request) => {
                match release_jobs.status(&request.job_id, &request.task_payload_digest) {
                    Ok(state) if release_terminal(state.state) => {
                        send(
                            &mut stream,
                            &Response::ReleaseExited(ReleaseExitedResponse {
                                version: PROTOCOL_VERSION,
                                job_id: state.job_id,
                                state: state.state,
                                exit_code: state.exit_code,
                                reason: state.reason.unwrap_or_else(|| "terminal".into()),
                                last_sequence: state.last_sequence,
                            }),
                            &config,
                        )
                        .await?;
                    }
                    Ok(state) => {
                        send(
                            &mut stream,
                            &Response::ReleaseStatus(ReleaseStatusResponse {
                                version: PROTOCOL_VERSION,
                                job_id: state.job_id,
                                state: state.state,
                                last_sequence: state.last_sequence,
                            }),
                            &config,
                        )
                        .await?;
                    }
                    Err(error) => send_release_error(&mut stream, &error, &config).await?,
                }
                break;
            }
            Request::ReleaseOutput(request) => {
                match release_jobs.output(
                    &request.job_id,
                    &request.task_payload_digest,
                    request.after_sequence,
                    request.max_frames,
                ) {
                    Ok(batch) => {
                        send(
                            &mut stream,
                            &Response::ReleaseOutput(
                                deploy_go_agent_executor::protocol::ReleaseOutputResponse {
                                    version: PROTOCOL_VERSION,
                                    job_id: request.job_id,
                                    frames: batch.frames,
                                    truncated: batch.truncated,
                                },
                            ),
                            &config,
                        )
                        .await?;
                    }
                    Err(error) => send_release_error(&mut stream, &error, &config).await?,
                }
                break;
            }
            Request::ReleaseCancel(request) => {
                match release_jobs.cancel(&request.job_id, &request.task_payload_digest) {
                    Ok(state) => {
                        send(
                            &mut stream,
                            &Response::ReleaseStatus(ReleaseStatusResponse {
                                version: PROTOCOL_VERSION,
                                job_id: state.job_id,
                                state: state.state,
                                last_sequence: state.last_sequence,
                            }),
                            &config,
                        )
                        .await?;
                    }
                    Err(error) => send_release_error(&mut stream, &error, &config).await?,
                }
                break;
            }
            Request::SelfTest(_) => {
                match deploy_go_agent_executor::self_test::run(&config.release_jobs_dir) {
                    Ok(output) => {
                        send(
                            &mut stream,
                            &Response::SelfTestResult(SelfTestResponse {
                                version: PROTOCOL_VERSION,
                                succeeded: true,
                                output,
                            }),
                            &config,
                        )
                        .await?;
                    }
                    Err(_) => send_error(&mut stream, "release_self_test_failed", &config).await?,
                }
                break;
            }
            _ => send_error(&mut stream, "unknown_session", &config).await?,
        }
    }
    drop(session.take());
    drop(claim.take());
    Ok(())
}

#[derive(Default)]
struct PeerIdentityRegistry {
    #[cfg(target_os = "linux")]
    pinned: Mutex<Option<PinnedPeer>>,
}

#[cfg(target_os = "linux")]
struct PinnedPeer {
    pid: i32,
    connections: usize,
}

struct PeerIdentityGuard {
    registry: Arc<PeerIdentityRegistry>,
    pid: Option<i32>,
}

impl PeerIdentityGuard {
    fn new(registry: Arc<PeerIdentityRegistry>, pid: Option<i32>) -> Self {
        Self { registry, pid }
    }
}

impl Drop for PeerIdentityGuard {
    fn drop(&mut self) {
        self.registry.release_if(self.pid);
    }
}

impl PeerIdentityRegistry {
    fn authorize(&self, _peer: deploy_go_agent_executor::peer_auth::PeerCredentials) -> bool {
        #[cfg(target_os = "linux")]
        {
            let Some(candidate) = _peer.pid else {
                return false;
            };
            let mut pinned = self.pinned.lock().expect("peer identity lock poisoned");
            match pinned.as_mut() {
                Some(current) if current.pid == candidate => {
                    current.connections += 1;
                }
                Some(current) if process_exists(current.pid) => return false,
                _ => {
                    *pinned = Some(PinnedPeer {
                        pid: candidate,
                        connections: 1,
                    });
                }
            }
        }
        true
    }

    fn release_if(&self, pid: Option<i32>) {
        #[cfg(target_os = "linux")]
        {
            let Some(pid) = pid else {
                return;
            };
            let mut pinned = self.pinned.lock().expect("peer identity lock poisoned");
            if let Some(current) = pinned.as_mut()
                && current.pid == pid
            {
                current.connections = current.connections.saturating_sub(1);
                if current.connections == 0 {
                    *pinned = None;
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = pid;
    }
}

#[cfg(target_os = "linux")]
fn process_exists(pid: i32) -> bool {
    (unsafe { libc::kill(pid, 0) }) == 0
        || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{PeerIdentityGuard, PeerIdentityRegistry};
    use deploy_go_agent_executor::peer_auth::PeerCredentials;
    use std::sync::Arc;

    #[test]
    fn connection_guard_releases_pinned_pid() {
        let registry = Arc::new(PeerIdentityRegistry::default());
        let credentials = |pid| PeerCredentials {
            uid: 1000,
            gid: 1000,
            pid: Some(pid),
        };

        assert!(registry.authorize(credentials(10001)));
        {
            let _guard = PeerIdentityGuard::new(Arc::clone(&registry), Some(10001));
        }
        assert!(registry.authorize(credentials(10002)));
    }

    #[test]
    fn rejects_different_pid_while_live_connection_is_pinned() {
        let registry = Arc::new(PeerIdentityRegistry::default());
        let live_pid = std::process::id() as i32;
        let credentials = |pid| PeerCredentials {
            uid: 1000,
            gid: 1000,
            pid: Some(pid),
        };

        assert!(registry.authorize(credentials(live_pid)));
        assert!(registry.authorize(credentials(live_pid)));
        assert!(!registry.authorize(credentials(live_pid.wrapping_add(1_000_000))));
    }

    #[test]
    fn same_pid_connection_remains_pinned_until_all_close() {
        let registry = Arc::new(PeerIdentityRegistry::default());
        let live_pid = std::process::id() as i32;
        let credentials = |pid| PeerCredentials {
            uid: 1000,
            gid: 1000,
            pid: Some(pid),
        };

        assert!(registry.authorize(credentials(live_pid)));
        assert!(registry.authorize(credentials(live_pid)));
        registry.release_if(Some(live_pid));
        assert!(!registry.authorize(credentials(live_pid.wrapping_add(1_000_000))));
        registry.release_if(Some(live_pid));
        assert!(registry.authorize(credentials(live_pid.wrapping_add(1_000_000))));
    }
}

fn close_reason(reason: deploy_go_agent_executor::protocol::CloseReason) -> &'static str {
    use deploy_go_agent_executor::protocol::CloseReason;
    match reason {
        CloseReason::AdministratorRequest => "administrator_request",
        CloseReason::BrowserDisconnected => "browser_disconnected",
        CloseReason::AuthorizationRevoked => "authorization_revoked",
        CloseReason::IdleTimeout => "idle_timeout",
        CloseReason::LifetimeExceeded => "lifetime_exceeded",
        CloseReason::ProtocolError => "protocol_error",
        CloseReason::PeerDisconnected => "peer_disconnected",
    }
}

fn request_identity(request: &Request) -> (u16, u64) {
    match request {
        Request::Probe(request) => (request.version, 0),
        Request::Open(value) => (value.version, value.sequence),
        Request::Input(value) => (value.version, value.sequence),
        Request::Resize(value) => (value.version, value.sequence),
        Request::Close(value) => (value.version, value.sequence),
        Request::ReleaseStart(value) => (value.version, 0),
        Request::ReleaseStatus(value) => (value.version, 0),
        Request::ReleaseOutput(value) => (value.version, 0),
        Request::ReleaseCancel(value) => (value.version, 0),
        Request::SelfTest(value) => (value.version, 0),
        Request::VersionProbe(value) => (value.version, 0),
        Request::RuntimeStatus(value) => (value.version, 0),
    }
}

async fn send(
    stream: &mut UnixStream,
    response: &Response,
    config: &ExecutorConfig,
) -> anyhow::Result<()> {
    write_message(stream, response, config.max_frame_bytes).await?;
    Ok(())
}

async fn send_error(
    stream: &mut UnixStream,
    code: &str,
    config: &ExecutorConfig,
) -> anyhow::Result<()> {
    send(
        stream,
        &Response::Error(ErrorResponse {
            version: PROTOCOL_VERSION,
            code: code.into(),
        }),
        config,
    )
    .await
}

async fn send_release_error(
    stream: &mut UnixStream,
    error: &ReleaseJobError,
    config: &ExecutorConfig,
) -> anyhow::Result<()> {
    let code = match error {
        ReleaseJobError::NotFound => "release_job_not_found",
        ReleaseJobError::Conflict => "release_job_conflict",
        ReleaseJobError::Storage => "release_storage_unavailable",
        ReleaseJobError::StorageLimit => "release_storage_limit_exceeded",
        ReleaseJobError::LowDisk => "release_storage_low_disk",
        ReleaseJobError::Spawn => "release_spawn_failed",
        ReleaseJobError::RecoveryBlocked => "release_recovery_blocked",
        ReleaseJobError::Invalid => "release_request_invalid",
    };
    send_error(stream, code, config).await
}

fn release_admission_error(
    error: &deploy_go_agent_executor::release::ReleaseAdmissionError,
) -> &'static str {
    use deploy_go_agent_executor::release::ReleaseAdmissionError;
    match error {
        ReleaseAdmissionError::InvalidRequest => "release_request_invalid",
        ReleaseAdmissionError::Authorization => "release_authorization_invalid",
        ReleaseAdmissionError::Binding => "release_authorization_binding_mismatch",
        ReleaseAdmissionError::PathEscape => "release_path_escape",
        ReleaseAdmissionError::UnsafeFile => "release_unsafe_file",
        ReleaseAdmissionError::DigestMismatch => "release_digest_mismatch",
        ReleaseAdmissionError::JobConflict => "release_job_conflict",
        ReleaseAdmissionError::Replayed => "release_authorization_replayed",
        ReleaseAdmissionError::Storage => "release_storage_unavailable",
    }
}

fn release_terminal(state: deploy_go_agent_executor::protocol::ReleaseJobState) -> bool {
    use deploy_go_agent_executor::protocol::ReleaseJobState;
    matches!(
        state,
        ReleaseJobState::Succeeded
            | ReleaseJobState::Failed
            | ReleaseJobState::Canceled
            | ReleaseJobState::TimedOut
    )
}

fn runtime_status_error_code(
    error: &deploy_go_agent_executor::runtime_status::RuntimeStatusError,
) -> &'static str {
    use deploy_go_agent_executor::runtime_status::RuntimeStatusError;
    match error {
        RuntimeStatusError::InvalidRequest => "runtime_status_invalid",
        RuntimeStatusError::Collect(_) => "runtime_status_collect_failed",
        RuntimeStatusError::InvalidOutput => "runtime_status_invalid_output",
    }
}
