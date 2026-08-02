mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;
use ulid::Ulid;

#[tokio::test]
async fn ordinary_user_cannot_access_system_management() {
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

    let settings = json_request(
        app.clone(),
        "GET",
        "/api/v1/settings",
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(settings.status(), StatusCode::FORBIDDEN);

    let credentials = json_request(
        app,
        "GET",
        "/api/v1/ssh-credentials",
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(credentials.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn application_grants_require_admin_csrf_and_audit_state_changes_once() {
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
    let user = common::response_json(created).await;
    let user_id = user["id"].as_str().unwrap();
    let application_id = format!("app_{}", Ulid::new());
    sqlx::query("INSERT INTO applications (id, name, slug, status) VALUES (?, ?, ?, 'active')")
        .bind(&application_id)
        .bind("Example")
        .bind("example")
        .execute(&pool)
        .await
        .unwrap();
    let uri = format!("/api/v1/users/{user_id}/applications/{application_id}");

    let missing_csrf = json_request(
        app.clone(),
        "PUT",
        &uri,
        json!({}),
        &[("cookie", &admin_cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);
    for _ in 0..2 {
        let response = json_request(
            app.clone(),
            "PUT",
            &uri,
            json!({}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    let grants = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/users/{user_id}/applications"),
        json!({}),
        &[("cookie", &admin_cookie)],
    )
    .await;
    assert_eq!(grants.status(), StatusCode::OK);
    let grants = response_json(grants).await;
    assert_eq!(grants["items"].as_array().unwrap().len(), 1);
    assert_eq!(grants["items"][0]["application_id"], application_id);

    for _ in 0..2 {
        let response = json_request(
            app.clone(),
            "DELETE",
            &uri,
            json!({}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    let mut actions: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_logs WHERE action LIKE 'application.grant%'")
            .fetch_all(&pool)
            .await
            .unwrap();
    actions.sort();
    assert_eq!(
        actions,
        vec!["application.grant", "application.grant.revoke"]
    );
}
