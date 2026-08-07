mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;
use tokio_tungstenite::tungstenite::{
    Error as WsError,
    client::IntoClientRequest,
    http::{HeaderValue, header::ORIGIN},
};

const TERMINAL_PROTOCOL: &str = "deploy-go-terminal.v1";

async fn terminal_fixture(
    app: &axum::Router,
    pool: &sqlx::SqlitePool,
    cookie: &str,
    csrf: &str,
) -> String {
    sqlx::query("INSERT INTO nodes(id,name,status,privileged_execution,work_root,secrets_root) VALUES('node_terminal','Terminal Node','online',1,'/work','/secrets')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,protocol_version,capabilities_json) VALUES('agent_terminal','node_terminal','2026-08-07T00:00:00Z',5,'[\"pty_terminal\"]')")
        .execute(pool)
        .await
        .unwrap();
    let response = json_request(
        app.clone(),
        "POST",
        "/api/v1/nodes/node_terminal/terminal-sessions",
        json!({}),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn terminal_request(
    address: std::net::SocketAddr,
    session_id: &str,
    cookie: Option<&str>,
    origin: &str,
    csrf: &str,
) -> tokio_tungstenite::tungstenite::http::Request<()> {
    let mut request = format!("ws://{address}/api/v1/terminal-sessions/{session_id}/stream")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert(ORIGIN, HeaderValue::from_str(origin).unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_str(&format!("{TERMINAL_PROTOCOL}, csrf.{csrf}")).unwrap(),
    );
    if let Some(cookie) = cookie {
        request
            .headers_mut()
            .insert("cookie", HeaderValue::from_str(cookie).unwrap());
    }
    request
}

async fn rejected_status(request: tokio_tungstenite::tungstenite::http::Request<()>) -> StatusCode {
    let error = tokio_tungstenite::connect_async(request).await.unwrap_err();
    let WsError::Http(response) = error else {
        panic!("期望 WebSocket HTTP 握手错误: {error}");
    };
    response.status()
}

#[tokio::test]
async fn terminal_websocket_requires_cookie_origin_administrator_owner_and_csrf() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let session_id = terminal_fixture(&app, &pool, &cookie, &csrf).await;
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status,display_name) SELECT 'usr_user','user',password_hash,'user','active','User' FROM users WHERE username='admin'")
        .execute(&pool)
        .await
        .unwrap();
    let (user_cookie, user_csrf) = common::login(app.clone(), "user", common::ADMIN_PASSWORD).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    assert_eq!(
        rejected_status(terminal_request(
            address,
            &session_id,
            None,
            "http://localhost",
            &csrf,
        ))
        .await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        rejected_status(terminal_request(
            address,
            &session_id,
            Some(&cookie),
            "http://attacker.invalid",
            &csrf,
        ))
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        rejected_status(terminal_request(
            address,
            &session_id,
            Some(&cookie),
            "http://localhost",
            "wrong-csrf-token",
        ))
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        rejected_status(terminal_request(
            address,
            &session_id,
            Some(&user_cookie),
            "http://localhost",
            &user_csrf,
        ))
        .await,
        StatusCode::FORBIDDEN
    );
    server.abort();
}

#[tokio::test]
async fn terminal_websocket_rejects_missing_session_and_offline_agent() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let session_id = terminal_fixture(&app, &pool, &cookie, &csrf).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    assert_eq!(
        rejected_status(terminal_request(
            address,
            "term_missing",
            Some(&cookie),
            "http://localhost",
            &csrf,
        ))
        .await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        rejected_status(terminal_request(
            address,
            &session_id,
            Some(&cookie),
            "http://localhost",
            &csrf,
        ))
        .await,
        StatusCode::CONFLICT
    );

    server.abort();
}
