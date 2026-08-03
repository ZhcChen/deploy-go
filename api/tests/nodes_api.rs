mod common;

use async_trait::async_trait;
use axum::http::StatusCode;
use common::{admin_session, json_request, response_json};
use deploy_go_agent_protocol::{
    Message, TaskAck, TaskAckDisposition, TaskLifecycleState, TaskResult, TaskState,
    TaskTerminalStatus,
};
use deploy_go_api::{
    AppState,
    agents::dispatcher::handle_agent_message,
    app,
    crypto::MasterKeyRing,
    db,
    executor::ssh::{CapabilityReport, NodeProbe, NodeProbeInput, ProbeError, ScannedHostKey},
};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;

#[derive(Clone)]
struct FakeProbe;

#[async_trait]
impl NodeProbe for FakeProbe {
    async fn scan_host_key(&self, node: &NodeProbeInput) -> Result<ScannedHostKey, ProbeError> {
        Ok(ScannedHostKey {
            host_key: format!(
                "[{}]:{} ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILM+rvN+ot98qgEN796jTiQfZfG1KaT0PtFDJ/XFSqti",
                node.host, node.port
            ),
            fingerprint: "SHA256:fixture-host-key".to_owned(),
        })
    }

    async fn check(
        &self,
        _node: &NodeProbeInput,
        _private_key: &[u8],
        _trusted_host_key: &str,
    ) -> Result<CapabilityReport, ProbeError> {
        unreachable!("节点 capability check 不得调用 SSH probe")
    }
}

async fn node_app() -> (axum::Router, sqlx::SqlitePool, AppState) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let state = AppState::new(pool.clone())
        .with_setup_token(common::SETUP_TOKEN)
        .with_master_key_ring(MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap())
        .with_node_probe(FakeProbe);
    (app(state.clone()), pool, state)
}

async fn create_credential(app: axum::Router, cookie: &str, csrf: &str) -> String {
    let response = json_request(
        app,
        "POST",
        "/api/v1/ssh-credentials",
        json!({"name":"Node Key"}),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    response_json(response).await["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn create_node(
    app: axum::Router,
    cookie: &str,
    csrf: &str,
    credential_id: Option<&str>,
) -> Value {
    let response = json_request(
        app,
        "POST",
        "/api/v1/nodes",
        json!({
            "name":"Node One", "host":"node.example.test", "port":22,
            "username":"deploy", "ssh_credential_id":credential_id,
            "work_root":"/srv/apps", "secrets_root":"/srv/secrets"
        }),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}

#[tokio::test]
async fn host_key_scan_remains_available_for_legacy_nodes() {
    let (app, _pool, _state) = node_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let credential_id = create_credential(app.clone(), &cookie, &csrf).await;
    let node = create_node(app.clone(), &cookie, &csrf, Some(&credential_id)).await;
    let response = json_request(
        app,
        "POST",
        &format!(
            "/api/v1/nodes/{}/host-key/scan",
            node["id"].as_str().unwrap()
        ),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn agent_check_persists_structured_capabilities() {
    let (app, pool, state) = node_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let node = create_node(app.clone(), &cookie, &csrf, None).await;
    let node_id = node["id"].as_str().unwrap();
    sqlx::query("UPDATE nodes SET status='online' WHERE id=?")
        .bind(node_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,connection_generation) VALUES('agent_node_check',?,'2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.1.0',1,1)")
        .bind(node_id).execute(&pool).await.unwrap();
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
    assert_eq!(check["status"], "running");
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
        "agent_node_check",
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
        "agent_node_check",
        1,
        &Message::TaskState(TaskState {
            task_id: task_id.clone(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(&state, "agent_node_check", 1, &Message::TaskResult(TaskResult {
        task_id: task_id.clone(), sequence: 2, status: TaskTerminalStatus::Succeeded,
        exit_code: None, error_code: None, summary: None,
        data: Some(json!({"os_name":"linux","architecture":"x86_64","disk_available_bytes":1048576,"work_root_accessible":true,"secrets_root_accessible":true}))
    })).await.unwrap();
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
    let node = create_node(app.clone(), &cookie, &csrf, None).await;
    let response = json_request(
        app,
        "POST",
        &format!("/api/v1/nodes/{}/checks", node["id"].as_str().unwrap()),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn node_input_rejects_ssh_option_injection() {
    let (app, _pool, _state) = node_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let response = json_request(
        app,
        "POST",
        "/api/v1/nodes",
        json!({"name":"Bad","host":"-oProxyCommand=bad","port":22,"username":"deploy","ssh_credential_id":null,"work_root":"/srv/apps","secrets_root":"/srv/secrets"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
