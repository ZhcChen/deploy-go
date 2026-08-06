use deploy_go_agent_protocol::{
    DeployEvent, DeployEventName, DeployEventStatus, DeploymentStage, Envelope, Environment,
    Message, PROTOCOL_VERSION, ReconcileReport, ReconciledTask, ReconciledTaskState,
    SecretLeasePurpose, SecretLeaseRequest, SecretLeaseResponse, TaskProgress, TaskResult,
    TaskTerminalStatus,
};
use serde_json::{Value, json};

fn schema() -> Value {
    serde_json::from_str(include_str!("../schema/agent-control.schema.json")).unwrap()
}

fn valid_task() -> Value {
    json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_01",
        "sent_at": "2026-08-03T03:00:00Z",
        "message": {
            "type": "task_dispatch",
            "task_id": "task_01",
            "idempotency_key": "idem-0123456789abcdef",
            "deadline_at": "2026-08-03T03:10:00Z",
            "payload_digest": "sha256:abc",
            "task": {
                "kind": "deployment_execute",
                "payload": {
                    "deployment_id": "dep_01",
                    "work_root": "/srv/example",
                    "script_path": "/srv/example/deploy.sh",
                    "argument_tokens": ["--environment", "production"],
                    "environment_file_references": [],
                    "timeout_seconds": 600,
                    "wrapper_version": "1"
                }
            }
        }
    })
}

#[test]
fn schema_and_rust_accept_the_current_task_sample() {
    let task = valid_task();
    let validator = jsonschema::validator_for(&schema()).unwrap();
    assert!(validator.is_valid(&task));
    let envelope: Envelope = serde_json::from_value(task).unwrap();
    envelope.validate_version().unwrap();
}

#[test]
fn schema_rejects_unknown_fields_shell_commands_and_versions() {
    let validator = jsonschema::validator_for(&schema()).unwrap();

    let mut extra = valid_task();
    extra["message"]["task"]["payload"]["shell_command"] = json!("id");
    assert!(!validator.is_valid(&extra));

    let mut future = valid_task();
    future["protocol_version"] = json!(3);
    assert!(!validator.is_valid(&future));

    let mut unknown = valid_task();
    unknown["message"]["type"] = json!("terminal_open");
    assert!(!validator.is_valid(&unknown));
}

#[test]
fn schema_keeps_raw_script_marker_out_of_the_control_envelope() {
    let schema_text = include_str!("../schema/agent-control.schema.json");
    assert!(!schema_text.contains("DEPLOY_GO_EVENT"));
    assert!(!schema_text.contains("DEPLOY_EVENT"));
    assert!(!schema_text.contains("{json}"));
}

#[test]
fn schema_accepts_a_serialized_reconcile_result() {
    let envelope = Envelope {
        protocol_version: PROTOCOL_VERSION,
        message_id: "msg_reconcile".into(),
        sent_at: "2026-08-03T03:00:00Z".into(),
        message: Message::ReconcileReport(ReconcileReport {
            tasks: vec![ReconciledTask {
                task_id: "task_01".into(),
                payload_digest: "sha256:abc".into(),
                state: ReconciledTaskState::Terminal,
                last_sequence: 3,
                result: Some(TaskResult {
                    task_id: "task_01".into(),
                    sequence: 3,
                    status: TaskTerminalStatus::Succeeded,
                    exit_code: Some(0),
                    error_code: None,
                    summary: None,
                    data: None,
                }),
            }],
        }),
    };
    let value = serde_json::to_value(&envelope).unwrap();
    let validator = jsonschema::validator_for(&schema()).unwrap();
    assert!(validator.is_valid(&value));
    assert_eq!(serde_json::from_value::<Envelope>(value).unwrap(), envelope);
}

