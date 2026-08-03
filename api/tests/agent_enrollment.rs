mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use deploy_go_api::agents::auth::token_hash;
use serde_json::{Value, json};

async fn create_agent(app: axum::Router, cookie: &str, csrf: &str, name: &str) -> Value {
    let response = json_request(
        app,
        "POST",
        "/api/v1/agents",
        json!({"name":name}),
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

#[tokio::test]
async fn create_and_enroll_consumes_the_token_without_persisting_plaintext() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    assert_eq!(created["agent"]["status"], "offline");
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
        json!({"name":"production-01"}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn enrollment_rejects_unsupported_protocol_without_consuming_token() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = create_agent(app.clone(), &cookie, &csrf, "production-01").await;
    let agent_id = created["agent"]["id"].as_str().unwrap();
    let enrollment_token = created["enrollment_token"].as_str().unwrap();
    let mut unsupported = enrollment_body(agent_id, enrollment_token);
    unsupported["protocol_version"] = json!(2);

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
