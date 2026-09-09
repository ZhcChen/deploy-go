use std::time::Duration;

use reqwest::{Client, redirect};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

pub fn new_agent_client(timeout: Duration) -> Client {
    Client::builder()
        .redirect(redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(timeout)
        .build()
        .expect("固定 Agent HTTPS client 配置有效")
}

pub fn new_runtime_probe_client() -> Client {
    Client::builder()
        .redirect(redirect::Policy::none())
        .no_proxy()
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .expect("固定 runtime probe client 配置有效")
}
