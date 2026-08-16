use deploy_go_agent_protocol::{
    AgentCapability, ArtifactPrepared, ArtifactUploadAuthorized, DeployEvent, DeployEventName,
    DeployEventStatus, DeploymentStage, Envelope, Environment, Message, MessageDirection,
    PROTOCOL_VERSION, ReconcileReport, ReconciledTask, ReconciledTaskState, ReleaseCheckoutMode,
    SecretLeasePurpose, SecretLeaseRequest, SecretLeaseResponse, TaskProgress, TaskResult,
    TaskTerminalStatus, TerminalSequenceError, TerminalSequenceTracker,
};
use serde_json::{Value, json};

fn schema() -> Value {
    serde_json::from_str(include_str!("../schema/agent-control.schema.json")).unwrap()
}

fn v11_schema() -> Value {
    serde_json::from_str(include_str!("../schema/agent-control-v11.schema.json")).unwrap()
}

#[test]
fn v11_schema_and_hello_ack_wire_shape_remain_immutable() {
    let schema = v11_schema();
    assert_eq!(
        schema["$id"],
        "https://deploy-go.local/schemas/agent-control-v11.json"
    );
    assert_eq!(schema["properties"]["protocol_version"]["const"], 11);
    assert!(schema["$defs"].get("node_telemetry").is_none());

    let ack = json!({
        "protocol_version": 11,
        "message_id": "msg_v11_ack",
        "sent_at": "2026-08-16T00:00:00Z",
        "message": {
            "type": "hello_ack",
            "connection_id": "conn_01",
            "connection_generation": 1,
            "protocol_version": 11,
            "heartbeat_interval_seconds": 15
        }
    });
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&ack));
    let parsed: Envelope = serde_json::from_value(ack).unwrap();
    let Message::HelloAck(ack) = parsed.message else {
        panic!("expected hello ack");
    };
    assert_eq!(ack.telemetry_interval_seconds, None);
    assert!(
        serde_json::to_value(Message::HelloAck(ack)).unwrap()["telemetry_interval_seconds"]
            .is_null()
    );

    let mut ack_with_telemetry = json!({
        "protocol_version": 11,
        "message_id": "msg_v11_ack",
        "sent_at": "2026-08-16T00:00:00Z",
        "message": {
            "type": "hello_ack",
            "connection_id": "conn_01",
            "connection_generation": 1,
            "protocol_version": 11,
            "heartbeat_interval_seconds": 15,
            "telemetry_interval_seconds": 30
        }
    });
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(!validator.is_valid(&ack_with_telemetry));
    ack_with_telemetry["message"] = telemetry_envelope()["message"].clone();
    assert!(!validator.is_valid(&ack_with_telemetry));
}

#[test]
fn v12_hello_ack_requires_a_valid_telemetry_interval() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let ack = json!({
        "protocol_version": 12,
        "message_id": "msg_v12_ack",
        "sent_at": "2026-08-16T00:00:00Z",
        "message": {
            "type": "hello_ack",
            "connection_id": "conn_01",
            "connection_generation": 1,
            "protocol_version": 12,
            "heartbeat_interval_seconds": 15,
            "telemetry_interval_seconds": 30
        }
    });
    assert!(validator.is_valid(&ack));
    let parsed: Envelope = serde_json::from_value(ack).unwrap();
    let Message::HelloAck(ack) = parsed.message else {
        panic!("expected hello ack");
    };
    assert!(ack.validate_for_envelope_version(12));

    let serialized = serde_json::to_value(Message::HelloAck(ack)).unwrap();
    assert_eq!(serialized["telemetry_interval_seconds"], 30);
    let mut missing_interval = serialized;
    missing_interval
        .as_object_mut()
        .unwrap()
        .remove("telemetry_interval_seconds");
    let envelope = json!({
        "protocol_version": 12,
        "message_id": "msg_v12_ack_missing_interval",
        "sent_at": "2026-08-16T00:00:00Z",
        "message": missing_interval
    });
    assert!(!validator.is_valid(&envelope));
}

