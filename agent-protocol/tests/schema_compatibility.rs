use deploy_go_agent_protocol::{
    Envelope, Message, PROTOCOL_VERSION, ReconcileReport, ReconciledTask, ReconciledTaskState,
    TaskResult, TaskTerminalStatus,
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
    future["protocol_version"] = json!(2);
    assert!(!validator.is_valid(&future));

    let mut unknown = valid_task();
    unknown["message"]["type"] = json!("terminal_open");
    assert!(!validator.is_valid(&unknown));
}

#[test]
fn schema_keeps_script_events_out_of_the_control_envelope() {
    let schema_text = include_str!("../schema/agent-control.schema.json");
    assert!(!schema_text.contains("DEPLOY_EVENT"));
    assert!(!schema_text.contains("deploy.finished"));
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
                }),
            }],
        }),
    };
    let value = serde_json::to_value(&envelope).unwrap();
    let validator = jsonschema::validator_for(&schema()).unwrap();
    assert!(validator.is_valid(&value));
    assert_eq!(serde_json::from_value::<Envelope>(value).unwrap(), envelope);
}
