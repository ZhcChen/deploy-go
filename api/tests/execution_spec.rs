#[cfg(unix)]
#[test]
fn canonical_symlink_escape_is_rejected() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let outside = temp.path().join("outside");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    symlink(&outside, root.join("escaped")).unwrap();
    let canonical_root = root.canonicalize().unwrap();
    let resolved = root.join("escaped").canonicalize().unwrap();
    assert!(
        deploy_go_api::execution_spec::validate_resolved_path(
            &canonical_root,
            &resolved,
            "req_test"
        )
        .is_err()
    );
}
