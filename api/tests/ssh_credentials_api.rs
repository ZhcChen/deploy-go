mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json};
use deploy_go_api::{AppState, app, db};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

async fn credential_app() -> (axum::Router, sqlx::SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    (
        app(AppState::new(pool.clone()).with_setup_token(common::SETUP_TOKEN)),
        pool,
    )
}

async fn insert_legacy_credential(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO ssh_credentials(id,name,algorithm,public_key,fingerprint,encrypted_private_key,nonce,key_version) VALUES('cred_legacy','Legacy Key','ed25519','ssh-ed25519 AAAAC3NzaLegacy','SHA256:legacy',X'010203',X'040506',1)")
        .execute(pool).await.unwrap();
}

#[tokio::test]
async fn legacy_credentials_are_read_only_and_never_expose_private_fields() {
    let (app, pool) = credential_app().await;
    insert_legacy_credential(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    for path in [
        "/api/v1/ssh-credentials",
        "/api/v1/ssh-credentials/cred_legacy",
    ] {
        let response =
            json_request(app.clone(), "GET", path, json!({}), &[("cookie", &cookie)]).await;
        assert_eq!(response.status(), StatusCode::OK);
        let serialized = response_json(response).await.to_string();
        assert!(serialized.contains("Legacy Key"));
        for forbidden in ["encrypted_private_key", "nonce", "PRIVATE KEY", "010203"] {
            assert!(!serialized.contains(forbidden));
        }
    }

    let create = json_request(
        app.clone(),
        "POST",
        "/api/v1/ssh-credentials",
        json!({"name":"New"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let rename = json_request(
        app,
        "PATCH",
        "/api/v1/ssh-credentials/cred_legacy",
        json!({"name":"Renamed"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(create.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(rename.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn deleting_legacy_credential_detaches_nodes_and_is_audited() {
    let (app, pool) = credential_app().await;
    insert_legacy_credential(&pool).await;
    sqlx::query("INSERT INTO nodes(id,name,host,port,username,ssh_credential_id,work_root,secrets_root,status) VALUES('node_legacy','Legacy Node','127.0.0.1',22,'deploy','cred_legacy','/srv/apps','/srv/secrets','unchecked')")
        .execute(&pool).await.unwrap();
    for index in 1..=20 {
        sqlx::query("INSERT INTO nodes(id,name,host,port,username,ssh_credential_id,work_root,secrets_root,status) VALUES(?,?,'127.0.0.1',22,'deploy',?,'/srv/apps','/srv/secrets','offline')")
            .bind(format!("node_extra_{index:02}"))
            .bind(format!("Extra Node {index:02}"))
            .bind("cred_legacy")
            .execute(&pool)
            .await
            .unwrap();
    }
    let (cookie, csrf) = admin_session(app.clone()).await;

    let missing_csrf = json_request(
        app.clone(),
        "DELETE",
        "/api/v1/ssh-credentials/cred_legacy",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);
    let deleted = json_request(
        app,
        "DELETE",
        "/api/v1/ssh-credentials/cred_legacy",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let credential_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ssh_credentials WHERE id='cred_legacy'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let binding: Option<String> =
        sqlx::query_scalar("SELECT ssh_credential_id FROM nodes WHERE id='node_legacy'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(credential_count, 0);
    assert_eq!(binding, None);
    let audit: (String, String) = sqlx::query_as(
        "SELECT action,summary_json FROM audit_logs WHERE resource_id='cred_legacy'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit.0, "ssh_credential.delete");
    assert!(audit.1.contains("node_legacy"));
    assert!(audit.1.contains("node_extra_20"));
}
