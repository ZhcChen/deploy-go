mod common;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use async_trait::async_trait;
use axum::{body::to_bytes, http::StatusCode};
use common::{admin_session, json_request, response_json};
use deploy_go_api::{
    AppState, app,
    crypto::MasterKeyRing,
    db,
    deployments::process_one,
    executor::{
        deployment::{DeploymentExecutor, ExecutionContext, OutputChunk},
        ssh::{CapabilityReport, NodeProbe, NodeProbeInput, ProbeError, ScannedHostKey},
    },
};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

#[derive(Clone)]
struct EndToEndProbe;

#[async_trait]
impl NodeProbe for EndToEndProbe {
    async fn scan_host_key(&self, node: &NodeProbeInput) -> Result<ScannedHostKey, ProbeError> {
        Ok(ScannedHostKey {
            host_key: format!(
                "[{}]:{} ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILM+rvN+ot98qgEN796jTiQfZfG1KaT0PtFDJ/XFSqti",
                node.host, node.port
            ),
            fingerprint: "SHA256:end-to-end-host".to_owned(),
        })
    }
    async fn check(
        &self,
        _: &NodeProbeInput,
        private_key: &[u8],
        _: &str,
    ) -> Result<CapabilityReport, ProbeError> {
        assert!(String::from_utf8_lossy(private_key).contains("OPENSSH PRIVATE KEY"));
        Ok(CapabilityReport {
            os_name: "Linux".to_owned(),
            architecture: "x86_64".to_owned(),
            disk_available_bytes: 1024 * 1024 * 1024,
        })
    }
}

#[derive(Clone)]
struct EndToEndExecutor {
    called: Arc<AtomicBool>,
}

#[async_trait]
impl DeploymentExecutor for EndToEndExecutor {
    async fn execute(
        &self,
        context: &ExecutionContext,
        output: tokio::sync::mpsc::Sender<OutputChunk>,
    ) -> Result<i32, ProbeError> {
        assert_eq!(context.argument_tokens, ["--release-version", "1.0.0"]);
        assert!(
            String::from_utf8_lossy(context.private_key.as_slice()).contains("OPENSSH PRIVATE KEY")
        );
        output
            .send(OutputChunk {
                stream: "stdout",
                bytes: b"deploying release\n".to_vec(),
            })
            .await
            .unwrap();
        let event = json!({"schema_version":1,"event":"deploy.finished","timestamp":"2026-07-31T00:00:00Z","status":"succeeded","deploy_id":context.deployment_id});
        output
            .send(OutputChunk {
                stream: "stdout",
                bytes: format!("DEPLOY_EVENT {event}\n").into_bytes(),
            })
            .await
            .unwrap();
        self.called.store(true, Ordering::SeqCst);
        Ok(0)
    }
    async fn cancel(&self, _: &ExecutionContext) -> Result<(), ProbeError> {
        Ok(())
    }
}

#[tokio::test]
async fn empty_database_reaches_a_successful_mock_deployment() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let called = Arc::new(AtomicBool::new(false));
    let state = AppState::new(pool.clone())
        .with_setup_token(common::SETUP_TOKEN)
        .with_master_key_ring(MasterKeyRing::from_raw(1, [9; 32], None).unwrap())
        .with_node_probe(EndToEndProbe)
        .with_deployment_executor(EndToEndExecutor {
            called: called.clone(),
        });
    let router = app(state.clone());
    let (cookie, csrf) = admin_session(router.clone()).await;

    let credential = response_json(
        json_request(
            router.clone(),
            "POST",
            "/api/v1/ssh-credentials",
            json!({"name":"End-to-end Key"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    assert!(
        credential["public_key"]
            .as_str()
            .unwrap()
            .starts_with("ssh-ed25519 ")
    );
    let node=response_json(json_request(router.clone(),"POST","/api/v1/nodes",json!({"name":"Fixture Node","host":"fixture.invalid","port":22,"username":"deploy","ssh_credential_id":credential["id"],"work_root":"/srv/apps","secrets_root":"/srv/secrets"}),&[("cookie",&cookie),("x-csrf-token",&csrf)]).await).await;
    let node_id = node["id"].as_str().unwrap();
    let scan = response_json(
        json_request(
            router.clone(),
            "POST",
            &format!("/api/v1/nodes/{node_id}/host-key/scan"),
            json!({}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let confirmed = json_request(
        router.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/host-key/confirm"),
        json!({"check_id":scan["check_id"],"snapshot_hash":scan["snapshot_hash"],"version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::OK);
    let checked = json_request(
        router.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/checks"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(checked.status(), StatusCode::CREATED);

    let application = response_json(
        json_request(
            router.clone(),
            "POST",
            "/api/v1/applications",
            json!({"name":"End-to-end App","slug":"end-to-end-app","description":""}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let app_id = application["id"].as_str().unwrap();
    let target=response_json(json_request(router.clone(),"POST",&format!("/api/v1/applications/{app_id}/targets"),json!({
        "node_id":node_id,"environment":"test","script_path":"/srv/apps/end-to-end/deploy.sh",
        "parameter_schema":{"type":"object","properties":{"release-version":{"type":"string","maxLength":32}},"required":["release-version"],"additionalProperties":false},
        "timeout_seconds":60,"verification_config":{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":1000},"secret_file_references":[]
    }),&[("cookie",&cookie),("x-csrf-token",&csrf)]).await).await;
    let target_id = target["id"].as_str().unwrap();
    let preview = response_json(
        json_request(
            router.clone(),
            "POST",
            &format!("/api/v1/deployment-targets/{target_id}/deployment-preview"),
            json!({"parameters":{"release-version":"1.0.0"}}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    let deployment=response_json(json_request(router.clone(),"POST",&format!("/api/v1/deployment-targets/{target_id}/deployments"),json!({"parameters":{"release-version":"1.0.0"},"snapshot_hash":preview["snapshot_hash"]}),&[("cookie",&cookie),("x-csrf-token",&csrf),("idempotency-key","end-to-end-deploy-0001")]).await).await;
    let deployment_id = deployment["id"].as_str().unwrap();
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id)
    );
    assert!(called.load(Ordering::SeqCst));

    let completed = response_json(
        json_request(
            router.clone(),
            "GET",
            &format!("/api/v1/deployments/{deployment_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(completed["status"], "succeeded");
    assert_eq!(completed["protocol_complete"], true);
    let logs = json_request(
        router,
        "GET",
        &format!("/api/v1/deployments/{deployment_id}/logs"),
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    let body = String::from_utf8(
        to_bytes(logs.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("deploying release"));
    assert!(body.contains("event: terminal"));
    assert!(!body.contains("OPENSSH PRIVATE KEY"));
}