#[test]
fn v12_schema_accepts_strict_agent_to_server_telemetry() {
    let telemetry = telemetry_envelope();
    assert!(
        jsonschema::validator_for(&schema())
            .unwrap()
            .is_valid(&telemetry)
    );
    let parsed: Envelope = serde_json::from_value(telemetry).unwrap();
    parsed
        .message
        .validate_direction(MessageDirection::AgentToServer)
        .unwrap();
    assert!(
        parsed
            .message
            .validate_direction(MessageDirection::ServerToAgent)
            .is_err()
    );
    let Message::NodeTelemetry(telemetry) = parsed.message else {
        panic!("expected node telemetry");
    };
    telemetry.validate().unwrap();
}

#[test]
fn telemetry_rejects_unknown_status_fields_and_invalid_values() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let mut unknown_status = telemetry_envelope();
    unknown_status["message"]["snapshot"]["cpu"]["status"] = json!("degraded");
    assert!(!validator.is_valid(&unknown_status));
    assert!(serde_json::from_value::<Envelope>(unknown_status).is_err());

    let mut unknown_field = telemetry_envelope();
    unknown_field["message"]["snapshot"]["network"]["interface"] = json!("eth0");
    assert!(!validator.is_valid(&unknown_field));
    assert!(serde_json::from_value::<Envelope>(unknown_field).is_err());

    let mut duplicate_gpu = telemetry_envelope();
    let gpu = duplicate_gpu["message"]["snapshot"]["gpus"][0].clone();
    duplicate_gpu["message"]["snapshot"]["gpus"] = json!([gpu.clone(), gpu]);
    let parsed: Envelope = serde_json::from_value(duplicate_gpu).unwrap();
    let Message::NodeTelemetry(telemetry) = parsed.message else {
        panic!("expected node telemetry");
    };
    assert!(telemetry.validate().is_err());
}

fn telemetry_envelope() -> Value {
    json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_telemetry_01",
        "sent_at": "2026-08-16T00:00:00Z",
        "message": {
            "type": "node_telemetry",
            "connection_generation": 7,
            "sample_sequence": 1,
            "captured_at": "2026-08-16T00:00:00Z",
            "snapshot": {
                "cpu": {"status": "available", "usage_percent": 42.5},
                "memory": {"status": "available", "total_bytes": 17179869184_u64, "used_bytes": 8589934592_u64, "usage_percent": 50.0},
                "work_root_disk": {"status": "available", "total_bytes": 107374182400_u64, "used_bytes": 53687091200_u64, "usage_percent": 50.0},
                "disk_io": {"status": "available", "read_bytes_per_second": 1024.0, "write_bytes_per_second": 2048.0, "busy_percent": 12.5},
                "network": {"status": "available", "receive_bytes_per_second": 4096.0, "transmit_bytes_per_second": 8192.0},
                "gpu_status": "available",
                "gpus": [{
                    "index": 0,
                    "status": "available",
                    "model": "NVIDIA Test GPU",
                    "utilization_percent": 25.0,
                    "memory_total_bytes": 8589934592_u64,
                    "memory_used_bytes": 2147483648_u64,
                    "temperature_celsius": 55.0
                }]
            }
        }
    })
}

#[test]
fn release_authorization_messages_are_directional_and_reject_execution_controls() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let request = terminal_envelope(json!({
        "type": "release_authorization_request",
        "task_id": "task_release",
        "authorization_id": "release_auth_01",
        "target_run_id": "run_01",
        "target_id": "target_01",
        "snapshot_hash": "a".repeat(64),
        "checkout_tree_digest": "b".repeat(64),
        "artifact_manifest_digest": "c".repeat(64),
        "artifacts": [{"relative_path":"api.tar.gz","digest":"d".repeat(64)}],
        "env_files": [{"relative_path":"api.env","digest":"e".repeat(64)}],
        "cancel_file": "/srv/deploy-go/tasks/task_release/cancel"
    }));
    assert!(validator.is_valid(&request));
    let request: Envelope = serde_json::from_value(request).unwrap();
    request
        .message
        .validate_direction(MessageDirection::AgentToServer)
        .unwrap();
    assert!(
        request
            .message
            .validate_direction(MessageDirection::ServerToAgent)
            .is_err()
    );

    let response = terminal_envelope(json!({
        "type": "release_authorization_response",
        "task_id": "task_release",
        "authorization_id": "release_auth_01",
        "authorization": "x".repeat(64),
        "error_code": null
    }));
    assert!(validator.is_valid(&response));
    let response: Envelope = serde_json::from_value(response).unwrap();
    response
        .message
        .validate_direction(MessageDirection::ServerToAgent)
        .unwrap();

    for field in ["command", "executable", "args", "environment"] {
        let mut unsafe_request = serde_json::to_value(&request).unwrap();
        unsafe_request["message"][field] = json!("id");
        assert!(!validator.is_valid(&unsafe_request));
        assert!(serde_json::from_value::<Envelope>(unsafe_request).is_err());
    }
}

