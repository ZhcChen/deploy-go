mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;

async fn node_fixture(pool: &sqlx::SqlitePool, status: &str, protocol: i64, capabilities: &str) {
    sqlx::query("INSERT INTO nodes(id,name,status,work_root,secrets_root) VALUES('node_terminal','Terminal Node',?,'/work','/secrets')")
        .bind(status).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,protocol_version,capabilities_json) VALUES('agent_terminal','node_terminal','2026-08-07T00:00:00Z',?,?)")
        .bind(protocol).bind(capabilities).execute(pool).await.unwrap();
}

#[tokio::test]
async fn administrator_can_create_then_close_a_session_with_a_v11_agent() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    node_fixture(&pool, "online", 11, "[\"pty_terminal\"]").await;
    let capability = json_request(
        app.clone(),
        "GET",
        "/api/v1/nodes/node_terminal/terminal-capability",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(capability.status(), StatusCode::OK);
    let capability = response_json(capability).await;
    assert_eq!(capability["available"], true);
    assert!(
        !capability
            .as_object()
            .expect("terminal capability must be an object")
            .contains_key("privileged_execution")
    );
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/nodes/node_terminal/terminal-sessions",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let session = response_json(created).await;
    assert_eq!(session["status"], "opening");
    let id = session["id"].as_str().unwrap();
    let closed = json_request(
        app,
        "POST",
        &format!("/api/v1/terminal-sessions/{id}/close"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(closed.status(), StatusCode::OK);
    assert_eq!(response_json(closed).await["status"], "closed");
}

#[tokio::test]
async fn ordinary_user_cannot_discover_or_create_a_terminal_session() {
    let (app, pool) = test_app().await;
    let (_cookie, _csrf) = admin_session(app.clone()).await;
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status,display_name) SELECT 'usr_user','user',password_hash,'user','active','User' FROM users WHERE identity='administrator'")
        .execute(&pool).await.unwrap();
    let (cookie, csrf) = common::login(app.clone(), "user", common::ADMIN_PASSWORD).await;
    node_fixture(&pool, "online", 11, "[\"pty_terminal\"]").await;
    for (method, path, body) in [
        (
            "GET",
            "/api/v1/nodes/node_terminal/terminal-capability",
            json!({}),
        ),
        (
            "POST",
            "/api/v1/nodes/node_terminal/terminal-sessions",
            json!({}),
        ),
    ] {
        let response = json_request(
            app.clone(),
            method,
            path,
            body,
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{method} {path}");
    }
}

#[tokio::test]
async fn create_returns_stable_gate_error_codes() {
    for (status, protocol, capabilities, expected) in [
        (
            "offline",
            11,
            "[\"pty_terminal\"]",
            "terminal_agent_offline",
        ),
        (
            "online",
            10,
            "[\"pty_terminal\"]",
            "terminal_protocol_unsupported",
        ),
        ("online", 11, "[]", "terminal_executor_unavailable"),
    ] {
        let (app, pool) = test_app().await;
        let (cookie, csrf) = admin_session(app.clone()).await;
        node_fixture(&pool, status, protocol, capabilities).await;
        let response = json_request(
            app,
            "POST",
            "/api/v1/nodes/node_terminal/terminal-sessions",
            json!({}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(response_json(response).await["code"], expected);
    }
}

#[tokio::test]
async fn v11_agent_can_create_a_session_and_active_conflict_is_stable() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    node_fixture(&pool, "online", 11, "[\"pty_terminal\"]").await;
    let first = json_request(
        app.clone(),
        "POST",
        "/api/v1/nodes/node_terminal/terminal-sessions",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let second = json_request(
        app,
        "POST",
        "/api/v1/nodes/node_terminal/terminal-sessions",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(
        response_json(second).await["code"],
        "terminal_session_active"
    );
}

#[tokio::test]
async fn revoking_agent_closes_its_active_session() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    node_fixture(&pool, "online", 11, "[\"pty_terminal\"]").await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/nodes/node_terminal/terminal-sessions",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let revoked = json_request(
        app,
        "POST",
        "/api/v1/agents/agent_terminal/revoke",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
    let state: (String, String) =
        sqlx::query_as("SELECT status,exit_reason FROM terminal_sessions WHERE id=?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, ("closed".into(), "agent_identity_revoked".into()));
}

#[tokio::test]
async fn retired_privileged_execution_route_is_not_found() {
    let (app, _pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let response = json_request(
        app,
        "PUT",
        "/api/v1/nodes/node_terminal/privileged-execution",
        json!({"enabled": true}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
