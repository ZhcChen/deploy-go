mod common;

use axum::http::StatusCode;
use common::{ADMIN_PASSWORD, admin_session, json_request, response_json, test_app};
use serde_json::json;

const FETCH_METADATA: [(&str, &str); 3] = [
    ("origin", "http://localhost"),
    ("sec-fetch-site", "same-origin"),
    ("sec-fetch-mode", "cors"),
];

#[tokio::test]
async fn setup_status_changes_after_initialization() {
    let (app, _) = test_app().await;
    let before = json_request(app.clone(), "GET", "/api/v1/setup", json!({}), &[]).await;
    assert_eq!(before.status(), StatusCode::OK);
    let before = response_json(before).await;
    assert_eq!(before["setup_required"], true);

    common::initialize_admin(app.clone()).await;
    let after = json_request(app, "GET", "/api/v1/setup", json!({}), &[]).await;
    let after = response_json(after).await;
    assert_eq!(after["setup_required"], false);
}

#[tokio::test]
async fn setup_is_one_time_and_closes_after_initialization() {
    let (app, _) = test_app().await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/setup",
        json!({"username":"admin", "password":ADMIN_PASSWORD}),
        &[("origin", "http://localhost")],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);

    let duplicate = json_request(
        app,
        "POST",
        "/api/v1/setup",
        json!({"username":"other", "password":ADMIN_PASSWORD}),
        &[("origin", "http://localhost")],
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn setup_and_login_reject_missing_foreign_and_port_mismatched_origins() {
    let rejected_origins = [
        None,
        Some("http://attacker.invalid"),
        Some("http://localhost:30101"),
    ];

    for origin in rejected_origins {
        let (app, pool) = test_app().await;
        let headers = origin
            .map(|value| vec![("origin", value)])
            .unwrap_or_default();
        let response = json_request(
            app,
            "POST",
            "/api/v1/setup",
            json!({"username":"admin", "password":ADMIN_PASSWORD}),
            &headers,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(users, 0);
    }

    let (app, pool) = test_app().await;
    common::initialize_admin(app.clone()).await;
    for origin in rejected_origins {
        let headers = origin
            .map(|value| vec![("origin", value)])
            .unwrap_or_default();
        let response = json_request(
            app.clone(),
            "POST",
            "/api/v1/auth/login",
            json!({"username":"admin", "password":ADMIN_PASSWORD}),
            &headers,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sessions, 0);
}

#[tokio::test]
async fn setup_accepts_each_configured_origin() {
    for origin in ["https://admin.example.test", "https://backup.example.test"] {
        let (app, _) = common::test_app_with_allowed_origins(vec![
            "https://admin.example.test".to_owned(),
            "https://backup.example.test".to_owned(),
        ])
        .await;
        let response = json_request(
            app,
            "POST",
            "/api/v1/setup",
            json!({"username":"admin", "password":ADMIN_PASSWORD}),
            &[("origin", origin)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED, "origin: {origin}");
    }
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

#[tokio::test]
async fn profile_and_preferences_persist_across_sessions_and_reject_extra_fields() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let updated = json_request(
        app.clone(),
        "PATCH",
        "/api/v1/auth/profile",
        json!({"display_name":"部署管理员"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(response_json(updated).await["display_name"], "部署管理员");

    let escalation = json_request(
        app.clone(),
        "PATCH",
        "/api/v1/auth/profile",
        json!({"display_name":"攻击者", "identity":"administrator"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(escalation.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = response_json(escalation).await;
    assert_eq!(error["code"], "validation_failed");
    assert!(error["request_id"].as_str().unwrap().starts_with("req_"));

    let defaults = json_request(
        app.clone(),
        "GET",
        "/api/v1/auth/preferences",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(response_json(defaults).await["version"], 1);
    let changed = json_request(
        app.clone(),
        "PUT",
        "/api/v1/auth/preferences",
        json!({
            "notify_deployment_failed":true,
            "notify_deployment_completed":false,
            "notify_node_unhealthy":true,
            "time_format":"12h",
            "follow_logs":false,
            "version":1
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(changed.status(), StatusCode::OK);
    assert_eq!(response_json(changed).await["version"], 2);

    let (new_cookie, _) = common::login(app.clone(), "admin", ADMIN_PASSWORD).await;
    let restored = json_request(
        app.clone(),
        "GET",
        "/api/v1/auth/preferences",
        json!({}),
        &[("cookie", &new_cookie)],
    )
    .await;
    let restored = response_json(restored).await;
    assert_eq!(restored["notify_deployment_completed"], false);
    assert_eq!(restored["time_format"], "12h");
    let restored_profile = json_request(
        app.clone(),
        "GET",
        "/api/v1/auth/profile",
        json!({}),
        &[("cookie", &new_cookie)],
    )
    .await;
    assert_eq!(
        response_json(restored_profile).await["display_name"],
        "部署管理员"
    );

    let stale = json_request(
        app.clone(),
        "PUT",
        "/api/v1/auth/preferences",
        json!({
            "notify_deployment_failed":false,
            "notify_deployment_completed":true,
            "notify_node_unhealthy":false,
            "time_format":"24h",
            "follow_logs":true,
            "version":1
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(stale).await["code"],
        "resource_version_conflict"
    );

    let unauthenticated = json_request(app, "GET", "/api/v1/auth/profile", json!({}), &[]).await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn csrf_refresh_supports_concurrent_tabs_until_logout() {
    let (app, _) = test_app().await;
    let (cookie, login_csrf) = admin_session(app.clone()).await;

    for headers in [
        vec![("cookie", cookie.as_str())],
        vec![
            ("cookie", cookie.as_str()),
            ("origin", "http://attacker.invalid"),
            ("sec-fetch-site", "same-origin"),
            ("sec-fetch-mode", "cors"),
        ],
    ] {
        let response = json_request(
            app.clone(),
            "POST",
            "/api/v1/auth/csrf",
            json!({}),
            &headers,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    let mut headers = vec![("cookie", cookie.as_str())];
    headers.extend(FETCH_METADATA);
    let first = json_request(
        app.clone(),
        "POST",
        "/api/v1/auth/csrf",
        json!({}),
        &headers,
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first = response_json(first).await["csrf_token"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = json_request(
        app.clone(),
        "POST",
        "/api/v1/auth/csrf",
        json!({}),
        &headers,
    )
    .await;
    let second = response_json(second).await["csrf_token"]
        .as_str()
        .unwrap()
        .to_owned();

    let first_tab = json_request(
        app.clone(),
        "PATCH",
        "/api/v1/auth/profile",
        json!({"display_name":"并发标签页"}),
        &[("cookie", &cookie), ("x-csrf-token", &first)],
    )
    .await;
    assert_eq!(first_tab.status(), StatusCode::OK);
    let original_tab = json_request(
        app.clone(),
        "PATCH",
        "/api/v1/auth/profile",
        json!({"display_name":"原始标签页"}),
        &[("cookie", &cookie), ("x-csrf-token", &login_csrf)],
    )
    .await;
    assert_eq!(original_tab.status(), StatusCode::OK);
    let logout = json_request(
        app,
        "POST",
        "/api/v1/auth/logout",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &second)],
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
}
