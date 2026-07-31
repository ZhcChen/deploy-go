use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use deploy_go_api::{AppState, app};
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

async fn test_state() -> AppState {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create test database");
    AppState::new(pool)
}

#[tokio::test]
async fn healthz_returns_service_status_and_request_id() {
    let response = app(test_state().await)
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn readyz_checks_database_connectivity() {
    let response = app(test_state().await)
        .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ready");
}

#[tokio::test]
async fn readyz_returns_service_unavailable_when_database_is_closed() {
    let state = test_state().await;
    state.pool().close().await;

    let response = app(state)
        .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "service_not_ready");
    assert!(
        json["request_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
}

#[tokio::test]
async fn openapi_document_is_available() {
    let response = app(test_state().await)
        .oneshot(
            Request::get("/api/v1/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["openapi"], "3.1.0");
    assert!(json["paths"]["/healthz"].is_object());
    assert!(json["paths"]["/readyz"].is_object());
    for path in [
        "/api/v1/setup",
        "/api/v1/auth/login",
        "/api/v1/auth/logout",
        "/api/v1/auth/me",
        "/api/v1/users",
        "/api/v1/users/{id}/status",
        "/api/v1/users/{id}/password",
        "/api/v1/users/{user_id}/applications/{application_id}",
        "/api/v1/settings",
        "/api/v1/ssh-credentials",
        "/api/v1/ssh-credentials/{id}",
        "/api/v1/nodes",
        "/api/v1/nodes/{id}",
        "/api/v1/nodes/{id}/status",
        "/api/v1/nodes/{id}/ssh-credential",
        "/api/v1/nodes/{id}/host-key/scan",
        "/api/v1/nodes/{id}/host-key/confirm",
        "/api/v1/nodes/{id}/checks",
    ] {
        assert!(json["paths"].get(path).is_some(), "OpenAPI 缺少 {path}");
    }
}
