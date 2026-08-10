mod common;

use axum::http::StatusCode;
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

#[tokio::test]
async fn administrator_creates_lists_revokes_and_updates_external_keys() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "one").await;
    seed_application(&pool, "app_two", "two").await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"CI 部署 Key","application_ids":["app_one"],"expires_at":"2099-01-01T00:00:00Z"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let token = created["token"].as_str().unwrap().to_owned();
    assert!(token.starts_with("dgx_"), "token 前缀不正确");
    let key_id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(created["application_ids"], json!(["app_one"]));

    let listed = json_request(
        app.clone(),
        "GET",
        "/api/v1/external-api-keys",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = response_json(listed).await;
    assert_eq!(listed["items"][0]["id"], json!(key_id));
    assert!(listed["items"][0].get("token").is_none());

    let shown = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/external-api-keys/{key_id}"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(shown.status(), StatusCode::OK);
    let shown = response_json(shown).await;
    assert_eq!(shown["name"], json!("CI 部署 Key"));

    let updated = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/external-api-keys/{key_id}/applications"),
        json!({"application_ids":["app_two"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["application_ids"], json!(["app_two"]));

    let revoked = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/external-api-keys/{key_id}/revoke"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::OK);
    let revoked = response_json(revoked).await;
    assert_eq!(revoked["status"], json!("disabled"));

    let audit = json_request(
        app.clone(),
        "GET",
        "/api/v1/audit-logs",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let audit = response_json(audit).await;
    let actions = audit["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["action"].as_str())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"external_api_key.create"));
    assert!(actions.contains(&"external_api_key.applications.update"));
    assert!(actions.contains(&"external_api_key.revoke"));

    let users = json_request(
        app,
        "GET",
        "/api/v1/users",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let users = response_json(users).await;
    let usernames = users["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["username"].as_str())
        .collect::<Vec<_>>();
    assert!(!usernames.contains(&"__deploy_go_external_api__"));
    let _ = pool;
}

#[tokio::test]
async fn external_api_key_creation_validates_applications_and_expiry() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "one").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let headers = [("cookie", cookie.as_str()), ("x-csrf-token", csrf.as_str())];

    let unknown_app = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"bad","application_ids":["app_missing"]}),
        &headers,
    )
    .await;
    assert_eq!(unknown_app.status(), StatusCode::NOT_FOUND);

    let duplicate_app = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"bad","application_ids":["app_one","app_one"]}),
        &headers,
    )
    .await;
    assert_eq!(duplicate_app.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expired = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"bad","application_ids":["app_one"],"expires_at":"2020-01-01T00:00:00Z"}),
        &headers,
    )
    .await;
    assert_eq!(expired.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn service_grants_are_synced_for_key_applications() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "one").await;
    seed_application(&pool, "app_two", "two").await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"grants","application_ids":["app_one"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let created = response_json(created).await;
    let key_id = created["id"].as_str().unwrap().to_owned();
    let grants: Vec<String> = sqlx::query_scalar(
        "SELECT application_id FROM user_application_grants WHERE user_id='usr_external_api_service' ORDER BY application_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(grants, vec!["app_one".to_owned()]);

    let updated = json_request(
        app,
        "PUT",
        &format!("/api/v1/external-api-keys/{key_id}/applications"),
        json!({"application_ids":["app_two"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let grants: Vec<String> = sqlx::query_scalar(
        "SELECT application_id FROM user_application_grants WHERE user_id='usr_external_api_service' ORDER BY application_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(grants.contains(&"app_two".to_owned()));
}