fn terminal_envelope(message: Value) -> Value {
    json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_terminal",
        "sent_at": "2026-08-07T00:00:00Z",
        "message": message
    })
}

#[test]
fn v8_terminal_messages_match_rust_and_schema() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let messages = [
        json!({"type":"terminal_open","session_id":"session_01","sequence":0,"columns":120,"rows":40,"connection_generation":7,"capability":"signed-capability-value-that-is-long-enough"}),
        json!({"type":"terminal_opened","session_id":"session_01","sequence":1}),
        json!({"type":"terminal_input","session_id":"session_01","sequence":2,"encoding":"base64","data":"aWQK"}),
        json!({"type":"terminal_output","session_id":"session_01","sequence":3,"encoding":"base64","data":"dWlkPTAK"}),
        json!({"type":"terminal_resize","session_id":"session_01","sequence":4,"columns":160,"rows":50}),
        json!({"type":"terminal_close","session_id":"session_01","sequence":5,"reason":"administrator_request"}),
        json!({"type":"terminal_exited","session_id":"session_01","sequence":6,"reason":"process_exited","exit_code":0}),
    ];
    for message in messages {
        let envelope = terminal_envelope(message);
        assert!(validator.is_valid(&envelope), "schema rejected {envelope}");
        assert!(serde_json::from_value::<Envelope>(envelope).is_ok());
    }
}

#[test]
fn terminal_schema_rejects_unsafe_open_invalid_frames_and_unknown_fields() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let invalid = [
        json!({"type":"terminal_open","session_id":"session_01","sequence":0,"columns":0,"rows":40}),
        json!({"type":"terminal_open","session_id":"session_01","sequence":1,"columns":120,"rows":40}),
        json!({"type":"terminal_open","session_id":"session_01","sequence":0,"columns":120,"rows":1001}),
        json!({"type":"terminal_open","session_id":"session_01","sequence":0,"columns":120,"rows":40,"command":"id"}),
        json!({"type":"terminal_open","session_id":"session_01","sequence":0,"columns":120,"rows":40,"shell":"/bin/bash"}),
        json!({"type":"terminal_open","session_id":"session_01","sequence":0,"columns":120,"rows":40,"user":"root"}),
        json!({"type":"terminal_open","session_id":"session_01","sequence":0,"columns":120,"rows":40,"env":{"TOKEN":"secret"}}),
        json!({"type":"terminal_input","session_id":"session_01","sequence":1,"encoding":"utf8","data":"id"}),
        json!({"type":"terminal_input","session_id":"session_01","sequence":1,"encoding":"base64","data":"x".repeat(87385)}),
        json!({"type":"terminal_resize","session_id":"session_01","sequence":2,"columns":120,"rows":40,"extra":true}),
    ];
    for message in invalid {
        let envelope = terminal_envelope(message);
        assert!(!validator.is_valid(&envelope), "schema accepted {envelope}");
        assert!(serde_json::from_value::<Envelope>(envelope).is_err());
    }
}

