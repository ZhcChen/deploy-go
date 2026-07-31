mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::{Value, json};

async fn setup_resources(
    app: axum::Router,
    pool: &sqlx::SqlitePool,
    cookie: &str,
    csrf: &str,
) -> (String, String) {
    let application = response_json(
        json_request(
            app,
            "POST",
            "/api/v1/applications",
            json!({"name":"Example API","slug":"example-api","description":""}),
            &[("cookie", cookie), ("x-csrf-token", csrf)],
        )
        .await,
    )
    .await;
    let application_id = application["id"].as_str().unwrap().to_owned();
    sqlx::query("INSERT INTO ssh_credentials (id,name,algorithm,public_key,fingerprint,encrypted_private_key,nonce,key_version) VALUES ('cred_target','Target Key','ed25519','ssh-ed25519 AAAA','SHA256:target',X'01',X'02',1)").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO nodes (id,name,host,port,username,ssh_credential_id,work_root,secrets_root,status) VALUES ('node_target','Target Node','node.test',22,'deploy','cred_target','/srv/apps','/srv/secrets','online')").execute(pool).await.unwrap();
    (application_id, "node_target".to_owned())
}

fn target_payload(node_id: &str, script_path: &str) -> Value {
    json!({
        "node_id":node_id,"environment":"production","script_path":script_path,
        "parameter_schema":{"type":"object","properties":{"VERSION":{"type":"string","maxLength":32}},"required":["VERSION"],"additionalProperties":false},
        "timeout_seconds":900,
        "verification_config":{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000},
        "secret_file_references":[{"environment_key":"DEPLOY_TOKEN_FILE","file_path":"/srv/secrets/example/token"}]
    })
}

#[tokio::test]
async fn target_validation_and_changes_produce_new_snapshot_hash() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let (application_id, node_id) = setup_resources(app.clone(), &pool, &cookie, &csrf).await;

    let invalid_path = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        target_payload(&node_id, "/srv/apps/../etc/passwd"),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(invalid_path.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let created = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        target_payload(&node_id, "/srv/apps/example/deploy.sh"),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let target = response_json(created).await;
    let target_id = target["id"].as_str().unwrap();
    let old_hash = target["snapshot_hash"].as_str().unwrap();
    let mut changed = target_payload(&node_id, "/srv/apps/example/deploy-v2.sh");
    changed["version"] = json!(1);
    let updated = json_request(
        app,
        "PATCH",
        &format!("/api/v1/deployment-targets/{target_id}"),
        changed,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_ne!(response_json(updated).await["snapshot_hash"], old_hash);
}

#[tokio::test]
async fn archived_application_and_invalid_secret_or_verification_are_rejected() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let (application_id, node_id) = setup_resources(app.clone(), &pool, &cookie, &csrf).await;
    let mut invalid_secret = target_payload(&node_id, "/srv/apps/example/deploy.sh");
    invalid_secret["secret_file_references"][0]["file_path"] = json!("/etc/shadow");
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        invalid_secret,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let mut invalid_verification = target_payload(&node_id, "/srv/apps/example/deploy.sh");
    invalid_verification["verification_config"] =
        json!({"type":"command","command":"curl localhost"});
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        invalid_verification,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/status"),
        json!({"status":"archived","version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let blocked = json_request(
        app,
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        target_payload(&node_id, "/srv/apps/example/deploy.sh"),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn ordinary_user_sees_only_granted_target_and_related_node() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let (application_id, node_id) = setup_resources(app.clone(), &pool, &cookie, &csrf).await;
    let target = response_json(
        json_request(
            app.clone(),
            "POST",
            &format!("/api/v1/applications/{application_id}/targets"),
            target_payload(&node_id, "/srv/apps/example/deploy.sh"),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let target_id = target["id"].as_str().unwrap();
    let user = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/users",
            json!({"username":"operator","password":"operator-password-long"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let user_id = user["id"].as_str().unwrap();
    let (user_cookie, _) = common::login(app.clone(), "operator", "operator-password-long").await;

    let hidden_target = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/deployment-targets/{target_id}"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    let hidden_node = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/nodes/{node_id}"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(hidden_target.status(), StatusCode::NOT_FOUND);
    assert_eq!(hidden_node.status(), StatusCode::NOT_FOUND);
    json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/users/{user_id}/applications/{application_id}"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let target_visible = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/deployment-targets/{target_id}"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    let node_visible = json_request(
        app,
        "GET",
        &format!("/api/v1/nodes/{node_id}"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(target_visible.status(), StatusCode::OK);
    assert_eq!(node_visible.status(), StatusCode::OK);
}
