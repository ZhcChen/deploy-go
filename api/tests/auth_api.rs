mod common;

use axum::http::StatusCode;
use common::{ADMIN_PASSWORD, SETUP_TOKEN, admin_session, json_request, response_json, test_app};
use serde_json::json;

#[tokio::test]
async fn setup_is_one_time_and_requires_token() {
    let (app, _) = test_app().await;
    let unauthorized = json_request(
        app.clone(),
        "POST",
        "/api/v1/setup",
        json!({"username":"admin", "password":ADMIN_PASSWORD}),
        &[
            ("x-setup-token", "wrong-token"),
            ("origin", "http://localhost"),
        ],
    )
    .await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    common::initialize_admin(app.clone()).await;
    let duplicate = json_request(
        app,
        "POST",
        "/api/v1/setup",
        json!({"username":"other", "password":ADMIN_PASSWORD}),
        &[
            ("x-setup-token", SETUP_TOKEN),
            ("origin", "http://localhost"),
        ],
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn login_sets_secure_cookie_and_logout_requires_csrf() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let missing_csrf = json_request(
        app.clone(),
        "POST",
        "/api/v1/auth/logout",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);

    let logout = json_request(
        app.clone(),
        "POST",
        "/api/v1/auth/logout",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let me = json_request(
        app,
        "GET",
        "/api/v1/auth/me",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_cookie_has_required_security_attributes() {
    let (app, _) = test_app().await;
    common::initialize_admin(app.clone()).await;
    let response = json_request(
        app,
        "POST",
        "/api/v1/auth/login",
        json!({"username":"admin", "password":ADMIN_PASSWORD}),
        &[("origin", "http://localhost")],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"].to_str().unwrap();
    assert!(cookie.starts_with("deploy_go_session="));
    assert!(cookie.contains("; Path=/"));
    assert!(cookie.contains("; HttpOnly"));
    assert!(cookie.contains("; SameSite=Lax"));
    assert!(cookie.contains("; Secure"));
    assert!(!cookie.contains(ADMIN_PASSWORD));
}

#[tokio::test]
async fn login_error_does_not_echo_password() {
    let (app, _) = test_app().await;
    common::initialize_admin(app.clone()).await;
    let response = json_request(
        app,
        "POST",
        "/api/v1/auth/login",
        json!({"username":"admin", "password":"secret-that-must-not-leak"}),
        &[("origin", "http://localhost")],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(response).await.to_string();
    assert!(!body.contains("secret-that-must-not-leak"));
}
