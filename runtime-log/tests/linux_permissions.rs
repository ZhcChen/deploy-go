#![cfg(target_os = "linux")]

use deploy_go_runtime_log::{
    LogPolicy, LogRecord, NODE_LOG_ROOT, SegmentStore, prepare_node_layout,
};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        process::CommandExt,
    },
    process::Command,
};

#[test]
#[ignore = "仅在显式隔离 Linux 容器中验证真实身份"]
fn root_components_are_readable_only_by_agent_and_not_business_runner() {
    assert!(std::path::Path::new("/.dockerenv").exists());
    assert_eq!(
        std::env::var("DEPLOY_GO_LOG_PERMISSION_FIXTURE").as_deref(),
        Ok("1")
    );
    assert_eq!(unsafe { libc::geteuid() }, 0);
    // 模拟 Ubuntu 的 root:syslog（fixture GID 1003）组可写目录；只操作隔离容器。
    assert_eq!(unsafe { libc::chown(c"/var/log".as_ptr(), 0, 1003) }, 0);
    fs::set_permissions("/var/log", fs::Permissions::from_mode(0o775)).unwrap();
    prepare_node_layout().unwrap();
    prepare_node_layout().unwrap();
    assert_eq!(
        fs::metadata("/var/log").unwrap().permissions().mode() & 0o777,
        0o775
    );
    let log_parent = fs::metadata("/var/log").unwrap();
    assert_eq!((log_parent.uid(), log_parent.gid()), (0, 1003));
    let root = std::path::Path::new(NODE_LOG_ROOT);
    for component in ["runner", "executor", "updater"] {
        let mut store = SegmentStore::open(root.join(component), LogPolicy::NODE).unwrap();
        store
            .append(LogRecord {
                sequence: 0,
                timestamp: "2026-10-08T00:00:00Z".into(),
                level: "INFO".into(),
                target: "fixture".into(),
                message: "fixture".into(),
                request_id: None,
                fields: BTreeMap::new(),
            })
            .unwrap();
    }
    let status = Command::new("sh").args(["-c", "cat /var/lib/deploy-go-agent-runtime-logs/runner/*.jsonl >/dev/null && ! touch /var/lib/deploy-go-agent-runtime-logs/runner/forbidden && touch /var/lib/deploy-go-agent-runtime-logs/agent/owned"])
        .uid(1001).gid(1001).status().unwrap();
    assert!(status.success());
    let status = Command::new("sh").args(["-c", "! cat /var/lib/deploy-go-agent-runtime-logs/runner/*.jsonl && ! cat /var/lib/deploy-go-agent-runtime-logs/agent/owned"])
        .uid(1002).gid(1002).status().unwrap();
    assert!(status.success());
    assert!(
        fs::read_dir(root.join("agent"))
            .unwrap()
            .any(|entry| entry.unwrap().file_name() == "owned")
    );
    fs::remove_file(root.join("agent/owned")).unwrap();
}
