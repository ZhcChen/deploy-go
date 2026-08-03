use std::{path::PathBuf, time::Duration};

use deploy_go_agent::config::{Config, ConfigError};

#[test]
fn explicit_wss_configuration_is_accepted() {
    let config = Config::parse(
        "wss://deploy.example.test/api/v1/agent/ws",
        PathBuf::from("/var/lib/deploy-go-agent"),
        30,
    )
    .unwrap();
    assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
    assert_eq!(
        config.credential_file,
        PathBuf::from("/var/lib/deploy-go-agent/credentials.json")
    );
}

#[test]
fn insecure_or_ambiguous_configuration_is_rejected() {
    for url in [
        "ws://deploy.example.test/ws",
        "wss://user@deploy.example.test/ws",
        "wss://deploy.example.test/ws?token=secret",
    ] {
        assert_eq!(
            Config::parse(url, PathBuf::from("/var/lib/deploy-go-agent"), 30),
            Err(ConfigError::InvalidControlUrl)
        );
    }
    assert_eq!(
        Config::parse(
            "wss://deploy.example.test/ws",
            PathBuf::from("relative"),
            30
        ),
        Err(ConfigError::InvalidDataDir)
    );
}
