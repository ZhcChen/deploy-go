use std::{fs, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{Router, body::Body, http::HeaderMap, response::Response, routing::get};
use deploy_go_agent::{
    connection::MessageHandler,
    env_sync::{EnvFileStore, EnvSecretClient},
    executor::Executor,
    task_handler::TaskHandler,
    token_refresh::{AccessProvider, PreparedAccess, TokenRefreshError},
};
use deploy_go_agent_protocol::{
    DeploymentReleaseTask, EnvSyncAction, EnvSyncTask, Envelope, Environment, MakeTarget, Message,
    PROTOCOL_VERSION, RequiredEnvVersion, TaskAckDisposition, TaskDispatch, TaskPayload,
    TaskTerminalStatus,
};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

struct StaticAccess;

#[async_trait]
impl AccessProvider for StaticAccess {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        Ok(PreparedAccess {
            access_token: "env-access-token-never-persisted".to_owned(),
            access_expires_at: "2099-01-01T00:00:00Z".to_owned(),
            rotation_id: None,
        })
    }

    async fn commit(&self, _rotation_id: &str) -> Result<(), TokenRefreshError> {
        Ok(())
    }
}

#[tokio::test]
async fn release_uses_task_env_copy_and_cleans_it_after_completion() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let secrets = directory.path().join("secrets");
    let work = directory.path().join("work");
    let checkout = work.join("checkout");
    let artifact = work.join("artifact");
    fs::create_dir(&secrets).unwrap();
    fs::create_dir_all(&checkout).unwrap();
    fs::create_dir(&artifact).unwrap();
    fs::write(
        checkout.join("Makefile"),
        "deploy-go-release:\n\t@bash release.sh\n",
    )
    .unwrap();
    fs::write(
        checkout.join("release.sh"),
        r#"#!/usr/bin/env bash
set -euo pipefail
test "$(cat "$DEPLOY_ENV_DIR/api.env")" = "API_SECRET=one"
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.started"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"api","module_name":"API"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"api","step_id":"api.release","step":"发布"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"api","step_id":"api.release","step":"发布"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"api","module_name":"API"}'
"#,
    )
    .unwrap();
    let artifact_file = artifact.join("api.tar");
    fs::write(&artifact_file, b"artifact").unwrap();
    fs::write(
        artifact.join("deploy-go-artifact.json"),
        serde_json::json!({
            "schema_version": 1,
            "release_version": "release-1",
            "commit_sha": "0123456789abcdef0123456789abcdef01234567",
            "artifacts": [{
                "module": "api",
                "path": "api.tar",
                "sha256": format!("{:x}", Sha256::digest(b"artifact")),
                "size": 8
            }]
        })
        .to_string(),
    )
    .unwrap();
    let store = EnvFileStore::new(secrets.clone()).unwrap();
    let content = b"API_SECRET=one\n";
    let digest = format!("{:x}", Sha256::digest(content));
    store
        .write("app-production", "api.env", content, &digest)
        .unwrap();
    let handler =
        TaskHandler::new(Executor::new(tasks.clone()).unwrap().with_runner_binary(
            std::path::Path::new(env!("CARGO_BIN_EXE_deploy-go-agent")).into(),
        ))
        .with_env_sync(
            EnvSecretClient::new(
                "https://127.0.0.1/".parse().unwrap(),
                Arc::new(StaticAccess),
                false,
            ),
            store,
        );
    let task = TaskPayload::DeploymentRelease(DeploymentReleaseTask {
        deployment_id: "deployment_env_release".to_owned(),
        target_code: "production".to_owned(),
        work_root: work.display().to_string(),
        checkout_dir: checkout.display().to_string(),
        artifact_dir: artifact.display().to_string(),
        environment: Environment::Production,
        release_version: "release-1".to_owned(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
        modules: vec!["api".to_owned()],
        make_target: MakeTarget::DeployGoRelease,
        timeout_seconds: 30,
        cancel_file: tasks.join("task_env_release/cancel").display().to_string(),
        privileged: false,
        artifact_download: None,
        repository_url: None,
        git_credential_lease_id: None,
        application_slug: Some("app-production".to_owned()),
        required_env: vec![RequiredEnvVersion {
            file_name: "api.env".to_owned(),
            env_version: 1,
            digest,
            action: EnvSyncAction::Write,
        }],
    });
    let payload_json = serde_json::to_string(&task).unwrap();
    let (sender, mut receiver) = mpsc::channel(64);
    handler
        .handle(
            Envelope {
                protocol_version: PROTOCOL_VERSION,
                message_id: "message_env_release".to_owned(),
                sent_at: chrono::Utc::now().to_rfc3339(),
                message: Message::TaskDispatch(TaskDispatch {
                    task_id: "task_env_release".to_owned(),
                    idempotency_key: "env-release-idempotency".to_owned(),
                    deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
                    payload_digest: format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes())),
                    task,
                }),
            },
            sender,
        )
        .await
        .unwrap();
    let mut result = None;
    let mut observed = Vec::new();
    while let Ok(Some(message)) =
        tokio::time::timeout(Duration::from_secs(10), receiver.recv()).await
    {
        if let Message::TaskResult(value) = &message {
            result = Some(value.clone());
        }
        observed.push(message);
        if result.is_some() {
            break;
        }
    }
    assert_eq!(
        result
            .unwrap_or_else(|| panic!("缺少任务结果：{observed:?}"))
            .status,
        TaskTerminalStatus::Succeeded
    );
    assert!(!tasks.join("task_env_release/env").exists());
    assert_eq!(
        fs::read(secrets.join("app-production/api.env")).unwrap(),
        content
    );
}

