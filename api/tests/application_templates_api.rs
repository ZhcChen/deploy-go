mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;

#[tokio::test]
async fn template_registry_is_read_only_and_hides_content_from_list() {
    let (app, _) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;

    let response = json_request(
        app.clone(),
        "GET",
        "/api/v1/application-templates",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0]["id"], "postgres");
    assert_eq!(items[0]["deployment_mechanism"], "image");
    assert_eq!(items[0]["default_image"], "postgres:18-alpine");
    assert_eq!(items[0]["default_port"], 5432);
    assert_eq!(items[1]["id"], "redis");
    assert_eq!(items[2]["id"], "valkey");
    assert_eq!(items[2]["default_image"], "valkey/valkey:9-alpine");
    assert_eq!(items[2]["default_port"], 6379);
    assert_eq!(items[3]["id"], "etcd");
    assert!(items[0]["digest"].as_str().unwrap().len() == 64);
    assert!(
        items[0]["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|file| file.get("content").is_none())
    );
    let postgres_env = items[0]["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"] == "postgres.env.example")
        .unwrap();
    assert_eq!(postgres_env["editable"], true);
    assert_eq!(postgres_env["sensitive"], true);
    assert_eq!(postgres_env["deploy_path"], "postgres.env");
    assert_eq!(postgres_env["delivery"], "env_lease");

    for template in items {
        let files = template["files"].as_array().unwrap();
        for path in ["deploy-go.yaml", "Makefile", "scripts/release.sh"] {
            let file = files.iter().find(|file| file["path"] == path).unwrap();
            assert_eq!(file["role"], "platform_managed");
            assert_eq!(file["delivery"], "platform_managed");
            assert_eq!(file["editable"], false);
            assert!(file["deploy_path"].is_null());
        }
    }
}

#[tokio::test]
async fn template_detail_and_file_endpoint_return_source_and_metadata() {
    let (app, _) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;

    let detail = json_request(
        app.clone(),
        "GET",
        "/api/v1/application-templates/redis",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail = response_json(detail).await;
    let compose = detail["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"] == "compose.yaml")
        .unwrap();
    assert!(
        compose["content"]
            .as_str()
            .unwrap()
            .contains("redis:7-alpine")
    );
    assert_eq!(compose["role"], "configuration");
    assert_eq!(compose["delivery"], "artifact");
    assert!(
        compose["description"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );

    let file = json_request(
        app.clone(),
        "GET",
        "/api/v1/application-templates/redis/file?path=config%2Fredis.conf",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(file.status(), StatusCode::OK);
    let file = response_json(file).await;
    assert_eq!(file["path"], "config/redis.conf");
    assert_eq!(file["format"], "ini");
    assert!(file["content"].as_str().unwrap().contains("appendonly yes"));

    let missing = json_request(
        app,
        "GET",
        "/api/v1/application-templates/redis/file?path=missing.conf",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response_json(missing).await["code"],
        "application_template_file_not_found"
    );
}

#[tokio::test]
async fn template_registry_requires_a_session() {
    let (app, _) = test_app().await;
    let response = json_request(app, "GET", "/api/v1/application-templates", json!({}), &[]).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
