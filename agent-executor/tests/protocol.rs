use deploy_go_agent_executor::{
    peer_auth::{PeerCredentials, PeerPolicy},
    protocol::{
        MAX_FRAME_BYTES, OpenRequest, PROTOCOL_VERSION, ReleaseStartRequest, Request, read_request,
        validate_dimensions, validate_request_sequence,
    },
};

fn framed(payload: &[u8]) -> Vec<u8> {
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(payload);
    frame
}

#[tokio::test]
async fn rejects_unknown_or_privilege_override_fields() {
    let payload = br#"{"operation":"open","version":1,"session_id":"s1","sequence":1,"rows":24,"cols":80,"shell":"/bin/bash"}"#;
    let error = read_request(&mut framed(payload).as_slice(), MAX_FRAME_BYTES)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("invalid protocol message"));
}

#[tokio::test]
async fn release_start_rejects_commands_arguments_and_arbitrary_environment() {
    let base = serde_json::json!({
        "operation":"release_start",
        "version":PROTOCOL_VERSION,
        "job_id":"release_01TEST",
        "authorization":"signed-release-authorization",
        "deployment_id":"deployment_01TEST",
        "target_run_id":"run_01TEST",
        "target_id":"target_01TEST",
        "node_id":"node_01TEST",
        "agent_id":"agent_01TEST",
        "snapshot_hash":format!("sha256:{}", "a".repeat(64)),
        "commit_sha":"0123456789abcdef0123456789abcdef01234567",
        "checkout_dir":"/srv/tasks/release/checkout",
        "artifact_dir":"/srv/tasks/release/artifacts",
        "env_dir":"/srv/tasks/release/env",
        "cancel_file":"/srv/tasks/release/cancel",
        "environment":"test",
        "release_version":"20260810000000",
        "modules":["api"],
        "target_code":"test",
        "task_payload_digest":format!("sha256:{}", "b".repeat(64)),
        "deadline_at":200
    });
    for (field, value) in [
        ("command", serde_json::json!("id")),
        ("executable", serde_json::json!("/bin/bash")),
        ("args", serde_json::json!(["-c", "id"])),
        ("make_target", serde_json::json!("other")),
        ("env", serde_json::json!({"TOKEN":"secret"})),
    ] {
        let mut unsafe_request = base.clone();
        unsafe_request[field] = value;
        let bytes = serde_json::to_vec(&unsafe_request).unwrap();
        assert!(
            read_request(&mut framed(&bytes).as_slice(), MAX_FRAME_BYTES)
                .await
                .is_err(),
            "accepted unsafe field {field}"
        );
    }
}

#[tokio::test]
async fn decodes_structured_release_start_without_execution_overrides() {
    let request = Request::ReleaseStart(ReleaseStartRequest {
        version: PROTOCOL_VERSION,
        job_id: "release_01TEST".into(),
        authorization: "signed-release-authorization".into(),
        deployment_id: "deployment_01TEST".into(),
        target_run_id: "run_01TEST".into(),
        target_id: "target_01TEST".into(),
        node_id: "node_01TEST".into(),
        agent_id: "agent_01TEST".into(),
        snapshot_hash: format!("sha256:{}", "a".repeat(64)),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        checkout_dir: "/srv/tasks/release/checkout".into(),
        artifact_dir: "/srv/tasks/release/artifacts".into(),
        env_dir: "/srv/tasks/release/env".into(),
        cancel_file: "/srv/tasks/release/cancel".into(),
        environment: "test".into(),
        release_version: "20260810000000".into(),
        modules: vec!["api".into()],
        target_code: "test".into(),
        task_payload_digest: format!("sha256:{}", "b".repeat(64)),
        deadline_at: 200,
    });
    let payload = serde_json::to_vec(&request).unwrap();
    assert_eq!(
        read_request(&mut framed(&payload).as_slice(), MAX_FRAME_BYTES)
            .await
            .unwrap(),
        Some(request)
    );
}

#[tokio::test]
async fn rejects_oversized_frame_before_allocating_payload() {
    let frame = ((MAX_FRAME_BYTES + 1) as u32).to_be_bytes();
    let error = read_request(&mut frame.as_slice(), MAX_FRAME_BYTES)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "frame exceeds configured limit");
}

#[tokio::test]
async fn decodes_strict_open_request() {
    let request = Request::Open(OpenRequest {
        version: 1,
        session_id: "s1".into(),
        sequence: 0,
        rows: 24,
        cols: 80,
        connection_generation: 7,
        capability: "signed-capability".into(),
    });
    let payload = serde_json::to_vec(&request).unwrap();
    assert_eq!(
        read_request(&mut framed(&payload).as_slice(), MAX_FRAME_BYTES)
            .await
            .unwrap(),
        Some(request)
    );
}

#[test]
fn authorizes_both_peer_uid_and_gid() {
    let policy = PeerPolicy {
        allowed_uid: 501,
        allowed_gid: 20,
    };
    assert!(policy.authorizes(PeerCredentials {
        uid: 501,
        gid: 20,
        pid: None
    }));
    assert!(!policy.authorizes(PeerCredentials {
        uid: 0,
        gid: 20,
        pid: None
    }));
    assert!(!policy.authorizes(PeerCredentials {
        uid: 501,
        gid: 0,
        pid: None
    }));
}

#[test]
fn terminal_dimensions_are_bounded() {
    assert!(validate_dimensions(24, 80));
    assert!(!validate_dimensions(1, 80));
    assert!(!validate_dimensions(24, 501));
}

#[test]
fn open_sequence_is_zero_and_followups_are_contiguous() {
    let open = Request::Open(OpenRequest {
        version: 1,
        session_id: "s1".into(),
        sequence: 0,
        rows: 24,
        cols: 80,
        connection_generation: 7,
        capability: "signed-capability".into(),
    });
    assert!(validate_request_sequence(&open, None));
    let mut invalid_open = open.clone();
    let Request::Open(value) = &mut invalid_open else {
        unreachable!()
    };
    value.sequence = 1;
    assert!(!validate_request_sequence(&invalid_open, None));
}