#[tokio::test]
async fn env_sync_task_fetches_over_https_boundary_and_never_journals_plaintext() {
    let content = Arc::new(b"SECRET=env-task-plaintext\n".to_vec());
    let served = content.clone();
    let app = Router::new().route(
        "/api/v1/agent/application-env-leases/envlease_test",
        get(move |headers: HeaderMap| {
            let served = served.clone();
            async move {
                assert_eq!(
                    headers.get("authorization").unwrap(),
                    "Bearer env-access-token-never-persisted"
                );
                Response::new(Body::from(served.as_ref().clone()))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let secrets = directory.path().join("secrets");
    fs::create_dir(&secrets).unwrap();
    let client = EnvSecretClient::new(
        format!("http://{address}/").parse().unwrap(),
        Arc::new(StaticAccess),
        true,
    );
    let handler = TaskHandler::new(Executor::new(tasks.clone()).unwrap())
        .with_env_sync(client, EnvFileStore::new(secrets.clone()).unwrap());
    let digest = format!("{:x}", Sha256::digest(content.as_slice()));
    let task = TaskPayload::EnvSync(EnvSyncTask {
        env_sync_id: "envsync_test".to_owned(),
        application_slug: "app-production".to_owned(),
        file_name: "api.env".to_owned(),
        env_version: 1,
        digest,
        lease_id: "envlease_test".to_owned(),
        action: EnvSyncAction::Write,
    });
    let payload_json = serde_json::to_string(&task).unwrap();
    let payload_digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    let dispatch = TaskDispatch {
        task_id: "task_env_sync".to_owned(),
        idempotency_key: "env-sync-idempotency-01".to_owned(),
        deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
        payload_digest,
        task,
    };
    let (sender, mut receiver) = mpsc::channel(16);
    handler
        .handle(
            Envelope {
                protocol_version: PROTOCOL_VERSION,
                message_id: "message_env_sync".to_owned(),
                sent_at: chrono::Utc::now().to_rfc3339(),
                message: Message::TaskDispatch(dispatch),
            },
            sender,
        )
        .await
        .unwrap();
    let mut terminal = false;
    for _ in 0..5 {
        let message = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        if matches!(message, Message::TaskResult(_)) {
            terminal = true;
            break;
        }
    }
    assert!(terminal);
    assert_eq!(
        fs::read(secrets.join("app-production/api.env")).unwrap(),
        content.as_slice()
    );
    let journal = fs::read_to_string(tasks.join("task_env_sync/journal.json")).unwrap();
    assert!(!journal.contains("env-task-plaintext"));
    assert!(!journal.contains("env-access-token"));
    server.abort();
}

#[tokio::test]
async fn release_is_rejected_before_execution_when_required_env_digest_does_not_match() {
    let directory = tempfile::tempdir().unwrap();
    let tasks = directory.path().join("tasks");
    let secrets = directory.path().join("secrets");
    fs::create_dir(&secrets).unwrap();
    let store = EnvFileStore::new(secrets).unwrap();
    let existing = b"A=old\n";
    store
        .write(
            "app-production",
            "api.env",
            existing,
            &format!("{:x}", Sha256::digest(existing)),
        )
        .unwrap();
    let stale = b"STALE=true\n";
    store
        .write(
            "app-production",
            "worker.env",
            stale,
            &format!("{:x}", Sha256::digest(stale)),
        )
        .unwrap();
    let handler = TaskHandler::new(Executor::new(tasks.clone()).unwrap()).with_env_sync(
        EnvSecretClient::new(
            "http://127.0.0.1:1/".parse().unwrap(),
            Arc::new(StaticAccess),
            false,
        ),
        store,
    );
    let task = TaskPayload::DeploymentRelease(DeploymentReleaseTask {
        deployment_id: "deployment_gate".to_owned(),
        target_code: "production".to_owned(),
        work_root: "/unused".to_owned(),
        checkout_dir: "/unused/checkout".to_owned(),
        artifact_dir: "/unused/artifact".to_owned(),
        environment: Environment::Production,
        release_version: "release-1".to_owned(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
        modules: vec!["api".to_owned()],
        make_target: MakeTarget::DeployGoRelease,
        timeout_seconds: 60,
        cancel_file: "/unused/cancel".to_owned(),
        privileged: false,
        artifact_download: None,
        repository_url: None,
        git_credential_lease_id: None,
        application_slug: Some("app-production".to_owned()),
        required_env: vec![
            RequiredEnvVersion {
                file_name: "api.env".to_owned(),
                env_version: 1,
                digest: format!("{:x}", Sha256::digest(existing)),
                action: deploy_go_agent_protocol::EnvSyncAction::Write,
            },
            RequiredEnvVersion {
                file_name: "worker.env".to_owned(),
                env_version: 2,
                digest: format!("{:x}", Sha256::digest([])),
                action: deploy_go_agent_protocol::EnvSyncAction::Delete,
            },
        ],
    });
    let payload_json = serde_json::to_string(&task).unwrap();
    let (sender, mut receiver) = mpsc::channel(4);
    handler
        .handle(
            Envelope {
                protocol_version: PROTOCOL_VERSION,
                message_id: "message_release_gate".to_owned(),
                sent_at: chrono::Utc::now().to_rfc3339(),
                message: Message::TaskDispatch(TaskDispatch {
                    task_id: "task_release_gate".to_owned(),
                    idempotency_key: "release-gate-idempotency".to_owned(),
                    deadline_at: (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339(),
                    payload_digest: format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes())),
                    task,
                }),
            },
            sender,
        )
        .await
        .unwrap();
    let Message::TaskAck(ack) = receiver.recv().await.unwrap() else {
        panic!("expected task ack");
    };
    assert_eq!(ack.disposition, TaskAckDisposition::Rejected);
    assert_eq!(ack.error_code.as_deref(), Some("env_gate_failed"));
    assert!(!tasks.join("task_release_gate/journal.json").exists());
}
