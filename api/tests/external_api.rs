mod common;

use axum::{Router, http::StatusCode};
use common::{admin_session, complete_pending_refs_query, json_request, response_json, test_app};
use deploy_go_api::{AppState, deployments::process_one};
use serde_json::json;
use sqlx::SqlitePool;

async fn seed_application(pool: &SqlitePool, id: &str, name: &str) {
    sqlx::query(
        "INSERT INTO applications(id,name,slug,description,status) VALUES(?,?,?,'','active')",
    )
    .bind(id)
    .bind(name)
    .bind(name.to_lowercase())
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_node_and_target(
    pool: &SqlitePool,
    node_id: &str,
    target_id: &str,
    application_id: &str,
) {
    sqlx::query("UPDATE applications SET environment='test' WHERE id=?")
        .bind(application_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,'外部节点','/srv/apps','/srv/secrets','online')",
    )
    .bind(node_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version) VALUES(?,?,'prod','0.2.0',7)",
    )
    .bind(format!("agent_{node_id}"))
    .bind(node_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES(?,?,?,'test','/srv/deploy.sh',60,'active')",
    )
    .bind(target_id)
    .bind(application_id)
    .bind(node_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_deployable_application(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO applications(id,name,slug,description,status) VALUES('app_deploy','Deploy App','deploy-app','','active')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_deploy','Deploy Node','/srv/apps','/srv/secrets','online')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version) VALUES('agent_deploy','node_deploy','prod','0.2.0',7)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_deploy','app_deploy','node_deploy','prod','/srv/deploy.sh',60,'active')",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_two_stage_deployable_application(pool: &SqlitePool) {
    seed_deployable_application(pool).await;
    sqlx::query("UPDATE applications SET parameter_schema=?, verification_config=? WHERE id='app_deploy'")
        .bind(
            json!({"type":"object","properties":{"release-version":{"type":"string","maxLength":32},"modules":{"type":"string","maxLength":512}},"required":["release-version","modules"],"additionalProperties":false})
                .to_string(),
        )
        .bind(json!({"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000}).to_string())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_targets SET execution_mode='two_stage',script_path='/srv/apps/deploy.sh',timeout_seconds=900 WHERE id='target_deploy'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_sources(id,application_id,repository_url,build_agent_id,source_policy,deployment_branch,source_version,status,version) VALUES('source_deploy','app_deploy','git@git.example.test:deploy-go/example.git','agent_deploy','branch','production',1,'verified',1)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_deploy_refs','agent_deploy','git_refs_query','git-refs:source_deploy:refs_1','sha256:refs','{}','succeeded','2099-08-06T00:00:00Z')")
        .execute(pool)
        .await
        .unwrap();
    let refs = json!([{"name":"production","ref":"refs/heads/production","sha":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"}]);
    sqlx::query("INSERT INTO git_ref_discoveries(id,application_source_id,source_version,task_id,status,refs_json,expires_at,finished_at) VALUES('refs_deploy','source_deploy',1,'task_deploy_refs','succeeded',?,'2099-08-06T00:00:00Z','2026-08-06T00:00:00Z')")
        .bind(refs.to_string())
        .execute(pool)
        .await
        .unwrap();
}

async fn create_key(
    app: &Router,
    cookie: &str,
    csrf: &str,
    name: &str,
    application_ids: &[&str],
) -> String {
    let response = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name": name, "application_ids": application_ids}),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    body["token"].as_str().unwrap().to_owned()
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

#[tokio::test]
async fn external_key_lists_only_granted_active_applications() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "One").await;
    seed_application(&pool, "app_two", "Two").await;
    seed_node_and_target(&pool, "node_one", "target_one", "app_one").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "只读 Key", &["app_one"]).await;

    let response = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications",
        json!({}),
        &[("authorization", &bearer(&token))],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let names = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["One"]);
    assert_eq!(body["items"][0]["environment"], json!("test"));

    let detail = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_one",
        json!({}),
        &[("authorization", &bearer(&token))],
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail = response_json(detail).await;
    assert_eq!(detail["name"], json!("One"));
    assert_eq!(detail["environment"], json!("test"));
    assert_eq!(detail["targets"][0]["id"], json!("target_one"));
    assert_eq!(detail["targets"][0]["environment"], json!("test"));
    assert_eq!(detail["targets"][0]["node_name"], json!("外部节点"));
    assert!(detail.get("script_path").is_none());
    assert!(detail["targets"][0].get("parameter_schema").is_none());

    let denied = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_two",
        json!({}),
        &[("authorization", &bearer(&token))],
    )
    .await;
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);

    let missing_auth = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications",
        json!({}),
        &[],
    )
    .await;
    assert_eq!(missing_auth.status(), StatusCode::UNAUTHORIZED);

    let token_two = create_key(&app, &cookie, &csrf, "第二个 Key", &["app_two"]).await;
    let response = json_request(
        app,
        "GET",
        "/external/v1/applications",
        json!({}),
        &[("authorization", &bearer(&token_two))],
    )
    .await;
    let body = response_json(response).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["items"][0]["id"], json!("app_two"));
}

