use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use async_trait::async_trait;
use deploy_go_agent::connection::{
    Backoff, ConnectionClient, ConnectionError, ControlConnector, ControlSession, MessageHandler,
    TokioWebSocketConnector,
};
use deploy_go_agent::token_refresh::{AccessProvider, PreparedAccess, TokenRefreshError};
use deploy_go_agent_protocol::{
    AuthRefreshed, Envelope, Hello, HelloAck, MIN_SUPPORTED_PROTOCOL_VERSION, Message,
    PROTOCOL_VERSION,
};
use tokio::sync::{mpsc, watch};
use url::Url;

#[derive(Default)]
struct NoopHandler;

#[async_trait]
impl MessageHandler for NoopHandler {
    async fn handle(
        &self,
        _envelope: Envelope,
        _outbound: mpsc::Sender<Message>,
    ) -> Result<(), ConnectionError> {
        Ok(())
    }

    fn active_task_ids(&self) -> Vec<String> {
        vec!["task_active".to_owned()]
    }
}

struct MockSession {
    received: VecDeque<Envelope>,
    sent: Arc<Mutex<Vec<Envelope>>>,
}

#[async_trait]
impl ControlSession for MockSession {
    async fn send(&mut self, envelope: &Envelope) -> Result<(), ConnectionError> {
        self.sent.lock().unwrap().push(envelope.clone());
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<Envelope>, ConnectionError> {
        if let Some(envelope) = self.received.pop_front() {
            Ok(Some(envelope))
        } else {
            std::future::pending().await
        }
    }
}

struct MockConnector {
    connections: Arc<Mutex<Vec<Instant>>>,
    sessions: Mutex<VecDeque<Result<Box<dyn ControlSession>, ConnectionError>>>,
}

struct RotatingAccessProvider {
    commits: Arc<Mutex<Vec<String>>>,
}

struct TemporarilyFailingAccessProvider {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl AccessProvider for TemporarilyFailingAccessProvider {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(PreparedAccess {
                access_token: "access_012345678901234567890123456789".to_owned(),
                access_expires_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
                rotation_id: None,
            })
        } else {
            Err(TokenRefreshError::Transport)
        }
    }

    async fn commit(&self, _rotation_id: &str) -> Result<(), TokenRefreshError> {
        Ok(())
    }
}

#[async_trait]
impl AccessProvider for RotatingAccessProvider {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        Ok(PreparedAccess {
            access_token: "access_012345678901234567890123456789".to_owned(),
            access_expires_at: "2099-08-03T03:30:00Z".to_owned(),
            rotation_id: Some("rotation_00000001".to_owned()),
        })
    }

    async fn commit(&self, rotation_id: &str) -> Result<(), TokenRefreshError> {
        self.commits.lock().unwrap().push(rotation_id.to_owned());
        Ok(())
    }
}

#[async_trait]
impl ControlConnector for MockConnector {
    async fn connect(
        &self,
        _url: &Url,
        _access_token: &str,
    ) -> Result<Box<dyn ControlSession>, ConnectionError> {
        self.connections.lock().unwrap().push(Instant::now());
        self.sessions
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Err(ConnectionError::Transport))
    }
}

fn hello() -> Hello {
    Hello {
        agent_id: "agent_01".to_owned(),
        agent_version: "0.1.0".to_owned(),
        min_protocol_version: MIN_SUPPORTED_PROTOCOL_VERSION,
        max_protocol_version: PROTOCOL_VERSION,
        os: "linux".to_owned(),
        architecture: "aarch64".to_owned(),
        capabilities: vec![],
    }
}

fn hello_ack() -> Envelope {
    hello_ack_with_version(PROTOCOL_VERSION)
}

fn hello_ack_with_version(protocol_version: u16) -> Envelope {
    deploy_go_agent::connection::envelope(Message::HelloAck(HelloAck {
        connection_id: "connection_01".to_owned(),
        connection_generation: 1,
        protocol_version,
        heartbeat_interval_seconds: 5,
    }))
}

#[tokio::test(start_paused = true)]
async fn hello_is_first_and_shutdown_stops_the_active_session() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let connector = Arc::new(MockConnector {
        connections: Arc::new(Mutex::new(Vec::new())),
        sessions: Mutex::new(VecDeque::from([Ok(Box::new(MockSession {
            received: VecDeque::from([hello_ack()]),
            sent: Arc::clone(&sent),
        }) as Box<dyn ControlSession>)])),
    });
    let client = ConnectionClient::new(
        connector,
        Arc::new(NoopHandler),
        Url::parse("wss://deploy.example.test/api/v1/agent/ws").unwrap(),
        "access-token-canary",
        hello(),
    );
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(async move { client.run_once(&mut shutdown_rx).await });
    while sent.lock().unwrap().len() < 2 {
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(5)).await;
    }
    shutdown_tx.send(true).unwrap();
    assert!(task.await.unwrap().is_ok());
    let messages = sent.lock().unwrap();
    assert!(matches!(
        messages.first().map(|item| &item.message),
        Some(Message::Hello(_))
    ));
    let Message::Heartbeat(heartbeat) = &messages[1].message else {
        panic!("second message must be a heartbeat");
    };
    assert_eq!(heartbeat.active_task_ids, ["task_active"]);
}

