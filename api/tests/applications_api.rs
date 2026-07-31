mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;

#[tokio::test]
async fn application_visibility_follows_grants_and_mutations_require_admin() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({"name":"Example API","slug":"example-api","description":"Example"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let application = response_json(created).await;
    let application_id = application["id"].as_str().unwrap();
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
    let (user_cookie, _) = common::login(app.clone(), "operator", "operator-password-long").await;

    let hidden = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
    let grant = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/users/{user_id}/applications/{application_id}"),
        json!({}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(grant.status(), StatusCode::NO_CONTENT);
    let visible = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(visible.status(), StatusCode::OK);
    let forbidden = json_request(
        app.clone(),
        "PATCH",
        &format!("/api/v1/applications/{application_id}"),
        json!({"name":"Changed","slug":"changed-app","description":"","version":1}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let archived = json_request(
        app,
        "PUT",
        &format!("/api/v1/applications/{application_id}/status"),
        json!({"status":"archived","version":1}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    let actions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE resource_id=? AND action LIKE 'application.%'",
    )
    .bind(application_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(actions, 3);
}
