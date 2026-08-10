mod common;

use axum::{
    Router,
    http::StatusCode,
};
use common::{admin_session, json_request, response_json, test_app};
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

async fn seed_node_and_target(pool: &SqlitePool, node_id: &str, target_id: &str, application_id: &str) {
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,'外部节点','/srv/apps','/srv/secrets','online')",
    )
    .bind(node_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES(?,?,?,'prod','/srv/deploy.sh',60,'active')",
    )
    .bind(target_id)
    .bind(application_id)
    .bind(node_id)
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
    assert_eq!(detail["targets"][0]["id"], json!("target_one"));
    assert_eq!(detail["targets"][0]["environment"], json!("prod"));
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