#[test]
fn schema_accepts_two_stage_task_payloads_and_rejects_abuse() {
    let validator = jsonschema::validator_for(&schema()).unwrap();

    let refs = json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_refs",
        "sent_at": "2026-08-06T03:00:00Z",
        "message": {
            "type": "task_dispatch",
            "task_id": "task_refs",
            "idempotency_key": "idem-refs-0123456789",
            "deadline_at": "2026-08-06T03:05:00Z",
            "payload_digest": "sha256:abc",
            "task": {
                "kind": "git_refs_query",
                "payload": {
                    "refs_query_id": "refs_01",
                    "repository_url": "git@git.example.test:deploy-go/example.git",
                    "git_credential_lease_id": "lease_01",
                    "timeout_seconds": 60
                }
            }
        }
    });
    let prepare = json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_prepare",
        "sent_at": "2026-08-06T03:00:00Z",
        "message": {
            "type": "task_dispatch",
            "task_id": "task_prepare",
            "idempotency_key": "idem-prepare-0123456789",
            "deadline_at": "2026-08-06T03:20:00Z",
            "payload_digest": "sha256:abc",
            "task": {
                "kind": "deployment_prepare",
                "payload": {
                    "deployment_id": "dep_01",
                    "source_policy": "branch",
                    "repository_url": "git@git.example.test:deploy-go/example.git",
                    "commit_sha": "0123456789abcdef0123456789abcdef01234567",
                    "checkout_dir": "/srv/tasks/task_prepare/checkout",
                    "work_root": "/srv/tasks/task_prepare",
                    "output_dir": "/srv/tasks/task_prepare/staging",
                    "environment": "staging",
                    "release_version": "20260806183000",
                    "modules": ["api", "web"],
                    "make_target": "deploy_go_prepare",
                    "git_credential_lease_id": "lease_01",
                    "timeout_seconds": 900
                }
            }
        }
    });
    let release = json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_release",
        "sent_at": "2026-08-06T03:00:00Z",
        "message": {
            "type": "task_dispatch",
            "task_id": "task_release",
            "idempotency_key": "idem-release-0123456789",
            "deadline_at": "2026-08-06T03:30:00Z",
            "payload_digest": "sha256:abc",
            "task": {
                "kind": "deployment_release",
                "payload": {
                    "deployment_id": "dep_01",
                    "target_code": "qfy-test",
                    "artifact_dir": "/srv/tasks/task_release/staging",
                    "environment": "prod",
                    "release_version": "20260806183000",
                    "commit_sha": "0123456789abcdef0123456789abcdef01234567",
                    "modules": ["api"],
                    "make_target": "deploy_go_release",
                    "timeout_seconds": 900,
                    "cancel_file": "/srv/tasks/task_release/cancel"
                }
            }
        }
    });
    for sample in [&refs, &prepare, &release] {
        assert!(validator.is_valid(sample));
        assert!(serde_json::from_value::<Envelope>(sample.clone()).is_ok());
    }

    let mut credential_url = prepare.clone();
    credential_url["message"]["task"]["payload"]["repository_url"] =
        json!("https://user:pass@git.example.test/deploy-go/example.git");
    assert!(!validator.is_valid(&credential_url));

    let mut arbitrary_target = release.clone();
    arbitrary_target["message"]["task"]["payload"]["make_target"] = json!("deploy");
    assert!(!validator.is_valid(&arbitrary_target));

    let mut path_escape = prepare.clone();
    path_escape["message"]["task"]["payload"]["checkout_dir"] = json!("../escape");
    assert!(!validator.is_valid(&path_escape));

    let mut bad_environment = prepare.clone();
    bad_environment["message"]["task"]["payload"]["environment"] = json!("production");
    assert!(!validator.is_valid(&bad_environment));
}

#[test]
fn schema_accepts_progress_and_secret_lease_messages() {
    let validator = jsonschema::validator_for(&schema()).unwrap();

    let progress = json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_progress",
        "sent_at": "2026-08-06T03:00:00Z",
        "message": {
            "type": "task_progress",
            "task_id": "task_prepare",
            "sequence": 1,
            "event": {
                "deploy_id": "dep_01",
                "stage": "prepare",
                "event": "deploy.module.started",
                "timestamp": "2026-08-06T03:00:01Z",
                "status": "started",
                "environment": "staging",
                "release_version": "20260806183000",
                "module": "api",
                "module_name": "API",
                "message": "build api"
            }
        }
    });
    let lease_request = json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_lease_request",
        "sent_at": "2026-08-06T03:00:00Z",
        "message": {
            "type": "secret_lease_request",
            "task_id": "task_prepare",
            "lease_id": "lease_01",
            "payload_digest": "sha256:abc",
            "purpose": "git_credential"
        }
    });
    let lease_response = json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_lease_response",
        "sent_at": "2026-08-06T03:00:01Z",
        "message": {
            "type": "secret_lease_response",
            "lease_id": "lease_01",
            "private_key": "opaque-private-key",
            "expires_at": "2026-08-06T03:05:00Z",
            "error_code": null
        }
    });
    for sample in [&progress, &lease_request, &lease_response] {
        assert!(validator.is_valid(sample));
        assert!(serde_json::from_value::<Envelope>(sample.clone()).is_ok());
    }

    let mut unknown_progress_field = progress.clone();
    unknown_progress_field["message"]["event"]["stage"] = json!("deploying");
    assert!(!validator.is_valid(&unknown_progress_field));

    let mut missing_context = progress.clone();
    missing_context["message"]["event"]
        .as_object_mut()
        .unwrap()
        .remove("deploy_id");
    assert!(!validator.is_valid(&missing_context));
}

