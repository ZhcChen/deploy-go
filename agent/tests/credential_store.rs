use std::fs;

use deploy_go_agent::credential_store::{AgentCredentials, CredentialError, CredentialStore};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn credentials(suffix: &str) -> AgentCredentials {
    AgentCredentials {
        agent_id: "agent_01".to_owned(),
        refresh_token: format!("refresh_{suffix}_012345678901234567890123456789"),
    }
}

#[test]
fn credentials_are_stored_atomically_with_owner_only_permissions() {
    let directory = tempfile::tempdir().unwrap();
    let data_dir = directory.path().join("agent-data");
    let path = data_dir.join("credentials.json");
    let store = CredentialStore::new(path.clone());

    store.store(&credentials("first")).unwrap();
    store.store(&credentials("second")).unwrap();
    assert_eq!(store.load().unwrap(), credentials("second"));
    let serialized = fs::read_to_string(&path).unwrap();
    assert!(!serialized.contains("first"));

    #[cfg(unix)]
    {
        assert_eq!(
            fs::metadata(&data_dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
#[cfg(unix)]
fn unsafe_directory_or_file_permissions_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let data_dir = directory.path().join("agent-data");
    fs::create_dir(&data_dir).unwrap();
    fs::set_permissions(&data_dir, fs::Permissions::from_mode(0o755)).unwrap();
    let path = data_dir.join("credentials.json");
    fs::write(&path, serde_json::to_vec(&credentials("unsafe")).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let store = CredentialStore::new(path.clone());
    assert!(matches!(
        store.load(),
        Err(CredentialError::UnsafeDirectoryPermissions)
    ));

    fs::set_permissions(&data_dir, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        store.load(),
        Err(CredentialError::UnsafeFilePermissions)
    ));
}

#[test]
#[cfg(unix)]
fn symbolic_link_credentials_are_rejected() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let data_dir = directory.path().join("agent-data");
    fs::create_dir(&data_dir).unwrap();
    fs::set_permissions(&data_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let target = directory.path().join("outside.json");
    fs::write(
        &target,
        serde_json::to_vec(&credentials("outside")).unwrap(),
    )
    .unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    let path = data_dir.join("credentials.json");
    symlink(&target, &path).unwrap();

    assert!(matches!(
        CredentialStore::new(path).load(),
        Err(CredentialError::Io(_))
    ));
}

#[test]
fn debug_output_does_not_expose_the_refresh_token() {
    let credentials = credentials("debug");
    let token = credentials.refresh_token.clone();
    let store = CredentialStore::new("/var/lib/deploy-go-agent/credentials.json".into());
    assert!(!format!("{store:?}").contains(&token));
    assert!(!format!("{credentials:?}").contains(&token));
    assert!(format!("{credentials:?}").contains("[REDACTED]"));
}
