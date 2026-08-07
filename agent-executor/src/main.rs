use deploy_go_agent_executor::{
    config::{DEFAULT_CONFIG_PATH, ExecutorConfig, LocalConfig, set_owned_permissions},
    peer_auth::{PeerPolicy, credentials},
    protocol::{
        ErrorResponse, ExitedResponse, OpenedResponse, OutputResponse, PROTOCOL_VERSION, Request,
        Response, read_request, validate_request_sequence, write_message,
    },
    pty::PtySession,
    session_claim::{SessionClaim, SessionRegistry},
};
use std::{sync::Arc, time::Instant};
use tokio::net::{UnixListener, UnixStream};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    if unsafe { libc::geteuid() } != 0 {
        anyhow::bail!("executor must run as root");
    }
    let raw = std::fs::read(DEFAULT_CONFIG_PATH)?;
    let local: LocalConfig = serde_json::from_slice(&raw)?;
    let config = Arc::new(ExecutorConfig::from(local));
    config.validate()?;
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
    loop {
        let (stream, _) = listener.accept().await?;
        let config = Arc::clone(&config);
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(error) = serve(stream, config, state).await {
                tracing::warn!(error = %error, "executor connection closed");
            }
        });
    }
}

async fn serve(
    mut stream: UnixStream,
    config: Arc<ExecutorConfig>,
    state: Arc<SessionRegistry>,
) -> anyhow::Result<()> {
    let peer = credentials(&stream)?;
    let policy = PeerPolicy {
        allowed_uid: config.allowed_uid,
        allowed_gid: config.allowed_gid,
    };
    if !policy.authorizes(peer) {
        anyhow::bail!("unauthorized local peer");
    }

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
                let Some(created_claim) = state.claim(&request.session_id) else {
                    send_error(&mut stream, "session_conflict", &config).await?;
                    continue;
                };
                let created = PtySession::spawn(
                    &config.shell,
                    request.rows,
                    request.cols,
                    config.output_buffer_frames,
                    config.close_grace,
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
            Request::Close(request) if request.session_id == session_id => break,
            _ => send_error(&mut stream, "unknown_session", &config).await?,
        }
    }
    drop(session.take());
    drop(claim.take());
    Ok(())
}

fn request_identity(request: &Request) -> (u16, u64) {
    match request {
        Request::Open(value) => (value.version, value.sequence),
        Request::Input(value) => (value.version, value.sequence),
        Request::Resize(value) => (value.version, value.sequence),
        Request::Close(value) => (value.version, value.sequence),
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