#[test]
fn rust_messages_round_trip_v2_progress_and_secret_lease() {
    let progress = Envelope {
        protocol_version: PROTOCOL_VERSION,
        message_id: "msg_progress".into(),
        sent_at: "2026-08-06T03:00:00Z".into(),
        message: Message::TaskProgress(TaskProgress {
            task_id: "task_prepare".into(),
            sequence: 1,
            event: DeployEvent {
                deploy_id: "dep_01".into(),
                stage: DeploymentStage::Prepare,
                event: DeployEventName::ModuleStarted,
                timestamp: "2026-08-06T03:00:01Z".into(),
                status: DeployEventStatus::Started,
                environment: Environment::Staging,
                release_version: "20260806183000".into(),
                target: None,
                module: Some("api".into()),
                module_name: Some("API".into()),
                step_id: None,
                step: None,
                message: Some("build api".into()),
                failure_stage: None,
                recovery_hint: None,
                candidate_release: None,
                current_release: None,
                current_switched: None,
            },
        }),
    };
    let request = Envelope {
        protocol_version: PROTOCOL_VERSION,
        message_id: "msg_lease_request".into(),
        sent_at: "2026-08-06T03:00:00Z".into(),
        message: Message::SecretLeaseRequest(SecretLeaseRequest {
            task_id: "task_prepare".into(),
            lease_id: "lease_01".into(),
            payload_digest: "sha256:abc".into(),
            purpose: SecretLeasePurpose::GitCredential,
        }),
    };
    let response = Envelope {
        protocol_version: PROTOCOL_VERSION,
        message_id: "msg_lease_response".into(),
        sent_at: "2026-08-06T03:00:01Z".into(),
        message: Message::SecretLeaseResponse(SecretLeaseResponse {
            lease_id: "lease_01".into(),
            private_key: "opaque-private-key".into(),
            expires_at: "2026-08-06T03:05:00Z".into(),
            error_code: None,
        }),
    };
    for envelope in [progress, request, response] {
        let value = serde_json::to_value(&envelope).unwrap();
        let validator = jsonschema::validator_for(&schema()).unwrap();
        assert!(validator.is_valid(&value));
        assert_eq!(serde_json::from_value::<Envelope>(value).unwrap(), envelope);
    }
}

#[test]
fn v1_legacy_deployment_execute_remains_supported() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let envelope: Envelope = serde_json::from_value(valid_task()).unwrap();
    assert!(validator.is_valid(&serde_json::to_value(&envelope).unwrap()));
    assert_eq!(PROTOCOL_VERSION, 2);
}

#[test]
fn schema_rejects_inline_private_keys_and_unknown_payload_fields() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let prepare = json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_prepare",
        "sent_at": "2026-08-06T03:00:00Z",
        "message": {
            "type": "task_dispatch",
            "task_id": "task_prepare",
            "idempotency_key": "idem-prepare-0123456789",
            "deadline_at": "2026-08-06T03:20:00Z",
            "payload_digest": "sha256:abc",
            "task": {
                "kind": "deployment_prepare",
                "payload": {
                    "deployment_id": "dep_01",
                    "source_policy": "branch",
                    "repository_url": "git@git.example.test:deploy-go/example.git",
                    "commit_sha": "0123456789abcdef0123456789abcdef01234567",
                    "checkout_dir": "/srv/tasks/task_prepare/checkout",
                    "work_root": "/srv/tasks/task_prepare",
                    "output_dir": "/srv/tasks/task_prepare/staging",
                    "environment": "staging",
                    "release_version": "20260806183000",
                    "modules": ["api"],
                    "make_target": "deploy_go_prepare",
                    "git_credential_lease_id": null,
                    "timeout_seconds": 900
                }
            }
        }
    });
    let mut with_key = prepare.clone();
    with_key["message"]["task"]["payload"]["private_key"] = json!("BEGIN PRIVATE KEY");
    assert!(!validator.is_valid(&with_key));

    let mut unknown = prepare.clone();
    unknown["message"]["task"]["payload"]["shell_command"] = json!("id");
    assert!(!validator.is_valid(&unknown));
}
