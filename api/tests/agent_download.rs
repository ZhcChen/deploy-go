mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use common::test_app;
use serde_json::Value;
use tower::ServiceExt;

async fn get_json(app: axum::Router, uri: &str) -> Value {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn serves_versioned_agent_release_artifacts_from_api() {
    let (app, _pool) = test_app().await;

    for version in ["0_1_0", "0.1.0"] {
        let manifest = get_json(
            app.clone(),
            &format!("/api/v1/agent/download/{version}/manifest.json"),
        )
        .await;
        assert_eq!(manifest["agent_version"], "0.1.0");
        assert_eq!(
            manifest["systemd_units"]["agent"]["url"],
            "https://deploy.example.test/api/v1/agent/download/0_1_0/systemd-unit/agent"
        );
        assert_eq!(
            manifest["systemd_units"]["executor"]["url"],
            "https://deploy.example.test/api/v1/agent/download/0_1_0/systemd-unit/executor"
        );
        assert_eq!(
            manifest["executor_config"]["url"],
            "https://deploy.example.test/api/v1/agent/download/0_1_0/executor-config"
        );
        for artifact in manifest["artifacts"].as_array().unwrap() {
            let component = artifact["component"].as_str().unwrap();
            let arch = artifact["architecture"].as_str().unwrap();
            assert_eq!(
                artifact["url"],
                format!(
                    "https://deploy.example.test/api/v1/agent/download/0_1_0/{component}/{arch}"
                )
            );
        }
    }

    let cases = [
        (
            "/api/v1/agent/download/0_1_0/agent/x86_64",
            "fixture-x86_64-agent\n",
        ),
        (
            "/api/v1/agent/download/0.1.0/agent/aarch64",
            "fixture-aarch64-agent\n",
        ),
        (
            "/api/v1/agent/download/0_1_0/executor/x86_64",
            "fixture-x86_64-executor\n",
        ),
        (
            "/api/v1/agent/download/0.1.0/executor/aarch64",
            "fixture-aarch64-executor\n",
        ),
    ];
    for (uri, expected) in cases {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/octet-stream"
        );
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(bytes.as_ref(), expected.as_bytes());
    }

    let unit = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/agent/download/0_1_0/systemd-unit/agent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unit.status(), StatusCode::OK);
    let bytes = to_bytes(unit.into_body(), usize::MAX).await.unwrap();
    assert_eq!(bytes.as_ref(), b"fixture-systemd-unit\n");

    let executor_unit = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/agent/download/0_1_0/systemd-unit/executor")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(executor_unit.status(), StatusCode::OK);
    let bytes = to_bytes(executor_unit.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(bytes.as_ref(), b"fixture-executor-systemd-unit\n");

    let executor_config = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/agent/download/0_1_0/executor-config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(executor_config.status(), StatusCode::OK);
}

#[tokio::test]
async fn rejects_unknown_version_or_architecture() {
    let (app, _pool) = test_app().await;
    for uri in [
        "/api/v1/agent/download/9_9_9/manifest.json",
        "/api/v1/agent/download/0_1_0/agent/riscv64",
        "/api/v1/agent/download/0_1_0/agent/../manifest.json",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(
            matches!(
                response.status(),
                StatusCode::NOT_FOUND | StatusCode::BAD_REQUEST
            ),
            "{uri}"
        );
    }
}
