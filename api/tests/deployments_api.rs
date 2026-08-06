mod common;

use axum::body::to_bytes;
use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;

async fn fixture(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_deploy','Deploy App','deploy-app','active')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_deploy','Deploy Node','/srv/apps','/srv/secrets','online')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version) VALUES('agent_deploy','node_deploy','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.1.0',1)").execute(pool).await.unwrap();
    let schema = json!({"type":"object","properties":{"release-version":{"type":"string","maxLength":32}},"required":["release-version"],"additionalProperties":false});
    let verification =
        json!({"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000});
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,parameter_schema,timeout_seconds,verification_config,status) VALUES('target_deploy','app_deploy','node_deploy','production','/srv/apps/deploy.sh',?,900,?,'active')")
        .bind(schema.to_string()).bind(verification.to_string()).execute(pool).await.unwrap();
}

async fn preview(app: axum::Router, cookie: &str) -> serde_json::Value {
    let response = json_request(
        app,
        "POST",
        "/api/v1/deployment-targets/target_deploy/deployment-preview",
        json!({"parameters":{"release-version":"1.0.0"}}),
        &[("cookie", cookie)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

#[tokio::test]
async fn preview_requires_an_online_active_agent_without_ssh_credentials() {
    let (app, pool) = test_app().await;
    fixture(&pool).await;
    let (cookie, _) = admin_session(app.clone()).await;
    preview(app.clone(), &cookie).await;

    sqlx::query("UPDATE agents SET revoked_at='2026-08-03T00:00:00Z' WHERE id='agent_deploy'")
        .execute(&pool)
        .await
        .unwrap();
    let unavailable = json_request(
        app,
        "POST",
        "/api/v1/deployment-targets/target_deploy/deployment-preview",
        json!({"parameters":{"release-version":"1.0.0"}}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(unavailable.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn confirm_is_idempotent_and_rejects_changed_content() {
    let (app, pool) = test_app().await;
    fixture(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let preview = preview(app.clone(), &cookie).await;
    let body =
        json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":preview["snapshot_hash"]});
    let headers = [
        ("cookie", cookie.as_str()),
        ("x-csrf-token", csrf.as_str()),
        ("idempotency-key", "deploy-request-0001"),
    ];
    let first = json_request(
        app.clone(),
        "POST",
        "/api/v1/deployment-targets/target_deploy/deployments",
        body.clone(),
        &headers,
    )
    .await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = response_json(first).await;
    let second = json_request(
        app.clone(),
        "POST",
        "/api/v1/deployment-targets/target_deploy/deployments",
        body,
        &headers,
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(response_json(second).await["id"], first["id"]);
    let conflict = json_request(
        app,
        "POST",
        "/api/v1/deployment-targets/target_deploy/deployments",
        json!({"parameters":{"release-version":"2.0.0"},"snapshot_hash":preview["snapshot_hash"]}),
        &headers,
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn stale_snapshot_and_ungranted_target_are_hidden() {
    let (app, pool) = test_app().await;
    fixture(&pool).await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let old = preview(app.clone(), &admin_cookie).await;
    sqlx::query("UPDATE deployment_targets SET version=version+1 WHERE id='target_deploy'")
        .execute(&pool)
        .await
        .unwrap();
    let stale = json_request(
        app.clone(),
        "POST",
        "/api/v1/deployment-targets/target_deploy/deployments",
        json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":old["snapshot_hash"]}),
        &[
            ("cookie", &admin_cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "deploy-request-0002"),
        ],
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    let user = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/users",
            json!({"username":"operator","password":"operator-password-long"}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let user_id = user["id"].as_str().unwrap();
    let (user_cookie, user_csrf) =
        common::login(app.clone(), "operator", "operator-password-long").await;
    let hidden = json_request(
        app.clone(),
        "POST",
        "/api/v1/deployment-targets/target_deploy/deployment-preview",
        json!({"parameters":{"release-version":"1.0.0"}}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
    json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/users/{user_id}/applications/app_deploy"),
        json!({}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let granted = preview(app.clone(), &user_cookie).await;
    let created = json_request(
        app,
        "POST",
        "/api/v1/deployment-targets/target_deploy/deployments",
        json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":granted["snapshot_hash"]}),
        &[
            ("cookie", &user_cookie),
            ("x-csrf-token", &user_csrf),
            ("idempotency-key", "deploy-request-0003"),
        ],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn queued_cancel_retry_and_log_resume_form_a_closed_loop() {
    let (app, pool) = test_app().await;
    fixture(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let preview = preview(app.clone(), &cookie).await;
    let created = response_json(json_request(app.clone(), "POST", "/api/v1/deployment-targets/target_deploy/deployments", json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":preview["snapshot_hash"]}), &[("cookie",&cookie),("x-csrf-token",&csrf),("idempotency-key","deploy-request-cancel-01")]).await).await;
    let id = created["id"].as_str().unwrap();
    let canceled = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/deployments/{id}/cancel"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(canceled.status(), StatusCode::OK);
    assert_eq!(response_json(canceled).await["status"], "canceled");
    sqlx::query("INSERT INTO deployment_logs(deployment_id,task_id,sequence,task_sequence,stream,content) VALUES(?,NULL,1,1,'stdout','first'),(?,NULL,2,2,'stderr','second')").bind(id).bind(id).execute(&pool).await.unwrap();
    let logs = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/deployments/{id}/logs?after=1"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(logs.status(), StatusCode::OK);
    let body = String::from_utf8(
        to_bytes(logs.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!body.contains("first"));
    assert!(body.contains("second"));
    assert!(body.contains("id: 2"));
    assert!(body.contains("event: terminal"));
    let retried = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/deployments/{id}/retry"),
        json!({}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "deploy-request-cancel-01"),
        ],
    )
    .await;
    assert_eq!(retried.status(), StatusCode::CREATED);
    let retried = response_json(retried).await;
    assert_eq!(retried["retry_of_id"], id);
    assert_ne!(retried["id"], id);
    let duplicate = json_request(
        app,
        "POST",
        &format!("/api/v1/deployments/{id}/retry"),
        json!({}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "deploy-request-cancel-01"),
        ],
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::OK);
    assert_eq!(response_json(duplicate).await["id"], retried["id"]);
}

#[tokio::test]
async fn deployment_list_uses_stable_cursor_pagination() {
    let (app, pool) = test_app().await;
    fixture(&pool).await;
    let (cookie, _) = admin_session(app.clone()).await;
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('requester','requester','hash','user','active')").execute(&pool).await.unwrap();
    for (id, created) in [
        ("d3", "2026-07-31T03:00:00Z"),
        ("d2", "2026-07-31T02:00:00Z"),
        ("d1", "2026-07-31T01:00:00Z"),
    ] {
        sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,created_at) VALUES(?,'target_deploy','requester','queued','queued',?,?, 'snapshot',?)").bind(id).bind(format!("page-key-{id}" )).bind(id).bind(created).execute(&pool).await.unwrap();
    }
    let first = json_request(
        app.clone(),
        "GET",
        "/api/v1/deployments?limit=2",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first = response_json(first).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 2);
    assert_eq!(first["items"][0]["id"], "d3");
    let cursor = first["next_cursor"].as_str().unwrap();
    let second = json_request(
        app,
        "GET",
        &format!("/api/v1/deployments?limit=2&after={cursor}"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let second = response_json(second).await;
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert_eq!(second["items"][0]["id"], "d1");
    assert!(second["next_cursor"].is_null());
}

async fn add_second_target(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_deploy_2','Deploy Node 2','/srv/apps','/srv/secrets','offline')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version) VALUES('agent_deploy_2','node_deploy_2','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.1.0',1)").execute(pool).await.unwrap();
    let schema = json!({"type":"object","properties":{"release-version":{"type":"string","maxLength":32}},"required":["release-version"],"additionalProperties":false});
    let verification =
        json!({"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000});
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,parameter_schema,timeout_seconds,verification_config,status) VALUES('target_deploy_2','app_deploy','node_deploy_2','production','/srv/apps/deploy.sh',?,900,?,'active')")
        .bind(schema.to_string()).bind(verification.to_string()).execute(pool).await.unwrap();
}

#[tokio::test]
async fn application_preview_requires_targets_and_includes_offline_agents() {
    let (app, pool) = test_app().await;
    fixture(&pool).await;
    add_second_target(&pool).await;
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES('env_preview','app_deploy','api.env','api','dotenv-v1',1,'digest-preview')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('env_version_preview','env_preview',1,'chacha20poly1305-application-env-v1',X'01',X'02',1,'digest-preview')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status,actual_version) VALUES('sync_preview_1','env_version_preview','target_deploy','node_deploy','agent_deploy','succeeded',1),('sync_preview_2','env_version_preview','target_deploy_2','node_deploy_2','agent_deploy_2','failed',NULL)").execute(&pool).await.unwrap();
    let (cookie, _) = admin_session(app.clone()).await;
    let response = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_deploy/deployment-preview",
        json!({"parameters":{"release-version":"1.0.0"}}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let preview = response_json(response).await;
    assert_eq!(preview["targets"].as_array().unwrap().len(), 2);
    assert_eq!(preview["targets"][0]["target_id"], "target_deploy");
    assert_eq!(preview["targets"][0]["env_gate_status"], "ready");
    assert_eq!(preview["targets"][1]["target_id"], "target_deploy_2");
    assert_eq!(preview["targets"][1]["agent_online"], false);
    assert_eq!(preview["targets"][1]["env_gate_status"], "failed");

    sqlx::query("UPDATE agents SET revoked_at='2026-08-03T01:00:00Z' WHERE id='agent_deploy_2'")
        .execute(&pool)
        .await
        .unwrap();
    let missing_agent = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_deploy/deployment-preview",
        json!({"parameters":{"release-version":"1.0.0"}}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_agent.status(), StatusCode::CONFLICT);

    sqlx::query(
        "UPDATE deployment_targets SET status='disabled' WHERE application_id='app_deploy'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let empty = json_request(
        app,
        "POST",
        "/api/v1/applications/app_deploy/deployment-preview",
        json!({"parameters":{"release-version":"1.0.0"}}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(empty.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn application_confirm_freezes_runs_and_aggregates_their_status() {
    let (app, pool) = test_app().await;
    fixture(&pool).await;
    add_second_target(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let preview = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/applications/app_deploy/deployment-preview",
            json!({"parameters":{"release-version":"1.0.0"}}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_deploy/deployments",
        json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":preview["snapshot_hash"]}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "application-deploy-0001"),
        ],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let deployment_id = created["id"].as_str().unwrap();
    assert_eq!(created["application_id"], "app_deploy");
    assert_eq!(created["target_runs"].as_array().unwrap().len(), 2);
    assert_eq!(created["status"], "queued");
    assert_eq!(created["phase"], "targets_pending");

    sqlx::query("UPDATE deployment_targets SET status='disabled' WHERE id='target_deploy_2'")
        .execute(&pool)
        .await
        .unwrap();
    let replay = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_deploy/deployments",
        json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":preview["snapshot_hash"]}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "application-deploy-0001"),
        ],
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(response_json(replay).await["id"], deployment_id);
    sqlx::query("UPDATE deployment_target_runs SET status='succeeded',phase='succeeded' WHERE deployment_id=? AND target_id='target_deploy'")
        .bind(deployment_id).execute(&pool).await.unwrap();
    sqlx::query("UPDATE deployment_target_runs SET status='failed',phase='release',error_code='release_failed' WHERE deployment_id=? AND target_id='target_deploy_2'")
        .bind(deployment_id).execute(&pool).await.unwrap();
    let shown = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/deployments/{deployment_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(shown["status"], "failed");
    assert_eq!(shown["phase"], "targets_failed");
    assert_eq!(shown["target_runs"].as_array().unwrap().len(), 2);

    let artifact_digest = "a".repeat(64);
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_digest,total_size,file_count,storage_key,status,upload_offset,upload_size,archive_digest,expires_at,verified_at) VALUES('artifact_retry',?,'manifest',1,1,?,'verified',10,10,?,'2099-01-01T00:00:00Z','2026-01-01T00:00:00Z')")
        .bind(deployment_id).bind(&artifact_digest).bind(&artifact_digest).execute(&pool).await.unwrap();

    let retried = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/deployments/{deployment_id}/retry"),
        json!({}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "application-retry-0001"),
        ],
    )
    .await;
    assert_eq!(retried.status(), StatusCode::CREATED);
    let retried = response_json(retried).await;
    let retried_runs = retried["target_runs"].as_array().unwrap();
    let reused = retried_runs
        .iter()
        .find(|run| run["target_id"] == "target_deploy")
        .unwrap();
    let pending = retried_runs
        .iter()
        .find(|run| run["target_id"] == "target_deploy_2")
        .unwrap();
    assert_eq!(reused["status"], "reused");
    assert_eq!(pending["status"], "pending");
    let pinned_artifact: Option<String> =
        sqlx::query_scalar("SELECT artifact_id FROM deployment_target_runs WHERE id=?")
            .bind(pending["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pinned_artifact.as_deref(), Some("artifact_retry"));
    let original_run = shown["target_runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["target_id"] == "target_deploy")
        .unwrap();
    assert_eq!(reused["source_run_id"], original_run["id"]);

    sqlx::query("UPDATE deployment_target_runs SET status='failed' WHERE deployment_id=? AND target_id='target_deploy'")
        .bind(deployment_id).execute(&pool).await.unwrap();
    let replay = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/deployments/{deployment_id}/retry"),
        json!({}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "application-retry-0001"),
        ],
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(response_json(replay).await["id"], retried["id"]);

    sqlx::query("UPDATE deployment_artifacts SET expires_at='2000-01-01T00:00:00Z' WHERE id='artifact_retry'")
        .execute(&pool).await.unwrap();
    let expired_retry = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/deployments/{deployment_id}/retry"),
        json!({}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "application-retry-expired"),
        ],
    )
    .await;
    assert_eq!(expired_retry.status(), StatusCode::CONFLICT);

    let retried_id = retried["id"].as_str().unwrap();
    let canceled = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/deployments/{retried_id}/cancel"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(canceled.status(), StatusCode::OK);
    assert_eq!(response_json(canceled).await["status"], "canceled");

    let task_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks WHERE deployment_id=?")
            .bind(deployment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task_count, 0);
}
