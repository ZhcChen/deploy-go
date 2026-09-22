mod common;

use axum::{Router, http::StatusCode};
use common::{admin_session, complete_pending_refs_query, json_request, response_json, test_app};
use deploy_go_api::{AppState, deployments::process_one};
use serde_json::{Value, json};
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
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json) VALUES(?,?,'prod','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')",
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
        "INSERT INTO applications(id,name,slug,description,status,environment) VALUES('app_deploy','Deploy App','deploy-app','','active','test')",
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
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json) VALUES('agent_deploy','node_deploy','test','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_deploy','app_deploy','node_deploy','test','/srv/deploy.sh',60,'active')",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_extra_node(pool: &SqlitePool, node_id: &str, agent_id: &str) {
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?,'/srv/apps','/srv/secrets','online')",
    )
    .bind(node_id)
    .bind(format!("额外节点 {node_id}"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json) VALUES(?,?,'test','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')",
    )
    .bind(agent_id)
    .bind(node_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_production_deployable_application(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO applications(id,name,slug,description,status,environment) VALUES('app_prod','Prod App','prod-app','','active','prod')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_prod','正式节点','/srv/apps','/srv/secrets','online')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json) VALUES('agent_prod','node_prod','prod','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_prod','app_prod','node_prod','prod','/srv/deploy.sh',60,'active')",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_non_production_application_with_production_target(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO applications(id,name,slug,description,status,environment) VALUES('app_drift','Drift App','drift-app','','active','test')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_drift','漂移节点','/srv/apps','/srv/secrets','online')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json) VALUES('agent_drift','node_drift','test','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_drift','app_drift','node_drift','prod','/srv/deploy.sh',60,'active')",
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
async fn external_deployment_logs_are_paginated_authorized_and_sanitized() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    seed_application(&pool, "app_other", "Other").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "诊断 Key", &["app_deploy"]).await;
    let other_token = create_key(&app, &cookie, &csrf, "其他诊断 Key", &["app_other"]).await;
    let auth = bearer(&token);
    let created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{}}),
        &[
            ("authorization", &auth),
            ("idempotency-key", "diagnostics-0001"),
        ],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let deployment_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    sqlx::query("UPDATE deployments SET status='failed',phase='failed',exit_code=2,finished_at=datetime('now') WHERE id=?")
        .bind(&deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_target_runs SET status='failed',phase='release',error_code='process_exited',result_summary='发布失败' WHERE deployment_id=?")
        .bind(&deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,task_sequence,stream,content) VALUES(?,?,?,?,?)")
        .bind(&deployment_id)
        .bind(1_i64)
        .bind(1_i64)
        .bind("stderr")
        .bind("连接数据库失败")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,task_sequence,stream,content) VALUES(?,?,?,?,?)")
        .bind(&deployment_id)
        .bind(3_i64)
        .bind(3_i64)
        .bind("stderr")
        .bind(
            "DATABASE_URL=postgres://deploy:database-secret@db.internal/app\nAWS_SECRET_ACCESS_KEY=aws-secret\nrequest Authorization: Bearer bearer-secret\n-----BEGIN PRIVATE KEY-----\nprivate-key-base64\n-----END PRIVATE KEY-----",
        )
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,task_sequence,stream,content) VALUES(?,?,?,?,?)")
        .bind(&deployment_id)
        .bind(2_i64)
        .bind(2_i64)
        .bind("stderr")
        .bind("password=super-secret")
        .execute(&pool)
        .await
        .unwrap();

    let first = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/logs?limit=1"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first = response_json(first).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 1);
    assert_eq!(first["items"][0]["sequence"], json!(1));
    assert_eq!(first["next_after"], json!(1));
    assert_eq!(first["terminal"], json!(false));

    let second = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/logs?after=1"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let second = response_json(second).await;
    assert_eq!(second["items"][0]["sequence"], json!(2));
    assert_eq!(second["items"][0]["content"], json!("[已脱敏：敏感日志]"));
    assert_eq!(second["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        second["items"][1]["content"],
        json!("[已脱敏：敏感日志]\n[已脱敏：敏感日志]\n[已脱敏：敏感日志]\n[已脱敏：私钥内容]")
    );
    assert_eq!(second["next_after"], serde_json::Value::Null);
    assert_eq!(second["terminal"], json!(true));

    let empty_page = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/logs?after=3"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let empty_page = response_json(empty_page).await;
    assert!(empty_page["items"].as_array().unwrap().is_empty());
    assert_eq!(empty_page["next_after"], serde_json::Value::Null);
    assert_eq!(empty_page["terminal"], json!(true));

    sqlx::query("INSERT INTO deployment_logs(deployment_id,sequence,task_sequence,stream,content) VALUES(?,?,?,?,?),(?,?,?,?,?),(?,?,?,?,?),(?,?,?,?,?),(?,?,?,?,?)")
        .bind(&deployment_id)
        .bind(4_i64)
        .bind(4_i64)
        .bind("stderr")
        .bind("-----BEGIN OPENSSH PRIVATE KEY-----")
        .bind(&deployment_id)
        .bind(5_i64)
        .bind(5_i64)
        .bind("stderr")
        .bind("YWJjZGVmZw==")
        .bind(&deployment_id)
        .bind(6_i64)
        .bind(6_i64)
        .bind("stderr")
        .bind("-----END OPENSSH PRIVATE KEY-----")
        .bind(&deployment_id)
        .bind(7_i64)
        .bind(7_i64)
        .bind("stderr")
        .bind("{\"token\":\"json-secret\",\"url\":\"https://example.test?access_token=query-secret\"}")
        .bind(&deployment_id)
        .bind(8_i64)
        .bind(8_i64)
        .bind("stderr")
        .bind("{\"private_key\":\"private-key-material\"}")
        .execute(&pool)
        .await
        .unwrap();
    let split_secrets = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/logs?after=3"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let split_secrets = response_json(split_secrets).await;
    assert_eq!(
        split_secrets["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["content"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "[已脱敏：私钥内容]",
            "",
            "",
            "[已脱敏：敏感日志]",
            "[已脱敏：敏感日志]",
        ]
    );
    let split_page = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/logs?after=4&limit=1"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let split_page = response_json(split_page).await;
    assert_eq!(split_page["items"][0]["content"], json!(""));
    assert_eq!(split_page["next_after"], json!(5));

    let detail = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let detail = response_json(detail).await;
    assert_eq!(detail["exit_code"], json!(2));
    assert_eq!(detail["target_runs"][0]["logs_available"], json!(true));
    assert_eq!(detail["target_runs"][0]["last_log_sequence"], json!(8));

    let invalid_limit = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/logs?limit=201"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(invalid_limit.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let denied = json_request(
        app,
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/logs"),
        json!({}),
        &[("authorization", &bearer(&other_token))],
    )
    .await;
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn external_deployment_diagnostics_explain_pre_start_failure_without_leaking_task_data() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    seed_application(&pool, "app_other", "Other").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "诊断 Key", &["app_deploy"]).await;
    let other_token = create_key(&app, &cookie, &csrf, "其他诊断 Key", &["app_other"]).await;
    let auth = bearer(&token);
    let created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{}}),
        &[
            ("authorization", &auth),
            ("idempotency-key", "diagnostics-0002"),
        ],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let deployment_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    sqlx::query("UPDATE deployments SET status='failed',phase='failed',result_summary='任务失败',exit_code=1,finished_at=datetime('now') WHERE id=?")
        .bind(&deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at,acknowledged_at,finished_at,result_json) VALUES('task_pre_start','agent_deploy',?,'deployment_execute','diag-1','sha256:diag','{}','failed','2099-08-06T00:00:00Z',datetime('now'),datetime('now'),?)")
        .bind(&deployment_id)
        .bind(json!({"error_code":"deploy_event_protocol_conflict","status":"failed","exit_code":1,"summary":"failed at /var/lib/deploy-go with AWS_SECRET_ACCESS_KEY=aws-secret"}).to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_logs(deployment_id,task_id,sequence,task_sequence,stream,content) VALUES(?,?,?,?,?,?)")
        .bind(&deployment_id)
        .bind("task_pre_start")
        .bind(1_i64)
        .bind(1_i64)
        .bind("stderr")
        .bind("password=secret")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_task_events(task_id,sequence,kind,payload_json) VALUES(?,?,'progress',?)")
        .bind("task_pre_start")
        .bind(1_i64)
        .bind(json!({
            "event": "deploy.finished",
            "message": "step_out_of_order: api.remote.seed 重复结束，{\"private_key\":\"private-key-material\"}"
        }).to_string())
        .execute(&pool)
        .await
        .unwrap();

    let response = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/diagnostics"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let response_status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        response_status,
        StatusCode::OK,
        "diagnostics response: {body}"
    );
    assert_eq!(body["diagnostic"]["origin"], json!("agent_result"));
    assert_eq!(
        body["diagnostic"]["error_code"],
        json!("deploy_event_protocol_conflict")
    );
    assert_eq!(body["diagnostic"]["exit_code"], json!(1));
    assert_eq!(body["diagnostic"]["logs_available"], json!(true));
    assert_eq!(
        body["diagnostic"]["tasks"][0]["execution_phase"],
        json!("execute")
    );
    assert_eq!(
        body["diagnostic"]["tasks"][0]["task_started_at"],
        Value::Null
    );
    assert_eq!(
        body["diagnostic"]["tasks"][0]["summary"],
        json!("部署任务返回失败（详情已脱敏）")
    );
    assert_eq!(
        body["diagnostic"]["protocol_detail"],
        json!("部署任务返回失败（详情已脱敏）")
    );
    assert_eq!(
        body["diagnostic"]["tasks"][0]["protocol_detail"],
        json!("部署任务返回失败（详情已脱敏）")
    );
    assert!(body.to_string().contains("agent_result"));
    assert!(!body.to_string().contains("/var/lib/deploy-go"));
    assert!(!body.to_string().contains("secret"));

    let detail = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/deployments/{deployment_id}"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(
        response_json(detail).await["diagnostic"]["origin"],
        json!("agent_result")
    );

    let denied = json_request(
        app,
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/diagnostics"),
        json!({}),
        &[("authorization", &bearer(&other_token))],
    )
    .await;
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn external_deployment_diagnostics_classify_expired_delivery_as_timeout() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "超时诊断 Key", &["app_deploy"]).await;
    let auth = bearer(&token);
    let created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/deployments",
        json!({"parameters":{}}),
        &[
            ("authorization", &auth),
            ("idempotency-key", "diagnostics-timeout-0001"),
        ],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let deployment_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    sqlx::query("UPDATE deployments SET status='failed',phase='failed' WHERE id=?")
        .bind(&deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_timeout','agent_deploy',?,'deployment_execute','diag-timeout-1','sha256:timeout','{}','delivered','2000-01-01T00:00:00Z')")
        .bind(&deployment_id)
        .execute(&pool)
        .await
        .unwrap();

    let response = json_request(
        app,
        "GET",
        &format!("/external/v1/deployments/{deployment_id}/diagnostics"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let body = response_json(response).await;
    assert_eq!(body["diagnostic"]["origin"], json!("agent_timeout"));
    assert_eq!(
        body["diagnostic"]["tasks"][0]["origin"],
        json!("agent_timeout")
    );
}

#[tokio::test]
async fn external_key_cannot_create_production_deployments() {
    let (app, pool) = test_app().await;
    seed_production_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "正式环境 Key", &["app_prod"]).await;
    let auth = bearer(&token);

    let listed = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = response_json(listed).await;
    assert_eq!(listed["items"][0]["id"], json!("app_prod"));
    assert_eq!(listed["items"][0]["environment"], json!("prod"));

    for (idempotency_key, payload) in [
        ("external-prod-app-0001", json!({"parameters":{}})),
        (
            "external-prod-target-0001",
            json!({"parameters":{},"target_id":"target_prod"}),
        ),
    ] {
        let response = json_request(
            app.clone(),
            "POST",
            "/external/v1/applications/app_prod/deployments",
            payload,
            &[
                ("authorization", &auth),
                ("idempotency-key", idempotency_key),
            ],
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response_json(response).await;
        assert_eq!(
            body["code"],
            json!("external_production_deployment_forbidden")
        );
    }

    let deployment_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployments WHERE application_id='app_prod'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(deployment_count, 0);
}

#[tokio::test]
async fn external_key_cannot_deploy_application_with_production_target() {
    let (app, pool) = test_app().await;
    seed_non_production_application_with_production_target(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "目标环境 Key", &["app_drift"]).await;
    let auth = bearer(&token);

    for (idempotency_key, payload) in [
        ("external-drift-app-0001", json!({"parameters":{}})),
        (
            "external-drift-target-0001",
            json!({"parameters":{},"target_id":"target_drift"}),
        ),
    ] {
        let response = json_request(
            app.clone(),
            "POST",
            "/external/v1/applications/app_drift/deployments",
            payload,
            &[
                ("authorization", &auth),
                ("idempotency-key", idempotency_key),
            ],
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response_json(response).await;
        assert_eq!(
            body["code"],
            json!("external_production_deployment_forbidden")
        );
    }

    let deployment_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployments WHERE application_id='app_drift'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(deployment_count, 0);
}

#[tokio::test]
async fn external_key_creates_non_production_application_and_binds_it() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "One").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "创建 Key", &["app_one"]).await;
    let auth = bearer(&token);

    let created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications",
        json!({
            "name": "Clickhouse 测试",
            "slug": "clickhouse-test",
            "description": "由外部 API 创建",
            "environment": "test",
            "tags": ["clickhouse", "test"],
            "parameter_schema": {
                "type": "object",
                "properties": {"release": {"type": "string"}},
                "required": [],
                "additionalProperties": false
            }
        }),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let application_id = created["id"].as_str().unwrap().to_owned();
    assert!(application_id.starts_with("app_"));
    assert_eq!(created["name"], json!("Clickhouse 测试"));
    assert_eq!(created["slug"], json!("clickhouse-test"));
    assert_eq!(created["environment"], json!("test"));
    assert_eq!(created["status"], json!("active"));
    assert_eq!(created["app_type"], json!("binary"));
    assert_eq!(created["type_version"], json!("1"));
    assert_eq!(created["tags"], json!(["clickhouse", "test"]));
    assert_eq!(created["version"], json!(1));
    assert_eq!(created["targets"], json!([]));

    let (environment, parameter_schema, verification_config): (String, String, String) =
        sqlx::query_as(
            "SELECT environment,parameter_schema,verification_config FROM applications WHERE id=?",
        )
        .bind(&application_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(environment, "test");
    assert!(parameter_schema.contains("release"));
    assert!(verification_config.contains("/healthz"));

    // 创建方 Key 自动获得新应用访问权，无需管理面再次授权。
    let listed = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let listed = response_json(listed).await;
    let listed_ids = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect::<Vec<_>>();
    assert!(listed_ids.contains(&application_id.as_str()));
    assert!(listed_ids.contains(&"app_one"));

    let shown = json_request(
        app.clone(),
        "GET",
        &format!("/external/v1/applications/{application_id}"),
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(shown.status(), StatusCode::OK);

    let bound: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM external_api_key_applications WHERE application_id=?",
    )
    .bind(&application_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bound, 1);
    let granted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_application_grants WHERE application_id=? AND user_id='usr_external_api_service'",
    )
    .bind(&application_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(granted, 1);
    let audit_detail: String = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE action='application.create' AND resource_id=?",
    )
    .bind(&application_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(audit_detail.contains("external_api_key_id"));

    // 其他 Key 仍然看不到新应用。
    let other_token = create_key(&app, &cookie, &csrf, "其他 Key", &["app_one"]).await;
    let hidden = json_request(
        app,
        "GET",
        &format!("/external/v1/applications/{application_id}"),
        json!({}),
        &[("authorization", &bearer(&other_token))],
    )
    .await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn external_key_cannot_create_production_application() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "One").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "创建限制 Key", &["app_one"]).await;
    let auth = bearer(&token);

    let forbidden = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications",
        json!({"name": "正式应用", "slug": "prod-app", "environment": "prod"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_json(forbidden).await["code"],
        json!("external_production_environment_forbidden")
    );

    let invalid = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications",
        json!({"name": "非法环境", "slug": "invalid-app", "environment": "production"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let duplicated = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications",
        json!({"name": "重复 Slug", "slug": "one", "environment": "test"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(duplicated.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(duplicated).await["code"],
        json!("application_slug_exists")
    );

    let created: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM applications WHERE slug IN ('prod-app','invalid-app')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(created, 0);
}

#[tokio::test]
async fn external_key_updates_non_production_application() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "编辑 Key", &["app_deploy"]).await;
    let auth = bearer(&token);

    let shown = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_deploy",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(shown.status(), StatusCode::OK);
    let shown = response_json(shown).await;
    let version = shown["version"].as_i64().unwrap();
    assert_eq!(shown["app_type"], json!("binary"));
    assert_eq!(shown["tags"], json!([]));

    let updated = json_request(
        app.clone(),
        "PATCH",
        "/external/v1/applications/app_deploy",
        json!({
            "version": version,
            "name": "Deploy App 2",
            "description": "updated",
            "environment": "staging",
            "tags": ["alpha", "beta"],
            "parameter_schema": {
                "type": "object",
                "properties": {"release-version": {"type": "string"}},
                "required": [],
                "additionalProperties": false
            },
            "verification_config": {
                "type": "http",
                "path": "/readyz",
                "expected_status": 200,
                "timeout_ms": 3000
            }
        }),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["name"], json!("Deploy App 2"));
    assert_eq!(updated["description"], json!("updated"));
    assert_eq!(updated["environment"], json!("staging"));
    assert_eq!(updated["tags"], json!(["alpha", "beta"]));
    assert_eq!(updated["version"], json!(version + 1));
    assert_eq!(updated["targets"][0]["environment"], json!("staging"));

    let (name, environment, parameter_schema, verification_config, version_after): (
        String,
        String,
        String,
        String,
        i64,
    ) = sqlx::query_as(
        "SELECT display_name,environment,parameter_schema,verification_config,version FROM applications WHERE id='app_deploy'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(name, "Deploy App 2");
    assert_eq!(environment, "staging");
    assert_eq!(version_after, version + 1);
    assert!(parameter_schema.contains("release-version"));
    assert!(verification_config.contains("/readyz"));

    let stale = json_request(
        app,
        "PATCH",
        "/external/v1/applications/app_deploy",
        json!({"version": version, "description": "stale"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn external_key_cannot_update_production_application() {
    let (app, pool) = test_app().await;
    seed_production_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "正式编辑 Key", &["app_prod"]).await;
    let auth = bearer(&token);

    let shown = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_prod",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let version = response_json(shown).await["version"].as_i64().unwrap();

    let denied = json_request(
        app.clone(),
        "PATCH",
        "/external/v1/applications/app_prod",
        json!({"version": version, "description": "denied"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let denied = response_json(denied).await;
    assert_eq!(
        denied["code"],
        json!("external_production_application_forbidden")
    );

    let description: String =
        sqlx::query_scalar("SELECT description FROM applications WHERE id='app_prod'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(description, "");
}

#[tokio::test]
async fn external_key_cannot_promote_application_to_production() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "提级 Key", &["app_deploy"]).await;
    let auth = bearer(&token);

    let shown = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_deploy",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let version = response_json(shown).await["version"].as_i64().unwrap();

    let denied = json_request(
        app,
        "PATCH",
        "/external/v1/applications/app_deploy",
        json!({"version": version, "environment": "prod"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let denied = response_json(denied).await;
    assert_eq!(
        denied["code"],
        json!("external_production_environment_forbidden")
    );

    let environment: String =
        sqlx::query_scalar("SELECT environment FROM applications WHERE id='app_deploy'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(environment, "test");
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
async fn external_key_manages_env_files_without_revealing_plaintext() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "Env Key", &["app_deploy"]).await;
    let auth = bearer(&token);

    let registered = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/env-files",
        json!({"files":[{"file_name":"api.env","module":"api","format":"dotenv-v1","content":"API_BASE=https://example.internal\n"}]}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(registered.status(), StatusCode::OK);
    let registered = response_json(registered).await;
    assert_eq!(registered["created"], json!(["api.env"]));

    let listed = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_deploy/env-files",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = response_json(listed).await;
    assert_eq!(listed["items"][0]["file_name"], json!("api.env"));
    assert_eq!(listed["items"][0]["current_version"], json!(1));
    assert!(listed["items"][0].get("content").is_none());
    let env_file_id = listed["items"][0]["id"].as_str().unwrap().to_owned();
    let version = listed["items"][0]["version"].as_i64().unwrap();

    let updated = json_request(
        app.clone(),
        "PUT",
        &format!("/external/v1/applications/app_deploy/env-files/{env_file_id}"),
        json!({"content":"API_BASE=https://example.internal\nAPI_MODE=fast\n","expected_version":version}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["current_version"], json!(2));
    assert_eq!(updated["version"], json!(version + 1));
    assert!(updated.get("content").is_none());
    let digest_after = updated["current_digest"].as_str().unwrap().to_owned();

    let stored: (String, i64) = sqlx::query_as(
        "SELECT current_digest,current_version FROM application_env_files WHERE id=?",
    )
    .bind(&env_file_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored, (digest_after, 2));
    let plaintext_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM application_env_versions WHERE env_file_id=? AND CAST(ciphertext AS TEXT) LIKE '%API_MODE=fast%'",
    )
    .bind(&env_file_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(plaintext_rows, 0);

    let deleted = json_request(
        app.clone(),
        "DELETE",
        &format!("/external/v1/applications/app_deploy/env-files/{env_file_id}"),
        json!({"expected_version": version + 1, "confirm_file_name": "api.env"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let after_delete = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_deploy/env-files",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let after_delete = response_json(after_delete).await;
    assert_eq!(after_delete["items"], json!([]));

    let re_registered = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/env-files",
        json!({"files":[{"file_name":"api.env","module":"api","format":"dotenv-v1","content":"API_BASE=https://restored.internal\n"}]}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(re_registered.status(), StatusCode::OK);
    assert_eq!(
        response_json(re_registered).await["created"],
        json!(["api.env"])
    );
    let restored: (i64, Option<String>) =
        sqlx::query_as("SELECT current_version,deleted_at FROM application_env_files WHERE id=?")
            .bind(&env_file_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(restored, (4, None));
}

#[tokio::test]
async fn external_key_manages_deployment_targets_for_non_production_application() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    seed_extra_node(&pool, "node_deploy2", "agent_deploy2").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "目标 Key", &["app_deploy"]).await;
    let auth = bearer(&token);

    let created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_deploy/targets",
        json!({
            "node_id": "node_deploy2",
            "target_code": "test-secondary",
            "script_path": "/srv/apps/deploy-secondary.sh",
            "timeout_seconds": 120
        }),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let target_id = created["id"].as_str().unwrap().to_owned();
    let version = created["version"].as_i64().unwrap();
    assert_eq!(created["status"], json!("active"));
    assert_eq!(created["timeout_seconds"], json!(120));
    assert_eq!(created["environment"], json!("test"));

    let updated = json_request(
        app.clone(),
        "PATCH",
        &format!("/external/v1/deployment-targets/{target_id}"),
        json!({
            "node_id": "node_deploy2",
            "target_code": "test-secondary",
            "script_path": "/srv/apps/deploy-secondary.sh",
            "timeout_seconds": 240,
            "version": version
        }),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["timeout_seconds"], json!(240));
    assert_eq!(updated["version"], json!(version + 1));

    let disabled = json_request(
        app.clone(),
        "PUT",
        &format!("/external/v1/deployment-targets/{target_id}/status"),
        json!({"status": "disabled", "version": version + 1}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);
    let disabled = response_json(disabled).await;
    assert_eq!(disabled["status"], json!("disabled"));

    let listed = json_request(
        app,
        "GET",
        "/external/v1/applications/app_deploy/targets",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    let listed = response_json(listed).await;
    assert_eq!(listed["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn external_key_manages_workspace_source_for_non_production_application() {
    let (app, pool) = test_app().await;
    seed_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "工作区 Key", &["app_deploy"]).await;
    let auth = bearer(&token);

    let missing = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_deploy/workspace-source",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    let saved = json_request(
        app.clone(),
        "PUT",
        "/external/v1/applications/app_deploy/workspace-source",
        json!({"build_agent_id": "agent_deploy", "workspace_path": "/srv/workspace"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(saved.status(), StatusCode::CREATED);
    let saved = response_json(saved).await;
    assert_eq!(saved["workspace_path"], json!("/srv/workspace"));
    assert_eq!(saved["build_agent_id"], json!("agent_deploy"));
    let version = saved["version"].as_i64().unwrap();

    let shown = json_request(
        app.clone(),
        "GET",
        "/external/v1/applications/app_deploy/workspace-source",
        json!({}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(shown.status(), StatusCode::OK);
    let shown = response_json(shown).await;
    assert_eq!(shown["workspace_version"], json!(1));

    let updated = json_request(
        app,
        "PUT",
        "/external/v1/applications/app_deploy/workspace-source",
        json!({
            "build_agent_id": "agent_deploy",
            "workspace_path": "/srv/workspace-2",
            "version": version
        }),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["workspace_path"], json!("/srv/workspace-2"));
    assert_eq!(updated["workspace_version"], json!(2));
}

#[tokio::test]
async fn external_key_cannot_configure_production_application() {
    let (app, pool) = test_app().await;
    seed_production_deployable_application(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let token = create_key(&app, &cookie, &csrf, "正式配置 Key", &["app_prod"]).await;
    let auth = bearer(&token);

    let env_denied = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_prod/env-files",
        json!({"files":[{"file_name":"prod.env","module":"api","format":"dotenv-v1","content":"A=1\n"}]}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(env_denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_json(env_denied).await["code"],
        json!("external_production_application_forbidden")
    );

    let target_denied = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_prod/targets",
        json!({
            "node_id": "node_prod",
            "script_path": "/srv/deploy.sh",
            "timeout_seconds": 60
        }),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(target_denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_json(target_denied).await["code"],
        json!("external_production_application_forbidden")
    );

    let target_status_denied = json_request(
        app.clone(),
        "PUT",
        "/external/v1/deployment-targets/target_prod/status",
        json!({"status": "disabled", "version": 1}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(target_status_denied.status(), StatusCode::FORBIDDEN);

    let workspace_denied = json_request(
        app,
        "PUT",
        "/external/v1/applications/app_prod/workspace-source",
        json!({"build_agent_id": "agent_prod", "workspace_path": "/srv/workspace"}),
        &[("authorization", &auth)],
    )
    .await;
    assert_eq!(workspace_denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_json(workspace_denied).await["code"],
        json!("external_production_application_forbidden")
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
            "/external/v1/applications/{id}/env-files",
            "/external/v1/applications/{id}/env-files/{env_file_id}",
            "/external/v1/applications/{id}/targets",
            "/external/v1/applications/{id}/workspace-source",
            "/external/v1/deployment-targets/{target_id}",
            "/external/v1/deployment-targets/{target_id}/status",
            "/external/v1/deployments/{id}",
            "/external/v1/deployments/{id}/cancel",
            "/external/v1/deployments/{id}/diagnostics",
            "/external/v1/deployments/{id}/logs",
        ]
    );
}
