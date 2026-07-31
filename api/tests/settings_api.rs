mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, test_app};
use serde_json::json;

#[tokio::test]
async fn settings_require_administrator_and_validate_limits() {
    let (app, pool) = test_app().await;
    let unauthenticated =
        json_request(app.clone(), "GET", "/api/v1/settings", json!({}), &[]).await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let (cookie, csrf) = admin_session(app.clone()).await;
    let missing_csrf = json_request(
        app.clone(),
        "PATCH",
        "/api/v1/settings",
        json!({"max_concurrent_deployments":4,"max_log_bytes":52428800,"log_retention_days":30,"version":1}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);
    let invalid = json_request(
        app.clone(),
        "PATCH",
        "/api/v1/settings",
        json!({"max_concurrent_deployments":0,"max_log_bytes":1,"log_retention_days":0,"version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let updated = json_request(
        app,
        "PATCH",
        "/api/v1/settings",
        json!({"max_concurrent_deployments":4,"max_log_bytes":52428800,"log_retention_days":30,"version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs WHERE action = 'settings.update'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audit_count, 1);
}
