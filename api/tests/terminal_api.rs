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
async fn administrator_can_enable_and_create_then_close_a_session() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    node_fixture(&pool, "online", 6, "[\"pty_terminal\"]").await;

    let enabled = json_request(
        app.clone(),
        "PUT",
        "/api/v1/nodes/node_terminal/privileged-execution",
        json!({"enabled":true}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(enabled.status(), StatusCode::OK);
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
async fn ordinary_user_cannot_discover_or_mutate_terminal_capability() {
    let (app, pool) = test_app().await;
    let (_cookie, _csrf) = admin_session(app.clone()).await;
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status,display_name) SELECT 'usr_user','user',password_hash,'user','active','User' FROM users WHERE identity='administrator'")
        .execute(&pool).await.unwrap();
    let (cookie, csrf) = common::login(app.clone(), "user", common::ADMIN_PASSWORD).await;
    node_fixture(&pool, "online", 6, "[\"pty_terminal\"]").await;
    for (method, path, body) in [
        (
            "GET",
            "/api/v1/nodes/node_terminal/terminal-capability",
            json!({}),
        ),
        (
            "PUT",
            "/api/v1/nodes/node_terminal/privileged-execution",
            json!({"enabled":true}),
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
        ("offline", 6, "[\"pty_terminal\"]", "terminal_agent_offline"),
        (
            "online",
            5,
            "[\"pty_terminal\"]",
            "terminal_protocol_unsupported",
        ),
        ("online", 6, "[]", "terminal_executor_unavailable"),
    ] {
        let (app, pool) = test_app().await;
        let (cookie, csrf) = admin_session(app.clone()).await;
        node_fixture(&pool, status, protocol, capabilities).await;
        sqlx::query("UPDATE nodes SET privileged_execution=1 WHERE id='node_terminal'")
            .execute(&pool)
            .await
            .unwrap();
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
async fn disabled_gate_and_active_conflict_are_stable() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    node_fixture(&pool, "online", 6, "[\"pty_terminal\"]").await;
    let disabled = json_request(
        app.clone(),
        "POST",
        "/api/v1/nodes/node_terminal/terminal-sessions",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(
        response_json(disabled).await["code"],
        "terminal_privileged_execution_disabled"
    );
    sqlx::query("UPDATE nodes SET privileged_execution=1 WHERE id='node_terminal'")
        .execute(&pool)
        .await
        .unwrap();
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
    node_fixture(&pool, "online", 6, "[\"pty_terminal\"]").await;
    sqlx::query("UPDATE nodes SET privileged_execution=1 WHERE id='node_terminal'")
        .execute(&pool)
        .await
        .unwrap();
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
