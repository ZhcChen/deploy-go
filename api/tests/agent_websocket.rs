mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use deploy_go_agent_protocol::{
    AuthRefresh, Envelope, Heartbeat, Hello, MIN_SUPPORTED_PROTOCOL_VERSION, Message,
    PROTOCOL_VERSION, TerminalOpened,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::{
    Message as WsMessage,
    client::IntoClientRequest,
    http::{HeaderValue, header::AUTHORIZATION},
};

async fn create_and_enroll(app: axum::Router) -> (Value, String, String) {
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/agents",
        json!({"name":"production-01","environment":"prod"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let enrolled = json_request(
        app,
        "POST",
        "/api/v1/agent/enroll",
        json!({
            "agent_id":created["agent"]["id"],
            "enrollment_token":created["enrollment_token"],
            "agent_version":"0.1.0",
            "protocol_version":PROTOCOL_VERSION,
            "hostname":"node-01",
            "os":"linux",
            "architecture":"x86_64"
        }),
        &[],
    )
    .await;
    assert_eq!(enrolled.status(), StatusCode::OK);
    (response_json(enrolled).await, cookie, csrf)
}

fn envelope(message: Message) -> WsMessage {
    envelope_version(PROTOCOL_VERSION, message)
}

fn envelope_version(version: u16, message: Message) -> WsMessage {
    WsMessage::Text(
        serde_json::to_string(&Envelope {
            protocol_version: version,
            message_id: "msg_test_00000001".to_owned(),
            sent_at: "2026-08-03T03:00:00Z".to_owned(),
            message,
        })
        .unwrap()
        .into(),
    )
}

async fn receive(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Envelope {
    let message = socket.next().await.unwrap().unwrap();
    let WsMessage::Text(text) = message else {
        panic!("期望文本消息");
    };
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn websocket_handshake_heartbeat_and_refresh_keep_the_node_online() {
    let (app, pool) = test_app().await;
    let (enrolled, cookie, csrf) = create_and_enroll(app.clone()).await;
    let agent_id = enrolled["agent_id"].as_str().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server_app = app.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, server_app).await.unwrap();
    });

    let mut request = format!("ws://{address}/api/v1/agent/control")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!(
            "Bearer {}",
            enrolled["access_token"].as_str().unwrap()
        ))
        .unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    socket
        .send(envelope(Message::Hello(Hello {
            agent_id: agent_id.to_owned(),
            agent_version: "0.1.0".to_owned(),
            min_protocol_version: MIN_SUPPORTED_PROTOCOL_VERSION,
            max_protocol_version: PROTOCOL_VERSION,
            os: "linux".to_owned(),
            architecture: "x86_64".to_owned(),
            capabilities: vec![],
        })))
        .await
        .unwrap();
    let hello_ack = receive(&mut socket).await;
    let Message::HelloAck(hello_ack) = hello_ack.message else {
        panic!("期望 hello_ack");
    };
    assert_eq!(hello_ack.protocol_version, PROTOCOL_VERSION);
    let status: String = sqlx::query_scalar(
        "SELECT n.status FROM nodes n JOIN agents a ON a.node_id=n.id WHERE a.id=?",
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "online");

    socket
        .send(envelope(Message::Heartbeat(Heartbeat {
            connection_generation: hello_ack.connection_generation,
            active_task_ids: vec![],
        })))
        .await
        .unwrap();
    assert!(matches!(
        receive(&mut socket).await.message,
        Message::HeartbeatAck(_)
    ));

    let mut takeover_request = format!("ws://{address}/api/v1/agent/control")
        .into_client_request()
        .unwrap();
    takeover_request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!(
            "Bearer {}",
            enrolled["access_token"].as_str().unwrap()
        ))
        .unwrap(),
    );
    let (mut newer_socket, _) = tokio_tungstenite::connect_async(takeover_request)
        .await
        .unwrap();
    newer_socket
        .send(envelope(Message::Hello(Hello {
            agent_id: agent_id.to_owned(),
            agent_version: "0.1.0".to_owned(),
            min_protocol_version: MIN_SUPPORTED_PROTOCOL_VERSION,
            max_protocol_version: PROTOCOL_VERSION,
            os: "linux".to_owned(),
            architecture: "x86_64".to_owned(),
            capabilities: vec![],
        })))
        .await
        .unwrap();
    let newer_ack = receive(&mut newer_socket).await;
    let Message::HelloAck(newer_ack) = newer_ack.message else {
        panic!("期望新连接 hello_ack");
    };
    assert!(
        newer_ack.connection_generation > hello_ack.connection_generation,
        "新连接必须取得更高代次"
    );
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
        .await
        .expect("旧连接未被及时关闭");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let status: String = sqlx::query_scalar(
        "SELECT n.status FROM nodes n JOIN agents a ON a.node_id=n.id WHERE a.id=?",
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "online", "旧连接清理不能覆盖新连接状态");
    socket = newer_socket;

    let rotation_id = "rotation_00000001";
    let refreshed = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/refresh",
        json!({"refresh_token":enrolled["refresh_token"],"rotation_id":rotation_id}),
        &[],
    )
    .await;
    assert_eq!(refreshed.status(), StatusCode::OK);
    let refreshed = response_json(refreshed).await;
    socket
        .send(envelope(Message::AuthRefresh(AuthRefresh {
            access_token: refreshed["access_token"].as_str().unwrap().to_owned(),
            rotation_id: rotation_id.to_owned(),
        })))
        .await
        .unwrap();
    let confirmation = receive(&mut socket).await;
    assert!(matches!(confirmation.message, Message::AuthRefreshed(_)));
    let committed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_refresh_credentials WHERE generation=1 AND committed_at IS NOT NULL AND revoked_at IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(committed, 1);

    socket
        .send(envelope(Message::AuthRefresh(AuthRefresh {
            access_token: refreshed["access_token"].as_str().unwrap().to_owned(),
            rotation_id: rotation_id.to_owned(),
        })))
        .await
        .unwrap();
    let repeated = receive(&mut socket).await;
    assert!(
        matches!(repeated.message, Message::AuthRefreshed(_)),
        "已确认轮换重复确认应保持幂等"
    );
    let revoke_reason: Option<String> = sqlx::query_scalar(
        "SELECT revoke_reason FROM agent_credential_families WHERE agent_id=?",
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(revoke_reason.is_none(), "幂等确认不得吊销凭证 family");

    let status: String = sqlx::query_scalar(
        "SELECT n.status FROM nodes n JOIN agents a ON a.node_id=n.id WHERE a.id=?",
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "online");

    let revoked = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/agents/{agent_id}/revoke"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
        .await
        .expect("管理员撤销后连接未及时关闭");
    let rejected = json_request(
        app,
        "POST",
        "/api/v1/agent/refresh",
        json!({"refresh_token":refreshed["refresh_token"],"rotation_id":"rotation_00000002"}),
        &[],
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
    for _ in 0..20 {
        let status: String = sqlx::query_scalar(
            "SELECT n.status FROM nodes n JOIN agents a ON a.node_id=n.id WHERE a.id=?",
        )
        .bind(agent_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        if status == "offline" {
            server.abort();
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    server.abort();
    panic!("管理员撤销后节点未离线");
}

#[tokio::test]
async fn websocket_negotiates_legacy_v1_agent_and_keeps_the_connection_alive() {
    let (app, pool) = test_app().await;
    let (enrolled, _, _) = create_and_enroll(app.clone()).await;
    let agent_id = enrolled["agent_id"].as_str().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut request = format!("ws://{address}/api/v1/agent/control")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!(
            "Bearer {}",
            enrolled["access_token"].as_str().unwrap()
        ))
        .unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    socket
        .send(envelope_version(
            1,
            Message::Hello(Hello {
                agent_id: agent_id.to_owned(),
                agent_version: "0.1.0".to_owned(),
                min_protocol_version: 1,
                max_protocol_version: 1,
                os: "linux".to_owned(),
                architecture: "x86_64".to_owned(),
                capabilities: vec![],
            }),
        ))
        .await
        .unwrap();
    let hello_ack = receive(&mut socket).await;
    let Message::HelloAck(hello_ack) = hello_ack.message else {
        panic!("期望 hello_ack");
    };
    assert_eq!(hello_ack.protocol_version, 1);
    let stored: i64 = sqlx::query_scalar("SELECT protocol_version FROM agents WHERE id=?")
        .bind(agent_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, 1);

    socket
        .send(envelope_version(
            1,
            Message::Heartbeat(Heartbeat {
                connection_generation: hello_ack.connection_generation,
                active_task_ids: vec![],
            }),
        ))
        .await
        .unwrap();
    let ack = receive(&mut socket).await;
    assert!(matches!(ack.message, Message::HeartbeatAck(_)));
    assert_eq!(ack.protocol_version, 1);

    socket
        .send(envelope_version(
            1,
            Message::TerminalOpened(TerminalOpened {
                session_id: "term_not_allowed".into(),
                sequence: 1,
            }),
        ))
        .await
        .unwrap();
    let rejected = receive(&mut socket).await;
    assert!(matches!(rejected.message, Message::ProtocolError(_)));
    assert_eq!(rejected.protocol_version, 1);

    server.abort();
}

#[tokio::test]
async fn websocket_rejects_missing_or_invalid_access_tokens() {
    let (app, _) = test_app().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let error = tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/agent/control"))
        .await
        .unwrap_err();
    let tokio_tungstenite::tungstenite::Error::Http(response) = error else {
        panic!("期望 HTTP 认证错误");
    };
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    server.abort();
}

#[tokio::test]
async fn api_restart_resets_agent_nodes_to_offline() {
    let (app, pool) = test_app().await;
    let (enrolled, _, _) = create_and_enroll(app).await;
    sqlx::query(
        "UPDATE nodes SET status='online' WHERE id=(SELECT node_id FROM agents WHERE id=?)",
    )
    .bind(enrolled["agent_id"].as_str().unwrap())
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        deploy_go_api::agents::websocket::reset_online_state(&pool)
            .await
            .unwrap(),
        1
    );
    let status: String = sqlx::query_scalar(
        "SELECT n.status FROM nodes n JOIN agents a ON a.node_id=n.id WHERE a.id=?",
    )
    .bind(enrolled["agent_id"].as_str().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "offline");
}
