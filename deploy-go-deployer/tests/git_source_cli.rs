//! Git 来源相关子命令：用本地脚本化服务验证实际发出的请求（方法、路径、请求体）与文本输出。
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Output},
    thread::{self, JoinHandle},
    time::Duration,
};

struct Recorded {
    method: String,
    path: String,
    body: Value,
}

/// 按顺序应答 `script` 中的响应，并记录每个收到的请求。
fn serve(script: Vec<(u16, Value)>) -> (String, JoinHandle<Vec<Recorded>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let mut recorded = Vec::new();
        listener.set_nonblocking(true).unwrap();
        for (status, response) in script {
            // 期望的请求迟迟不来（例如命令回退为少发请求）时在超时后结束，让断言失败而不是挂起。
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() >= deadline {
                            return recorded;
                        }
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(error) => panic!("mock 服务 accept 失败：{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 2048];
            let header_end = loop {
                if let Some(position) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    break position + 4;
                }
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buf[..n]);
            };
            let head = String::from_utf8_lossy(&request[..header_end]).to_string();
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            while request.len() < header_end + length {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buf[..n]);
            }
            let mut first = head.lines().next().unwrap().split_whitespace();
            let method = first.next().unwrap().to_owned();
            let path = first.next().unwrap().to_owned();
            let body = serde_json::from_slice(&request[header_end..]).unwrap_or(Value::Null);
            recorded.push(Recorded { method, path, body });
            let text = response.to_string();
            write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            )
            .unwrap();
        }
        recorded
    });
    (base, handle)
}

fn run(base: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deploy-go-deployer"))
        .env("DEPLOY_GO_API_BASE_URL", base)
        .env("DEPLOY_GO_API_KEY", "dgx_fixture_git_source_cli")
        .env("NO_PROXY", "127.0.0.1")
        .args(args)
        .output()
        .unwrap()
}

fn source(version: i64) -> Value {
    json!({
        "id": "source_1", "application_id": "app_1",
        "repository_url": "git@git.example.test:team/app.git",
        "git_credential_id": "gitcred_1", "git_credential_name": "部署密钥",
        "build_agent_id": "agent_1", "build_agent_name": "构建节点",
        "source_policy": "branch",
        "source_materialization": {"mode": "sparse", "paths": ["Makefile", "scripts/**"]},
        "deployment_branch": null, "branch_verified_at": null, "status": "draft",
        "created_at": "2026-10-22T00:00:00Z", "updated_at": "2026-10-22T00:00:00Z",
        "version": version
    })
}

#[test]
fn set_git_source_reads_current_version_and_sends_only_declared_fields() {
    let (base, server) = serve(vec![(200, source(4)), (200, source(5))]);
    let output = run(
        &base,
        &[
            "set-git-source",
            "app_1",
            "--repository-url",
            "git@git.example.test:team/app.git",
            "--git-credential-id",
            "gitcred_1",
            "--build-agent-id",
            "agent_1",
            "--materialization",
            "sparse",
            "--sparse-path",
            "scripts/**",
            "--sparse-path",
            "Makefile",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = server.join().unwrap();
    assert_eq!(
        (requests[0].method.as_str(), requests[0].path.as_str()),
        ("GET", "/external/v1/applications/app_1/git-source")
    );
    assert_eq!(
        (requests[1].method.as_str(), requests[1].path.as_str()),
        ("PUT", "/external/v1/applications/app_1/git-source")
    );
    let body = requests[1].body.as_object().unwrap();
    let mut keys: Vec<&str> = body.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "build_agent_id",
            "git_credential_id",
            "repository_url",
            "source_materialization",
            "source_policy",
            "version"
        ]
    );
    assert_eq!(body["version"], json!(4));
    assert_eq!(
        body["source_materialization"]["paths"],
        json!(["scripts/**", "Makefile"])
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("git@git.example.test:team/app.git") && text.contains("sparse"),
        "{text}"
    );
}

#[test]
fn set_git_source_rejects_inconsistent_materialization_without_any_request() {
    // 没有可连接的服务：参数校验必须在发请求之前失败。
    for args in [
        vec!["--materialization", "sparse"],
        vec!["--materialization", "full", "--sparse-path", "a"],
    ] {
        let mut command = vec![
            "set-git-source",
            "app_1",
            "--repository-url",
            "git@git.example.test:team/app.git",
            "--build-agent-id",
            "agent_1",
        ];
        command.extend(args);
        let output = run("http://127.0.0.1:9", &command);
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("sparse"), "{stderr}");
    }
}

