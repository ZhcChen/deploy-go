#![allow(dead_code)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, Response},
};
use deploy_go_api::{AppState, app, crypto::MasterKeyRing, db};
use serde_json::{Value, json};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tower::ServiceExt;

pub const SETUP_TOKEN: &str = "setup-token-for-tests";
pub const ADMIN_PASSWORD: &str = "correct horse battery staple";

pub async fn test_app() -> (Router, SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let state = AppState::new(pool.clone())
        .with_setup_token(SETUP_TOKEN)
        .with_master_key_ring(MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap());
    (app(state), pool)
}

pub async fn json_request(
    app: Router,
    method: &str,
    uri: &str,
    body: Value,
    headers: &[(&str, &str)],
) -> Response<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    app.oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

pub async fn response_json(response: Response<Body>) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

pub async fn initialize_admin(app: Router) {
    let response = json_request(
        app,
        "POST",
        "/api/v1/setup",
        json!({"username":"admin", "password": ADMIN_PASSWORD}),
        &[
            ("x-setup-token", SETUP_TOKEN),
            ("origin", "http://localhost"),
        ],
    )
    .await;
    assert_eq!(response.status(), 201);
}

pub async fn login(app: Router, username: &str, password: &str) -> (String, String) {
    let response = json_request(
        app,
        "POST",
        "/api/v1/auth/login",
        json!({"username":username, "password":password}),
        &[("origin", "http://localhost")],
    )
    .await;
    assert_eq!(response.status(), 200);
    let cookie = response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body = response_json(response).await;
    (cookie, body["csrf_token"].as_str().unwrap().to_owned())
}

pub async fn admin_session(app: Router) -> (String, String) {
    initialize_admin(app.clone()).await;
    login(app, "admin", ADMIN_PASSWORD).await
}
