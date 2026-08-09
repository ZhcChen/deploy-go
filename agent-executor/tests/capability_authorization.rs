use deploy_go_agent_executor::{
    authorization::{AuthorizationError, CapabilityAuthorizer},
    protocol::OpenRequest,
};
use deploy_go_terminal_capability::{CapabilitySigner, Claims, SCHEMA_VERSION};

fn request(signer: &CapabilitySigner, issued_at: i64, expires_at: i64) -> OpenRequest {
    OpenRequest {
        version: 1,
        session_id: "term_01TEST".into(),
        sequence: 0,
        rows: 24,
        cols: 80,
        connection_generation: 7,
        capability: signer
            .sign(&Claims {
                schema_version: SCHEMA_VERSION,
                capability_id: "cap_01TEST".into(),
                node_id: "node_01TEST".into(),
                agent_id: "agent_01TEST".into(),
                session_id: "term_01TEST".into(),
                connection_generation: 7,
                issued_at,
                expires_at,
            })
            .unwrap(),
    }
}

fn authorizer(signer: &CapabilitySigner, replay_dir: std::path::PathBuf) -> CapabilityAuthorizer {
    CapabilityAuthorizer::new(
        signer.verifier(),
        "node_01TEST".into(),
        "agent_01TEST".into(),
        replay_dir,
    )
}

#[test]
fn consumes_once_and_rejects_replay_after_authorizer_restart() {
    let signer = CapabilitySigner::from_seed([7_u8; 32]);
    let replay = tempfile::tempdir().unwrap();
    let request = request(&signer, 100, 115);
    authorizer(&signer, replay.path().into())
        .authorize(&request, 110)
        .unwrap();
    assert_eq!(
        authorizer(&signer, replay.path().into()).authorize(&request, 110),
        Err(AuthorizationError::Replayed)
    );
}

#[test]
fn rejects_expired_tampered_and_wrong_binding_capabilities() {
    let signer = CapabilitySigner::from_seed([7_u8; 32]);
    let replay = tempfile::tempdir().unwrap();
    let expired = request(&signer, 100, 115);
    assert_eq!(
        authorizer(&signer, replay.path().into()).authorize(&expired, 116),
        Err(AuthorizationError::InvalidCapability)
    );
    let mut tampered = request(&signer, 100, 115);
    tampered.session_id = "term_OTHER".into();
    assert_eq!(
        authorizer(&signer, replay.path().into()).authorize(&tampered, 110),
        Err(AuthorizationError::InvalidCapability)
    );
    let mut wrong_generation = request(&signer, 100, 115);
    wrong_generation.connection_generation = 8;
    assert_eq!(
        authorizer(&signer, replay.path().into()).authorize(&wrong_generation, 110),
        Err(AuthorizationError::InvalidCapability)
    );
}

#[test]
fn rejects_symlink_replay_directory() {
    use std::os::unix::fs::symlink;

    let signer = CapabilitySigner::from_seed([7_u8; 32]);
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    std::fs::create_dir(&target).unwrap();
    let replay = root.path().join("replay");
    symlink(&target, &replay).unwrap();

    assert_eq!(
        authorizer(&signer, replay).authorize(&request(&signer, 100, 115), 110),
        Err(AuthorizationError::ReplayStore)
    );
}

#[test]
fn rejects_group_or_world_writable_replay_parent() {
    use std::os::unix::fs::PermissionsExt;

    let signer = CapabilitySigner::from_seed([7_u8; 32]);
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("executor");
    let replay = parent.join("replay");
    std::fs::create_dir_all(&replay).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o777)).unwrap();

    assert_eq!(
        authorizer(&signer, replay).authorize(&request(&signer, 100, 115), 110),
        Err(AuthorizationError::ReplayStore)
    );
}
