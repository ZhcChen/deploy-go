mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;

#[tokio::test]
async fn administrator_generates_git_credential_without_exposing_private_key() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let missing_csrf = json_request(
        app.clone(),
        "POST",
        "/api/v1/git-credentials",
        json!({"name":"GitHub Deploy Key"}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/git-credentials",
        json!({"name":"GitHub Deploy Key"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = response_json(created).await;
    assert!(body["id"].as_str().unwrap().starts_with("git_cred_"));
    assert_eq!(body["name"], "GitHub Deploy Key");
    assert_eq!(body["algorithm"], "ed25519");
    assert_eq!(body["status"], "active");
    assert!(
        body["public_key"]
            .as_str()
            .unwrap()
            .starts_with("ssh-ed25519 ")
    );
    assert!(body["fingerprint"].as_str().unwrap().starts_with("SHA256:"));
    let serialized = body.to_string();
    for forbidden in [
        "encrypted_private_key",
        "nonce",
        "PRIVATE KEY",
        "BEGIN OPENSSH",
    ] {
        assert!(!serialized.contains(forbidden));
    }

    let (id, fingerprint): (String, String) = sqlx::query_as(
        "SELECT id, fingerprint FROM git_credentials WHERE name = 'GitHub Deploy Key'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(id, body["id"].as_str().unwrap());
    assert_eq!(fingerprint, body["fingerprint"].as_str().unwrap());
    let row: (Vec<u8>, Vec<u8>, i64) = sqlx::query_as(
        "SELECT encrypted_private_key, nonce, key_version FROM git_credentials WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!row.0.is_empty());
    assert_eq!(row.1.len(), 12);
    assert_eq!(row.2, 1);

    let duplicate = json_request(
        app.clone(),
        "POST",
        "/api/v1/git-credentials",
        json!({"name":"github deploy key"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    let invalid = json_request(
        app.clone(),
        "POST",
        "/api/v1/git-credentials",
        json!({"name":""}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn git_credentials_list_show_and_archive_are_administrator_only() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/git-credentials",
        json!({"name":"Production Key"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let credential = response_json(created).await;
    let id = credential["id"].as_str().unwrap().to_owned();

    let list = json_request(
        app.clone(),
        "GET",
        "/api/v1/git-credentials",
        json!({}),
        &[("cookie", &admin_cookie)],
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let list = response_json(list).await;
    assert_eq!(list["items"][0]["id"], id);

    let show = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/git-credentials/{id}"),
        json!({}),
        &[("cookie", &admin_cookie)],
    )
    .await;
    assert_eq!(show.status(), StatusCode::OK);
    assert_eq!(response_json(show).await["id"], id);

    let archived = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/git-credentials/{id}/status"),
        json!({"status":"archived","version":1}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    assert_eq!(response_json(archived).await["status"], "archived");

    let stale = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/git-credentials/{id}/status"),
        json!({"status":"active","version":1}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);

    let audit: (String, String) = sqlx::query_as(
        "SELECT action, summary_json FROM audit_logs WHERE resource_id = ? ORDER BY created_at",
    )
    .bind(&id)
    .fetch_all(&pool)
    .await
    .unwrap()
    .pop()
    .unwrap();
    assert_eq!(audit.0, "git_credential.status.update");
    assert!(audit.1.contains("archived"));

    let operator_created = json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({"username":"operator", "password":"operator-password-long"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(operator_created.status(), StatusCode::CREATED);
    let (operator_cookie, _) =
        common::login(app.clone(), "operator", "operator-password-long").await;
    let forbidden = json_request(
        app,
        "GET",
        "/api/v1/git-credentials",
        json!({}),
        &[("cookie", &operator_cookie)],
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn git_credential_validation_rejects_unknown_fields_and_bad_status() {
    let (app, _) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let unknown = json_request(
        app.clone(),
        "POST",
        "/api/v1/git-credentials",
        json!({"name":"Key","private_key":"secret"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/git-credentials",
        json!({"name":"Key"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let invalid_status = json_request(
        app,
        "PUT",
        &format!("/api/v1/git-credentials/{id}/status"),
        json!({"status":"deleted","version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(invalid_status.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
