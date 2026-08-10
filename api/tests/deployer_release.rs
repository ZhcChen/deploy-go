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
async fn serves_versioned_deployer_release_artifacts_from_api() {
    let (app, _pool) = test_app().await;

    for version in ["0_2_0", "0.2.0"] {
        let manifest = get_json(
            app.clone(),
            &format!("/api/v1/deployer/download/{version}/manifest.json"),
        )
        .await;
        assert_eq!(manifest["deployer_version"], "0.2.0");
        for artifact in manifest["artifacts"].as_array().unwrap() {
            let arch = artifact["architecture"].as_str().unwrap();
            assert_eq!(
                artifact["url"],
                format!(
                    "https://deploy.example.test/api/v1/deployer/download/0_2_0/deployer/{arch}"
                )
            );
        }
    }

    for (uri, expected) in [
        (
            "/api/v1/deployer/download/0_2_0/deployer/x86_64",
            "fixture-x86_64-deployer\n",
        ),
        (
            "/api/v1/deployer/download/0.2.0/deployer/aarch64",
            "fixture-aarch64-deployer\n",
        ),
    ] {
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
}

#[tokio::test]
async fn rejects_unknown_deployer_version_or_architecture() {
    let (app, _pool) = test_app().await;
    for uri in [
        "/api/v1/deployer/download/9_9_9/manifest.json",
        "/api/v1/deployer/download/0_2_0/deployer/riscv64",
        "/api/v1/deployer/download/0_2_0/deployer/../manifest.json",
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