#[tokio::test]
async fn revoked_or_expired_external_keys_are_rejected() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "One").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "将被吊销", &["app_one"]).await;
    let listed = json_request(
        app.clone(),
        "GET",
        "/api/v1/external-api-keys",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let listed = response_json(listed).await;
    let key_id = listed["items"][0]["id"].as_str().unwrap().to_owned();
    let revoked = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/external-api-keys/{key_id}/revoke"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::OK);
    let denied = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications",
        json!({}),
        &[("authorization", &bearer(&token))],
    )
    .await;
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let bad_token = json_request(
        app,
        "GET",
        "/external/v1/applications",
        json!({}),
        &[("authorization", "Bearer dgx_invalid")],
    )
    .await;
    assert_eq!(bad_token.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn external_key_creates_target_and_application_deployments_idempotently() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "部署 Key", &["app_deploy"]).await;
    let auth = bearer(&token);

    let created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{}}),
        &[
            ("authorization", &auth),
            ("idempotency-key", "external-app-0001"),
        ],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    assert_eq!(created["application_name"], json!("Deploy App"));
    assert_eq!(created["target_runs"].as_array().unwrap().len(), 1);
    let deployment_id = created["id"].as_str().unwrap().to_owned();

    let repeated = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{}}),
        &[
            ("authorization", &auth),
            ("idempotency-key", "external-app-0001"),
        ],
    )
    .await;
    assert_eq!(repeated.status(), StatusCode::OK);
    assert_eq!(response_json(repeated).await["id"], json!(deployment_id));

    let shown = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(shown.status(), StatusCode::OK);
    assert_eq!(response_json(shown).await["id"], json!(deployment_id));

    let target_created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{},"target_id":"target_deploy"}),
        &[
            ("authorization", &auth),
            ("idempotency-key", "external-target-0001"),
        ],
    )
    .await;
    assert_eq!(target_created.status(), StatusCode::CREATED);
    let target_created = response_json(target_created).await;
    assert_eq!(target_created["target_id"], json!("target_deploy"));

    let canceled = json_request(
        app.clone(),
        "POST",
        &format!("/external/v1/deployments/{deployment_id}/cancel"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(canceled.status(), StatusCode::OK);
    let canceled = response_json(canceled).await;
    assert_eq!(canceled["status"], json!("canceled"));

    let denied = json_request(
        app,
        "GET",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{}}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(denied.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn external_deployments_validate_snapshot_and_parameters() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    sqlx::query("UPDATE applications SET parameter_schema=? WHERE id='app_deploy'")
        .bind(
            json!({"type":"object","properties":{"release-version":{"type":"string"}},"required":["release-version"],"additionalProperties":false})
                .to_string(),
        )
        .execute(&pool)
        .await
        .unwrap();
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "校验 Key", &["app_deploy"]).await;
    let auth = bearer(&token);

    let invalid_parameters = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{}}),
        &[
            ("authorization", &auth),
            ("idempotency-key", "external-schema-0001"),
        ],
    )
    .await;
    assert_eq!(
        invalid_parameters.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let stale_snapshot = json_request(
        app,
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":"stale"}),
        &[
            ("authorization", &auth),
            ("idempotency-key", "external-schema-0002"),
        ],
    )
    .await;
    assert_eq!(stale_snapshot.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn external_two_stage_deployment_uses_cross_node_targets_and_run() {
    let (app, pool) = test_app().await;
    seed_two_stage_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "两阶段 Key", &["app_deploy"]).await;

    let refs_done = complete_pending_refs_query(
        AppState::new(pool.clone()),
        "agent_deploy",
        0,
        json!([{"name":"production","ref":"refs/heads/production","sha":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"}]),
    )
    .await;
    let created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{"release-version":"20260811120000","modules":"api,admin"}}),
        &[
            ("authorization", &bearer(&token)),
            ("idempotency-key", "external-two-stage-0001"),
        ],
    )
    .await;
    refs_done.await.unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let deployment_id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(created["target_runs"].as_array().unwrap().len(), 1);

    let snapshot: serde_json::Value =
        sqlx::query_scalar("SELECT snapshot_json FROM deployments WHERE id=?")
            .bind(&deployment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(snapshot["targets"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["targets"][0]["target_id"], json!("target_deploy"));
    assert_eq!(snapshot["targets"][0]["agent_id"], json!("agent_deploy"));
    let run_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployment_target_runs WHERE deployment_id=?")
            .bind(&deployment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(run_count, 1);

    let state = AppState::new(pool.clone()).with_cross_node_artifacts_enabled(true);
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id.as_str())
    );
    let prepare: (String, String, String) = sqlx::query_as(
        "SELECT kind,stage,status FROM agent_tasks WHERE deployment_id=? AND stage='prepare'",
    )
    .bind(&deployment_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        prepare,
        (
            "deployment_prepare".to_owned(),
            "prepare".to_owned(),
            "queued".to_owned()
        )
    );
}

#[tokio::test]
async fn external_openapi_endpoint_is_public_and_contains_only_deploy_paths() {
    let (app, _) = test_app().await;
    let response = json_request(app, "GET", "/external/v1/openapi.json", json!({}), &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let paths = body["paths"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "/external/v1/applications",
            "/external/v1/applications/{id}",
            "/external/v1/applications/{id}/deployments",
            "/external/v1/deployments/{id}",
            "/external/v1/deployments/{id}/cancel",
        ]
    );
}
