mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, test_app};
use serde_json::json;

#[tokio::test]
async fn legacy_settings_and_patch_preserve_capacity() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    sqlx::query("INSERT INTO system_settings(key,value_json,version) VALUES('runtime',?,1)")
        .bind(json!({"max_concurrent_deployments":2,"max_log_bytes":52428800,"log_retention_days":30,"version":1}).to_string())
        .execute(&pool).await.unwrap();
    let shown = json_request(
        app.clone(),
        "GET",
        "/api/v1/settings",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let body = axum::body::to_bytes(shown.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["max_total_log_bytes"], 2_u64 * 1024 * 1024 * 1024);
    for (version, capacity) in [(1, Some(3_u64 * 1024 * 1024 * 1024)), (2, None)] {
        let mut payload = json!({"max_concurrent_deployments":4,"max_log_bytes":52428800,"log_retention_days":30,"version":version});
        if let Some(capacity) = capacity {
            payload["max_total_log_bytes"] = json!(capacity);
        }
        let response = json_request(
            app.clone(),
            "PATCH",
            "/api/v1/settings",
            payload,
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["max_total_log_bytes"], 3_u64 * 1024 * 1024 * 1024);
    }
    let response = json_request(app, "PATCH", "/api/v1/settings", json!({"max_concurrent_deployments":4,"max_log_bytes":52428800,"max_total_log_bytes":0,"log_retention_days":30,"version":3}), &[("cookie", &cookie), ("x-csrf-token", &csrf)]).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn concurrent_capacity_updates_do_not_overwrite_the_winner() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let headers = [("cookie", cookie.as_str()), ("x-csrf-token", csrf.as_str())];
    let payload = |capacity| json!({"max_concurrent_deployments":2,"max_log_bytes":52428800,"max_total_log_bytes":capacity,"log_retention_days":30,"version":1});
    let (first, second) = tokio::join!(
        json_request(
            app.clone(),
            "PATCH",
            "/api/v1/settings",
            payload(3_u64 * 1024 * 1024 * 1024),
            &headers
        ),
        json_request(
            app,
            "PATCH",
            "/api/v1/settings",
            payload(4_u64 * 1024 * 1024 * 1024),
            &headers
        )
    );
    let statuses = [first.status(), second.status()];
    assert_eq!(
        statuses
            .iter()
            .filter(|&&status| status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|&&status| status == StatusCode::CONFLICT)
            .count(),
        1
    );
    let value: String =
        sqlx::query_scalar("SELECT value_json FROM system_settings WHERE key='runtime'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let value: serde_json::Value = serde_json::from_str(&value).unwrap();
    assert_eq!(value["version"], 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM audit_logs WHERE action='settings.update'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}

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
