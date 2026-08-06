use std::{fs, path::Path};

use deploy_go_agent::staging::{StagingError, StagingLimits, verify_artifact_dir};
use sha2::{Digest, Sha256};

fn sha256_hex(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn manifest(
    release_version: &str,
    commit_sha: &str,
    entries: &[(&str, &str, &str, u64)],
) -> String {
    let artifacts = entries
        .iter()
        .map(|(module, path, sha, size)| {
            format!(r#"{{"module":"{module}","path":"{path}","sha256":"{sha}","size":{size}}}"#)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"schema_version":1,"release_version":"{release_version}","commit_sha":"{commit_sha}","artifacts":[{artifacts}]}}"#
    )
}

fn write_artifact(dir: &Path, relative: &str, content: &[u8]) -> (String, u64) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    (sha256_hex(content), content.len() as u64)
}

fn limits() -> StagingLimits {
    StagingLimits {
        size_limit_bytes: 1024 * 1024,
        max_files: 16,
    }
}

fn valid_fixture() -> (tempfile::TempDir, String) {
    let directory = tempfile::tempdir().unwrap();
    let artifact_dir = directory.path().join("artifacts");
    fs::create_dir(&artifact_dir).unwrap();
    let (sha, size) = write_artifact(&artifact_dir, "demo/app.txt", b"hello\n");
    fs::write(
        artifact_dir.join("deploy-go-artifact.json"),
        manifest(
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &[("demo", "demo/app.txt", &sha, size)],
        ),
    )
    .unwrap();
    (directory, artifact_dir.display().to_string())
}

#[test]
fn valid_artifact_directory_passes() {
    let (directory, artifact_dir) = valid_fixture();
    let result = verify_artifact_dir(
        Path::new(&artifact_dir),
        "0.1.0",
        "0123456789abcdef0123456789abcdef01234567",
        &["demo".to_owned()],
        &limits(),
    )
    .unwrap();
    assert_eq!(result.release_version, "0.1.0");
    assert_eq!(result.artifacts.len(), 1);
    assert!(directory.path().exists());
}

#[test]
fn tampered_checksum_is_rejected() {
    let (_directory, artifact_dir) = valid_fixture();
    fs::write(Path::new(&artifact_dir).join("demo/app.txt"), b"tampered\n").unwrap();
    assert!(matches!(
        verify_artifact_dir(
            Path::new(&artifact_dir),
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &limits(),
        ),
        Err(StagingError::ChecksumMismatch | StagingError::SizeMismatch)
    ));
}

#[test]
fn undeclared_file_is_rejected() {
    let (_directory, artifact_dir) = valid_fixture();
    fs::write(Path::new(&artifact_dir).join("demo/extra.txt"), b"extra").unwrap();
    assert!(matches!(
        verify_artifact_dir(
            Path::new(&artifact_dir),
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &limits(),
        ),
        Err(StagingError::UndeclaredFile)
    ));
}

#[test]
fn missing_file_is_rejected() {
    let (_directory, artifact_dir) = valid_fixture();
    fs::remove_file(Path::new(&artifact_dir).join("demo/app.txt")).unwrap();
    assert!(matches!(
        verify_artifact_dir(
            Path::new(&artifact_dir),
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &limits(),
        ),
        Err(StagingError::MissingFile)
    ));
}

#[cfg(unix)]
#[test]
fn symlink_inside_artifact_directory_is_rejected() {
    let (_directory, artifact_dir) = valid_fixture();
    std::os::unix::fs::symlink(
        Path::new(&artifact_dir).join("demo/app.txt"),
        Path::new(&artifact_dir).join("demo/link.txt"),
    )
    .unwrap();
    assert!(matches!(
        verify_artifact_dir(
            Path::new(&artifact_dir),
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &limits(),
        ),
        Err(StagingError::SymlinkForbidden)
    ));
}

#[test]
fn path_escape_and_missing_manifest_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let artifact_dir = directory.path().join("artifacts");
    fs::create_dir(&artifact_dir).unwrap();
    fs::write(
        artifact_dir.join("deploy-go-artifact.json"),
        manifest(
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &[("demo", "../outside", "0", 0)],
        ),
    )
    .unwrap();
    assert!(matches!(
        verify_artifact_dir(
            &artifact_dir,
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &limits(),
        ),
        Err(StagingError::InvalidManifest | StagingError::PathEscape)
    ));

    let empty = directory.path().join("empty");
    fs::create_dir(&empty).unwrap();
    assert!(matches!(
        verify_artifact_dir(
            &empty,
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &limits()
        ),
        Err(StagingError::MissingManifest)
    ));
}

#[test]
fn duplicate_module_and_module_mismatch_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let artifact_dir = directory.path().join("artifacts");
    fs::create_dir(&artifact_dir).unwrap();
    let (sha, size) = write_artifact(&artifact_dir, "demo/app.txt", b"hello\n");
    fs::write(
        artifact_dir.join("deploy-go-artifact.json"),
        format!(
            r#"{{"schema_version":1,"release_version":"0.1.0","commit_sha":"0123456789abcdef0123456789abcdef01234567","artifacts":[{{"module":"demo","path":"demo/app.txt","sha256":"{sha}","size":{size}}},{{"module":"demo","path":"demo/app.txt","sha256":"{sha}","size":{size}}}]}}"#
        ),
    )
    .unwrap();
    assert!(matches!(
        verify_artifact_dir(
            &artifact_dir,
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &limits(),
        ),
        Err(StagingError::DuplicateModule | StagingError::ModuleMismatch)
    ));

    assert!(matches!(
        verify_artifact_dir(
            &artifact_dir,
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["other".to_owned()],
            &limits(),
        ),
        Err(StagingError::ModuleMismatch)
    ));
}

#[test]
fn release_version_commit_and_limits_are_enforced() {
    let (_directory, artifact_dir) = valid_fixture();
    assert!(matches!(
        verify_artifact_dir(
            Path::new(&artifact_dir),
            "0.2.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &limits(),
        ),
        Err(StagingError::ReleaseVersionMismatch)
    ));
    assert!(matches!(
        verify_artifact_dir(
            Path::new(&artifact_dir),
            "0.1.0",
            "abcdef0123456789abcdef0123456789abcdef01",
            &["demo".to_owned()],
            &limits(),
        ),
        Err(StagingError::CommitMismatch)
    ));
    assert!(matches!(
        verify_artifact_dir(
            Path::new(&artifact_dir),
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &StagingLimits {
                size_limit_bytes: 1,
                max_files: 16,
            },
        ),
        Err(StagingError::LimitExceeded)
    ));
    assert!(matches!(
        verify_artifact_dir(
            Path::new(&artifact_dir),
            "0.1.0",
            "0123456789abcdef0123456789abcdef01234567",
            &["demo".to_owned()],
            &StagingLimits {
                size_limit_bytes: 1024 * 1024,
                max_files: 1,
            },
        ),
        Err(StagingError::LimitExceeded)
    ));
}
