mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json};
use deploy_go_api::{AppState, app, crypto::MasterKeyRing, db};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

async fn credential_app() -> (axum::Router, sqlx::SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let state = AppState::new(pool.clone())
        .with_setup_token(common::SETUP_TOKEN)
        .with_master_key_ring(MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap());
    (app(state), pool)
}

#[tokio::test]
async fn generated_key_is_valid_openssh_and_private_key_never_leaks() {
    let (app, pool) = credential_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/ssh-credentials",
        json!({"name":"Production"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = response_json(created).await;
    assert_eq!(body["algorithm"], "ed25519");
    let public_key = body["public_key"].as_str().unwrap();
    ssh_key::PublicKey::from_openssh(public_key).unwrap();
    assert!(body["fingerprint"].as_str().unwrap().starts_with("SHA256:"));
    let serialized = body.to_string();
    assert!(!serialized.contains("PRIVATE KEY"));
    assert!(!serialized.contains("encrypted_private_key"));
    assert!(!serialized.contains("nonce"));

    let encrypted: Vec<u8> =
        sqlx::query_scalar("SELECT encrypted_private_key FROM ssh_credentials WHERE id = ?")
            .bind(body["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!encrypted.is_empty());
    assert!(!String::from_utf8_lossy(&encrypted).contains("PRIVATE KEY"));

    let listed = json_request(
        app,
        "GET",
        "/api/v1/ssh-credentials",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    assert!(
        !response_json(listed)
            .await
            .to_string()
            .contains("PRIVATE KEY")
    );
}

#[tokio::test]
async fn rename_and_delete_require_admin_csrf_and_are_audited() {
    let (app, pool) = credential_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/ssh-credentials",
        json!({"name":"Primary"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let body = response_json(created).await;
    let id = body["id"].as_str().unwrap();

    let missing_csrf = json_request(
        app.clone(),
        "PATCH",
        &format!("/api/v1/ssh-credentials/{id}"),
        json!({"name":"Renamed", "version":1}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);
    let renamed = json_request(
        app.clone(),
        "PATCH",
        &format!("/api/v1/ssh-credentials/{id}"),
        json!({"name":"Renamed", "version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    let deleted = json_request(
        app,
        "DELETE",
        &format!("/api/v1/ssh-credentials/{id}"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let actions: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_logs WHERE resource_id = ? ORDER BY rowid")
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        actions,
        vec![
            "ssh_credential.create",
            "ssh_credential.rename",
            "ssh_credential.delete"
        ]
    );
}

#[tokio::test]
async fn bound_key_cannot_be_deleted_and_returns_node_summary() {
    let (app, pool) = credential_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/ssh-credentials",
        json!({"name":"Bound"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let body = response_json(created).await;
    let id = body["id"].as_str().unwrap();
    sqlx::query("INSERT INTO nodes (id, name, host, port, username, ssh_credential_id, work_root, secrets_root, status) VALUES ('node_1', 'Node One', '127.0.0.1', 22, 'deploy', ?, '/srv/apps', '/srv/secrets', 'unchecked')")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let response = json_request(
        app,
        "DELETE",
        &format!("/api/v1/ssh-credentials/{id}"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let error = response_json(response).await;
    assert_eq!(error["code"], "credential_in_use");
    assert_eq!(error["details"]["nodes"][0]["id"], "node_1");
    assert_eq!(error["details"]["nodes"][0]["name"], "Node One");
}
