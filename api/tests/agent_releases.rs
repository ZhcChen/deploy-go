mod common;

use std::path::Path;

use axum::{Router, http::StatusCode};
use common::{admin_session, json_request, response_json};
use deploy_go_api::{AppState, agents::AgentInstallation, app, crypto::MasterKeyRing, db};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;

fn write_manifest(release_dir: &Path, version: &str) {
    let dir = release_dir.join(version);
    std::fs::create_dir_all(&dir).unwrap();
    let mut manifest: Value = serde_json::from_slice(include_bytes!(
        "../../agent/tests/fixtures/release/0.1.0/deploy-go-agent-manifest.json"
    ))
    .unwrap();
    manifest["agent_version"] = json!(version);
    manifest["executor_version"] = json!(version);
    std::fs::write(
        dir.join("deploy-go-agent-manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}

async fn test_app_with_release_dir(release_dir: &Path) -> Router {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let state = AppState::new(pool)
        .with_allowed_origins(vec!["http://localhost".to_owned()])
        .with_master_key_ring(MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap())
        .with_agent_installation(
            AgentInstallation::from_dir(
                "https://deploy.example.test".parse().unwrap(),
                release_dir.to_path_buf(),
            )
            .unwrap(),
        );
    app(state)
}

#[tokio::test]
async fn administrator_can_list_and_clean_historical_agent_releases() {
    let release_dir =
        std::env::temp_dir().join(format!("deploy-go-agent-releases-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&release_dir);
    write_manifest(&release_dir, "0.1.0");
    write_manifest(&release_dir, "0.2.0");

    let app = test_app_with_release_dir(&release_dir).await;
    let (cookie, csrf) = admin_session(app.clone()).await;

    let response = json_request(
        app.clone(),
        "GET",
        "/api/v1/agent/releases",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["current_version"], "0.2.0");
    assert_eq!(
        body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["version"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["0.1.0", "0.2.0"]
    );
    assert_eq!(body["items"][0]["active"], false);
    assert_eq!(body["items"][1]["active"], true);

    let response = json_request(
        app.clone(),
        "DELETE",
        "/api/v1/agent/releases/0.1.0",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(!release_dir.join("0.1.0").exists());

    let response = json_request(
        app.clone(),
        "DELETE",
        "/api/v1/agent/releases/0.2.0",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

    std::fs::remove_dir_all(release_dir).unwrap();
}
