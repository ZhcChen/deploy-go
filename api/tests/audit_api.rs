mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;

#[tokio::test]
async fn audit_logs_are_admin_only_filterable_and_paginated() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    for username in ["operator-one", "operator-two"] {
        let created = json_request(
            app.clone(),
            "POST",
            "/api/v1/users",
            json!({"username":username,"password":"operator-password-long"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await;
        assert_eq!(created.status(), StatusCode::CREATED);
    }
    let first = json_request(
        app.clone(),
        "GET",
        "/api/v1/audit-logs?action=user.create&limit=1",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first = response_json(first).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 1);
    assert_eq!(first["items"][0]["action"], "user.create");
    let cursor = first["next_cursor"].as_str().unwrap();
    let second = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/audit-logs?action=user.create&limit=1&after={cursor}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert!(second["next_cursor"].is_null());

    let (user_cookie, _) =
        common::login(app.clone(), "operator-one", "operator-password-long").await;
    let forbidden = json_request(
        app,
        "GET",
        "/api/v1/audit-logs",
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}
