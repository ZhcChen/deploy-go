use std::{fs, os::unix::fs::PermissionsExt};

use deploy_go_agent::env_sync::{EnvFileStore, EnvSyncError};
use sha2::{Digest, Sha256};

fn digest(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}

#[test]
fn writes_and_replaces_env_atomically_with_private_permissions() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("secrets");
    fs::create_dir(&root).unwrap();
    let store = EnvFileStore::new(root.clone()).unwrap();

    let first = b"SECRET=first\n";
    let path = store
        .write("voucher-production", "api.env", first, &digest(first))
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), first);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let second = b"SECRET=second\n";
    store
        .write("voucher-production", "api.env", second, &digest(second))
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), second);
    assert!(fs::read_dir(path.parent().unwrap()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".env-sync-")
    }));
}

#[test]
fn failed_validation_preserves_the_previous_file() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("secrets");
    fs::create_dir(&root).unwrap();
    let store = EnvFileStore::new(root.clone()).unwrap();
    let initial = b"A=old\n";
    let path = store
        .write("app-production", "api.env", initial, &digest(initial))
        .unwrap();

    assert!(matches!(
        store.write("app-production", "api.env", b"A=new\n", &digest(b"other")),
        Err(EnvSyncError::DigestMismatch)
    ));
    assert_eq!(fs::read(path).unwrap(), initial);
}

#[test]
fn rejects_parent_symlink_hardlink_and_non_regular_targets() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("secrets");
    let outside = directory.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    symlink(&outside, root.join("linked-app")).unwrap();
    let store = EnvFileStore::new(root.clone()).unwrap();
    let content = b"A=1\n";
    assert!(matches!(
        store.write("linked-app", "api.env", content, &digest(content)),
        Err(EnvSyncError::UnsafeTarget)
    ));
    assert!(!outside.join("api.env").exists());

    fs::create_dir(root.join("safe-app")).unwrap();
    fs::write(root.join("safe-app/original"), b"old").unwrap();
    fs::hard_link(
        root.join("safe-app/original"),
        root.join("safe-app/api.env"),
    )
    .unwrap();
    assert!(matches!(
        store.write("safe-app", "api.env", content, &digest(content)),
        Err(EnvSyncError::UnsafeTarget)
    ));

    fs::remove_file(root.join("safe-app/api.env")).unwrap();
    fs::create_dir(root.join("safe-app/api.env")).unwrap();
    assert!(matches!(
        store.delete("safe-app", "api.env"),
        Err(EnvSyncError::UnsafeTarget)
    ));
}

#[test]
fn delete_is_idempotent_and_never_follows_a_symlink() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("secrets");
    fs::create_dir(&root).unwrap();
    let store = EnvFileStore::new(root.clone()).unwrap();
    let content = b"A=1\n";
    let path = store
        .write("app-test", "api.env", content, &digest(content))
        .unwrap();
    store.delete("app-test", "api.env").unwrap();
    store.delete("app-test", "api.env").unwrap();
    assert!(!path.exists());

    let outside = directory.path().join("outside.env");
    fs::write(&outside, b"outside").unwrap();
    symlink(&outside, &path).unwrap();
    assert!(matches!(
        store.delete("app-test", "api.env"),
        Err(EnvSyncError::UnsafeTarget)
    ));
    assert_eq!(fs::read(outside).unwrap(), b"outside");
}
