use deploy_go_agent_executor::{
    peer_auth::{PeerCredentials, PeerPolicy},
    protocol::{
        MAX_FRAME_BYTES, OpenRequest, Request, read_request, validate_dimensions,
        validate_request_sequence,
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
    assert!(policy.authorizes(PeerCredentials { uid: 501, gid: 20 }));
    assert!(!policy.authorizes(PeerCredentials { uid: 0, gid: 20 }));
    assert!(!policy.authorizes(PeerCredentials { uid: 501, gid: 0 }));
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
    });
    assert!(validate_request_sequence(&open, None));
    let mut invalid_open = open.clone();
    let Request::Open(value) = &mut invalid_open else {
        unreachable!()
    };
    value.sequence = 1;
    assert!(!validate_request_sequence(&invalid_open, None));
}
