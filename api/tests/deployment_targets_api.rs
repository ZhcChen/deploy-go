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
            json!({"name":"Example API","slug":"example-api","description":"","environment":"prod"}),
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
        "node_id":node_id,"script_path":script_path,
        "parameter_schema":{"type":"object","properties":{"release-version":{"type":"string","maxLength":32}},"required":["release-version"],"additionalProperties":false},
        "timeout_seconds":900,
        "verification_config":{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000},
        "secret_file_references":[{"environment_key":"DEPLOY_TOKEN_FILE","file_path":"/srv/secrets/example/token"}]
    })
}

fn image_target_payload(node_id: &str, host_port: u16) -> Value {
    json!({
        "node_id":node_id,
        "script_path":"/srv/apps/ignored",
        "parameter_schema":{"type":"object","properties":{},"additionalProperties":false},
        "timeout_seconds":900,
        "verification_config":{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000},
        "secret_file_references":[],
        "execution_mode":"image",
        "privileged_release":true,
        "privileged_release_confirmed":true,
        "image_spec":{
            "template":"redis",
            "image":"docker.io/library/redis:7-alpine",
            "host_port":host_port,
            "env_files":["compose.env","redis.env"]
        }
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
    assert_eq!(target["environment"], "prod");
    let mut rejected_environment = target_payload(&node_id, "/srv/apps/example/deploy-bang.sh");
    rejected_environment["environment"] = json!("test");
    let rejected_environment = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        rejected_environment,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(
        rejected_environment.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
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
    assert_eq!(first_page["items"][0]["environment"], "prod");
    assert!(first_page["next_cursor"].is_null());
    sqlx::query("UPDATE deployment_targets SET environment='test' WHERE id=?")
        .bind(target_id)
        .execute(&pool)
        .await
        .unwrap();
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
    let updated = response_json(updated).await;
    assert_eq!(updated["environment"], "test");
    assert_ne!(updated["snapshot_hash"], old_hash);
}

#[tokio::test]
async fn target_environment_inherits_and_follows_application_environment() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let (application_id, node_id) = setup_resources(app.clone(), &pool, &cookie, &csrf).await;

    let created = response_json(
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
    assert_eq!(created["environment"], "prod");
    let target_id = created["id"].as_str().unwrap().to_owned();

    let updated = response_json(
        json_request(
            app.clone(),
            "PATCH",
            &format!("/api/v1/applications/{application_id}"),
            json!({
                "name":"Example API",
                "slug":"example-api",
                "description":"",
                "environment":"test",
                "version":1
            }),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(updated["environment"], "test");

    let target = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/deployment-targets/{target_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(target["environment"], "test");
    assert_eq!(target["version"], 2);
    let sync_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE resource_id=? AND action='deployment_target.environment.sync'",
    )
    .bind(&application_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sync_audits, 1);

    let testing_application = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/applications",
            json!({
                "name":"Testing API",
                "slug":"testing-api",
                "description":"",
                "environment":"test"
            }),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let testing_id = testing_application["id"].as_str().unwrap();
    let testing_target = response_json(
        json_request(
            app,
            "POST",
            &format!("/api/v1/applications/{testing_id}/targets"),
            target_payload(&node_id, "/srv/apps/testing/deploy.sh"),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(testing_target["environment"], "test");
}

#[tokio::test]
async fn privileged_release_requires_two_stage_confirmation_and_changes_snapshot() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let (application_id, node_id) = setup_resources(app.clone(), &pool, &cookie, &csrf).await;
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,protocol_version) VALUES('agent_target','node_target','2026-08-10T00:00:00Z',7)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_sources(id,application_id,repository_url,build_agent_id,source_policy,deployment_branch,status,created_by) SELECT 'source_target',?,'git@github.com:example/app.git','agent_target','branch','production','verified',id FROM users WHERE identity='administrator' LIMIT 1")
        .bind(&application_id)
        .execute(&pool)
        .await
        .unwrap();

    let mut unconfirmed = target_payload(&node_id, "/srv/apps/example/deploy.sh");
    unconfirmed["execution_mode"] = json!("two_stage");
    unconfirmed["privileged_release"] = json!(true);
    let rejected = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        unconfirmed.clone(),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);

    unconfirmed["privileged_release_confirmed"] = json!(true);
    let created = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        unconfirmed,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let target = response_json(created).await;
    assert_eq!(target["privileged_release"], true);
    let old_hash = target["snapshot_hash"].as_str().unwrap();

    let mut disabled = target_payload(&node_id, "/srv/apps/example/deploy.sh");
    disabled["execution_mode"] = json!("two_stage");
    disabled["privileged_release"] = json!(false);
    disabled["version"] = target["version"].clone();
    let updated = response_json(
        json_request(
            app,
            "PATCH",
            &format!(
                "/api/v1/deployment-targets/{}",
                target["id"].as_str().unwrap()
            ),
            disabled,
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(updated["privileged_release"], false);
    assert_ne!(updated["snapshot_hash"], old_hash);
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

#[tokio::test]
async fn image_target_requires_v8_privileged_agent_and_registered_env() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let (application_id, node_id) = setup_resources(app.clone(), &pool, &cookie, &csrf).await;
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_image','node_target','2026-08-11T00:00:00Z','2026-08-11T00:00:00Z','0.3.0',8,'[\"pty_terminal\"]')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_digest) VALUES('env_redis',?,'redis.env','redis','dotenv-v1','digest')")
        .bind(&application_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_digest) VALUES('env_compose',?,'compose.env','compose','dotenv-v1','digest')")
        .bind(&application_id)
        .execute(&pool)
        .await
        .unwrap();

    let missing_capability = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        image_target_payload(&node_id, 6379),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(missing_capability.status(), StatusCode::CONFLICT);
    sqlx::query("UPDATE agents SET capabilities_json='[\"pty_terminal\",\"privileged_release\"]' WHERE id='agent_image'")
        .execute(&pool)
        .await
        .unwrap();

    let mut unregistered_env = image_target_payload(&node_id, 6379);
    unregistered_env["image_spec"]["env_files"] =
        json!(["compose.env", "redis.env", "missing.env"]);
    let blocked_env = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        unregistered_env,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(blocked_env.status(), StatusCode::CONFLICT);

    let created = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        image_target_payload(&node_id, 6379),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let target = response_json(created).await;
    let target_id = target["id"].as_str().unwrap();
    assert_eq!(target["execution_mode"], "image");
    assert_eq!(target["privileged_release"], true);
    assert_eq!(target["script_path"], "");
    assert_eq!(target["image_spec"]["template"], "redis");
    assert_eq!(
        target["image_spec"]["image"],
        "docker.io/library/redis:7-alpine"
    );
    assert_eq!(target["image_spec"]["host_port"], 6379);
    let old_hash = target["snapshot_hash"].as_str().unwrap();

    let mut updated = image_target_payload(&node_id, 6380);
    updated["version"] = target["version"].clone();
    let updated = response_json(
        json_request(
            app.clone(),
            "PATCH",
            &format!("/api/v1/deployment-targets/{target_id}"),
            updated,
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(updated["image_spec"]["host_port"], 6380);
    assert_ne!(updated["snapshot_hash"], old_hash);

    let mut back_to_script = target_payload(&node_id, "/srv/apps/example/deploy.sh");
    back_to_script["version"] = updated["version"].clone();
    let downgraded = response_json(
        json_request(
            app,
            "PATCH",
            &format!("/api/v1/deployment-targets/{target_id}"),
            back_to_script,
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(downgraded["execution_mode"], "script");
    assert!(downgraded["image_spec"].is_null());
}

#[tokio::test]
async fn image_target_rejects_unsafe_or_incomplete_specs() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let (application_id, node_id) = setup_resources(app.clone(), &pool, &cookie, &csrf).await;
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_image','node_target','2026-08-11T00:00:00Z','2026-08-11T00:00:00Z','0.3.0',8,'[\"privileged_release\"]')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_digest) VALUES('env_redis',?,'redis.env','redis','dotenv-v1','digest')")
        .bind(&application_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_digest) VALUES('env_compose',?,'compose.env','compose','dotenv-v1','digest')")
        .bind(&application_id)
        .execute(&pool)
        .await
        .unwrap();

    let mut missing_required_env = image_target_payload(&node_id, 6379);
    missing_required_env["image_spec"]["env_files"] = json!(["redis.env"]);
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        missing_required_env,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut no_confirmation = image_target_payload(&node_id, 6379);
    no_confirmation["privileged_release_confirmed"] = json!(false);
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        no_confirmation,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut no_spec = image_target_payload(&node_id, 6379);
    no_spec["image_spec"] = Value::Null;
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        no_spec,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut unsafe_image = image_target_payload(&node_id, 6379);
    unsafe_image["image_spec"]["image"] = json!("redis:7-alpine; id");
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        unsafe_image,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut script_with_spec = target_payload(&node_id, "/srv/apps/example/deploy.sh");
    script_with_spec["image_spec"] = image_target_payload(&node_id, 6379)["image_spec"].clone();
    let response = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        script_with_spec,
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    sqlx::query("UPDATE agents SET protocol_version=7 WHERE id='agent_image'")
        .execute(&pool)
        .await
        .unwrap();
    let response = json_request(
        app,
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        image_target_payload(&node_id, 6379),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}