#[test]
fn refresh_git_branches_wait_polls_until_succeeded_and_lists_branches() {
    let queued = json!({
        "id": "refs_1", "application_source_id": "source_1", "source_version": 2,
        "task_id": "task_1", "status": "queued", "refs": [], "error_code": null,
        "expires_at": null, "created_at": "2026-10-22T00:00:00Z", "finished_at": null
    });
    let mut done = queued.clone();
    done["status"] = json!("succeeded");
    done["refs"] = json!([{"name": "main", "ref": "refs/heads/main", "sha": "0123456789abcdef0123456789abcdef01234567"}]);
    let (base, server) = serve(vec![(202, queued), (200, done)]);
    let output = run(&base, &["refresh-git-branches", "app_1", "--wait"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = server.join().unwrap();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(
        requests[0].path,
        "/external/v1/applications/app_1/git-source/refreshes"
    );
    assert_eq!(
        requests[1].path,
        "/external/v1/applications/app_1/git-source/refreshes/refs_1"
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("succeeded") && text.contains("main"),
        "{text}"
    );
}

#[test]
fn refresh_git_branches_wait_fails_when_discovery_fails() {
    let failed = json!({
        "id": "refs_1", "application_source_id": "source_1", "source_version": 2,
        "task_id": "task_1", "status": "failed", "refs": [], "error_code": "git_authentication_failed",
        "expires_at": null, "created_at": "2026-10-22T00:00:00Z", "finished_at": null
    });
    let (base, server) = serve(vec![(202, failed)]);
    let output = run(&base, &["refresh-git-branches", "app_1", "--wait"]);
    server.join().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("git_authentication_failed"));
}

#[test]
fn set_git_branch_reads_version_when_omitted() {
    let mut fixed = source(5);
    fixed["status"] = json!("verified");
    fixed["deployment_branch"] = json!("main");
    let (base, server) = serve(vec![(200, source(4)), (200, fixed)]);
    let output = run(&base, &["set-git-branch", "app_1", "--branch", "main"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2, "应先读取现有版本再设置分支");
    assert_eq!(requests[1].method, "PUT");
    assert_eq!(
        requests[1].path,
        "/external/v1/applications/app_1/git-source/branch"
    );
    assert_eq!(requests[1].body, json!({"branch": "main", "version": 4}));
    assert!(String::from_utf8(output.stdout).unwrap().contains("main"));
}

#[test]
fn directory_commands_render_agents_and_credentials_without_extra_fields() {
    let agents = json!({"items": [
        {"agent_id": "agent_1", "node_id": "node_1", "node_name": "构建节点", "node_status": "online", "ready": true, "unavailable_code": null},
        {"agent_id": "agent_2", "node_id": "node_2", "node_name": "离线节点", "node_status": "offline", "ready": false, "unavailable_code": "agent_offline"}
    ]});
    let (base, server) = serve(vec![(200, agents)]);
    let output = run(&base, &["list-build-agents"]);
    assert_eq!(server.join().unwrap()[0].path, "/external/v1/build-agents");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("agent_1") && text.contains("agent_offline"),
        "{text}"
    );

    let credentials = json!({"items": [
        {"id": "gitcred_1", "name": "部署密钥", "fingerprint": "SHA256:abc", "status": "active"}
    ]});
    let (base, server) = serve(vec![(200, credentials)]);
    let output = run(&base, &["list-git-credentials"]);
    assert_eq!(
        server.join().unwrap()[0].path,
        "/external/v1/git-credentials"
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("gitcred_1") && text.contains("SHA256:abc"),
        "{text}"
    );
}

#[test]
fn set_app_status_reads_application_version_when_omitted() {
    let (base, server) = serve(vec![
        (
            200,
            json!({"id": "app_1", "name": "应用", "version": 7, "targets": []}),
        ),
        (
            200,
            json!({"id": "app_1", "status": "archived", "version": 8}),
        ),
    ]);
    let output = run(&base, &["set-app-status", "app_1", "--status", "archived"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2, "应先读取应用版本再切换状态");
    assert_eq!(requests[0].path, "/external/v1/applications/app_1");
    assert_eq!(
        (requests[1].method.as_str(), requests[1].path.as_str()),
        ("PUT", "/external/v1/applications/app_1/status")
    );
    assert_eq!(
        requests[1].body,
        json!({"status": "archived", "version": 7})
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("archived") && text.contains('8'), "{text}");
}

#[test]
fn set_app_status_with_explicit_version_sends_a_single_request() {
    let (base, server) = serve(vec![(
        200,
        json!({"id": "app_1", "status": "active", "version": 10}),
    )]);
    let output = run(
        &base,
        &[
            "set-app-status",
            "app_1",
            "--status",
            "active",
            "--version",
            "9",
        ],
    );
    assert!(output.status.success());
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].body, json!({"status": "active", "version": 9}));
}

#[test]
fn set_app_status_rejects_unknown_status_before_any_request() {
    let output = run(
        "http://127.0.0.1:9",
        &["set-app-status", "app_1", "--status", "deleted"],
    );
    assert!(!output.status.success());
}

#[test]
fn retry_env_sync_encodes_target_and_reports_count() {
    let (base, server) = serve(vec![(200, json!({"retried": 2}))]);
    let output = run(
        &base,
        &["retry-env-sync", "app_1", "envf_1", "--target-id", "t a&b"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = server.join().unwrap();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(
        requests[0].path,
        "/external/v1/applications/app_1/env-files/envf_1/sync-retry?target_id=t%20a%26b"
    );
    assert!(String::from_utf8(output.stdout).unwrap().contains('2'));
}

#[test]
fn list_config_files_renders_metadata_table() {
    let listing = json!({"items": [{
        "id": "cfg_1", "path": "compose.yml", "label": "编排", "format": "yaml", "role": "configuration",
        "delivery": "artifact", "sensitive": false, "editable": true, "current_version": 3,
        "current_digest": "sha256:abc", "status": "ready", "updated_at": "2026-10-22T00:00:00Z", "version": 1
    }]});
    let (base, server) = serve(vec![(200, listing)]);
    let output = run(&base, &["list-config-files", "app_1"]);
    assert_eq!(
        server.join().unwrap()[0].path,
        "/external/v1/applications/app_1/config-files"
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("compose.yml") && text.contains("artifact"),
        "{text}"
    );
}
