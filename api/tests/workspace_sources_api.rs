mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::{Value, json};

async fn setup_resources(
    app: axum::Router,
    pool: &sqlx::SqlitePool,
    cookie: &str,
    csrf: &str,
) -> String {
    let application = response_json(
        json_request(
            app,
            "POST",
            "/api/v1/applications",
            json!({"name":"Workspace App","slug":"workspace-app","description":"","environment":"prod"}),
            &[("cookie", cookie), ("x-csrf-token", csrf)],
        )
        .await,
    )
    .await;
    let application_id = application["id"].as_str().unwrap().to_owned();
    sqlx::query("INSERT INTO nodes (id,name,work_root,secrets_root,status) VALUES ('node_ws','Workspace Node','/srv/apps','/srv/secrets','online')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_ws','node_ws','2026-09-02T00:00:00Z','2026-09-02T00:00:00Z','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')")
        .execute(pool)
        .await
        .unwrap();
    application_id
}

fn workspace_source_payload(build_agent_id: &str, path: &str) -> Value {
    json!({
        "build_agent_id": build_agent_id,
        "workspace_path": path,
    })
}

#[tokio::test]
async fn workspace_source_requires_v14_agent_and_survives_edits_with_version_bump() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let application_id = setup_resources(app.clone(), &pool, &cookie, &csrf).await;

    let created = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/workspace-source"),
        workspace_source_payload("agent_ws", "/srv/workspaces/clickhouse"),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED, "{created:?}");
    let source = response_json(created).await;
    let source_id = source["id"].as_str().unwrap().to_owned();
    assert_eq!(source["workspace_version"], 1);
    assert_eq!(source["status"], "verified");

    let shown = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}/workspace-source"),
        Value::Null,
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(shown.status(), StatusCode::OK);
    let shown = response_json(shown).await;
    assert_eq!(shown["id"], source_id);
    assert_eq!(shown["workspace_path"], "/srv/workspaces/clickhouse");

    let updated = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/workspace-source"),
        json!({
            "build_agent_id":"agent_ws",
            "workspace_path":"/srv/workspaces/clickhouse-9",
            "version": shown["version"],
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK, "{updated:?}");
    let updated = response_json(updated).await;
    assert_eq!(updated["workspace_path"], "/srv/workspaces/clickhouse-9");
    assert_eq!(updated["workspace_version"], 2);
}

#[tokio::test]
async fn workspace_source_rejects_legacy_agent_unsafe_path_and_duplicate_target_validation() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let application_id = setup_resources(app.clone(), &pool, &cookie, &csrf).await;

    let legacy = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/workspace-source"),
        workspace_source_payload("agent_ws", "/srv/workspaces/clickhouse"),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(legacy.status(), StatusCode::CREATED);

    sqlx::query("UPDATE agents SET protocol_version=13 WHERE id='agent_ws'")
        .execute(&pool)
        .await
        .unwrap();
    let legacy_target = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        json!({
            "node_id":"node_ws",
            "target_code":"ws-prod",
            "script_path":"",
            "timeout_seconds":900,
            "secret_file_references":[],
            "execution_mode":"two_stage_script",
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(legacy_target.status(), StatusCode::CONFLICT);

    sqlx::query("UPDATE agents SET protocol_version=14 WHERE id='agent_ws'")
        .execute(&pool)
        .await
        .unwrap();
    let created = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/applications/{application_id}/targets"),
        json!({
            "node_id":"node_ws",
            "target_code":"ws-prod",
            "script_path":"",
            "timeout_seconds":900,
            "secret_file_references":[],
            "execution_mode":"two_stage_script",
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED, "{created:?}");
    let target = response_json(created).await;
    assert_eq!(target["execution_mode"], "two_stage_script");

    let db_mode: (String, bool) =
        sqlx::query_as("SELECT execution_mode,workspace_script FROM deployment_targets WHERE id=?")
            .bind(target["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(db_mode.0, "two_stage");
    assert!(db_mode.1);
}
