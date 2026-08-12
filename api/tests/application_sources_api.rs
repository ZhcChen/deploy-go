mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{admin_session, json_request, response_json, test_app};
use serde_json::{Value, json};

async fn build_agent_fixture(pool: &sqlx::SqlitePool, agent_id: &str) {
    sqlx::query(
        "INSERT INTO nodes (id, name, work_root, secrets_root, status) VALUES ('node_build', 'build-node', '/var/lib/deploy-go-agent/apps', '/var/lib/deploy-go-agent/secrets', 'online')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO agents (id, node_id, registered_at, last_seen_at, agent_version, protocol_version, connection_generation) VALUES (?, 'node_build', '2026-08-06T00:00:00Z', '2026-08-06T00:00:00Z', '0.1.0', 2, 1)")
        .bind(agent_id)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn application_source_save_refresh_and_branch_lifecycle() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({"name":"Example API","slug":"example-api","description":"Example","environment":"prod"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let application_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let credential = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/git-credentials",
            json!({"name":"Deploy Key"}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let credential_id = credential["id"].as_str().unwrap().to_owned();
    build_agent_fixture(&pool, "agent_build").await;

    let missing_source = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({}),
        &[("cookie", &admin_cookie)],
    )
    .await;
    assert_eq!(missing_source.status(), StatusCode::NOT_FOUND);

    let invalid_url = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({
            "repository_url": "https://user:pass@git.example.test/deploy-go/example.git",
            "git_credential_id": credential_id,
            "build_agent_id": "agent_build"
        }),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(invalid_url.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let saved = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({
            "repository_url": "git@git.example.test:deploy-go/example.git",
            "git_credential_id": credential_id,
            "build_agent_id": "agent_build",
            "source_policy": "branch"
        }),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(saved.status(), StatusCode::CREATED);
    let source = response_json(saved).await;
    assert_eq!(source["status"], "draft");
    assert!(source["deployment_branch"].is_null());
    assert_eq!(source["git_credential_id"], credential_id);
    assert_eq!(source["build_agent_id"], "agent_build");
    let version = source["version"].as_i64().unwrap();
    let serialized = source.to_string();
    for forbidden in ["PRIVATE KEY", "encrypted_private_key", "user:pass"] {
        assert!(!serialized.contains(forbidden));
    }

    let refresh = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/source/refreshes"),
        json!({}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let refresh_status = refresh.status();
    let discovery = response_json(refresh).await;
    if refresh_status != StatusCode::ACCEPTED {
        let tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        let discoveries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM git_ref_discoveries")
            .fetch_one(&pool)
            .await
            .unwrap();
        let leases: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM git_secret_leases")
            .fetch_one(&pool)
            .await
            .unwrap();
        panic!(
            "刷新 refs 失败: {discovery} tasks={tasks} discoveries={discoveries} leases={leases}"
        );
    }
    let discovery_id = discovery["id"].as_str().unwrap().to_owned();
    let task_id = discovery["task_id"].as_str().unwrap().to_owned();
    assert_eq!(discovery["status"], "queued");
    assert_eq!(discovery["source_version"], 1);

    let (kind, payload_json): (String, String) =
        sqlx::query_as("SELECT kind, payload_json FROM agent_tasks WHERE id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(kind, "git_refs_query");
    let payload: Value = serde_json::from_str(&payload_json).unwrap();
    assert_eq!(payload["kind"], "git_refs_query");
    assert_eq!(
        payload["payload"]["repository_url"],
        "git@git.example.test:deploy-go/example.git"
    );
    assert!(
        payload["payload"]["git_credential_lease_id"]
            .as_str()
            .is_some()
    );
    assert!(!payload_json.contains("PRIVATE KEY"));
    let lease_status: String =
        sqlx::query_scalar("SELECT status FROM git_secret_leases WHERE task_id=?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(lease_status, "issued");

    let duplicate = response_json(
        json_request(
            app.clone(),
            "POST",
            &format!("/api/v1/applications/{application_id}/source/refreshes"),
            json!({}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(duplicate["id"], discovery_id);

    let branch_before_result = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/source/branch"),
        json!({"branch":"main","version":version}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(branch_before_result.status(), StatusCode::CONFLICT);

    let future = (Utc::now() + Duration::minutes(30)).to_rfc3339();
    sqlx::query("UPDATE git_ref_discoveries SET status='succeeded',refs_json=?,expires_at=?,finished_at=? WHERE id=?")
        .bind(json!([
            {"name":"main","ref":"refs/heads/main","sha":"0123456789abcdef0123456789abcdef01234567"},
            {"name":"develop","ref":"refs/heads/develop","sha":"1123456789abcdef0123456789abcdef01234567"}
        ]).to_string())
        .bind(&future)
        .bind(&future)
        .bind(&discovery_id)
        .execute(&pool)
        .await
        .unwrap();

    let missing_branch = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/source/branch"),
        json!({"branch":"release/1.x","version":version}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(missing_branch.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(missing_branch).await["code"],
        "git_branch_not_found"
    );

    let fixed = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/source/branch"),
        json!({"branch":"main","version":version}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(fixed.status(), StatusCode::OK);
    let fixed = response_json(fixed).await;
    assert_eq!(fixed["status"], "verified");
    assert_eq!(fixed["deployment_branch"], "main");
    assert!(fixed["branch_verified_at"].is_string());
    assert_eq!(fixed["version"], 2);

    let shown = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/applications/{application_id}/source"),
            json!({}),
            &[("cookie", &admin_cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(shown["deployment_branch"], "main");

    let changed = json_request(
        app,
        "PUT",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({
            "repository_url": "git@git.example.test:deploy-go/other.git",
            "git_credential_id": null,
            "build_agent_id": "agent_build",
            "version": 2
        }),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(changed.status(), StatusCode::OK);
    let changed = response_json(changed).await;
    assert_eq!(changed["status"], "draft");
    assert!(changed["deployment_branch"].is_null());
    assert!(changed["git_credential_id"].is_null());
    assert_eq!(changed["version"], 3);
}

#[tokio::test]
async fn application_source_is_readable_by_granted_user_but_not_editable() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({"name":"Read App","slug":"read-app","description":"","environment":"prod"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let application_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
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
    let user_id = user["id"].as_str().unwrap().to_owned();
    let (user_cookie, _) = common::login(app.clone(), "operator", "operator-password-long").await;
    json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/users/{user_id}/applications/{application_id}"),
        json!({}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    build_agent_fixture(&pool, "agent_build").await;
    json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({
            "repository_url": "git@git.example.test:deploy-go/read-only.git",
            "git_credential_id": null,
            "build_agent_id": "agent_build"
        }),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;

    let read = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(read.status(), StatusCode::OK);
    let edit = json_request(
        app,
        "PUT",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({
            "repository_url": "git@git.example.test:deploy-go/other.git",
            "git_credential_id": null,
            "build_agent_id": "agent_build"
        }),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(edit.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn offline_agent_and_archived_dependencies_block_source_mutations() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({"name":"Blocked App","slug":"blocked-app","description":"","environment":"prod"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let application_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let credential = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/git-credentials",
            json!({"name":"Blocked Key"}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let credential_id = credential["id"].as_str().unwrap().to_owned();
    build_agent_fixture(&pool, "agent_build").await;
    sqlx::query("UPDATE nodes SET status='offline' WHERE id='node_build'")
        .execute(&pool)
        .await
        .unwrap();
    let offline = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({
            "repository_url": "git@git.example.test:deploy-go/example.git",
            "git_credential_id": null,
            "build_agent_id": "agent_build"
        }),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(offline.status(), StatusCode::CONFLICT);
    assert_eq!(response_json(offline).await["code"], "agent_offline");

    sqlx::query("UPDATE nodes SET status='online' WHERE id='node_build'")
        .execute(&pool)
        .await
        .unwrap();
    json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/source"),
        json!({
            "repository_url": "git@git.example.test:deploy-go/example.git",
            "git_credential_id": credential_id,
            "build_agent_id": "agent_build"
        }),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let archived_credential = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/git-credentials/{credential_id}/status"),
        json!({"status":"archived","version":1}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(archived_credential.status(), StatusCode::OK);
    let refresh = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/source/refreshes"),
        json!({}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(refresh.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(refresh).await["code"],
        "git_credential_unavailable"
    );

    let app_row: (i64,) = sqlx::query_as("SELECT version FROM applications WHERE id=?")
        .bind(&application_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let archived_app = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/status"),
        json!({"status":"archived","version":app_row.0}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(archived_app.status(), StatusCode::OK);
    let refresh_after_archive = json_request(
        app,
        "POST",
        &format!("/api/v1/applications/{application_id}/source/refreshes"),
        json!({}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(refresh_after_archive.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(refresh_after_archive).await["code"],
        "application_not_active"
    );
}
