mod common;

use axum::http::StatusCode;
use base64::{Engine, engine::general_purpose::STANDARD};
use deploy_go_api::{agents::auth::token_hash, crypto::MasterKeyRing};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use common::{ADMIN_PASSWORD, admin_session, json_request, response_json, test_app};

async fn seed_application(pool: &SqlitePool, id: &str, suffix: &str) {
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES(?,?,?,'active')")
        .bind(id)
        .bind(format!("Env {suffix}"))
        .bind(format!("env-{suffix}"))
        .execute(pool)
        .await
        .unwrap();
}

async fn seed_env(pool: &SqlitePool, application_id: &str, env_id: &str, content: &str) {
    let version_id = format!("{env_id}_version_1");
    let digest = format!("{:x}", Sha256::digest(content.as_bytes()));
    let encrypted = MasterKeyRing::from_raw(1, [7; 32], None)
        .unwrap()
        .encrypt_application_env(application_id, env_id, &version_id, content.as_bytes())
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_version,current_digest) VALUES(?,?,?,'api','dotenv-v1',1,?)")
        .bind(env_id).bind(application_id).bind(format!("{env_id}.env")).bind(&digest).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES(?,?,1,'chacha20poly1305-application-env-v1',?,?,1,?)")
        .bind(version_id).bind(env_id).bind(encrypted.ciphertext).bind(encrypted.nonce).bind(digest).execute(pool).await.unwrap();
}

