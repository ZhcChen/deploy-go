mod common;

use axum::http::StatusCode;
use common::{ADMIN_PASSWORD, admin_session, json_request, response_json, test_app};
use deploy_go_api::{AppState, agents::auth::token_hash, app, crypto::MasterKeyRing, db};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;

async fn create_agent(app: axum::Router, cookie: &str, csrf: &str, name: &str) -> Value {
    let response = json_request(
        app,
        "POST",
        "/api/v1/agents",
        json!({"name":name,"environment":"staging"}),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}

fn enrollment_body(agent_id: &str, token: &str) -> Value {
    json!({
        "agent_id":agent_id,
        "enrollment_token":token,
        "agent_version":"0.1.0",
        "protocol_version":1,
        "hostname":"node-01",
        "os":"linux",
        "architecture":"x86_64"
    })
}

async fn enroll_agent(app: axum::Router, created: &Value) -> Value {
    let response = json_request(
        app,
        "POST",
        "/api/v1/agent/enroll",
        enrollment_body(
            created["agent"]["id"].as_str().unwrap(),
            created["enrollment_token"].as_str().unwrap(),
        ),
        &[],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

async fn refresh_agent(
    app: axum::Router,
    refresh_token: &str,
    rotation_id: &str,
) -> axum::response::Response {
    json_request(
        app,
        "POST",
        "/api/v1/agent/refresh",
        json!({"refresh_token":refresh_token,"rotation_id":rotation_id}),
        &[],
    )
    .await
}

#[tokio::test]
async fn create_and_enroll_consumes_the_token_without_persisting_plaintext() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    assert_eq!(created["agent"]["status"], "offline");
    let install_command = created["install_command"].as_str().unwrap();
    assert!(install_command.contains("https://deploy.example.test/api/v1/agent/install"));
    assert!(install_command.contains("wss://deploy.example.test/api/v1/agent/control"));
    assert!(
        install_command
            .contains("https://deploy.example.test/api/v1/agent/download/0_1_0/manifest.json")
    );
    assert!(install_command.contains(created["enrollment_token"].as_str().unwrap()));
    assert!(install_command.contains("'DEPLOY_GO_AGENT_ENROLLMENT_TOKEN="));
    assert!(!install_command.contains("read -r -s -p 'Enrollment token: '"));
    let agent_id = created["agent"]["id"].as_str().unwrap();
    let enrollment_token = created["enrollment_token"].as_str().unwrap();
    let stored_enrollment_hash: Vec<u8> =
        sqlx::query_scalar("SELECT token_hash FROM agent_enrollment_tokens WHERE agent_id=?")
            .bind(agent_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        stored_enrollment_hash,
        token_hash("enrollment", enrollment_token)
    );
    assert_ne!(stored_enrollment_hash, enrollment_token.as_bytes());

    let response = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/enroll",
        enrollment_body(agent_id, enrollment_token),
        &[],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let tokens = response_json(response).await;
    assert_eq!(tokens["agent_id"], agent_id);
    for field in ["access_token", "refresh_token"] {
        assert!(tokens[field].as_str().unwrap().starts_with("dga_"));
    }
    let database_dump = sqlx::query_scalar::<_, String>(
        "SELECT hex(token_hash) FROM agent_refresh_credentials LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!database_dump.contains(tokens["refresh_token"].as_str().unwrap()));

    let duplicate = json_request(
        app,
        "POST",
        "/api/v1/agent/enroll",
        enrollment_body(agent_id, enrollment_token),
        &[],
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn administrator_can_bind_agent_to_legacy_node_without_changing_target_identity() {
    let (app, pool) = test_app().await;
    sqlx::query("INSERT INTO nodes(id,name,host,port,username,work_root,secrets_root,status) VALUES('node_legacy','Legacy Node','127.0.0.1',22,'deploy','/srv/apps','/srv/secrets','offline')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_legacy','Legacy App','legacy-app','active')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_legacy','app_legacy','node_legacy','production','/srv/apps/deploy.sh',60,'active')")
        .execute(&pool).await.unwrap();
    let (cookie, csrf) = admin_session(app.clone()).await;

    let response = json_request(
        app.clone(),
        "POST",
        "/api/v1/agents",
        json!({"name":"Legacy Node","node_id":"node_legacy","environment":"staging"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = response_json(response).await;
    assert_eq!(created["agent"]["node_id"], "node_legacy");
    assert_eq!(created["agent"]["name"], "Legacy Node");
    let target_node: String =
        sqlx::query_scalar("SELECT node_id FROM deployment_targets WHERE id='target_legacy'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(target_node, "node_legacy");

    let duplicate = json_request(
        app,
        "POST",
        "/api/v1/agents",
        json!({"name":"Legacy Node","node_id":"node_legacy","environment":"staging"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn enrollment_rejects_wrong_agent_expired_token_and_concurrent_reuse() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let first = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let second = create_agent(app.clone(), &cookie, &csrf, "production-02").await;
    let first_id = first["agent"]["id"].as_str().unwrap();
    let first_token = first["enrollment_token"].as_str().unwrap();
    let second_id = second["agent"]["id"].as_str().unwrap();

    let wrong_agent = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/enroll",
        enrollment_body(second_id, first_token),
        &[],
    )
    .await;
    assert_eq!(wrong_agent.status(), StatusCode::UNAUTHORIZED);

    sqlx::query(
        "UPDATE agent_enrollment_tokens SET expires_at='2000-01-01T00:00:00Z' WHERE agent_id=?",
    )
    .bind(second_id)
    .execute(&pool)
    .await
    .unwrap();
    let expired = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/enroll",
        enrollment_body(second_id, second["enrollment_token"].as_str().unwrap()),
        &[],
    )
    .await;
    assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);

    let body = enrollment_body(first_id, first_token);
    let (left, right) = tokio::join!(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/agent/enroll",
            body.clone(),
            &[]
        ),
        json_request(app, "POST", "/api/v1/agent/enroll", body, &[])
    );
    let statuses = [left.status(), right.status()];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::UNAUTHORIZED)
            .count(),
        1
    );
}

#[tokio::test]
async fn agent_management_requires_administrator_csrf() {
    let (app, _) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    let missing_csrf = json_request(
        app,
        "POST",
        "/api/v1/agents",
        json!({"name":"production-01","environment":"staging"}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn create_refuses_to_issue_command_without_trusted_release_config() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let app = app(AppState::new(pool.clone())
        .with_master_key_ring(MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap()));
    common::initialize_admin(app.clone()).await;
    let (cookie, csrf) = common::login(app.clone(), "admin", ADMIN_PASSWORD).await;

    let response = json_request(
        app,
        "POST",
        "/api/v1/agents",
        json!({"name":"production-01","environment":"staging"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response_json(response).await["code"],
        "agent_installation_unavailable"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn create_agent_rejects_unknown_environment() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let response = json_request(
        app,
        "POST",
        "/api/v1/agents",
        json!({"name":"bad-env","environment":"pre"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn installer_route_ignores_request_host_and_contains_no_credentials() {
    let (app, _) = test_app().await;
    let response = json_request(
        app,
        "GET",
        "/api/v1/agent/install",
        json!(null),
        &[("host", "attacker.example")],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let script = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(script.starts_with("#!/usr/bin/env bash"));
    assert!(!script.contains("attacker.example"));
    assert!(!script.contains("dga_"));
}

#[tokio::test]
async fn revoked_agent_receives_an_explicit_rebind_command() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let agent_id = created["agent"]["id"].as_str().unwrap();
    let revoke = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/agents/{agent_id}/revoke"),
        json!(null),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(revoke.status(), StatusCode::NO_CONTENT);

    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/agents/{agent_id}/install-command"),
        json!(null),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let command = response_json(response).await;
    assert!(
        command["install_command"]
            .as_str()
            .unwrap()
            .contains("'DEPLOY_GO_AGENT_REBIND=1'")
    );

    let enrolled = json_request(
        app,
        "POST",
        "/api/v1/agent/enroll",
        enrollment_body(agent_id, command["enrollment_token"].as_str().unwrap()),
        &[],
    )
    .await;
    assert_eq!(enrolled.status(), StatusCode::OK);
}

#[tokio::test]
async fn administrator_lists_agent_runtime_status_and_ordinary_user_is_forbidden() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let agent_id = created["agent"]["id"].as_str().unwrap();
    sqlx::query("UPDATE agents SET agent_version='0.1.0',hostname='node-01',architecture='x86_64',last_seen_at='2026-08-03T03:00:00Z' WHERE id=?")
        .bind(agent_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE nodes SET status='online' WHERE id=?")
        .bind(created["agent"]["node_id"].as_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let list = json_request(
        app.clone(),
        "GET",
        "/api/v1/agents?limit=20",
        json!(null),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let list = response_json(list).await;
    assert_eq!(list["items"][0]["status"], "online");
    assert_eq!(list["items"][0]["agent_version"], "0.1.0");

    let show = json_request(
        app,
        "GET",
        &format!("/api/v1/agents/{agent_id}"),
        json!(null),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(show.status(), StatusCode::OK);
    assert_eq!(response_json(show).await["hostname"], "node-01");
}

#[tokio::test]
async fn enrollment_rejects_unsupported_protocol_without_consuming_token() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let agent_id = created["agent"]["id"].as_str().unwrap();
    let enrollment_token = created["enrollment_token"].as_str().unwrap();
    let mut unsupported = enrollment_body(agent_id, enrollment_token);
    unsupported["protocol_version"] = json!(deploy_go_agent_protocol::PROTOCOL_VERSION + 1);

    let rejected = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/enroll",
        unsupported,
        &[],
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let supported = json_request(
        app,
        "POST",
        "/api/v1/agent/enroll",
        enrollment_body(agent_id, enrollment_token),
        &[],
    )
    .await;
    assert_eq!(supported.status(), StatusCode::OK);
}

#[tokio::test]
async fn refresh_rotation_is_idempotent_for_the_same_rotation_id() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let enrolled = enroll_agent(app.clone(), &created).await;
    let original = enrolled["refresh_token"].as_str().unwrap();

    let first = refresh_agent(app.clone(), original, "rotation_00000001").await;
    assert_eq!(first.status(), StatusCode::OK);
    let first = response_json(first).await;
    let replay = refresh_agent(app, original, "rotation_00000001").await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(response_json(replay).await, first);
    assert_ne!(first["refresh_token"], enrolled["refresh_token"]);
    assert_ne!(first["access_token"], enrolled["access_token"]);

    let generations: Vec<i64> =
        sqlx::query_scalar("SELECT generation FROM agent_refresh_credentials ORDER BY generation")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(generations, vec![1, 2]);
}

#[tokio::test]
async fn refresh_token_reuse_revokes_the_credential_family_and_is_audited() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let enrolled = enroll_agent(app.clone(), &created).await;
    let original = enrolled["refresh_token"].as_str().unwrap();
    let rotated = refresh_agent(app.clone(), original, "rotation_00000001").await;
    assert_eq!(rotated.status(), StatusCode::OK);
    let rotated = response_json(rotated).await;

    let reuse = refresh_agent(app.clone(), original, "rotation_00000002").await;
    assert_eq!(reuse.status(), StatusCode::UNAUTHORIZED);
    let successor_rejected = refresh_agent(
        app,
        rotated["refresh_token"].as_str().unwrap(),
        "rotation_00000003",
    )
    .await;
    assert_eq!(successor_rejected.status(), StatusCode::UNAUTHORIZED);

    let reason: String =
        sqlx::query_scalar("SELECT revoke_reason FROM agent_credential_families WHERE agent_id=?")
            .bind(created["agent"]["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(reason, "refresh_token_reuse");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE action='agent.refresh_token_reuse' AND resource_id=?",
    )
    .bind(created["agent"]["id"].as_str().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn refresh_rejects_invalid_rotation_and_expired_credentials() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let enrolled = enroll_agent(app.clone(), &created).await;
    let original = enrolled["refresh_token"].as_str().unwrap();

    let invalid = refresh_agent(app.clone(), original, "short").await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
    sqlx::query("UPDATE agent_refresh_credentials SET expires_at='2000-01-01T00:00:00Z'")
        .execute(&pool)
        .await
        .unwrap();
    let expired = refresh_agent(app, original, "rotation_00000001").await;
    assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn confirmed_refresh_token_reuse_revokes_the_family() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let enrolled = enroll_agent(app.clone(), &created).await;
    let original = enrolled["refresh_token"].as_str().unwrap();
    let rotated = refresh_agent(app.clone(), original, "rotation_00000001").await;
    assert_eq!(rotated.status(), StatusCode::OK);
    sqlx::query("UPDATE agent_refresh_credentials SET committed_at='2026-08-03T03:00:00Z',revoked_at='2026-08-03T03:00:00Z' WHERE generation=1")
        .execute(&pool)
        .await
        .unwrap();

    let reuse = refresh_agent(app, original, "rotation_00000001").await;
    assert_eq!(reuse.status(), StatusCode::UNAUTHORIZED);
    let reason: String =
        sqlx::query_scalar("SELECT revoke_reason FROM agent_credential_families LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(reason, "refresh_token_reuse");
}
