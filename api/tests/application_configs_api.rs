mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use deploy_go_api::crypto::{EncryptedSecret, MasterKeyRing};
use serde_json::json;

#[tokio::test]
async fn template_application_creation_clones_configuration_files_without_list_content() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({
            "name": "Postgres Workspace",
            "slug": "postgres-workspace",
            "environment": "prod",
            "app_type": "postgres",
            "type_version": "18",
            "template_id": "postgres"
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let application_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let list = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}/config-files"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let body = response_json(list).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert!(items.iter().all(|item| item.get("content").is_none()));
    assert!(items.iter().any(|item| {
        item["path"] == "postgres.env.example" && item["deploy_path"] == "postgres.env"
    }));
    let sensitive_file_id = items
        .iter()
        .find(|item| item["path"] == "postgres.env.example")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let sensitive_update = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/application-config-files/{sensitive_file_id}"),
        json!({"content":"POSTGRES_PASSWORD=weak-password\n","expected_version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(sensitive_update.status(), StatusCode::FORBIDDEN);
    let sensitive_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE action='application_config.update'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sensitive_audits, 0);

    let binding: (i64, String) = sqlx::query_as(
        "SELECT enabled,status FROM application_template_bindings WHERE application_id=?",
    )
    .bind(application_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(binding, (1, "active".to_owned()));
}

#[tokio::test]
async fn template_application_creation_rejects_mismatched_type_without_partial_state() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let response = json_request(
        app,
        "POST",
        "/api/v1/applications",
        json!({
            "name": "Mismatched Workspace",
            "slug": "mismatched-workspace",
            "environment": "prod",
            "app_type": "redis",
            "type_version": "7",
            "template_id": "postgres"
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let application_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM applications WHERE slug='mismatched-workspace'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let binding_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM application_template_bindings")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(application_count, 0);
    assert_eq!(binding_count, 0);
}

#[tokio::test]
async fn configuration_versions_are_optimistic_and_restore_creates_a_new_version() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({
            "name": "Redis Workspace",
            "slug": "redis-workspace",
            "environment": "prod",
            "app_type": "redis",
            "type_version": "7",
            "template_id": "redis"
        }),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let application_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let list = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/applications/{application_id}/config-files"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    let file = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path"] == "compose.env.example")
        .unwrap();
    let file_id = file["id"].as_str().unwrap();
    let update_path = format!("/api/v1/application-config-files/{file_id}");
    let update_headers = [("cookie", cookie.as_str()), ("x-csrf-token", csrf.as_str())];

    let update_a = json_request(
        app.clone(),
        "PUT",
        &update_path,
        json!({
            "content": "# keep this comment\nREDIS_PORT=6380\nTZ=Asia/Shanghai\n",
            "expected_version": 1
        }),
        &update_headers,
    );
    let update_b = json_request(
        app.clone(),
        "PUT",
        &update_path,
        json!({"content":"REDIS_PORT=6381\n", "expected_version":1}),
        &update_headers,
    );
    let (first, second) = tokio::join!(update_a, update_b);
    let (updated, stale) = if first.status() == StatusCode::OK {
        (first, second)
    } else {
        (second, first)
    };
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["current_version"], 2);
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(stale).await["code"],
        "resource_version_conflict"
    );

    let restored = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/application-config-files/{file_id}/restore"),
        json!({"version": 1, "expected_version": 2}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(restored.status(), StatusCode::OK);
    assert_eq!(response_json(restored).await["current_version"], 3);

    let versions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM application_config_versions WHERE application_config_file_id=?",
    )
    .bind(file_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(versions, 3);
}

#[tokio::test]
async fn legacy_image_initialization_is_idempotent_and_preserves_existing_env() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    sqlx::query("INSERT INTO applications(id,name,slug,app_type,type_version,environment,status) VALUES('app_legacy_config','Legacy Redis','legacy-config','redis','7','prod','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_legacy_config','Legacy Node','/srv/apps','/srv/secrets','offline')")
        .execute(&pool)
        .await
        .unwrap();
    let image_spec = r#"{"template":"redis","image":"redis:7-alpine","host_port":6379,"env_files":["compose.env","redis.env"]}"#;
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,privileged_release,image_spec_json,status) VALUES('target_legacy_config','app_legacy_config','node_legacy_config','prod','image','',60,1,?,'active')")
        .bind(image_spec)
        .execute(&pool)
        .await
        .unwrap();
    let ring = MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap();
    let original = b"REDIS_PASSWORD=old-secret\nTZ=Asia/Shanghai\n";
    let encrypted = ring
        .encrypt_application_env(
            "app_legacy_config",
            "legacy_env_redis",
            "legacy_env_version",
            original,
        )
        .unwrap();
    sqlx::query("INSERT INTO application_env_files(id,application_id,file_name,module,format,current_digest) VALUES('legacy_env_redis','app_legacy_config','REDIS.ENV','redis','dotenv-v1','legacy-digest')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_env_versions(id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest) VALUES('legacy_env_version','legacy_env_redis',1,'chacha20poly1305-application-env-v1',?,?,?,'legacy-digest')")
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce)
        .bind(encrypted.key_version)
        .execute(&pool)
        .await
        .unwrap();

    let initialized = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_legacy_config/config-files/initialize",
        json!({"target_id":"target_legacy_config"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(initialized.status(), StatusCode::OK);
    let initialized = response_json(initialized).await;
    assert_eq!(initialized["created"], true);
    assert_eq!(initialized["status"], "draft");
    let binding_id = initialized["binding_id"].as_str().unwrap().to_owned();

    let repeated = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_legacy_config/config-files/initialize",
        json!({"target_id":"target_legacy_config"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(repeated.status(), StatusCode::OK);
    assert_eq!(response_json(repeated).await["created"], false);
    let env_version: i64 = sqlx::query_scalar(
        "SELECT current_version FROM application_env_files WHERE id='legacy_env_redis'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(env_version, 1);
    let (config_file_id, config_version_id): (String, String) = sqlx::query_as(
        "SELECT f.id,v.id FROM application_config_files f JOIN application_config_versions v ON v.application_config_file_id=f.id AND v.config_version=f.current_version WHERE f.application_id='app_legacy_config' AND f.deploy_path='redis.env'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let encrypted_config: (Vec<u8>, Vec<u8>, i64) = sqlx::query_as(
        "SELECT ciphertext,nonce,key_version FROM application_config_versions WHERE id=?",
    )
    .bind(&config_version_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let decrypted = ring
        .decrypt_application_config(
            "app_legacy_config",
            &config_file_id,
            &config_version_id,
            &EncryptedSecret {
                ciphertext: encrypted_config.0,
                nonce: encrypted_config.1,
                key_version: encrypted_config.2,
            },
        )
        .unwrap();
    assert_eq!(decrypted.as_slice(), original);

    let conflict = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/app_legacy_config/config-files/initialize",
        json!({"target_id":"target_legacy_config","template_id":"postgres"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(conflict).await["code"],
        "application_config_template_conflict"
    );

    let deleted = json_request(
        app.clone(),
        "DELETE",
        "/api/v1/applications/app_legacy_config/config-files",
        json!({"binding_id":binding_id}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    let deleted_at: Option<String> = sqlx::query_scalar(
        "SELECT deleted_at FROM application_config_files WHERE application_id='app_legacy_config' AND deploy_path='redis.env'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(deleted_at.is_some());
    let deleted_file = json_request(
        app,
        "GET",
        &format!("/api/v1/application-config-files/{config_file_id}"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(deleted_file.status(), StatusCode::NOT_FOUND);
}
