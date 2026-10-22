//! 对外应用管理（specs/005 阶段 D）：应用启停、Env 同步重试、配置文件元数据。
//! 写入必须经过 Key 绑定与非生产守卫；配置文件只返回元数据，永不返回内容。
mod common;

use axum::{Router, http::StatusCode};
use common::{ADMIN_PASSWORD, admin_session, json_request, login, response_json, test_app};
use serde_json::{Value, json};
use sqlx::SqlitePool;

const PROD_APP_FORBIDDEN: &str = "external_production_application_forbidden";
const SECRET_MARKER: &str = "TOP_SECRET_CONFIG_VALUE_9f3a";

struct Fixture {
    app: Router,
    pool: SqlitePool,
    auth: String,
}

/// 非生产应用 `app_test`（带目标 `target_test`）、生产应用 `app_prod`、未绑定应用 `app_other`。
async fn fixture() -> Fixture {
    let (app, pool) = test_app().await;
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_a','节点 A','/srv/apps','/srv/secrets','online')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json) VALUES('agent_a','node_a','test','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')",
    )
    .execute(&pool)
    .await
    .unwrap();
    for (id, environment) in [
        ("app_test", "test"),
        ("app_prod", "prod"),
        ("app_other", "test"),
    ] {
        sqlx::query(
            "INSERT INTO applications(id,name,slug,description,status,environment) VALUES(?,?,?,'','active',?)",
        )
        .bind(id)
        .bind(id)
        .bind(id.replace('_', "-"))
        .bind(environment)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_test','app_test','node_a','test','/srv/deploy.sh',60,'active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/external-api-keys",
            json!({"name":"应用管理 Key","application_ids":["app_test","app_prod"]}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    Fixture {
        app,
        pool,
        auth: format!("Bearer {}", created["token"].as_str().unwrap()),
    }
}

impl Fixture {
    async fn call(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value, String) {
        let response = json_request(
            self.app.clone(),
            method,
            path,
            body,
            &[
                ("authorization", &self.auth),
                ("idempotency-key", "external-app-mgmt-0001"),
            ],
        )
        .await;
        let status = response.status();
        let cache = response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned();
        (status, response_json(response).await, cache)
    }

    async fn application_version(&self, id: &str) -> i64 {
        sqlx::query_scalar("SELECT version FROM applications WHERE id=?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn archive_and_reactivate_application_with_optimistic_lock_and_audit() {
    let f = fixture().await;
    let path = "/external/v1/applications/app_test/status";
    let version = f.application_version("app_test").await;

    let (status, archived, _) = f
        .call("PUT", path, json!({"status":"archived","version":version}))
        .await;
    assert_eq!(status, StatusCode::OK, "{archived}");
    assert_eq!(archived["status"], json!("archived"));
    assert_eq!(archived["version"], json!(version + 1));
    let mut keys: Vec<&str> = archived
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["id", "status", "version"],
        "响应只含 id/status/version"
    );

    // 归档后应用从 Key 的应用列表和部署入口消失，但仍可经状态接口重新启用。
    let (_, listed, _) = f.call("GET", "/external/v1/applications", json!({})).await;
    let ids: Vec<&str> = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect();
    assert!(
        !ids.contains(&"app_test"),
        "归档应用不应出现在列表: {ids:?}"
    );
    let (status, _, _) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/deployments",
            json!({"parameters":{}}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "归档应用不能再发起部署");

    // 过期 version 为 409，且不改变状态。
    let (status, body, _) = f
        .call("PUT", path, json!({"status":"active","version":version}))
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::CONFLICT, &json!("resource_version_conflict"))
    );

    let (status, active, _) = f
        .call(
            "PUT",
            path,
            json!({"status":"active","version":version + 1}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{active}");
    assert_eq!(active["status"], json!("active"));
    let (status, _, _) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/deployments",
            json!({"parameters":{}}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "重新启用后可以部署");

    let audits: Vec<String> = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE action='application.status.update' ORDER BY created_at,id",
    )
    .fetch_all(&f.pool)
    .await
    .unwrap();
    assert_eq!(audits.len(), 2);
    assert!(
        audits
            .iter()
            .all(|summary| summary.contains("external_api_key_id"))
    );
}

#[tokio::test]
async fn application_status_rejects_invalid_requests_without_side_effects() {
    let f = fixture().await;
    let version = f.application_version("app_test").await;
    let path = "/external/v1/applications/app_test/status";

    for body in [
        json!({"status":"deleted","version":version}),
        json!({"status":"active"}),
        json!({"status":"active","version":version,"environment":"prod"}),
    ] {
        let (status, _, _) = f.call("PUT", path, body.clone()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
    assert_eq!(f.application_version("app_test").await, version);

    // 生产应用（含已归档的生产应用）与未绑定应用。
    let prod_version = f.application_version("app_prod").await;
    let (status, body, _) = f
        .call(
            "PUT",
            "/external/v1/applications/app_prod/status",
            json!({"status":"archived","version":prod_version}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(PROD_APP_FORBIDDEN))
    );
    sqlx::query("UPDATE applications SET status='archived' WHERE id='app_prod'")
        .execute(&f.pool)
        .await
        .unwrap();
    let (status, body, _) = f
        .call(
            "PUT",
            "/external/v1/applications/app_prod/status",
            json!({"status":"active","version":prod_version}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(PROD_APP_FORBIDDEN)),
        "已归档的生产应用也不能经对外 API 重新启用"
    );
    let (status, _, _) = f
        .call(
            "PUT",
            "/external/v1/applications/app_other/status",
            json!({"status":"archived","version":1}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let other: String = sqlx::query_scalar("SELECT status FROM applications WHERE id='app_other'")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(other, "active");
}

async fn register_env(f: &Fixture, application_id: &str) -> (String, i64) {
    let (status, registered, _) = f
        .call(
            "POST",
            &format!("/external/v1/applications/{application_id}/env-files"),
            json!({"files":[{"file_name":"api.env","module":"api","format":"dotenv-v1","content":"A=1\n"}]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{registered}");
    let (_, listed, _) = f
        .call(
            "GET",
            &format!("/external/v1/applications/{application_id}/env-files"),
            json!({}),
        )
        .await;
    (
        listed["items"][0]["id"].as_str().unwrap().to_owned(),
        listed["items"][0]["version"].as_i64().unwrap(),
    )
}

#[tokio::test]
async fn env_sync_retry_requeues_only_failed_syncs_of_the_owned_file() {
    let f = fixture().await;
    let (env_file_id, _) = register_env(&f, "app_test").await;
    let sync_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM application_env_syncs")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(sync_count, 1, "登记会为目标产生一条待同步记录");
    let path = format!("/external/v1/applications/app_test/env-files/{env_file_id}/sync-retry");

    // 没有失败的同步：重试 0 条。
    let (status, body, _) = f.call("POST", &path, json!({})).await;
    assert_eq!((status, &body["retried"]), (StatusCode::OK, &json!(0)));

    sqlx::query("UPDATE application_env_syncs SET status='failed',error_code='sync_failed',error_message='boom'")
        .execute(&f.pool)
        .await
        .unwrap();
    // 指定不存在的目标：不影响任何同步。
    let (status, body, _) = f
        .call(
            "POST",
            &format!("{path}?target_id=target_missing"),
            json!({}),
        )
        .await;
    assert_eq!((status, &body["retried"]), (StatusCode::OK, &json!(0)));
    let (status, body, _) = f.call("POST", &path, json!({})).await;
    assert_eq!(
        (status, &body["retried"]),
        (StatusCode::OK, &json!(1)),
        "{body}"
    );
    let (state, error): (String, Option<String>) =
        sqlx::query_as("SELECT status,error_code FROM application_env_syncs")
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!((state.as_str(), error), ("pending", None));
    assert!(!body.to_string().contains("A=1"), "响应不含 Env 内容");

    let audit: String = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE action='application_env.sync_retry' ORDER BY created_at DESC,id DESC LIMIT 1",
    )
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert!(audit.contains("external_api_key_id") && !audit.contains("A=1"));

    // 其他应用的 Env 文件不能借本应用路径重试：404；生产应用 403。
    let (status, _, _) = f
        .call(
            "POST",
            &format!("/external/v1/applications/app_prod/env-files/{env_file_id}/sync-retry"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    sqlx::query("INSERT INTO external_api_key_applications(api_key_id,application_id,granted_by) SELECT id,'app_other',created_by FROM external_api_keys")
        .execute(&f.pool)
        .await
        .unwrap();
    let (status, _, _) = f
        .call(
            "POST",
            &format!("/external/v1/applications/app_other/env-files/{env_file_id}/sync-retry"),
            json!({}),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Env 文件必须属于路径中的应用"
    );
}

async fn seed_config_files(pool: &SqlitePool, application_id: &str) {
    sqlx::query("INSERT INTO application_template_bindings(id,application_id,template_id,template_version,template_digest,source,status) VALUES(?,?,'redis','7','template-digest','template_creation','active')")
        .bind(format!("binding_{application_id}"))
        .bind(application_id)
        .execute(pool)
        .await
        .unwrap();
    for (id, path, sensitive, deleted) in [
        ("cfg_plain", "compose.yml", 0, false),
        ("cfg_secret", "secrets.env", 1, false),
        ("cfg_deleted", "old.env", 0, true),
    ] {
        sqlx::query("INSERT INTO application_config_files(id,binding_id,application_id,path,deploy_path,label,format,language,role,delivery,sensitive,editable,description,recommended_changes,template_source_digest,current_digest,deleted_at) VALUES(?,?,?,?,?,'标签','dotenv','dotenv','configuration','artifact',?,1,'内部说明文本','建议修改项','template-digest-visible','current-digest-visible',?)")
            .bind(format!("{id}_{application_id}"))
            .bind(format!("binding_{application_id}"))
            .bind(application_id)
            .bind(path)
            .bind(format!("deploy/{path}"))
            .bind(sensitive)
            .bind(deleted.then_some("2026-10-01T00:00:00Z"))
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO application_config_versions(id,application_config_file_id,application_id,config_version,algorithm,ciphertext,nonce,key_version,digest,source) VALUES(?,?,?,1,'chacha20poly1305-application-config-v1',?,X'02',1,'digest','template')")
            .bind(format!("ver_{id}_{application_id}"))
            .bind(format!("{id}_{application_id}"))
            .bind(application_id)
            .bind(SECRET_MARKER.as_bytes())
            .execute(pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn config_file_listing_returns_metadata_only() {
    let f = fixture().await;
    seed_config_files(&f.pool, "app_test").await;

    let (status, listed, cache) = f
        .call(
            "GET",
            "/external/v1/applications/app_test/config-files",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(cache, "no-store");
    let items = listed["items"].as_array().unwrap();
    let paths: Vec<&str> = items
        .iter()
        .map(|item| item["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        paths,
        vec!["compose.yml", "secrets.env"],
        "已删除的文件不列出"
    );

    let mut plain_keys: Vec<&str> = items[0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    plain_keys.sort();
    assert_eq!(
        plain_keys,
        vec![
            "current_digest",
            "current_version",
            "delivery",
            "editable",
            "format",
            "id",
            "label",
            "path",
            "role",
            "sensitive",
            "status",
            "updated_at",
            "version"
        ]
    );
    assert!(items[0]["current_digest"].is_string(), "非敏感文件返回摘要");
    assert_eq!(items[1]["sensitive"], json!(true));
    assert!(
        items[1].get("current_digest").is_none(),
        "敏感文件不返回摘要"
    );

    let text = listed.to_string();
    for forbidden in [
        SECRET_MARKER,
        "content",
        "deploy/",
        "内部说明文本",
        "建议修改项",
        "template-digest-visible",
        "binding_",
        "ciphertext",
    ] {
        assert!(
            !text.contains(forbidden),
            "响应不应包含 {forbidden}: {text}"
        );
    }
}

#[tokio::test]
async fn config_file_listing_is_guarded_by_binding_and_environment() {
    let f = fixture().await;
    seed_config_files(&f.pool, "app_prod").await;
    let (status, body, _) = f
        .call(
            "GET",
            "/external/v1/applications/app_prod/config-files",
            json!({}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(PROD_APP_FORBIDDEN))
    );
    assert!(!body.to_string().contains("compose.yml"));
    let (status, _, _) = f
        .call(
            "GET",
            "/external/v1/applications/app_other/config-files",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // 未配置模板的非生产应用：空列表而不是错误。
    let (status, listed, _) = f
        .call(
            "GET",
            "/external/v1/applications/app_test/config-files",
            json!({}),
        )
        .await;
    assert_eq!((status, &listed["items"]), (StatusCode::OK, &json!([])));
}

/// 通过管理接口以管理员身份归档应用（审计里没有外部 Key 标识）。
async fn archive_as_administrator(f: &Fixture, application_id: &str) -> i64 {
    let (cookie, csrf) = login(f.app.clone(), "admin", ADMIN_PASSWORD).await;
    let version = f.application_version(application_id).await;
    let response = json_request(
        f.app.clone(),
        "PUT",
        &format!("/api/v1/applications/{application_id}/status"),
        json!({"status":"archived","version":version}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    version + 1
}

#[tokio::test]
async fn key_cannot_reactivate_an_application_archived_by_an_administrator() {
    let f = fixture().await;
    let version = archive_as_administrator(&f, "app_test").await;
    let (status, body, _) = f
        .call(
            "PUT",
            "/external/v1/applications/app_test/status",
            json!({"status":"active","version":version}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (
            StatusCode::FORBIDDEN,
            &json!("external_application_reactivation_forbidden")
        ),
        "{body}"
    );
    let state: String = sqlx::query_scalar("SELECT status FROM applications WHERE id='app_test'")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(state, "archived", "管理员归档的应用不能被 Key 恢复");
    assert_eq!(f.application_version("app_test").await, version);
}

#[tokio::test]
async fn key_cannot_reactivate_an_application_archived_by_another_key() {
    let f = fixture().await;
    let (cookie, csrf) = login(f.app.clone(), "admin", ADMIN_PASSWORD).await;
    let other = response_json(
        json_request(
            f.app.clone(),
            "POST",
            "/api/v1/external-api-keys",
            json!({"name":"另一个 Key","application_ids":["app_test"]}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let other_auth = format!("Bearer {}", other["token"].as_str().unwrap());
    let version = f.application_version("app_test").await;
    let archived = json_request(
        f.app.clone(),
        "PUT",
        "/external/v1/applications/app_test/status",
        json!({"status":"archived","version":version}),
        &[("authorization", &other_auth)],
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    let (status, body, _) = f
        .call(
            "PUT",
            "/external/v1/applications/app_test/status",
            json!({"status":"active","version":version + 1}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (
            StatusCode::FORBIDDEN,
            &json!("external_application_reactivation_forbidden")
        )
    );
}

#[tokio::test]
async fn env_sync_retry_refuses_non_production_application_with_a_production_target() {
    let f = fixture().await;
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_prod','正式节点','/srv/apps','/srv/secrets','online')")
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_prod_drift','app_test','node_prod','prod','/srv/deploy.sh',60,'active')")
        .execute(&f.pool)
        .await
        .unwrap();
    let (env_file_id, _) = register_env(&f, "app_test").await;
    sqlx::query("UPDATE application_env_syncs SET status='failed',error_code='sync_failed'")
        .execute(&f.pool)
        .await
        .unwrap();
    let (status, body, _) = f
        .call(
            "POST",
            &format!("/external/v1/applications/app_test/env-files/{env_file_id}/sync-retry"),
            json!({}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (
            StatusCode::FORBIDDEN,
            &json!("external_production_deployment_forbidden")
        )
    );
    let failed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM application_env_syncs WHERE status='failed'")
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert!(failed >= 1, "被拒绝的重试不得重新排队任何同步");
}
