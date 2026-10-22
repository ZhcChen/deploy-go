mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;
use sqlx::SqlitePool;

async fn seed_application(pool: &SqlitePool, id: &str, name: &str) {
    sqlx::query(
        "INSERT INTO applications(id,name,slug,description,status) VALUES(?,?,?,'','active')",
    )
    .bind(id)
    .bind(name)
    .bind(name.to_lowercase())
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn administrator_creates_lists_revokes_and_updates_external_keys() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "one").await;
    seed_application(&pool, "app_two", "two").await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"CI 部署 Key","application_ids":["app_one"],"expires_at":"2099-01-01T00:00:00Z"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let token = created["token"].as_str().unwrap().to_owned();
    assert!(token.starts_with("dgx_"), "token 前缀不正确");
    let key_id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(created["application_ids"], json!(["app_one"]));

    let listed = json_request(
        app.clone(),
        "GET",
        "/api/v1/external-api-keys",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = response_json(listed).await;
    assert_eq!(listed["items"][0]["id"], json!(key_id));
    assert!(listed["items"][0].get("token").is_none());

    let shown = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/external-api-keys/{key_id}"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(shown.status(), StatusCode::OK);
    let shown = response_json(shown).await;
    assert_eq!(shown["name"], json!("CI 部署 Key"));

    let updated = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/external-api-keys/{key_id}/applications"),
        json!({"application_ids":["app_two"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["application_ids"], json!(["app_two"]));

    let revoked = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/external-api-keys/{key_id}/revoke"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::OK);
    let revoked = response_json(revoked).await;
    assert_eq!(revoked["status"], json!("disabled"));

    let audit = json_request(
        app.clone(),
        "GET",
        "/api/v1/audit-logs",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let audit = response_json(audit).await;
    let actions = audit["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["action"].as_str())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"external_api_key.create"));
    assert!(actions.contains(&"external_api_key.applications.update"));
    assert!(actions.contains(&"external_api_key.revoke"));

    let users = json_request(
        app,
        "GET",
        "/api/v1/users",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let users = response_json(users).await;
    let usernames = users["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["username"].as_str())
        .collect::<Vec<_>>();
    assert!(!usernames.contains(&"__deploy_go_external_api__"));
    let _ = pool;
}

#[tokio::test]
async fn external_api_key_creation_validates_applications_and_expiry() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "one").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let headers = [("cookie", cookie.as_str()), ("x-csrf-token", csrf.as_str())];

    let unknown_app = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"bad","application_ids":["app_missing"]}),
        &headers,
    )
    .await;
    assert_eq!(unknown_app.status(), StatusCode::NOT_FOUND);

    let duplicate_app = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"bad","application_ids":["app_one","app_one"]}),
        &headers,
    )
    .await;
    assert_eq!(duplicate_app.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expired = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"bad","application_ids":["app_one"],"expires_at":"2020-01-01T00:00:00Z"}),
        &headers,
    )
    .await;
    assert_eq!(expired.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn service_grants_are_synced_for_key_applications() {
    let (app, pool) = test_app().await;
    seed_application(&pool, "app_one", "one").await;
    seed_application(&pool, "app_two", "two").await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"grants","application_ids":["app_one"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    let created = response_json(created).await;
    let key_id = created["id"].as_str().unwrap().to_owned();
    let grants: Vec<String> = sqlx::query_scalar(
        "SELECT application_id FROM user_application_grants WHERE user_id='usr_external_api_service' ORDER BY application_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(grants, vec!["app_one".to_owned()]);

    let updated = json_request(
        app,
        "PUT",
        &format!("/api/v1/external-api-keys/{key_id}/applications"),
        json!({"application_ids":["app_two"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let grants: Vec<String> = sqlx::query_scalar(
        "SELECT application_id FROM user_application_grants WHERE user_id='usr_external_api_service' ORDER BY application_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(grants.contains(&"app_two".to_owned()));
}

async fn seed_node(pool: &SqlitePool, id: &str) {
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?,'/srv/apps','/srv/secrets','online')",
    )
    .bind(id)
    .bind(format!("节点 {id}"))
    .execute(pool)
    .await
    .unwrap();
}

async fn create_key_for(app: &axum::Router, cookie: &str, csrf: &str) -> String {
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"节点白名单 Key","application_ids":[]}),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn administrator_replaces_key_node_allowlist_and_audits_it() {
    let (app, pool) = test_app().await;
    seed_node(&pool, "node_b").await;
    seed_node(&pool, "node_a").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let key_id = create_key_for(&app, &cookie, &csrf).await;
    let path = format!("/api/v1/external-api-keys/{key_id}/nodes");

    // 新建的 Key 没有任何节点：默认拒绝。
    let shown = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/external-api-keys/{key_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(shown["node_ids"], json!([]));
    let version = shown["version"].as_i64().unwrap();

    let updated = json_request(
        app.clone(),
        "PUT",
        &path,
        json!({"node_ids":["node_b","node_a"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["node_ids"], json!(["node_a", "node_b"]));
    assert_eq!(updated["version"], json!(version + 1));

    let audits: Vec<(String, String)> = sqlx::query_as(
        "SELECT action,summary_json FROM audit_logs WHERE action='external_api_key.nodes.update'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(audits.len(), 1);
    assert!(audits[0].1.contains("node_a") && audits[0].1.contains("node_b"));

    // 整体替换：再次保存空集合会清空。
    let cleared = json_request(
        app.clone(),
        "PUT",
        &path,
        json!({"node_ids":[]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(cleared.status(), StatusCode::OK);
    assert_eq!(response_json(cleared).await["node_ids"], json!([]));
}

#[tokio::test]
async fn key_node_allowlist_rejects_invalid_requests_without_changes() {
    let (app, pool) = test_app().await;
    seed_node(&pool, "node_a").await;
    seed_node(&pool, "node_archived").await;
    sqlx::query("UPDATE nodes SET archived_at=datetime('now') WHERE id='node_archived'")
        .execute(&pool)
        .await
        .unwrap();
    let (cookie, csrf) = admin_session(app.clone()).await;
    let key_id = create_key_for(&app, &cookie, &csrf).await;
    let path = format!("/api/v1/external-api-keys/{key_id}/nodes");
    let headers = [("cookie", cookie.as_str()), ("x-csrf-token", csrf.as_str())];

    // 缺少 CSRF 被拒绝。
    let no_csrf = json_request(
        app.clone(),
        "PUT",
        &path,
        json!({"node_ids":["node_a"]}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(no_csrf.status(), StatusCode::FORBIDDEN);

    for (body, status) in [
        (json!({"node_ids":["node_missing"]}), StatusCode::NOT_FOUND),
        (json!({"node_ids":["node_archived"]}), StatusCode::NOT_FOUND),
        (
            json!({"node_ids":["node_a","node_a"]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (json!({"node_ids":[""]}), StatusCode::UNPROCESSABLE_ENTITY),
        (
            json!({"node_ids":[],"extra":1}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let response = json_request(app.clone(), "PUT", &path, body.clone(), &headers).await;
        assert_eq!(response.status(), status, "请求体 {body}");
    }

    let unknown_key = json_request(
        app.clone(),
        "PUT",
        "/api/v1/external-api-keys/key_missing/nodes",
        json!({"node_ids":["node_a"]}),
        &headers,
    )
    .await;
    assert_eq!(unknown_key.status(), StatusCode::NOT_FOUND);

    let granted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_api_key_nodes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(granted, 0, "被拒绝的请求不得写入任何节点");
}

#[tokio::test]
async fn non_administrator_cannot_change_key_node_allowlist() {
    let (app, pool) = test_app().await;
    seed_node(&pool, "node_a").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let key_id = create_key_for(&app, &cookie, &csrf).await;
    let response = json_request(
        app,
        "PUT",
        &format!("/api/v1/external-api-keys/{key_id}/nodes"),
        json!({"node_ids":["node_a"]}),
        &[],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn archived_nodes_are_hidden_from_the_summary_and_do_not_block_saving() {
    let (app, pool) = test_app().await;
    seed_node(&pool, "node_a").await;
    seed_node(&pool, "node_b").await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let key_id = create_key_for(&app, &cookie, &csrf).await;
    let path = format!("/api/v1/external-api-keys/{key_id}/nodes");
    let headers = [("cookie", cookie.as_str()), ("x-csrf-token", csrf.as_str())];
    let saved = json_request(
        app.clone(),
        "PUT",
        &path,
        json!({"node_ids":["node_a","node_b"]}),
        &headers,
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);

    sqlx::query("UPDATE nodes SET archived_at=datetime('now') WHERE id='node_a'")
        .execute(&pool)
        .await
        .unwrap();
    let shown = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/external-api-keys/{key_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(
        shown["node_ids"],
        json!(["node_b"]),
        "已归档节点不应再显示为已授权"
    );

    // 管理面按摘要回传（不含已归档节点）即可保存，不会被已归档节点卡住。
    let resaved = json_request(app.clone(), "PUT", &path, json!({"node_ids":[]}), &headers).await;
    assert_eq!(resaved.status(), StatusCode::OK);
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_api_key_nodes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0, "整体替换会一并清掉已归档节点的旧授权行");
}
