mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::{Value, json};
use sqlx::SqlitePool;

async fn setup(app: axum::Router, pool: &SqlitePool) -> (String, String, String, String) {
    let (cookie, csrf) = admin_session(app).await;
    sqlx::query("INSERT INTO applications(id,name,slug,app_type,type_version,environment,status) VALUES('app_runtime','Redis 7','redis-7','redis','7','prod','active')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes(id,name,host,port,username,work_root,secrets_root,status) VALUES('node_runtime','Redis Node','node.test',22,'deploy','/srv/apps','/srv/secrets','online')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_runtime','node_runtime','2026-08-12T00:00:00Z','2026-08-12T00:00:00Z','0.2.0',9,'[\"pty_terminal\",\"privileged_release\",\"runtime_status_probe\"]')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,target_code,environment,script_path,timeout_seconds,status) VALUES('target_runtime','app_runtime','node_runtime','shared-prod-redis','prod','/unused',60,'active')")
        .execute(pool)
        .await
        .unwrap();
    (
        "app_runtime".to_owned(),
        "target_runtime".to_owned(),
        cookie,
        csrf,
    )
}

#[tokio::test]
async fn runtime_status_probe_creates_v9_task_and_survives_reprobe() {
    let (app, pool) = test_app().await;
    let (application_id, target_id, cookie, csrf) = setup(app.clone(), &pool).await;

    let created = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/runtime-status"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::ACCEPTED);
    let status = response_json(created).await;
    assert_eq!(status["status"], "pending");
    assert_eq!(status["target_id"], target_id);
    assert_eq!(status["target_code"], "shared-prod-redis");
    let runtime_status_id = status["runtime_status_id"].as_str().unwrap().to_owned();

    let task: (String, String, i64) = sqlx::query_as(
        "SELECT kind,runtime_status_id,protocol_version FROM agent_tasks task JOIN agents agent ON agent.id=task.agent_id WHERE task.runtime_status_id=?",
    )
    .bind(&runtime_status_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(task.0, "system_inspect");
    assert_eq!(task.1, runtime_status_id);
    assert_eq!(task.2, 9);
    let payload: String =
        sqlx::query_scalar("SELECT payload_json FROM agent_tasks WHERE runtime_status_id=?")
            .bind(&runtime_status_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let payload: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["kind"], "runtime_status_probe");
    assert_eq!(payload["payload"]["app_type"], "redis");
    assert_eq!(payload["payload"]["target_code"], "shared-prod-redis");
    assert!(payload["payload"].get("command").is_none());
    assert!(payload["payload"].get("env").is_none());

    let duplicate = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/runtime-status"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    sqlx::query("UPDATE application_runtime_statuses SET status='succeeded',payload_json=?,observed_at='2026-08-12T00:00:01Z',updated_at='2026-08-12T00:00:01Z' WHERE runtime_status_id=?")
        .bind(r#" [{"Name":"deploy-go-shared-prod-redis-redis-1","Service":"redis","State":"running","Health":"healthy"}] "#)
        .bind(&runtime_status_id)
        .execute(&pool)
        .await
        .unwrap();
    let shown = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/applications/{application_id}/runtime-status"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(shown["status"], "succeeded");
    assert_eq!(shown["payload"][0]["State"], "running");
    assert_eq!(shown["payload"][0]["Health"], "healthy");

    let reprobe = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/runtime-status"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(reprobe.status(), StatusCode::ACCEPTED);
    let reprobe = response_json(reprobe).await;
    assert_ne!(reprobe["runtime_status_id"], runtime_status_id);
    assert_eq!(reprobe["status"], "pending");
    let rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM application_runtime_statuses WHERE target_id=?")
            .bind(target_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 2);
}

#[tokio::test]
async fn runtime_status_rejects_incompatible_agent_and_marks_failed() {
    let (app, pool) = test_app().await;
    let (application_id, _target_id, cookie, csrf) = setup(app.clone(), &pool).await;
    sqlx::query("UPDATE agents SET protocol_version=8,capabilities_json='[\"pty_terminal\",\"privileged_release\"]' WHERE id='agent_runtime'")
        .execute(&pool)
        .await
        .unwrap();

    let created = json_request(
        app,
        "POST",
        &format!("/api/v1/applications/{application_id}/runtime-status"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CONFLICT);
    let status: String = sqlx::query_scalar(
        "SELECT status FROM application_runtime_statuses WHERE target_id='target_runtime'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let error_code: String = sqlx::query_scalar(
        "SELECT error_code FROM application_runtime_statuses WHERE target_id='target_runtime'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "failed");
    assert_eq!(error_code, "runtime_status_agent_incompatible");
}

#[tokio::test]
async fn runtime_status_requires_admin_for_mutation_and_supports_target_selection() {
    let (app, pool) = test_app().await;
    let (application_id, _target_id, cookie, csrf) = setup(app.clone(), &pool).await;
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,target_code,environment,script_path,timeout_seconds,status) VALUES('target_runtime_2','app_runtime','node_runtime','shared-prod-redis-2','test','/unused',60,'active')")
        .execute(&pool)
        .await
        .unwrap();

    let missing_target = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/runtime-status"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(missing_target.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let user = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/users",
            json!({"username":"viewer","password":"viewer-password-long"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let user_id = user["id"].as_str().unwrap();
    let (user_cookie, user_csrf) =
        common::login(app.clone(), "viewer", "viewer-password-long").await;
    let forbidden = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/runtime-status"),
        json!({}),
        &[("cookie", &user_cookie), ("x-csrf-token", &user_csrf)],
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let _ = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/users/{user_id}/applications/{application_id}"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let hidden = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}/runtime-status?target_id=target_runtime"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);

    let created = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/runtime-status?target_id=target_runtime"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::ACCEPTED);
    let shown = json_request(
        app,
        "GET",
        &format!("/api/v1/applications/{application_id}/runtime-status?target_id=target_runtime"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(shown.status(), StatusCode::OK);
    assert_eq!(response_json(shown).await["status"], "pending");
}
