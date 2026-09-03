mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;

#[tokio::test]
async fn application_visibility_follows_grants_and_mutations_require_admin() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({"name":"Example API","slug":"example-api","description":"Example","environment":"prod"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let application = response_json(created).await;
    let application_id = application["id"].as_str().unwrap();
    assert_eq!(application["environment"], "prod");
    assert!(application["last_deployed_at"].is_null());
    let user = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/users",
            json!({"username":"operator","password":"operator-password-long"}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let user_id = user["id"].as_str().unwrap();
    let (user_cookie, _) = common::login(app.clone(), "operator", "operator-password-long").await;

    let hidden = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
    let grant = json_request(
        app.clone(),
        "PUT",
        &format!("/api/v1/users/{user_id}/applications/{application_id}"),
        json!({}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(grant.status(), StatusCode::NO_CONTENT);
    let visible = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications/{application_id}"),
        json!({}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(visible.status(), StatusCode::OK);
    let forbidden = json_request(
        app.clone(),
        "PATCH",
        &format!("/api/v1/applications/{application_id}"),
        json!({"name":"Changed","slug":"changed-app","description":"","environment":"prod","version":1}),
        &[("cookie", &user_cookie)],
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let archived = json_request(
        app,
        "PUT",
        &format!("/api/v1/applications/{application_id}/status"),
        json!({"status":"archived","version":1}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    let actions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE resource_id=? AND action LIKE 'application.%'",
    )
    .bind(application_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(actions, 3);
}

#[tokio::test]
async fn application_list_paginates_and_filter_starts_a_new_cursor_chain() {
    let (app, pool) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    for (id, name, status, created_at) in [
        ("app_page_1", "One", "active", "2026-08-01T00:00:01Z"),
        ("app_page_2", "Two", "archived", "2026-08-01T00:00:02Z"),
        ("app_page_3", "Three", "active", "2026-08-01T00:00:03Z"),
    ] {
        sqlx::query("INSERT INTO applications(id,name,slug,description,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?)")
            .bind(id).bind(name).bind(id).bind("").bind(status).bind(created_at).bind(created_at)
            .execute(&pool).await.unwrap();
    }

    let first_response = json_request(
        app.clone(),
        "GET",
        "/api/v1/applications?limit=2",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(first_response.status(), StatusCode::OK);
    let first = response_json(first_response).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 2);
    assert_eq!(first["items"][0]["id"], "app_page_1");
    let cursor = first["next_cursor"].as_str().unwrap();
    let second_response = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications?limit=2&after={cursor}"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(second_response.status(), StatusCode::OK);
    let second = response_json(second_response).await;
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert_eq!(second["items"][0]["id"], "app_page_3");
    assert!(second["next_cursor"].is_null());

    let filtered_response = json_request(
        app.clone(),
        "GET",
        "/api/v1/applications?limit=1&status=active",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(filtered_response.status(), StatusCode::OK);
    let filtered = response_json(filtered_response).await;
    assert_eq!(filtered["items"][0]["id"], "app_page_1");
    assert!(filtered["next_cursor"].is_string());
    let invalid = json_request(
        app,
        "GET",
        "/api/v1/applications?after=not-a-cursor",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn application_list_filters_by_environment_and_keeps_filter_across_cursor_pages() {
    let (app, pool) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    for (id, name, environment, created_at) in [
        ("app_env_1", "Test One", "test", "2026-08-01T00:00:01Z"),
        ("app_env_2", "Prod One", "prod", "2026-08-01T00:00:02Z"),
        ("app_env_3", "Test Two", "test", "2026-08-01T00:00:03Z"),
    ] {
        sqlx::query("INSERT INTO applications(id,name,slug,description,environment,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)")
            .bind(id).bind(name).bind(id).bind("").bind(environment).bind("active").bind(created_at).bind(created_at)
            .execute(&pool).await.unwrap();
    }

    let first_response = json_request(
        app.clone(),
        "GET",
        "/api/v1/applications?limit=1&environment=test",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(first_response.status(), StatusCode::OK);
    let first = response_json(first_response).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 1);
    assert_eq!(first["items"][0]["id"], "app_env_1");
    let cursor = first["next_cursor"].as_str().unwrap();

    let second_response = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/applications?limit=1&environment=test&after={cursor}"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(second_response.status(), StatusCode::OK);
    let second = response_json(second_response).await;
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert_eq!(second["items"][0]["id"], "app_env_3");
    assert!(second["next_cursor"].is_null());

    let invalid = json_request(
        app,
        "GET",
        "/api/v1/applications?environment=production",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn application_creation_accepts_valkey_and_etcd_templates() {
    let (app, pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;

    for (name, slug, app_type, type_version, template_id) in [
        ("Valkey 9", "valkey-9", "valkey", "9", "valkey"),
        ("etcd 3.6", "etcd-3-6", "etcd", "3.6", "etcd"),
    ] {
        let created = json_request(
            app.clone(),
            "POST",
            "/api/v1/applications",
            json!({
                "name": name,
                "slug": slug,
                "environment": "prod",
                "app_type": app_type,
                "type_version": type_version,
                "template_id": template_id,
            }),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await;
        assert_eq!(created.status(), StatusCode::CREATED, "{name} 创建失败");
        let application = response_json(created).await;
        assert_eq!(application["app_type"], app_type);
        assert_eq!(application["type_version"], type_version);
    }

    let binding_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM application_template_bindings")
            .fetch_one(&pool)
            .await
            .unwrap();
    let env_file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM application_env_files")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(binding_count, 2);
    assert_eq!(env_file_count, 4);
}

#[tokio::test]
async fn application_creation_allows_duplicate_names_but_rejects_duplicate_slugs() {
    let (app, _pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;

    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({"name":"Valkey 9","slug":"valkey-9","description":"","environment":"prod","app_type":"valkey","type_version":"9"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);

    let duplicate_name = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({"name":"Valkey 9","slug":"valkey-9-new","description":"","environment":"prod","app_type":"valkey","type_version":"9"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(duplicate_name.status(), StatusCode::CREATED);
    let duplicate_body = response_json(duplicate_name).await;
    assert_eq!(duplicate_body["name"], "Valkey 9");
    assert_eq!(duplicate_body["slug"], "valkey-9-new");

    let slug_conflict = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications",
        json!({"name":"Valkey 9 New","slug":"valkey-9","description":"","environment":"prod","app_type":"valkey","type_version":"9"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(slug_conflict.status(), StatusCode::CONFLICT);
    let slug_body = response_json(slug_conflict).await;
    assert_eq!(slug_body["code"], "application_slug_exists");
    assert_eq!(slug_body["message"], "应用 slug 已存在");
}

#[tokio::test]
async fn application_tags_support_multiple_tags_filter_and_replacement() {
    let (app, _pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;

    let first = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/applications",
            json!({"name":"First","slug":"first-tagged","environment":"prod","app_type":"binary","type_version":"1","tags":["platform","middleware"]}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(first["tags"], json!(["middleware", "platform"]));

    let second = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/applications",
            json!({"name":"Second","slug":"second-tagged","environment":"prod","app_type":"binary","type_version":"1","tags":["platform","voucher"]}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(second["tags"], json!(["platform", "voucher"]));

    let middleware_list = response_json(
        json_request(
            app.clone(),
            "GET",
            "/api/v1/applications?tag=middleware",
            json!({}),
            &[("cookie", &admin_cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(middleware_list["items"].as_array().unwrap().len(), 1);
    assert_eq!(middleware_list["items"][0]["id"], first["id"]);

    let platform_list = response_json(
        json_request(
            app.clone(),
            "GET",
            "/api/v1/applications?tag=platform",
            json!({}),
            &[("cookie", &admin_cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(platform_list["items"].as_array().unwrap().len(), 2);

    let tag_options = response_json(
        json_request(
            app.clone(),
            "GET",
            "/api/v1/application-tags",
            json!({}),
            &[("cookie", &admin_cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(
        tag_options["tags"],
        json!(["middleware", "platform", "voucher"])
    );

    let version = first["version"].as_i64().unwrap();
    let updated = response_json(
        json_request(
            app.clone(),
            "PATCH",
            &format!("/api/v1/applications/{}", first["id"].as_str().unwrap()),
            json!({"name":"First","slug":"first-tagged","description":"","environment":"prod","app_type":"binary","type_version":"1","tags":["voucher"],"version":version}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(updated["tags"], json!(["voucher"]));
    let middleware_after = response_json(
        json_request(
            app.clone(),
            "GET",
            "/api/v1/applications?tag=middleware",
            json!({}),
            &[("cookie", &admin_cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(middleware_after["items"].as_array().unwrap().len(), 0);

    let _second_owned = second;
}

#[tokio::test]
async fn application_update_without_tags_preserves_existing_tags() {
    let (app, _pool) = test_app().await;
    let (admin_cookie, csrf) = admin_session(app.clone()).await;

    let created = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/applications",
            json!({"name":"Keep Tags","slug":"keep-tags","environment":"prod","app_type":"binary","type_version":"1","tags":["project-alpha"]}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let version = created["version"].as_i64().unwrap();
    let updated = response_json(
        json_request(
            app.clone(),
            "PATCH",
            &format!("/api/v1/applications/{}", created["id"].as_str().unwrap()),
            json!({"name":"Keep Tags","slug":"keep-tags","description":"updated","environment":"prod","app_type":"binary","type_version":"1","version":version}),
            &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert_eq!(updated["tags"], json!(["project-alpha"]));
}
