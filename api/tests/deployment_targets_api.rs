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
        "parameter_schema":{"type":"object","properties":{"release-version":{"type":"string","maxLength":32}},"required":["release-version"],"additionalProperties":false},
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
    let mut punctuation_environment = target_payload(&node_id, "/srv/apps/example/deploy-bang.sh");
    punctuation_environment["environment"] = json!("!");
    let punctuation_target = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        punctuation_environment,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(punctuation_target.status(), StatusCode::CREATED);
    let first_page = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/applications/{application_id}/targets?limit=1"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(first_page["items"][0]["environment"], "!");
    assert!(first_page["next_cursor"].is_string());
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

#[tokio::test]
async fn two_stage_target_requires_verified_source_and_v2_agent() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let (application_id, node_id) = setup_resources(app.clone(), &pool, &cookie, &csrf).await;
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version) VALUES('agent_target','node_target','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.2.0',2)").execute(&pool).await.unwrap();
    let mut two_stage = target_payload(&node_id, "/srv/apps/example/deploy.sh");
    two_stage["execution_mode"] = json!("two_stage");
    two_stage["environment"] = json!("test");
    let missing_source = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        two_stage.clone(),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(missing_source.status(), StatusCode::CONFLICT);

    sqlx::query("INSERT INTO application_sources(id,application_id,repository_url,build_agent_id,source_policy,deployment_branch,source_version,status,version) VALUES('source_target',?,'git@git.example.test:deploy-go/example.git','agent_target','branch','main',1,'verified',1)")
        .bind(&application_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_target_refs','agent_target','git_refs_query','git-refs:source_target:refs','sha256:refs','{}','succeeded','2099-08-06T00:00:00Z')").execute(&pool).await.unwrap();
    let refs = json!([{"name":"main","ref":"refs/heads/main","sha":"0123456789abcdef0123456789abcdef01234567"}]);
    sqlx::query("INSERT INTO git_ref_discoveries(id,application_source_id,source_version,task_id,status,refs_json,expires_at,finished_at) VALUES('refs_target','source_target',1,'task_target_refs','succeeded',?,'2099-08-06T00:00:00Z','2026-08-06T00:00:00Z')")
        .bind(refs.to_string())
        .execute(&pool)
        .await
        .unwrap();
    let created = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        two_stage.clone(),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    assert_eq!(response_json(created).await["execution_mode"], "two_stage");

    sqlx::query("UPDATE agents SET protocol_version=1 WHERE id='agent_target'")
        .execute(&pool)
        .await
        .unwrap();
    let mut old_agent = target_payload(&node_id, "/srv/apps/example/deploy.sh");
    old_agent["execution_mode"] = json!("two_stage");
    old_agent["environment"] = json!("test");
    old_agent["version"] = json!(1);
    let blocked = json_request(
        app,
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        old_agent,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::CONFLICT);
}
