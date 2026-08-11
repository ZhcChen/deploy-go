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
async fn env_metadata_includes_sanitized_per_target_syncs_for_granted_users() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    seed_application(&pool, "app_env_list", "list").await;
    seed_application(&pool, "app_env_hidden", "hidden").await;
    seed_env(&pool, "app_env_list", "env_list", "SECRET=initial\n").await;
    for (suffix, status) in [
        ("failed_a", "failed"),
        ("failed_b", "failed"),
        ("succeeded", "succeeded"),
    ] {
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES(?,?, 'offline')")
            .bind(format!("node_{suffix}"))
            .bind(format!("Node {suffix}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES(?,'app_env_list',?,'prod','/unused',60,'active')")
            .bind(format!("target_{suffix}"))
            .bind(format!("node_{suffix}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,status,actual_version,attempt_count,error_code,error_message,last_attempt_at,synced_at) VALUES(?, 'env_list_version_1',?,?, ?,?,1,?,?,?,?)")
            .bind(format!("sync_{suffix}"))
            .bind(format!("target_{suffix}"))
            .bind(format!("node_{suffix}"))
            .bind(status)
            .bind((status == "succeeded").then_some(1_i64))
            .bind((status == "failed").then_some("env_sync_digest_mismatch"))
            .bind((status == "failed").then_some("SECRET=must-not-leak"))
            .bind("2026-08-07T03:00:00Z")
            .bind((status == "succeeded").then_some("2026-08-07T03:00:01Z"))
            .execute(&pool)
            .await
            .unwrap();
    }

    let admin_list = json_request(
        app.clone(),
        "GET",
        "/api/v1/applications/app_env_list/env-files",
        json!(null),
        &[("cookie", &admin_cookie)],
    )
    .await;
    assert_eq!(admin_list.status(), StatusCode::OK);
    let body = response_json(admin_list).await;
    assert_eq!(body["items"][0]["syncs"].as_array().unwrap().len(), 3);
    let failed = body["items"][0]["syncs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|sync| sync["target_id"] == "target_failed_a")
        .unwrap();
    assert_eq!(failed["node_id"], "node_failed_a");
    assert_eq!(failed["node_name"], "Node failed_a");
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["actual_version"], json!(null));
    assert_eq!(failed["last_attempt_at"], "2026-08-07T03:00:00Z");
    assert_eq!(failed["synced_at"], json!(null));
    assert_eq!(failed["error_code"], "env_sync_digest_mismatch");
    assert_eq!(failed["error_message"], "Env 同步失败");
    assert!(!body.to_string().contains("must-not-leak"));

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({"username":"env-reader", "password":"env-reader-password"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let user_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let admin_id: String =
        sqlx::query_scalar("SELECT id FROM users WHERE identity='administrator'")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO user_application_grants(user_id,application_id,granted_by) VALUES(?,'app_env_list',?)").bind(&user_id).bind(admin_id).execute(&pool).await.unwrap();
    let (user_cookie, user_csrf) =
        common::login(app.clone(), "env-reader", "env-reader-password").await;
    let granted = json_request(
        app.clone(),
        "GET",
        "/api/v1/applications/app_env_list/env-files",
        json!(null),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(granted.status(), StatusCode::OK);
    assert_eq!(
        response_json(granted).await["items"][0]["syncs"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let hidden = json_request(
        app.clone(),
        "GET",
        "/api/v1/applications/app_env_hidden/env-files",
        json!(null),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
    let plaintext = json_request(
        app.clone(),
        "GET",
        "/api/v1/application-env-files/env_list",
        json!(null),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(plaintext.status(), StatusCode::FORBIDDEN);
    let user_retry = json_request(
        app.clone(),
        "POST",
        "/api/v1/application-env-files/env_list/sync-retry?target_id=target_failed_a",
        json!(null),
        &[("cookie", &user_cookie), ("x-csrf-token", &user_csrf)],
    )
    .await;
    assert_eq!(user_retry.status(), StatusCode::FORBIDDEN);

    let retried = json_request(
        app,
        "POST",
        "/api/v1/application-env-files/env_list/sync-retry?target_id=target_failed_a",
        json!(null),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(retried.status(), StatusCode::OK);
    assert_eq!(response_json(retried).await["retried"], 1);
    let states: Vec<(String, String)> = sqlx::query_as("SELECT target_id,status FROM application_env_syncs WHERE env_version_id='env_list_version_1' ORDER BY target_id").fetch_all(&pool).await.unwrap();
    assert_eq!(
        states,
        [
            ("target_failed_a".to_owned(), "pending".to_owned()),
            ("target_failed_b".to_owned(), "failed".to_owned()),
            ("target_succeeded".to_owned(), "succeeded".to_owned())
        ]
    );
}

#[tokio::test]
async fn plaintext_crud_requires_admin_reauthentication_csrf_and_optimistic_version() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    seed_application(&pool, "app_env_a", "a").await;
    seed_application(&pool, "app_env_b", "b").await;
    seed_env(&pool, "app_env_a", "env_a", "SECRET=initial\n").await;
    seed_env(&pool, "app_env_b", "env_b", "OTHER=initial\n").await;
    for suffix in ["failed", "succeeded"] {
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES(?,?, 'offline')")
            .bind(format!("node_env_{suffix}"))
            .bind(format!("Env {suffix}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,protocol_version) VALUES(?,?,4)")
            .bind(format!("agent_env_{suffix}"))
            .bind(format!("node_env_{suffix}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES(?,'app_env_a',?,'prod','/unused',60,'active')")
            .bind(format!("target_env_{suffix}"))
            .bind(format!("node_env_{suffix}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO application_env_syncs(id,env_version_id,target_id,node_id,agent_id,status,actual_version) VALUES(?, 'env_a_version_1',?,?,?, ?,?)")
            .bind(format!("sync_env_{suffix}"))
            .bind(format!("target_env_{suffix}"))
            .bind(format!("node_env_{suffix}"))
            .bind(format!("agent_env_{suffix}"))
            .bind(suffix)
            .bind((suffix == "succeeded").then_some(1_i64))
            .execute(&pool)
            .await
            .unwrap();
    }

    let retried = json_request(
        app.clone(),
        "POST",
        "/api/v1/application-env-files/env_a/sync-retry",
        json!(null),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(retried.status(), StatusCode::OK);
    assert_eq!(response_json(retried).await["retried"], 1);
    let sync_states: Vec<String> = sqlx::query_scalar(
        "SELECT status FROM application_env_syncs WHERE env_version_id='env_a_version_1' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(sync_states, ["pending", "succeeded"]);

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

#[tokio::test]
async fn admin_registration_creates_initial_env_without_agent_lease() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    seed_application(&pool, "app_env_admin", "admin-register").await;
    seed_application(&pool, "app_env_admin_other", "admin-register-other").await;
    let created_user = json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({"username":"env-admin-reader","password":"env-admin-reader-password"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created_user.status(), StatusCode::CREATED);
    let (user_cookie, user_csrf) =
        common::login(app.clone(), "env-admin-reader", "env-admin-reader-password").await;
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node_admin_register','Admin Register Node','offline')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_admin_register','app_env_admin','node_admin_register','prod','/unused',60,'active')")
        .execute(&pool)
        .await
        .unwrap();

    let grant_response = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_admin/env-reveal-grants",
        json!({"password":ADMIN_PASSWORD,"action":"read_write"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(grant_response.status(), StatusCode::OK);
    let grant = response_json(grant_response).await["grant_token"]
        .as_str()
        .unwrap()
        .to_owned();

    let missing_grant = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_admin/env-files/register",
        json!({"files":[{"file_name":"api.env","module":"api","format":"dotenv-v1","content":"SECRET=initial\n"}]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    let cross_application = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_admin_other/env-files/register",
        json!({"files":[{"file_name":"api.env","module":"api","format":"dotenv-v1","content":"SECRET=other\n"}]}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(cross_application.status(), StatusCode::FORBIDDEN);

    let content_api = "SECRET=api-initial\nPORT=8080\n";
    let content_redis = "REDIS_PASSWORD=redis-initial\n";
    let registered = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_admin/env-files/register",
        json!({"files":[
            {"file_name":"api.env","module":"api","format":"dotenv-v1","content":content_api},
            {"file_name":"redis.env","module":"redis","format":"dotenv-v1","content":content_redis}
        ]}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(registered.status(), StatusCode::OK);
    assert_eq!(registered.headers()["cache-control"], "no-store");
    assert_eq!(
        response_json(registered).await["created"],
        json!(["api.env", "redis.env"])
    );

    let api_file_id: String = sqlx::query_scalar(
        "SELECT id FROM application_env_files WHERE application_id='app_env_admin' AND file_name='api.env'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let (current_version, current_digest): (i64, String) = sqlx::query_as(
        "SELECT current_version,current_digest FROM application_env_files WHERE id=?",
    )
    .bind(&api_file_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(current_version, 1);
    assert_eq!(
        current_digest,
        format!("{:x}", Sha256::digest(content_api.as_bytes()))
    );
    let (ciphertext, stored_digest): (Vec<u8>, String) = sqlx::query_as(
        "SELECT ciphertext,digest FROM application_env_versions WHERE env_file_id=? AND env_version=1",
    )
    .bind(&api_file_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored_digest, current_digest);
    assert!(
        !ciphertext
            .windows(content_api.len())
            .any(|window| window == content_api.as_bytes())
    );
    let sync_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM application_env_syncs WHERE env_version_id=(SELECT id FROM application_env_versions WHERE env_file_id=? AND env_version=1) AND status='pending'",
    )
    .bind(&api_file_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sync_count, 1);

    let audit: String = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE action='application_env.register_admin' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!audit.contains("api-initial") && !audit.contains("redis-initial"));

    let duplicate = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_admin/env-files/register",
        json!({"files":[{"file_name":"api.env","module":"api","format":"dotenv-v1","content":"SECRET=overwrite\n"}]}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(duplicate).await["code"],
        "env_file_already_registered"
    );

    let invalid = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_admin/env-files/register",
        json!({"files":[{"file_name":"broken.env","module":"api","format":"dotenv-v1","content":"SECRET=must-not-leak\nSECRET=duplicate\n"}]}),
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

    let non_admin = json_request(
        app,
        "POST",
        "/api/v1/applications/app_env_admin/env-files/register",
        json!({"files":[{"file_name":"api.env","module":"api","format":"dotenv-v1","content":"SECRET=denied\n"}]}),
        &[
            ("cookie", &user_cookie),
            ("x-csrf-token", &user_csrf),
            ("x-env-reveal-grant", &grant),
        ],
    )
    .await;
    assert_eq!(non_admin.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn delete_env_rejects_files_referenced_by_active_image_targets() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    seed_application(&pool, "app_env_image_ref", "image-ref").await;
    seed_env(&pool, "app_env_image_ref", "compose", "SECRET=compose\n").await;
    seed_env(
        &pool,
        "app_env_image_ref",
        "redis",
        "REDIS_PASSWORD=redis\n",
    )
    .await;
    sqlx::query("INSERT INTO nodes(id,name,status,work_root,secrets_root) VALUES('node_env_image_ref','Image Ref Node','online','/srv/apps','/srv/secrets')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_env_image_ref','node_env_image_ref','2026-08-11T00:00:00Z','2026-08-11T00:00:00Z','0.3.0',8,'[\"privileged_release\"]')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status,work_root,secrets_root) VALUES('node_env_image_ref_b','Image Ref Node B','online','/srv/apps','/srv/secrets')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES('agent_env_image_ref_b','node_env_image_ref_b','2026-08-11T00:00:00Z','2026-08-11T00:00:00Z','0.3.0',8,'[\"privileged_release\"]')")
        .execute(&pool)
        .await
        .unwrap();
    let image_spec = |env_files: &str| {
        format!(
            r#"{{"template":"redis","image":"docker.io/library/redis:7-alpine","host_port":6379,"env_files":{env_files}}}"#
        )
    };
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,parameter_schema,timeout_seconds,verification_config,privileged_release,image_spec_json,status) VALUES('target_env_image_a','app_env_image_ref','node_env_image_ref','prod','image','','{}',60,'{}',1,?,'active')")
        .bind(image_spec(r#"["compose.env","redis.env"]"#))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,parameter_schema,timeout_seconds,verification_config,privileged_release,image_spec_json,status) VALUES('target_env_image_b','app_env_image_ref','node_env_image_ref_b','prod','image','','{}',60,'{}',1,?,'active')")
        .bind(image_spec(r#"["redis.env"]"#))
        .execute(&pool)
        .await
        .unwrap();

    let grant_response = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_env_image_ref/env-reveal-grants",
        json!({"password":ADMIN_PASSWORD,"action":"delete"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(grant_response.status(), StatusCode::OK);
    let delete_grant = response_json(grant_response).await["grant_token"]
        .as_str()
        .unwrap()
        .to_owned();
    let compose_id: String = sqlx::query_scalar(
        "SELECT id FROM application_env_files WHERE application_id='app_env_image_ref' AND file_name='compose.env'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let redis_id: String = sqlx::query_scalar(
        "SELECT id FROM application_env_files WHERE application_id='app_env_image_ref' AND file_name='redis.env'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let blocked = json_request(
        app.clone(),
        "DELETE",
        &format!("/api/v1/application-env-files/{compose_id}"),
        json!({"expected_version":1,"confirm_file_name":"compose.env"}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &delete_grant),
        ],
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::CONFLICT);
    let blocked = response_json(blocked).await;
    assert_eq!(blocked["code"], "env_file_referenced_by_image_target");
    assert_eq!(blocked["details"]["target_count"], 1);
    assert_eq!(blocked["details"]["target_ids"][0], "target_env_image_a");

    sqlx::query("UPDATE deployment_targets SET image_spec_json=? WHERE id='target_env_image_a'")
        .bind(image_spec(r#"["redis.env"]"#))
        .execute(&pool)
        .await
        .unwrap();
    let released = json_request(
        app.clone(),
        "DELETE",
        &format!("/api/v1/application-env-files/{compose_id}"),
        json!({"expected_version":1,"confirm_file_name":"compose.env"}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &delete_grant),
        ],
    )
    .await;
    assert_eq!(released.status(), StatusCode::NO_CONTENT);

    let redis_blocked = json_request(
        app.clone(),
        "DELETE",
        &format!("/api/v1/application-env-files/{redis_id}"),
        json!({"expected_version":1,"confirm_file_name":"redis.env"}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &delete_grant),
        ],
    )
    .await;
    assert_eq!(redis_blocked.status(), StatusCode::CONFLICT);
    sqlx::query("UPDATE deployment_targets SET image_spec_json=? WHERE id='target_env_image_a'")
        .bind(image_spec(r#"["compose.env"]"#))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_targets SET status='disabled' WHERE id='target_env_image_b'")
        .execute(&pool)
        .await
        .unwrap();
    let disabled_released = json_request(
        app,
        "DELETE",
        &format!("/api/v1/application-env-files/{redis_id}"),
        json!({"expected_version":1,"confirm_file_name":"redis.env"}),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("x-env-reveal-grant", &delete_grant),
        ],
    )
    .await;
    assert_eq!(disabled_released.status(), StatusCode::NO_CONTENT);
}
