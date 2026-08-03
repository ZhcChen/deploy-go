mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_agent_installation};
use deploy_go_agent_protocol::{
    Message, TaskAck, TaskAckDisposition, TaskLifecycleState, TaskResult, TaskState,
    TaskTerminalStatus,
};
use deploy_go_api::{AppState, agents::dispatcher::handle_agent_message, app, db};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;

async fn node_app() -> (axum::Router, sqlx::SqlitePool, AppState) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let state = AppState::new(pool.clone())
        .with_setup_token(common::SETUP_TOKEN)
        .with_agent_installation(test_agent_installation());
    (app(state.clone()), pool, state)
}

async fn create_agent(app: axum::Router, cookie: &str, csrf: &str) -> Value {
    let response = json_request(
        app,
        "POST",
        "/api/v1/agents",
        json!({"name":"Node One"}),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}

#[tokio::test]
async fn legacy_node_write_and_host_key_routes_are_retired() {
    let (app, _pool, _state) = node_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf).await;
    let node_id = created["agent"]["node_id"].as_str().unwrap();

    for (method, path) in [
        ("POST", "/api/v1/nodes".to_owned()),
        ("PATCH", format!("/api/v1/nodes/{node_id}")),
        ("POST", format!("/api/v1/nodes/{node_id}/host-key/scan")),
        ("POST", format!("/api/v1/nodes/{node_id}/host-key/confirm")),
    ] {
        let response = json_request(
            app.clone(),
            method,
            &path,
            json!({}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await;
        assert!(
            matches!(
                response.status(),
                StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
            ),
            "{method} {path} 未退役"
        );
    }
}

#[tokio::test]
async fn agent_check_persists_structured_capabilities() {
    let (app, pool, state) = node_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf).await;
    let node_id = created["agent"]["node_id"].as_str().unwrap();
    let agent_id = created["agent"]["id"].as_str().unwrap();
    sqlx::query("UPDATE nodes SET status='online' WHERE id=?")
        .bind(node_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET registered_at='2026-08-03T00:00:00Z',last_seen_at='2026-08-03T00:00:00Z',agent_version='0.1.0',protocol_version=1,connection_generation=1 WHERE id=?")
        .bind(agent_id).execute(&pool).await.unwrap();

    let response = json_request(
        app,
        "POST",
        &format!("/api/v1/nodes/{node_id}/checks"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let check = response_json(response).await;
    let (task_id, digest): (String, String) =
        sqlx::query_as("SELECT id,payload_digest FROM agent_tasks WHERE node_check_id=?")
            .bind(check["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='delivered' WHERE id=?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        agent_id,
        1,
        &Message::TaskAck(TaskAck {
            task_id: task_id.clone(),
            payload_digest: digest,
            disposition: TaskAckDisposition::Accepted,
            error_code: None,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        agent_id,
        1,
        &Message::TaskState(TaskState {
            task_id: task_id.clone(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        agent_id,
        1,
        &Message::TaskResult(TaskResult {
            task_id,
            sequence: 2,
            status: TaskTerminalStatus::Succeeded,
            exit_code: None,
            error_code: None,
            summary: None,
            data: Some(json!({"os_name":"linux","architecture":"x86_64","disk_available_bytes":1048576,"work_root_accessible":true,"secrets_root_accessible":true})),
        }),
    )
    .await
    .unwrap();
    let stored: (String, String, i64) =
        sqlx::query_as("SELECT status,os_name,disk_available_bytes FROM node_checks WHERE id=?")
            .bind(check["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        stored,
        ("succeeded".to_owned(), "linux".to_owned(), 1_048_576)
    );
}

#[tokio::test]
async fn check_without_online_agent_is_rejected() {
    let (app, _pool, _state) = node_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf).await;
    let node_id = created["agent"]["node_id"].as_str().unwrap();
    let response = json_request(
        app,
        "POST",
        &format!("/api/v1/nodes/{node_id}/checks"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}
