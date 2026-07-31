mod common;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::http::StatusCode;
use common::{admin_session, json_request, response_json};
use deploy_go_api::{
    AppState, app,
    crypto::MasterKeyRing,
    db,
    executor::ssh::{CapabilityReport, NodeProbe, NodeProbeInput, ProbeError, ScannedHostKey},
};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;

#[derive(Clone)]
struct FakeProbe {
    check_error: Arc<Mutex<Option<ProbeError>>>,
}

impl FakeProbe {
    fn successful() -> Self {
        Self {
            check_error: Arc::new(Mutex::new(None)),
        }
    }

    fn fail_with(&self, error: ProbeError) {
        *self.check_error.lock().unwrap() = Some(error);
    }
}

#[async_trait]
impl NodeProbe for FakeProbe {
    async fn scan_host_key(&self, node: &NodeProbeInput) -> Result<ScannedHostKey, ProbeError> {
        Ok(ScannedHostKey {
            host_key: format!(
                "[{}]:{} ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILM+rvN+ot98qgEN796jTiQfZfG1KaT0PtFDJ/XFSqti",
                node.host, node.port
            ),
            fingerprint: "SHA256:fixture-host-key".to_owned(),
        })
    }

    async fn check(
        &self,
        _node: &NodeProbeInput,
        private_key: &[u8],
        trusted_host_key: &str,
    ) -> Result<CapabilityReport, ProbeError> {
        assert!(String::from_utf8_lossy(private_key).contains("OPENSSH PRIVATE KEY"));
        assert!(trusted_host_key.contains("ssh-ed25519"));
        if let Some(error) = self.check_error.lock().unwrap().clone() {
            return Err(error);
        }
        Ok(CapabilityReport {
            os_name: "Linux".to_owned(),
            architecture: "x86_64".to_owned(),
            disk_available_bytes: 1024 * 1024,
        })
    }
}

async fn node_app(probe: FakeProbe) -> (axum::Router, sqlx::SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let state = AppState::new(pool.clone())
        .with_setup_token(common::SETUP_TOKEN)
        .with_master_key_ring(MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap())
        .with_node_probe(probe);
    (app(state), pool)
}

async fn create_credential(app: axum::Router, cookie: &str, csrf: &str) -> String {
    let response = json_request(
        app,
        "POST",
        "/api/v1/ssh-credentials",
        json!({"name":"Node Key"}),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    response_json(response).await["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn create_node(
    app: axum::Router,
    cookie: &str,
    csrf: &str,
    credential_id: Option<&str>,
) -> Value {
    let response = json_request(
        app,
        "POST",
        "/api/v1/nodes",
        json!({
            "name":"Node One", "host":"node.example.test", "port":22,
            "username":"deploy", "ssh_credential_id":credential_id,
            "work_root":"/srv/apps", "secrets_root":"/srv/secrets"
        }),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}

#[tokio::test]
async fn host_key_confirmation_then_check_persists_capabilities() {
    let probe = FakeProbe::successful();
    let (app, pool) = node_app(probe).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let credential_id = create_credential(app.clone(), &cookie, &csrf).await;
    let node = create_node(app.clone(), &cookie, &csrf, Some(&credential_id)).await;
    let node_id = node["id"].as_str().unwrap();

    let before_confirmation = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/checks"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(before_confirmation.status(), StatusCode::CONFLICT);

    let scan = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/host-key/scan"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(scan.status(), StatusCode::CREATED);
    let scan = response_json(scan).await;
    let confirm = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/host-key/confirm"),
        json!({"check_id":scan["check_id"],"snapshot_hash":scan["snapshot_hash"],"version":1}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(confirm.status(), StatusCode::OK);

    let checked = json_request(
        app,
        "POST",
        &format!("/api/v1/nodes/{node_id}/checks"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(checked.status(), StatusCode::CREATED);
    let check = response_json(checked).await;
    assert_eq!(check["status"], "succeeded");
    assert_eq!(check["os_name"], "Linux");
    let status: String = sqlx::query_scalar("SELECT status FROM nodes WHERE id = ?")
        .bind(node_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "online");
}

#[tokio::test]
async fn unbind_blocks_check_and_probe_failures_are_classified() {
    let probe = FakeProbe::successful();
    let (app, _) = node_app(probe.clone()).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let credential_id = create_credential(app.clone(), &cookie, &csrf).await;
    let node = create_node(app.clone(), &cookie, &csrf, Some(&credential_id)).await;
    let node_id = node["id"].as_str().unwrap();
    let scan = response_json(
        json_request(
            app.clone(),
            "POST",
            &format!("/api/v1/nodes/{node_id}/host-key/scan"),
            json!({}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let confirmed = response_json(
        json_request(
            app.clone(),
            "POST",
            &format!("/api/v1/nodes/{node_id}/host-key/confirm"),
            json!({"check_id":scan["check_id"],"snapshot_hash":scan["snapshot_hash"],"version":1}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    probe.fail_with(ProbeError::new("authentication_failed", "SSH 身份验证失败"));
    let failed = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/nodes/{node_id}/checks"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(
        response_json(failed).await["failure_code"],
        "authentication_failed"
    );

    let version = confirmed["version"].as_i64().unwrap() + 1;
    let unbound = json_request(
        app.clone(),
        "DELETE",
        &format!("/api/v1/nodes/{node_id}/ssh-credential"),
        json!({"version":version}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(unbound.status(), StatusCode::OK);
    assert_eq!(response_json(unbound).await["status"], "missing_credential");
    let blocked = json_request(
        app,
        "POST",
        &format!("/api/v1/nodes/{node_id}/checks"),
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn node_input_rejects_ssh_option_injection() {
    let (app, _) = node_app(FakeProbe::successful()).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let response = json_request(
        app,
        "POST",
        "/api/v1/nodes",
        json!({"name":"Bad","host":"-oProxyCommand=bad","port":22,"username":"deploy","ssh_credential_id":null,"work_root":"/srv/apps","secrets_root":"/srv/secrets"}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