#[test]
fn terminal_sequence_and_direction_are_explicitly_enforced() {
    let open: Envelope = serde_json::from_value(terminal_envelope(
        json!({"type":"terminal_open","session_id":"session_01","sequence":0,"columns":120,"rows":40,"connection_generation":7,"capability":"signed-capability-value-that-is-long-enough"}),
    ))
    .unwrap();
    open.message
        .validate_direction(MessageDirection::ServerToAgent)
        .unwrap();
    assert!(
        open.message
            .validate_direction(MessageDirection::AgentToServer)
            .is_err()
    );

    let mut server_tracker = TerminalSequenceTracker::new("session_01");
    server_tracker.accept("session_01", 0).unwrap();
    server_tracker.accept("session_01", 1).unwrap();
    assert_eq!(
        server_tracker.accept("session_01", 1),
        Err(TerminalSequenceError::Unexpected {
            expected: 2,
            received: 1,
        })
    );
    assert_eq!(
        server_tracker.accept("session_01", 3),
        Err(TerminalSequenceError::Unexpected {
            expected: 2,
            received: 3,
        })
    );
    assert_eq!(
        server_tracker.accept("another_session", 2),
        Err(TerminalSequenceError::WrongSession)
    );

    let mut agent_tracker = TerminalSequenceTracker::with_first_sequence("session_01", 1);
    agent_tracker.accept("session_01", 1).unwrap();
    agent_tracker.accept("session_01", 2).unwrap();
}

#[test]
fn legacy_hello_is_rejected_by_version_validation() {
    let hello = json!({
        "protocol_version": 4,
        "message_id": "msg_v4_hello",
        "sent_at": "2026-08-07T00:00:00Z",
        "message": {
            "type": "hello",
            "agent_id": "agent_01",
            "agent_version": "0.1.0",
            "min_protocol_version": 1,
            "max_protocol_version": 4,
            "os": "linux",
            "architecture": "x86_64"
        }
    });
    let parsed: Envelope = serde_json::from_value(hello).unwrap();
    match &parsed.message {
        Message::Hello(hello) => assert!(hello.capabilities.is_empty()),
        _ => panic!("expected hello"),
    }
    assert!(parsed.validate_version().is_err());
    assert!(
        serde_json::from_value::<Envelope>(valid_task())
            .unwrap()
            .validate_version()
            .is_ok()
    );
    assert_eq!(AgentCapability::PtyTerminal.to_string(), "pty_terminal");
    assert_eq!(
        AgentCapability::PrivilegedRelease.to_string(),
        "privileged_release"
    );
}

#[test]
fn release_privileged_field_is_wire_compatible_and_schema_rejects_unknown_controls() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let mut task = valid_release_task();
    assert!(validator.is_valid(&task));
    let parsed: Envelope = serde_json::from_value(task.clone()).unwrap();
    let Message::TaskDispatch(dispatch) = parsed.message else {
        panic!("expected task dispatch");
    };
    let deploy_go_agent_protocol::TaskPayload::DeploymentRelease(release) = dispatch.task else {
        panic!("expected release task");
    };
    assert!(!release.privileged);

    task["message"]["task"]["payload"]["privileged"] = json!(true);
    task["message"]["task"]["payload"]["privileged_context"] = json!({
        "target_run_id":"run_01",
        "target_id":"target_01",
        "node_id":"node_01",
        "agent_id":"agent_01",
        "snapshot_hash":"a".repeat(64)
    });
    assert!(validator.is_valid(&task));
    let serialized =
        serde_json::to_value(serde_json::from_value::<Envelope>(task).unwrap()).unwrap();
    assert_eq!(serialized["message"]["task"]["payload"]["privileged"], true);
    assert_eq!(
        serialized["message"]["task"]["payload"]["privileged_context"]["target_id"],
        "target_01"
    );

    let mut unsafe_task = valid_release_task();
    unsafe_task["message"]["task"]["payload"]["command"] = json!("id");
    assert!(!validator.is_valid(&unsafe_task));
    assert!(serde_json::from_value::<Envelope>(unsafe_task).is_err());
}