#[tokio::test(start_paused = true)]
async fn outbound_messages_use_the_negotiated_protocol_version() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let connector = Arc::new(MockConnector {
        connections: Arc::new(Mutex::new(Vec::new())),
        sessions: Mutex::new(VecDeque::from([Ok(Box::new(MockSession {
            received: VecDeque::from([hello_ack_with_version(2)]),
            sent: Arc::clone(&sent),
        }) as Box<dyn ControlSession>)])),
    });
    let client = ConnectionClient::new(
        connector,
        Arc::new(NoopHandler),
        Url::parse("wss://deploy.example.test/api/v1/agent/ws").unwrap(),
        "access-token-canary",
        hello(),
    );
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(async move { client.run_once(&mut shutdown_rx).await });
    while sent.lock().unwrap().len() < 2 {
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(5)).await;
    }
    shutdown_tx.send(true).unwrap();
    assert!(task.await.unwrap().is_ok());
    let messages = sent.lock().unwrap();
    assert_eq!(messages[0].protocol_version, PROTOCOL_VERSION);
    assert_eq!(messages[1].protocol_version, 2);
    assert!(matches!(messages[1].message, Message::Heartbeat(_)));
}

#[tokio::test]
async fn pending_rotation_is_confirmed_before_the_session_continues() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let commits = Arc::new(Mutex::new(Vec::new()));
    let connector = Arc::new(MockConnector {
        connections: Arc::new(Mutex::new(Vec::new())),
        sessions: Mutex::new(VecDeque::from([Ok(Box::new(MockSession {
            received: VecDeque::from([
                hello_ack(),
                deploy_go_agent::connection::envelope(Message::AuthRefreshed(AuthRefreshed {
                    rotation_id: "rotation_00000001".to_owned(),
                    access_expires_at: "2099-08-03T03:30:00Z".to_owned(),
                })),
            ]),
            sent: Arc::clone(&sent),
        }) as Box<dyn ControlSession>)])),
    });
    let client = ConnectionClient::with_access_provider(
        connector,
        Arc::new(NoopHandler),
        Url::parse("wss://deploy.example.test/api/v1/agent/control").unwrap(),
        Arc::new(RotatingAccessProvider {
            commits: Arc::clone(&commits),
        }),
        hello(),
    );
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(async move { client.run_once(&mut shutdown_rx).await });
    while commits.lock().unwrap().is_empty() {
        tokio::task::yield_now().await;
    }
    shutdown_tx.send(true).unwrap();
    assert!(task.await.unwrap().is_ok());
    assert_eq!(*commits.lock().unwrap(), ["rotation_00000001"]);
    let messages = sent.lock().unwrap();
    assert!(matches!(messages[0].message, Message::Hello(_)));
    let Message::AuthRefresh(refresh) = &messages[1].message else {
        panic!("第二条消息必须确认 pending rotation");
    };
    assert_eq!(refresh.rotation_id, "rotation_00000001");
}

#[tokio::test(start_paused = true)]
async fn temporary_refresh_failure_keeps_the_authenticated_session_open() {
    let calls = Arc::new(AtomicUsize::new(0));
    let connector = Arc::new(MockConnector {
        connections: Arc::new(Mutex::new(Vec::new())),
        sessions: Mutex::new(VecDeque::from([Ok(Box::new(MockSession {
            received: VecDeque::from([hello_ack()]),
            sent: Arc::new(Mutex::new(Vec::new())),
        }) as Box<dyn ControlSession>)])),
    });
    let client = ConnectionClient::with_access_provider(
        connector,
        Arc::new(NoopHandler),
        Url::parse("wss://deploy.example.test/api/v1/agent/control").unwrap(),
        Arc::new(TemporarilyFailingAccessProvider {
            calls: Arc::clone(&calls),
        }),
        hello(),
    );
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(async move { client.run_once(&mut shutdown_rx).await });
    while calls.load(Ordering::SeqCst) < 2 {
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(30)).await;
    }
    assert!(!task.is_finished());
    shutdown_tx.send(true).unwrap();
    assert!(task.await.unwrap().is_ok());
}

#[tokio::test]
async fn reconnect_failures_are_backed_off_without_a_busy_loop() {
    let connections = Arc::new(Mutex::new(Vec::new()));
    let connector = Arc::new(MockConnector {
        connections: Arc::clone(&connections),
        sessions: Mutex::new(VecDeque::new()),
    });
    let client = ConnectionClient::new(
        connector,
        Arc::new(NoopHandler),
        Url::parse("wss://deploy.example.test/api/v1/agent/ws").unwrap(),
        "access-token-canary",
        hello(),
    )
    .with_backoff(Backoff::new(
        Duration::from_millis(10),
        Duration::from_millis(20),
        0.0,
    ));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(async move { client.run(shutdown_rx).await });
    while connections.lock().unwrap().len() < 3 {
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    shutdown_tx.send(true).unwrap();
    task.await.unwrap();
    let attempts = connections.lock().unwrap();
    assert!(attempts[1].duration_since(attempts[0]) >= Duration::from_millis(9));
    assert!(attempts[2].duration_since(attempts[1]) >= Duration::from_millis(18));
}

#[tokio::test]
#[allow(clippy::result_large_err)]
async fn websocket_access_token_uses_authorization_header_not_the_url() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (observed_tx, observed_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut observed_tx = Some(observed_tx);
        let _socket = tokio_tungstenite::accept_hdr_async(
            stream,
            move |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                  response| {
                let authorization = request
                    .headers()
                    .get("authorization")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                observed_tx
                    .take()
                    .unwrap()
                    .send((request.uri().to_string(), authorization))
                    .unwrap();
                Ok(response)
            },
        )
        .await
        .unwrap();
    });
    let url = Url::parse(&format!("ws://{address}/agent/ws")).unwrap();
    let _session = TokioWebSocketConnector
        .connect(&url, "access-token-canary")
        .await
        .unwrap();
    let (uri, authorization) = observed_rx.await.unwrap();
    assert_eq!(uri, "/agent/ws");
    assert_eq!(authorization.as_deref(), Some("Bearer access-token-canary"));
    server.await.unwrap();
}
