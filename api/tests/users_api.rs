mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;

#[tokio::test]
async fn administrator_creates_and_disables_user_and_sessions_are_revoked() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({"username":"operator", "password":"operator-password-long"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let user = response_json(created).await;
    let user_id = user["id"].as_str().unwrap();
    let (user_cookie, _) = common::login(app.clone(), "operator", "operator-password-long").await;

    let disabled = json_request(
        app.clone(),
        "PATCH",
        &format!("/api/v1/users/{user_id}/status"),
        json!({"status":"disabled", "version":1}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);

    let me = json_request(
        app,
        "GET",
        "/api/v1/auth/me",
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);

    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_logs WHERE action LIKE 'user.%' ORDER BY created_at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(actions, vec!["user.create", "user.status.update"]);
}

#[tokio::test]
async fn administrator_cannot_be_disabled() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let me = json_request(
        app.clone(),
        "GET",
        "/api/v1/auth/me",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let admin = response_json(me).await;

    let response = json_request(
        app,
        "PATCH",
        &format!("/api/v1/users/{}/status", admin["id"].as_str().unwrap()),
        json!({"status":"disabled", "version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn password_reset_requires_csrf_revokes_sessions_and_is_audited() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({"username":"operator", "password":"operator-password-long"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let user = response_json(created).await;
    let user_id = user["id"].as_str().unwrap();
    let (user_cookie, _) = common::login(app.clone(), "operator", "operator-password-long").await;
    let new_password = "new-operator-password-long";

    let missing_csrf = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/users/{user_id}/password"),
        json!({"password":new_password, "version":1}),
        &[("cookie", &admin_cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);

    let reset = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/users/{user_id}/password"),
        json!({"password":new_password, "version":1}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(reset.status(), StatusCode::NO_CONTENT);
    let me = json_request(
        app.clone(),
        "GET",
        "/api/v1/auth/me",
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
    common::login(app, "operator", new_password).await;

    let summary: String = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE action = 'user.password.reset'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!summary.contains(new_password));
}

#[tokio::test]
async fn user_list_requires_administrator() {
    let (app, _) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({"username":"operator", "password":"operator-password-long"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let (user_cookie, _) = common::login(app.clone(), "operator", "operator-password-long").await;
    let response = json_request(
        app,
        "GET",
        "/api/v1/users",
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn administrator_reads_user_detail_with_profile_fields() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({
            "username":"operator",
            "password":"operator-password-long",
            "display_name":"运维用户",
            "email":"operator@example.com"
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let user = response_json(created).await;
    let detail = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/users/{}", user["id"].as_str().unwrap()),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail = response_json(detail).await;
    assert_eq!(detail["display_name"], "运维用户");
    assert_eq!(detail["email"], "operator@example.com");

    let (user_cookie, _) = common::login(
        app.clone(),
        "operator@example.com",
        "operator-password-long",
    )
    .await;
    let forbidden = json_request(
        app,
        "GET",
        &format!("/api/v1/users/{}", user["id"].as_str().unwrap()),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}
