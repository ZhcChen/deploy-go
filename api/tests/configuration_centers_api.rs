mod common;

use common::{admin_session, json_request, response_json, test_app};
use deploy_go_api::crypto::{EncryptedSecret, MasterKeyRing};
use serde_json::json;
use sqlx::SqlitePool;

async fn stored_password(pool: &SqlitePool) -> Vec<u8> {
    let row: (String, Vec<u8>, Vec<u8>, i64) = sqlx::query_as(
        "SELECT configuration_center_credentials.id, ciphertext, nonce, key_version FROM configuration_centers JOIN configuration_center_credentials ON configuration_center_credentials.id = configuration_centers.credential_id WHERE configuration_centers.id = 'platform'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let encrypted = EncryptedSecret {
        ciphertext: row.1,
        nonce: row.2,
        key_version: row.3,
    };
    MasterKeyRing::from_raw(1, [7_u8; 32], None)
        .unwrap()
        .decrypt_etcd_admin_credential(&row.0, &encrypted)
        .unwrap()
        .to_vec()
}

#[tokio::test]
async fn platform_configuration_center_is_write_only_and_versioned() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let response = json_request(
        app.clone(),
        "GET",
        "/api/v1/configuration-centers/platform",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "unconfigured");
    assert_eq!(body["password_configured"], false);

    let response = json_request(
        app.clone(),
        "PUT",
        "/api/v1/configuration-centers/platform",
        json!({
            "endpoints": ["http://127.0.0.1:2379/", "http://127.0.0.1:2379"],
            "username": "root",
            "password": "test-etcd-password",
            "version": 0
        }),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("origin", "http://localhost"),
        ],
    )
    .await;
    assert_eq!(response.status(), 200);
    let body = response_json(response).await;
    assert_eq!(body["endpoints"], json!(["http://127.0.0.1:2379"]));
    assert_eq!(body["password_configured"], true);
    assert_eq!(body["version"], 1);
    assert!(!body.to_string().contains("test-etcd-password"));
    assert!(body.get("password").is_none());
    assert!(body.get("ciphertext").is_none());
    assert_eq!(stored_password(&pool).await, b"test-etcd-password");

    let response = json_request(
        app.clone(),
        "PUT",
        "/api/v1/configuration-centers/platform",
        json!({
            "endpoints": ["http://localhost:2379"],
            "username": "root",
            "version": 0
        }),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("origin", "http://localhost"),
        ],
    )
    .await;
    assert_eq!(response.status(), 409);
    let body = response_json(response).await;
    assert_eq!(body["code"], "resource_version_conflict");

    let response = json_request(
        app.clone(),
        "PUT",
        "/api/v1/configuration-centers/platform",
        json!({
            "endpoints": ["http://localhost:2379"],
            "username": "root",
            "version": 1
        }),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("origin", "http://localhost"),
        ],
    )
    .await;
    assert_eq!(response.status(), 200);
    assert_eq!(stored_password(&pool).await, b"test-etcd-password");

    let response = json_request(
        app,
        "DELETE",
        "/api/v1/configuration-centers/platform",
        json!({"version": 2}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("origin", "http://localhost"),
        ],
    )
    .await;
    assert_eq!(response.status(), 200);
    let body = response_json(response).await;
    assert_eq!(body["status"], "unconfigured");
    assert_eq!(body["password_configured"], false);
}

#[tokio::test]
async fn platform_configuration_center_requires_administrator_and_master_key() {
    let (app, _) = test_app().await;
    let response = json_request(
        app.clone(),
        "PUT",
        "/api/v1/configuration-centers/platform",
        json!({
            "endpoints": ["http://127.0.0.1:2379"],
            "username": "root",
            "password": "test-etcd-password",
            "version": 0
        }),
        &[("origin", "http://localhost")],
    )
    .await;
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn invalid_platform_endpoint_is_rejected_without_persisting_secret() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let response = json_request(
        app,
        "PUT",
        "/api/v1/configuration-centers/platform",
        json!({
            "endpoints": ["http://user:password@example.test:2379"],
            "username": "root",
            "password": "test-etcd-password",
            "version": 0
        }),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("origin", "http://localhost"),
        ],
    )
    .await;
    assert_eq!(response.status(), 422);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM configuration_centers")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