#[test]
fn artifact_checkout_mode_is_wire_compatible_without_template_payload() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let mut task = valid_release_task();
    task["message"]["task"]["payload"]["checkout_mode"] = json!("artifact");
    assert!(validator.is_valid(&task));
    let parsed: Envelope = serde_json::from_value(task.clone()).unwrap();
    let Message::TaskDispatch(dispatch) = parsed.message else {
        panic!("expected task dispatch");
    };
    let deploy_go_agent_protocol::TaskPayload::DeploymentRelease(release) = dispatch.task else {
        panic!("expected release task");
    };
    assert_eq!(release.checkout_mode, ReleaseCheckoutMode::Artifact);
    let serialized =
        serde_json::to_value(serde_json::from_value::<Envelope>(task).unwrap()).unwrap();
    assert_eq!(
        serialized["message"]["task"]["payload"]["checkout_mode"],
        "artifact"
    );

    let mut template_payload = valid_release_task();
    template_payload["message"]["task"]["payload"]["image_spec"] = json!({
        "template": "etcd",
        "image": "gcr.io/etcd-development/etcd:v3.6.14"
    });
    assert!(!validator.is_valid(&template_payload));
    assert!(serde_json::from_value::<Envelope>(template_payload).is_err());
}

fn valid_release_task() -> Value {
    json!({
        "protocol_version": PROTOCOL_VERSION,
        "message_id": "msg_release",
        "sent_at": "2026-08-10T00:00:00Z",
        "message": {
            "type": "task_dispatch",
            "task_id": "task_release",
            "idempotency_key": "deployment:dep_01:release",
            "deadline_at": "2026-08-10T00:10:00Z",
            "payload_digest": "sha256:abc",
            "task": {
                "kind": "deployment_release",
                "payload": {
                    "deployment_id": "dep_01",
                    "target_code": "test",
                    "work_root": "/srv/deploy-go",
                    "checkout_dir": "/srv/deploy-go/deployments/dep_01/checkout",
                    "artifact_dir": "/srv/deploy-go/deployments/dep_01/staging",
                    "environment": "test",
                    "release_version": "20260810000000",
                    "commit_sha": "0123456789abcdef0123456789abcdef01234567",
                    "modules": ["api"],
                    "make_target": "deploy_go_release",
                    "timeout_seconds": 600,
                    "cancel_file": "/srv/deploy-go/deployments/dep_01/cancel"
                }
            }
        }
    })
}

#[test]
fn artifact_authorization_messages_round_trip_without_bytes_or_tokens() {
    let messages = [
        Message::ArtifactPrepared(ArtifactPrepared {
            task_id: "task_prepare".into(),
            authorization_id: "artifact_auth_1".into(),
            deployment_id: "deployment_1".into(),
            manifest_json: r#"{"schema_version":1}"#.into(),
            manifest_digest: "a".repeat(64),
            total_size: 1,
            file_count: 1,
            archive_size: 1024,
            archive_digest: "b".repeat(64),
        }),
        Message::ArtifactUploadAuthorized(ArtifactUploadAuthorized {
            task_id: "task_prepare".into(),
            authorization_id: "artifact_auth_1".into(),
            lease_id: Some("artifact_lease_1".into()),
            error_code: None,
        }),
    ];
    let validator = jsonschema::validator_for(&schema()).unwrap();
    for message in messages {
        let envelope = Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: "message_artifact".into(),
            sent_at: "2026-08-07T00:00:00Z".into(),
            message,
        };
        let value = serde_json::to_value(&envelope).unwrap();
        assert!(validator.is_valid(&value));
        let serialized = serde_json::to_string(&value).unwrap();
        assert!(!serialized.contains("access_token"));
        assert_eq!(serde_json::from_value::<Envelope>(value).unwrap(), envelope);
    }
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
    future["protocol_version"] = json!(PROTOCOL_VERSION + 1);
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
                    "work_root": "/srv/tasks/task_release",
                    "checkout_dir": "/srv/tasks/task_release/checkout",
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
fn legacy_deployment_execute_is_rejected() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let envelope: Envelope = serde_json::from_value(valid_task()).unwrap();
    assert!(validator.is_valid(&serde_json::to_value(&envelope).unwrap()));
    let mut legacy = valid_task();
    legacy["protocol_version"] = json!(1);
    assert!(!validator.is_valid(&legacy));
    assert!(
        serde_json::from_value::<Envelope>(legacy)
            .unwrap()
            .validate_version()
            .is_err()
    );
    assert_eq!(PROTOCOL_VERSION, 12);
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
