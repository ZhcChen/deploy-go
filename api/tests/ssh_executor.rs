use deploy_go_api::executor::ssh::{NodeProbe, NodeProbeInput, OpenSshProbe};

#[tokio::test]
async fn openssh_probe_uses_isolated_fixture_with_strict_arguments() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ssh");
    let probe = OpenSshProbe::with_programs(
        fixtures.join("mock-ssh.sh").display().to_string(),
        fixtures.join("mock-keyscan.sh").display().to_string(),
    );
    let node = NodeProbeInput {
        id: "node_1".to_owned(),
        host: "node.example.test".to_owned(),
        port: 22,
        username: "deploy".to_owned(),
        work_root: "/srv/apps".to_owned(),
    };

    let scanned = probe.scan_host_key(&node).await.unwrap();
    assert!(scanned.fingerprint.starts_with("SHA256:"));
    let report = probe
        .check(&node, b"fixture-private-key", &scanned.host_key)
        .await
        .unwrap();
    assert_eq!(report.os_name, "Linux");
    assert_eq!(report.disk_available_bytes, 1_048_576);
}
