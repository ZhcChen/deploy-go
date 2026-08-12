mod common;

use axum::{body::to_bytes, http::StatusCode};
use chrono::{Duration, Utc};
use common::{
    RELEASE_SIGNER_SEED, TERMINAL_SIGNER_SEED, admin_session, complete_pending_refs_query,
    json_request, response_json, test_agent_installation,
};
use deploy_go_agent_protocol::{
    Message, OutputStream, TaskAck, TaskAckDisposition, TaskLifecycleState, TaskOutput, TaskResult,
    TaskState, TaskTerminalStatus,
};
use deploy_go_api::{
    AppState, agents::dispatcher::handle_agent_message, app, crypto::MasterKeyRing, db,
    deployments::process_one,
};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;

fn enrollment_body(created: &Value) -> Value {
    json!({
        "agent_id": created["agent"]["id"],
        "enrollment_token": created["enrollment_token"],
        "agent_version": "0.1.0",
        "protocol_version": 1,
        "hostname": "fixture-node",
        "os": "linux",
        "architecture": "x86_64"
    })
}

#[tokio::test]
async fn empty_database_reaches_agent_deployment_and_resumable_sse_without_ssh() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let state = AppState::new(pool.clone())
        .with_master_key_ring(MasterKeyRing::from_raw(1, [9; 32], None).unwrap())
        .with_terminal_signer(deploy_go_terminal_capability::CapabilitySigner::from_seed(
            TERMINAL_SIGNER_SEED,
        ))
        .with_release_signer(deploy_go_release_authorization::ReleaseSigner::from_seed(
            RELEASE_SIGNER_SEED,
        ))
        .with_agent_installation(test_agent_installation());
    let router = app(state.clone());
    let (cookie, csrf) = admin_session(router.clone()).await;

    let created = response_json(
        json_request(
            router.clone(),
            "POST",
            "/api/v1/agents",
            json!({"name":"Fixture Node","environment":"test"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(created["agent"]["status"], "offline");
    let agent_id = created["agent"]["id"].as_str().unwrap();
    let node_id = created["agent"]["node_id"].as_str().unwrap();
    let enrolled = json_request(
        router.clone(),
        "POST",
        "/api/v1/agent/enroll",
        enrollment_body(&created),
        &[],
    )
    .await;
    assert_eq!(enrolled.status(), StatusCode::OK);
    let tokens = response_json(enrolled).await;
    assert_eq!(tokens["agent_id"], agent_id);
    assert!(tokens["access_token"].as_str().unwrap().starts_with("dga_"));
    sqlx::query("UPDATE nodes SET status='online' WHERE id=?")
        .bind(node_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE agents SET last_seen_at='2026-08-03T00:00:00Z',connection_generation=1 WHERE id=?",
    )
    .bind(agent_id)
    .execute(&pool)
    .await
    .unwrap();

    let application = response_json(
        json_request(
            router.clone(),
            "POST",
            "/api/v1/applications",
            json!({"name":"End-to-end App","slug":"end-to-end-app","description":"","environment":"test"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let app_id = application["id"].as_str().unwrap();
    let target = response_json(
        json_request(
            router.clone(),
            "POST",
            &format!("/api/v1/applications/{app_id}/targets"),
            json!({
                "node_id":node_id,
                "script_path":"/var/lib/deploy-go-agent/apps/end-to-end/deploy.sh",
                "parameter_schema":{"type":"object","properties":{"release-version":{"type":"string","maxLength":32}},"required":["release-version"],"additionalProperties":false},
                "timeout_seconds":60,
                "verification_config":{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":1000},
                "secret_file_references":[]
            }),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let target_id = target["id"].as_str().unwrap();
    let preview = response_json(
        json_request(
            router.clone(),
            "POST",
            &format!("/api/v1/deployment-targets/{target_id}/deployment-preview"),
            json!({"parameters":{"release-version":"1.0.0"}}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    let deployment = response_json(
        json_request(
            router.clone(),
            "POST",
            &format!("/api/v1/deployment-targets/{target_id}/deployments"),
            json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":preview["snapshot_hash"]}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf), ("idempotency-key", "agent-end-to-end-0001")],
        )
        .await,
    )
    .await;
    let deployment_id = deployment["id"].as_str().unwrap();
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id)
    );

    let (task_id, digest): (String, String) =
        sqlx::query_as("SELECT id,payload_digest FROM agent_tasks WHERE deployment_id=?")
            .bind(deployment_id)
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
        &Message::TaskOutput(TaskOutput {
            task_id: task_id.clone(),
            sequence: 2,
            stream: OutputStream::Stdout,
            text: "deploying release\n".to_owned(),
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
            sequence: 3,
            status: TaskTerminalStatus::Succeeded,
            exit_code: Some(0),
            error_code: None,
            summary: Some("部署完成".to_owned()),
            data: None,
        }),
    )
    .await
    .unwrap();

    let completed = response_json(
        json_request(
            router.clone(),
            "GET",
            &format!("/api/v1/deployments/{deployment_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(completed["status"], "succeeded");
    assert_eq!(completed["protocol_complete"], true);
    let logs = json_request(
        router,
        "GET",
        &format!("/api/v1/deployments/{deployment_id}/logs"),
        json!({}),
        &[("cookie", &cookie), ("last-event-id", "0")],
    )
    .await;
    let body = String::from_utf8(
        to_bytes(logs.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("deploying release"));
    assert!(body.contains("event: terminal"));
    assert!(!body.contains("PRIVATE KEY"));
    assert!(!body.contains(tokens["refresh_token"].as_str().unwrap()));
}

#[tokio::test]
async fn two_stage_deployment_reaches_success_through_agent_protocol_messages() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let state = AppState::new(pool.clone())
        .with_master_key_ring(MasterKeyRing::from_raw(1, [9; 32], None).unwrap())
        .with_terminal_signer(deploy_go_terminal_capability::CapabilitySigner::from_seed(
            TERMINAL_SIGNER_SEED,
        ))
        .with_release_signer(deploy_go_release_authorization::ReleaseSigner::from_seed(
            RELEASE_SIGNER_SEED,
        ))
        .with_agent_installation(test_agent_installation());
    let router = app(state.clone());
    let (cookie, csrf) = admin_session(router.clone()).await;

    let created = response_json(
        json_request(
            router.clone(),
            "POST",
            "/api/v1/agents",
            json!({"name":"Two Stage Node","environment":"test"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let agent_id = created["agent"]["id"].as_str().unwrap().to_owned();
    let node_id = created["agent"]["node_id"].as_str().unwrap().to_owned();
    let enrolled = json_request(
        router.clone(),
        "POST",
        "/api/v1/agent/enroll",
        json!({
            "agent_id": agent_id,
            "enrollment_token": created["enrollment_token"],
            "agent_version": "0.2.0",
            "protocol_version": 2,
            "hostname": "two-stage-node",
            "os": "linux",
            "architecture": "x86_64"
        }),
        &[],
    )
    .await;
    assert_eq!(enrolled.status(), StatusCode::OK);
    sqlx::query("UPDATE nodes SET status='online' WHERE id=?")
        .bind(&node_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE agents SET last_seen_at='2026-08-06T00:00:00Z',connection_generation=2 WHERE id=?",
    )
    .bind(&agent_id)
    .execute(&pool)
    .await
    .unwrap();

    let application = response_json(
        json_request(
            router.clone(),
            "POST",
            "/api/v1/applications",
            json!({"name":"Two Stage App","slug":"two-stage-app","description":"","environment":"test"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let app_id = application["id"].as_str().unwrap().to_owned();
    let source = response_json(
        json_request(
            router.clone(),
            "PUT",
            &format!("/api/v1/applications/{app_id}/source"),
            json!({
                "repository_url":"git@git.example.test:deploy-go/example.git",
                "git_credential_id": null,
                "build_agent_id": agent_id,
                "source_policy":"branch"
            }),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let source_id = source["id"].as_str().unwrap().to_owned();
    let source_version = source["version"].as_i64().unwrap();
    let future = (Utc::now() + Duration::minutes(30)).to_rfc3339();
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_refs_e2e',?,'git_refs_query','git-refs:e2e:1','sha256:refs','{}','succeeded',?)")
        .bind(&agent_id)
        .bind(&future)
        .execute(&pool)
        .await
        .unwrap();
    let refs = json!([
        {"name":"main","ref":"refs/heads/main","sha":"0123456789abcdef0123456789abcdef01234567"},
        {"name":"develop","ref":"refs/heads/develop","sha":"abcdefabcdefabcdefabcdefabcdefabcdefabcd"}
    ]);
    sqlx::query("INSERT INTO git_ref_discoveries(id,application_source_id,source_version,task_id,status,refs_json,expires_at,finished_at) VALUES('refs_e2e',?,1,'task_refs_e2e','succeeded',?,?,?)")
        .bind(&source_id)
        .bind(refs.to_string())
        .bind(&future)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    let fixed = json_request(
        router.clone(),
        "PUT",
        &format!("/api/v1/applications/{app_id}/source/branch"),
        json!({"branch":"main","version":source_version}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(fixed.status(), StatusCode::OK);
    let target = response_json(
        json_request(
            router.clone(),
            "POST",
            &format!("/api/v1/applications/{app_id}/targets"),
            json!({
                "node_id": node_id,
                "execution_mode":"two_stage",
                "script_path":"/var/lib/deploy-go-agent/apps/two-stage/deploy.sh",
                "parameter_schema":{"type":"object","properties":{"release-version":{"type":"string","maxLength":32},"modules":{"type":"string","maxLength":512}},"required":["release-version","modules"],"additionalProperties":false},
                "timeout_seconds":900,
                "verification_config":{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":1000},
                "secret_file_references":[]
            }),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(target["status"], "active");
    let target_id = target["id"].as_str().unwrap().to_owned();

    let parameters = json!({"release-version":"20260806183000","modules":"api,admin"});
    let refs_done = complete_pending_refs_query(
        state.clone(),
        &agent_id,
        2,
        json!([
            {"name":"main","ref":"refs/heads/main","sha":"0123456789abcdef0123456789abcdef01234567"},
            {"name":"develop","ref":"refs/heads/develop","sha":"abcdefabcdefabcdefabcdefabcdefabcdefabcd"}
        ]),
    )
    .await;
    let preview = response_json(
        json_request(
            router.clone(),
            "POST",
            &format!("/api/v1/deployment-targets/{target_id}/deployment-preview"),
            json!({"parameters": parameters}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    refs_done.await.unwrap();
    assert_eq!(preview["execution_mode"], "two_stage");
    assert_eq!(
        preview["resolved_commit_sha"],
        "0123456789abcdef0123456789abcdef01234567"
    );
    let created_deployment = json_request(
        router.clone(),
        "POST",
        &format!("/api/v1/deployment-targets/{target_id}/deployments"),
        json!({"parameters": parameters, "snapshot_hash": preview["snapshot_hash"]}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "two-stage-e2e-0001"),
        ],
    )
    .await;
    assert_eq!(created_deployment.status(), StatusCode::CREATED);
    let deployment_id = response_json(created_deployment).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id.as_str())
    );
    let prepare_task: (String, String) = sqlx::query_as(
        "SELECT id,status FROM agent_tasks WHERE deployment_id=? AND stage='prepare'",
    )
    .bind(&deployment_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(prepare_task.1, "queued");
    sqlx::query("UPDATE agent_tasks SET status='running' WHERE id=?")
        .bind(&prepare_task.0)
        .execute(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        &agent_id,
        2,
        &Message::TaskState(TaskState {
            task_id: prepare_task.0.clone(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        &agent_id,
        2,
        &Message::TaskOutput(TaskOutput {
            task_id: prepare_task.0.clone(),
            sequence: 2,
            stream: OutputStream::Stdout,
            text: "prepare output\n".to_owned(),
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        &agent_id,
        2,
        &Message::TaskResult(TaskResult {
            task_id: prepare_task.0,
            sequence: 3,
            status: TaskTerminalStatus::Succeeded,
            exit_code: Some(0),
            error_code: None,
            summary: Some("准备完成".to_owned()),
            data: None,
        }),
    )
    .await
    .unwrap();

    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id.as_str())
    );
    let release_task: (String, String) = sqlx::query_as(
        "SELECT id,status FROM agent_tasks WHERE deployment_id=? AND stage='release'",
    )
    .bind(&deployment_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(release_task.1, "queued");
    sqlx::query("UPDATE agent_tasks SET status='running' WHERE id=?")
        .bind(&release_task.0)
        .execute(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        &agent_id,
        2,
        &Message::TaskState(TaskState {
            task_id: release_task.0.clone(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        &agent_id,
        2,
        &Message::TaskOutput(TaskOutput {
            task_id: release_task.0.clone(),
            sequence: 2,
            stream: OutputStream::Stdout,
            text: "release output\n".to_owned(),
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        &agent_id,
        2,
        &Message::TaskResult(TaskResult {
            task_id: release_task.0,
            sequence: 3,
            status: TaskTerminalStatus::Succeeded,
            exit_code: Some(0),
            error_code: None,
            summary: Some("发布完成".to_owned()),
            data: None,
        }),
    )
    .await
    .unwrap();

    let completed = response_json(
        json_request(
            router.clone(),
            "GET",
            &format!("/api/v1/deployments/{deployment_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(completed["status"], "succeeded");
    assert_eq!(completed["protocol_complete"], true);
    let stages: Vec<String> = completed["stage_tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["stage"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(stages, vec!["prepare".to_owned(), "release".to_owned()]);

    let logs = json_request(
        router,
        "GET",
        &format!("/api/v1/deployments/{deployment_id}/logs"),
        json!({}),
        &[("cookie", &cookie), ("last-event-id", "0")],
    )
    .await;
    let body = String::from_utf8(
        to_bytes(logs.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("prepare output"));
    assert!(body.contains("release output"));
    assert!(body.contains("\"stage\":\"prepare\""));
    assert!(body.contains("\"stage\":\"release\""));
    assert!(body.contains("event: terminal"));
}
