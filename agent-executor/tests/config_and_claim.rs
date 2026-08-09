use deploy_go_agent_executor::{config::ExecutorConfig, session_claim::SessionRegistry};
use std::{path::PathBuf, sync::Arc};

#[test]
fn shell_must_be_absolute_existing_regular_and_executable() {
    let home = tempfile::tempdir().unwrap();
    let mut config = ExecutorConfig::system(1, 1);
    config.home = home.path().to_path_buf();
    config.shell = PathBuf::from("bin/sh");
    assert!(config.validate().is_err());
    config.shell = PathBuf::from("/bin/sh");
    assert!(config.validate().is_ok());
    config.shell = PathBuf::from("/definitely/missing/deploy-go-shell");
    assert!(config.validate().is_err());
    config.shell = PathBuf::from("/bin/sh");
    config.home = PathBuf::from("relative/root");
    assert!(config.validate().is_err());
}

#[test]
fn claim_is_released_on_every_drop_path() {
    let registry = Arc::new(SessionRegistry::default());
    let first = registry.claim("session-1").unwrap();
    assert!(registry.claim("session-2").is_none());
    drop(first);
    assert_eq!(registry.active(), None);
    let second = registry.claim("session-2").unwrap();
    assert_eq!(registry.active().as_deref(), Some("session-2"));
    drop(second);
    assert_eq!(registry.active(), None);
}

#[test]
fn cleanup_failure_permanently_blocks_new_sessions() {
    let registry = Arc::new(SessionRegistry::default());
    let claim = registry.claim("session-1").unwrap();
    registry.block_after_cleanup_failure();
    drop(claim);

    assert!(registry.cleanup_failed());
    assert!(registry.claim("session-2").is_none());
}
