mod common;

use axum::http::StatusCode;
use base64::{Engine, engine::general_purpose::STANDARD};
use common::{TERMINAL_SIGNER_SEED, admin_session, json_request, response_json, test_app};
use deploy_go_agent_protocol::{
    AgentCapability, Envelope, Hello, MIN_SUPPORTED_PROTOCOL_VERSION, Message, PROTOCOL_VERSION,
    TerminalBytesEncoding, TerminalExitReason, TerminalExited, TerminalOpened, TerminalOutput,
};
use deploy_go_terminal_capability::{CapabilitySigner, ExpectedBinding};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::{
    Message as WsMessage,
    client::IntoClientRequest,
    http::{HeaderValue, header::AUTHORIZATION},
};

fn agent_envelope(message: Message) -> WsMessage {
    WsMessage::Text(
        serde_json::to_string(&Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: "msg_terminal_test".to_owned(),
            sent_at: "2026-08-07T00:00:00Z".to_owned(),
            message,
        })
        .unwrap()
        .into(),
    )
}

async fn receive_agent(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Envelope {
    let WsMessage::Text(text) = socket.next().await.unwrap().unwrap() else {
        panic!("期望 Agent 文本消息");
    };
    serde_json::from_str(&text).unwrap()
}

async fn receive_browser(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Value {
    let WsMessage::Text(text) = socket.next().await.unwrap().unwrap() else {
        panic!("期望浏览器文本消息");
    };
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn browser_terminal_bridges_v6_agent_and_persists_only_final_metadata() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/agents",
        json!({"name":"terminal-node","environment":"prod"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let node_id = created["agent"]["node_id"].as_str().unwrap().to_owned();
    let enrolled = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/enroll",
        json!({
            "agent_id":created["agent"]["id"],
            "enrollment_token":created["enrollment_token"],
            "agent_version":"0.1.0",
            "protocol_version":PROTOCOL_VERSION,
            "hostname":"terminal-node",
            "os":"linux",
            "architecture":"x86_64"
        }),
        &[],
    )
    .await;
    assert_eq!(enrolled.status(), StatusCode::OK);
    let enrolled = response_json(enrolled).await;
    let agent_id = enrolled["agent_id"].as_str().unwrap().to_owned();
    sqlx::query("UPDATE nodes SET privileged_execution=1 WHERE id=?")
        .bind(&node_id)
        .execute(&pool)
        .await
        .unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server_app = app.clone();
    let server = tokio::spawn(async move { axum::serve(listener, server_app).await.unwrap() });

    let mut agent_request = format!("ws://{address}/api/v1/agent/control")
        .into_client_request()
        .unwrap();
    agent_request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!(
            "Bearer {}",
            enrolled["access_token"].as_str().unwrap()
        ))
        .unwrap(),
    );
    let (mut agent_socket, _) = tokio_tungstenite::connect_async(agent_request)
        .await
        .unwrap();
    agent_socket
        .send(agent_envelope(Message::Hello(Hello {
            agent_id: agent_id.clone(),
            agent_version: "0.1.0".to_owned(),
            min_protocol_version: MIN_SUPPORTED_PROTOCOL_VERSION,
            max_protocol_version: PROTOCOL_VERSION,
            os: "linux".to_owned(),
            architecture: "x86_64".to_owned(),
            capabilities: vec![AgentCapability::PtyTerminal],
        })))
        .await
        .unwrap();
    assert!(matches!(
        receive_agent(&mut agent_socket).await.message,
        Message::HelloAck(_)
    ));

    let created_session = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/terminal-sessions"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created_session.status(), StatusCode::CREATED);
    let session_id = response_json(created_session).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let mut browser_request =
        format!("ws://{address}/api/v1/terminal-sessions/{session_id}/stream")
            .into_client_request()
            .unwrap();
    browser_request
        .headers_mut()
        .insert("cookie", HeaderValue::from_str(&cookie).unwrap());
    browser_request
        .headers_mut()
        .insert("origin", HeaderValue::from_static("http://localhost"));
    browser_request.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_str(&format!("deploy-go-terminal.v1, csrf.{csrf}")).unwrap(),
    );
    let (mut browser_socket, response) = tokio_tungstenite::connect_async(browser_request)
        .await
        .unwrap();
    assert_eq!(
        response.headers().get("sec-websocket-protocol").unwrap(),
        "deploy-go-terminal.v1"
    );
    browser_socket
        .send(WsMessage::Text(
            json!({"type":"open","columns":120,"rows":36})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let open = receive_agent(&mut agent_socket).await;
    let Message::TerminalOpen(open) = open.message else {
        panic!("期望 terminal_open");
    };
    assert_eq!(open.session_id, session_id);
    assert_eq!((open.sequence, open.columns, open.rows), (0, 120, 36));
    CapabilitySigner::from_seed(TERMINAL_SIGNER_SEED)
        .verifier()
        .verify(
            &open.capability,
            &ExpectedBinding {
                node_id: &node_id,
                agent_id: &agent_id,
                session_id: &session_id,
                connection_generation: open.connection_generation,
            },
            chrono::Utc::now().timestamp(),
        )
        .unwrap();

    agent_socket
        .send(agent_envelope(Message::TerminalOpened(TerminalOpened {
            session_id: session_id.clone(),
            sequence: 1,
        })))
        .await
        .unwrap();
    assert_eq!(receive_browser(&mut browser_socket).await["type"], "opened");

    let secret_input = b"echo secret-terminal-input\n";
    browser_socket
        .send(WsMessage::Text(
            json!({
                "type":"input",
                "sequence":1,
                "encoding":"base64",
                "data":STANDARD.encode(secret_input)
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    let Message::TerminalInput(input) = receive_agent(&mut agent_socket).await.message else {
        panic!("期望 terminal_input");
    };
    assert_eq!(input.sequence, 1);

    let secret_output = b"secret-terminal-output\r\n";
    agent_socket
        .send(agent_envelope(Message::TerminalOutput(TerminalOutput {
            session_id: session_id.clone(),
            sequence: 2,
            encoding: TerminalBytesEncoding::Base64,
            data: STANDARD.encode(secret_output),
        })))
        .await
        .unwrap();
    let output = receive_browser(&mut browser_socket).await;
    assert_eq!(output["type"], "output");
    assert_eq!(output["data"], STANDARD.encode(secret_output));

    agent_socket
        .send(agent_envelope(Message::TerminalExited(TerminalExited {
            session_id: session_id.clone(),
            sequence: 3,
            reason: TerminalExitReason::ProcessExited,
            exit_code: Some(0),
        })))
        .await
        .unwrap();
    let exited = receive_browser(&mut browser_socket).await;
    assert_eq!(exited["type"], "exited");
    assert_eq!(exited["exit_code"], 0);

    let stored: (String, i64, i64, String, i64) = sqlx::query_as(
        "SELECT status,input_bytes,output_bytes,exit_reason,exit_code FROM terminal_sessions WHERE id=?",
    )
    .bind(&session_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored.0, "closed");
    assert_eq!(stored.1, secret_input.len() as i64);
    assert_eq!(stored.2, secret_output.len() as i64);
    assert_eq!((stored.3.as_str(), stored.4), ("process_exited", 0));

    let audit: Vec<String> = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE resource_type='terminal_session' AND resource_id=?",
    )
    .bind(&session_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    let joined = audit.join("\n");
    assert!(!joined.contains("secret-terminal-input"));
    assert!(!joined.contains("secret-terminal-output"));
    assert!(joined.contains("\"input_bytes\":27"));
    assert!(joined.contains("\"output_bytes\":24"));

    let second = json_request(
        app,
        "POST",
        &format!("/api/v1/nodes/{node_id}/terminal-sessions"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(second.status(), StatusCode::CREATED);
    let second_id = response_json(second).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut second_request = format!("ws://{address}/api/v1/terminal-sessions/{second_id}/stream")
        .into_client_request()
        .unwrap();
    second_request
        .headers_mut()
        .insert("cookie", HeaderValue::from_str(&cookie).unwrap());
    second_request
        .headers_mut()
        .insert("origin", HeaderValue::from_static("http://localhost"));
    second_request.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_str(&format!("deploy-go-terminal.v1, csrf.{csrf}")).unwrap(),
    );
    let (mut second_browser, _) = tokio_tungstenite::connect_async(second_request)
        .await
        .unwrap();
    second_browser
        .send(WsMessage::Text(
            json!({"type":"open","columns":80,"rows":24})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert!(matches!(
        receive_agent(&mut agent_socket).await.message,
        Message::TerminalOpen(_)
    ));
    agent_socket
        .send(agent_envelope(Message::TerminalOpened(TerminalOpened {
            session_id: second_id.clone(),
            sequence: 1,
        })))
        .await
        .unwrap();
    assert_eq!(receive_browser(&mut second_browser).await["type"], "opened");
    drop(second_browser);
    let close = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        receive_agent(&mut agent_socket),
    )
    .await
    .expect("浏览器断线后 API 未关闭 Agent 终端");
    let Message::TerminalClose(close) = close.message else {
        panic!("期望 terminal_close");
    };
    assert_eq!(close.session_id, second_id);
    let disconnected: (String, String) =
        sqlx::query_as("SELECT status,exit_reason FROM terminal_sessions WHERE id=?")
            .bind(second_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        disconnected,
        ("interrupted".into(), "browser_disconnected".into())
    );

    server.abort();
}

#[tokio::test]
async fn api_restart_interrupts_active_terminal_sessions() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    sqlx::query("INSERT INTO nodes(id,name,status,privileged_execution,work_root,secrets_root) VALUES('node_terminal','Terminal Node','online',1,'/work','/secrets')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,protocol_version,capabilities_json) VALUES('agent_terminal','node_terminal','2026-08-07T00:00:00Z',6,'[\"pty_terminal\"]')")
        .execute(&pool)
        .await
        .unwrap();
    let created = json_request(
        app,
        "POST",
        "/api/v1/nodes/node_terminal/terminal-sessions",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let session_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

    assert_eq!(
        deploy_go_api::terminals::store::interrupt_active_sessions(&pool)
            .await
            .unwrap(),
        1
    );
    let state: (String, String) =
        sqlx::query_as("SELECT status,exit_reason FROM terminal_sessions WHERE id=?")
            .bind(session_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, ("interrupted".into(), "api_restarted".into()));
}
