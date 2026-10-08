use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
    time::Duration,
};

fn query(body: Value, json_output: bool, command: &[&str]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buf = [0; 1024];
        while !request.windows(4).any(|v| v == b"\r\n\r\n") {
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buf[..n]);
        }
        assert!(request.starts_with(b"GET /external/v1/"));
        let body = body.to_string();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    });
    let mut cli = Command::new(env!("CARGO_BIN_EXE_deploy-go-deployer"));
    cli.env("DEPLOY_GO_API_BASE_URL", format!("http://{addr}"))
        .env("DEPLOY_GO_API_KEY", "dgx_fixture_source_output")
        .env("NO_PROXY", "127.0.0.1");
    if json_output {
        cli.arg("--json");
    }
    let output = cli.args(command).output().unwrap();
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn show_app_exposes_configuration_in_text_and_json_and_handles_old_api() {
    let mut body = json!({"id":"app_fixture","name":"Fixture","targets":[],"sources":{
        "git":{"deployment_branch":"test","status":"verified","source_version":2,"build_agent_id":"agent_fixture","build_node_id":"node_fixture","build_node_status":"online","source_materialization":{"mode":"full","paths":[]}},
        "workspace":null
    }});
    let text = query(body.clone(), false, &["show-app", "app_fixture"]);
    assert!(text.contains("Git 配置分支：test"));
    assert!(text.contains("固定工作区 来源：未配置"));
    assert!(text.contains("来源版本=2"));
    let output: Value =
        serde_json::from_str(&query(body.clone(), true, &["show-app", "app_fixture"])).unwrap();
    assert_eq!(output, body);
    body.as_object_mut().unwrap().remove("sources");
    assert!(query(body, false, &["show-app", "app_fixture"]).contains("控制面尚未提供"));
}

#[test]
fn deployment_output_distinguishes_version_branch_and_commit() {
    let body = json!({"id":"deployment_fixture","target_runs":[],"release_version":"release-label","source_policy":"branch","deployment_branch":"test","resolved_commit_sha":"6403e1457aa6c982b9dcd14cf4ab86609244191b"});
    let text = query(body, false, &["status", "deployment_fixture"]);
    assert!(text.contains("发布版本：release-label"));
    assert!(text.contains("固定分支：test"));
    assert!(text.contains("Git 提交：6403e1457aa6c982b9dcd14cf4ab86609244191b"));
}
