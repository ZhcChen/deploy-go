mod common;

use axum::http::StatusCode;
use common::{
    RELEASE_SIGNER_SEED, TERMINAL_SIGNER_SEED, admin_session, json_request, response_json,
    test_agent_installation,
};
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
        .with_terminal_signer(deploy_go_terminal_capability::CapabilitySigner::from_seed(
            TERMINAL_SIGNER_SEED,
        ))
        .with_release_signer(deploy_go_release_authorization::ReleaseSigner::from_seed(
            RELEASE_SIGNER_SEED,
        ))
        .with_agent_installation(test_agent_installation());
    (app(state.clone()), pool, state)
}

async fn create_agent(app: axum::Router, cookie: &str, csrf: &str) -> Value {
    let response = json_request(
        app,
        "POST",
        "/api/v1/agents",
        json!({"name":"Node One","environment":"dev"}),
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
    sqlx::query("UPDATE agents SET registered_at='2026-08-03T00:00:00Z',last_seen_at='2026-08-03T00:00:00Z',agent_version='0.1.0',protocol_version=11,capabilities_json='[\"pty_terminal\",\"privileged_release\"]',connection_generation=1 WHERE id=?")
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

#[tokio::test]
async fn archive_hides_node_from_default_list_and_requires_admin() {
    let (app, _pool, _state) = node_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf).await;
    let node_id = created["agent"]["node_id"].as_str().unwrap();

    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/archive"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let list = json_request(
        app.clone(),
        "GET",
        "/api/v1/nodes",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let body = response_json(list).await;
    assert!(
        body["items"].as_array().unwrap().is_empty(),
        "归档节点不应出现在默认列表"
    );

    let archived_list = json_request(
        app.clone(),
        "GET",
        "/api/v1/nodes?archived=true",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(archived_list.status(), StatusCode::OK);
    let body = response_json(archived_list).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["items"][0]["id"].as_str().unwrap(), node_id);
    assert!(body["items"][0]["archived_at"].as_str().is_some());

    let show = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/nodes/{node_id}"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(show.status(), StatusCode::OK);
    let body = response_json(show).await;
    assert!(body["archived_at"].as_str().is_some());

    // 非管理员无法归档。
    let created_user = json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({"username":"operator-archive", "password":"operator-password-long"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created_user.status(), StatusCode::CREATED);
    let (user_cookie, _) =
        common::login(app.clone(), "operator-archive", "operator-password-long").await;
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/unarchive"),
        json!({}),
        &[("cookie", &user_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 恢复后回到默认列表。
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/unarchive"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let list = json_request(
        app,
        "GET",
        "/api/v1/nodes",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let body = response_json(list).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn archiving_node_with_active_deployment_is_rejected() {
    let (app, pool, _state) = node_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf).await;
    let node_id = created["agent"]["node_id"].as_str().unwrap();

    sqlx::query(
        "INSERT INTO applications(id,name,slug,status) VALUES('app_archive','App','app-archive','active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_id: String = sqlx::query_scalar("SELECT id FROM users WHERE username='admin'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target_archive','app_archive',?,'dev','script','/srv/deploy.sh',60,'active')",
    )
    .bind(node_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployments(id,application_id,target_id,status,phase,queued_at,requested_by,idempotency_key,request_hash,snapshot_hash) VALUES('deploy_archive','app_archive','target_archive','running','deploying','2026-08-03T00:00:00Z',?,'k','h','s')",
    )
    .bind(&admin_id)
    .execute(&pool)
    .await
    .unwrap();

    let response = json_request(
        app,
        "POST",
        &format!("/api/v1/nodes/{node_id}/archive"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn archived_node_check_and_deployment_scheduling_are_blocked() {
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
    sqlx::query("UPDATE agents SET registered_at='2026-08-03T00:00:00Z',last_seen_at='2026-08-03T00:00:00Z',agent_version='0.1.0',protocol_version=11,capabilities_json='[\"pty_terminal\",\"privileged_release\"]',connection_generation=1 WHERE id=?")
        .bind(agent_id).execute(&pool).await.unwrap();
    sqlx::query("UPDATE nodes SET archived_at='2026-08-03T00:00:00Z' WHERE id=?")
        .bind(node_id)
        .execute(&pool)
        .await
        .unwrap();

    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/checks"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // 归档节点即使 Agent 在线也不会派发部署任务。
    let task_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks WHERE kind='deployment_prepare'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task_count, 0);
    let _ = state;
}