#[tokio::test]
async fn plaintext_crud_requires_admin_reauthentication_csrf_and_optimistic_version() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    seed_application(&pool, "app_env_a", "a").await;
    seed_application(&pool, "app_env_b", "b").await;
    seed_env(&pool, "app_env_a", "env_a", "SECRET=initial\n").await;
    seed_env(&pool, "app_env_b", "env_b", "OTHER=initial\n").await;

    let denied = json_request(
        app.clone(),
        "GET",
        "/api/v1/application-env-files/env_a",
        json!(null),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let grant_response = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_a/env-reveal-grants",
        json!({"password":ADMIN_PASSWORD,"action":"read_write"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(grant_response.status(), StatusCode::OK);
    assert_eq!(grant_response.headers()["cache-control"], "no-store");
    assert_eq!(grant_response.headers()["pragma"], "no-cache");
    let grant = response_json(grant_response).await["grant_token"]
        .as_str()
        .unwrap()
        .to_owned();

    let cross_application = json_request(
        app.clone(),
        "GET",
        "/api/v1/application-env-files/env_b",
        json!(null),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(cross_application.status(), StatusCode::FORBIDDEN);

    let revealed = json_request(
        app.clone(),
        "GET",
        "/api/v1/application-env-files/env_a",
        json!(null),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(revealed.status(), StatusCode::OK);
    assert_eq!(revealed.headers()["cache-control"], "no-store");
    let revealed = response_json(revealed).await;
    assert_eq!(revealed["content"], "SECRET=initial\n");
    assert_eq!(revealed["version"], 1);

    let sensitive_invalid = "SECRET=must-not-leak\nSECRET=duplicate\n";
    let invalid = json_request(
        app.clone(),
        "PUT",
        "/api/v1/application-env-files/env_a",
        json!({"content":sensitive_invalid,"expected_version":1}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        !response_json(invalid)
            .await
            .to_string()
            .contains("must-not-leak")
    );

    let updated = json_request(
        app.clone(),
        "PUT",
        "/api/v1/application-env-files/env_a",
        json!({"content":"SECRET=changed\n","expected_version":1}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(response_json(updated).await["version"], 2);

    let delete_with_read_write = json_request(
        app.clone(),
        "DELETE",
        "/api/v1/application-env-files/env_a",
        json!({"expected_version":2,"confirm_file_name":"env_a.env"}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(delete_with_read_write.status(), StatusCode::FORBIDDEN);

    let delete_grant_response = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_a/env-reveal-grants",
        json!({"password":ADMIN_PASSWORD,"action":"delete"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(delete_grant_response.status(), StatusCode::OK);
    let delete_grant = response_json(delete_grant_response).await["grant_token"]
        .as_str()
        .unwrap()
        .to_owned();
    let reveal_with_delete = json_request(
        app.clone(),
        "GET",
        "/api/v1/application-env-files/env_a",
        json!(null),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &delete_grant),
        ],
    )
    .await;
    assert_eq!(reveal_with_delete.status(), StatusCode::FORBIDDEN);

    let stale = json_request(
        app.clone(),
        "PUT",
        "/api/v1/application-env-files/env_a",
        json!({"content":"SECRET=stale\n","expected_version":1}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(stale).await["code"],
        "resource_version_conflict"
    );

    let audit: Vec<String> = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE resource_type='application_env_file'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(audit.iter().all(|summary| !summary.contains("initial")
        && !summary.contains("changed")
        && !summary.contains("must-not-leak")));

    sqlx::query("UPDATE users SET version=version+1 WHERE identity='administrator'")
        .execute(&pool)
        .await
        .unwrap();
    let invalidated = json_request(
        app,
        "GET",
        "/api/v1/application-env-files/env_a",
        json!(null),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(invalidated.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn registration_lease_creates_once_and_later_declaration_does_not_overwrite() {
    let (app, pool) = test_app().await;
    common::initialize_admin(app.clone()).await;
    let admin_id: String =
        sqlx::query_scalar("SELECT id FROM users WHERE identity='administrator'")
            .fetch_one(&pool)
            .await
            .unwrap();
    seed_application(&pool, "app_register", "register").await;
    sqlx::query(
        "INSERT INTO nodes(id,name,status) VALUES('node_register','Register Node','offline')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment) VALUES('agent_register','node_register','prod')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_credential_families(id,agent_id) VALUES('family_register','agent_register')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_access_sessions(id,family_id,agent_id,token_hash,token_key_version,expires_at) VALUES('access_register','family_register','agent_register',?,1,'2099-01-01T00:00:00Z')").bind(token_hash("access","register-token")).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_register','app_register','node_register','prod','/unused',60,'active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES('dep_register','app_register','target_register',?,'running','preparing','register-key','request','snapshot')").bind(&admin_id).execute(&pool).await.unwrap();

    let commit = "0123456789abcdef0123456789abcdef01234567";
    let first_content = b"SECRET=from-repository\n";
    let first_digest = format!("{:x}", Sha256::digest(first_content));
    let manifest = json!({"schema_version":1,"commit_sha":commit,"files":[{"file_name":"api.env","module":"api","sha256":first_digest,"size":first_content.len(),"format":"dotenv-v1"}]}).to_string();
    let manifest_digest = format!("{:x}", Sha256::digest(manifest.as_bytes()));
    sqlx::query("INSERT INTO application_env_registration_leases(id,application_id,deployment_id,agent_id,commit_sha,manifest_digest,status,expires_at) VALUES('lease_register','app_register','dep_register','agent_register',?,?,'active','2099-01-01T00:00:00Z')").bind(commit).bind(&manifest_digest).execute(&pool).await.unwrap();
    let registered = json_request(app.clone(),"POST","/api/v1/agent/env-registration-leases/lease_register/register",json!({"manifest_json":manifest,"files":[{"file_name":"api.env","content_base64":STANDARD.encode(first_content)}]}),&[("authorization","Bearer register-token")]).await;
    assert_eq!(registered.status(), StatusCode::OK);
    assert_eq!(
        response_json(registered).await["created"],
        json!(["api.env"])
    );
    let original_ciphertext:Vec<u8>=sqlx::query_scalar("SELECT ciphertext FROM application_env_versions WHERE env_file_id=(SELECT id FROM application_env_files WHERE application_id='app_register' AND file_name='api.env')").fetch_one(&pool).await.unwrap();
    let lease_purpose: String = sqlx::query_scalar(
        "SELECT purpose FROM application_env_registration_leases WHERE id='lease_register'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lease_purpose, "env_registration");

    sqlx::query("UPDATE deployments SET status='succeeded',phase='completed',finished_at='2026-08-07T00:00:00Z' WHERE id='dep_register'").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES('dep_register_2','app_register','target_register',?,'running','preparing','register-key-2','request','snapshot')").bind(&admin_id).execute(&pool).await.unwrap();
    let second_content = b"SECRET=must-not-overwrite\n";
    let second_digest = format!("{:x}", Sha256::digest(second_content));
    let second_manifest=json!({"schema_version":1,"commit_sha":commit,"files":[{"file_name":"api.env","module":"api","sha256":second_digest,"size":second_content.len(),"format":"dotenv-v1"}]}).to_string();
    let second_manifest_digest = format!("{:x}", Sha256::digest(second_manifest.as_bytes()));
    sqlx::query("INSERT INTO application_env_registration_leases(id,application_id,deployment_id,agent_id,commit_sha,manifest_digest,status,expires_at) VALUES('lease_register_2','app_register','dep_register_2','agent_register',?,?,'active','2099-01-01T00:00:00Z')").bind(commit).bind(second_manifest_digest).execute(&pool).await.unwrap();
    let declared = json_request(
        app,
        "POST",
        "/api/v1/agent/env-registration-leases/lease_register_2/register",
        json!({"manifest_json":second_manifest,"files":[]}),
        &[("authorization", "Bearer register-token")],
    )
    .await;
    assert_eq!(declared.status(), StatusCode::OK);
    assert_eq!(
        response_json(declared).await["declared"],
        json!(["api.env"])
    );
    let current_version:i64=sqlx::query_scalar("SELECT current_version FROM application_env_files WHERE application_id='app_register' AND file_name='api.env'").fetch_one(&pool).await.unwrap();
    let ciphertext:Vec<u8>=sqlx::query_scalar("SELECT ciphertext FROM application_env_versions WHERE env_file_id=(SELECT id FROM application_env_files WHERE application_id='app_register' AND file_name='api.env')").fetch_one(&pool).await.unwrap();
    assert_eq!(current_version, 1);
    assert_eq!(ciphertext, original_ciphertext);
    let audit:String=sqlx::query_scalar("SELECT summary_json FROM audit_logs WHERE action='application_env.register' ORDER BY created_at DESC LIMIT 1").fetch_one(&pool).await.unwrap();
    assert!(!audit.contains("must-not-overwrite"));
}
