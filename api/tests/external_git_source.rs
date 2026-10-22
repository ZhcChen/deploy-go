//! 对外 Git 来源：目录、保存、分支发现与设置部署分支。
//! 写入必须经过 Key 绑定、非生产守卫与节点白名单；响应、日志、审计不含凭证内容。
mod common;

use axum::{Router, http::StatusCode};
use common::{admin_session, complete_pending_refs_query, json_request, response_json, test_app};
use deploy_go_api::AppState;
use serde_json::{Value, json};
use sqlx::SqlitePool;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const REPO: &str = "git@git.example.test:deploy-go/example.git";

async fn seed_node(pool: &SqlitePool, node_id: &str, agent_id: &str, status: &str) {
    sqlx::query(
        "INSERT INTO nodes(id,name,host,port,username,work_root,secrets_root,status) VALUES(?,?,'10.9.8.7',22,'deploy-user','/srv/private-root','/srv/secrets',?)",
    )
    .bind(node_id)
    .bind(format!("节点 {node_id}"))
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json,connection_generation) VALUES(?,?,'test','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]',0)",
    )
    .bind(agent_id)
    .bind(node_id)
    .execute(pool)
    .await
    .unwrap();
}

struct Fixture {
    app: Router,
    pool: SqlitePool,
    cookie: String,
    csrf: String,
    key_id: String,
    auth: String,
    credential_id: String,
}

/// 非生产应用 `app_test`、生产应用 `app_prod`、另一个未绑定应用 `app_other`；
/// node_a / node_b 在线，node_c 离线；Key 绑定 app_test、app_second、app_prod。
async fn fixture() -> Fixture {
    let (app, pool) = test_app().await;
    seed_node(&pool, "node_a", "agent_a", "online").await;
    seed_node(&pool, "node_b", "agent_b", "online").await;
    seed_node(&pool, "node_c", "agent_c", "offline").await;
    for (id, environment) in [
        ("app_test", "test"),
        ("app_second", "test"),
        ("app_prod", "prod"),
        ("app_other", "test"),
    ] {
        sqlx::query(
            "INSERT INTO applications(id,name,slug,description,status,environment) VALUES(?,?,?,'','active',?)",
        )
        .bind(id)
        .bind(id)
        .bind(id.replace('_', "-"))
        .bind(environment)
        .execute(&pool)
        .await
        .unwrap();
    }
    let (cookie, csrf) = admin_session(app.clone()).await;
    let credential = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/git-credentials",
            json!({"name":"外部部署密钥"}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let created = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/external-api-keys",
            json!({"name":"Git 来源 Key","application_ids":["app_test","app_second","app_prod"]}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    let key_id = created["id"].as_str().unwrap().to_owned();
    Fixture {
        app,
        pool,
        cookie,
        csrf,
        key_id,
        auth: format!("Bearer {}", created["token"].as_str().unwrap()),
        credential_id: credential["id"].as_str().unwrap().to_owned(),
    }
}

impl Fixture {
    async fn call(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        let response = json_request(
            self.app.clone(),
            method,
            path,
            body,
            &[("authorization", &self.auth)],
        )
        .await;
        let status = response.status();
        (status, response_json(response).await)
    }

    async fn count_sources(&self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM application_sources")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    fn save_body(&self, agent: &str) -> Value {
        json!({
            "repository_url": REPO,
            "git_credential_id": self.credential_id,
            "build_agent_id": agent,
            "source_policy": "branch"
        })
    }

    async fn save_source(&self, application_id: &str) -> Value {
        let (status, body) = self
            .call(
                "PUT",
                &format!("/external/v1/applications/{application_id}/git-source"),
                self.save_body("agent_a"),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body
    }
}

fn assert_no_secrets(value: &Value) {
    let text = value.to_string();
    for forbidden in [
        "PRIVATE KEY",
        "encrypted_private_key",
        "public_key",
        "10.9.8.7",
        "deploy-user",
        "/srv/private-root",
        "lease_",
    ] {
        assert!(
            !text.contains(forbidden),
            "响应不应包含 {forbidden}: {text}"
        );
    }
}

#[tokio::test]
async fn directories_list_agents_and_credential_metadata_only() {
    let f = fixture().await;

    let (status, agents) = f.call("GET", "/external/v1/build-agents", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let items = agents["items"].as_array().unwrap();
    let ids: Vec<&str> = items
        .iter()
        .map(|item| item["agent_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["agent_a", "agent_b", "agent_c"]);
    assert_eq!(items[0]["ready"], json!(true));
    assert_eq!(items[0]["node_id"], json!("node_a"));
    assert_eq!(items[2]["ready"], json!(false));
    assert_eq!(items[2]["unavailable_code"], json!("agent_offline"));
    assert_no_secrets(&agents);

    let (status, credentials) = f
        .call("GET", "/external/v1/git-credentials", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    let item = &credentials["items"][0];
    assert_eq!(item["id"], json!(f.credential_id));
    assert_eq!(item["status"], json!("active"));
    let mut keys: Vec<&str> = item
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(keys, vec!["fingerprint", "id", "name", "status"]);
    assert_no_secrets(&credentials);
}

#[tokio::test]
async fn saving_git_source_checks_guards_and_never_leaks_secrets() {
    let f = fixture().await;
    let path = "/external/v1/applications/app_test/git-source";

    // 未配置来源时读取为 404。
    let (status, _) = f.call("GET", path, json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 离线的节点：与管理面一致返回 409 agent_offline。
    let (status, body) = f.call("PUT", path, f.save_body("agent_c")).await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::CONFLICT, &json!("agent_offline"))
    );
    // 不存在的 Agent：404。
    let (status, _) = f.call("PUT", path, f.save_body("agent_missing")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // 生产应用与未绑定应用。
    let (status, body) = f
        .call(
            "PUT",
            "/external/v1/applications/app_prod/git-source",
            f.save_body("agent_a"),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (
            StatusCode::FORBIDDEN,
            &json!("external_production_application_forbidden")
        )
    );
    let (status, _) = f
        .call(
            "PUT",
            "/external/v1/applications/app_other/git-source",
            f.save_body("agent_a"),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // 带凭证的仓库地址被拒绝。
    let mut with_userinfo = f.save_body("agent_a");
    with_userinfo["repository_url"] = json!("https://user:pass@git.example.test/x.git");
    let (status, _) = f.call("PUT", path, with_userinfo).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    // 未知字段（例如试图塞入 lease 或私钥）被拒绝。
    let mut unknown = f.save_body("agent_a");
    unknown["private_key"] = json!("secret");
    let (status, _) = f.call("PUT", path, unknown).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let sources: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM application_sources")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(sources, 0, "所有被拒绝的请求都不得写入来源");

    // 成功保存（含 sparse 物化），响应不含任何敏感或节点连接信息。
    let mut sparse = f.save_body("agent_a");
    sparse["source_materialization"] = json!({"mode":"sparse","paths":["scripts/**","Makefile"]});
    let (status, saved) = f.call("PUT", path, sparse).await;
    assert_eq!(status, StatusCode::CREATED, "{saved}");
    assert_eq!(saved["status"], json!("draft"));
    assert_eq!(
        saved["source_materialization"]["paths"],
        json!(["Makefile", "scripts/**"])
    );
    assert_no_secrets(&saved);

    // 再次保存必须带 version；错误 version 为 409。
    let (status, _) = f.call("PUT", path, f.save_body("agent_a")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let mut stale = f.save_body("agent_a");
    stale["version"] = json!(99);
    let (status, body) = f.call("PUT", path, stale).await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::CONFLICT, &json!("resource_version_conflict"))
    );
    let mut update = f.save_body("agent_a");
    update["version"] = saved["version"].clone();
    let (status, _) = f.call("PUT", path, update).await;
    assert_eq!(status, StatusCode::OK);

    // 审计带 Key 标识，且不含凭证内容。
    let audits: Vec<String> = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE action LIKE 'application_source.%'",
    )
    .fetch_all(&f.pool)
    .await
    .unwrap();
    assert_eq!(audits.len(), 2);
    for summary in &audits {
        assert!(summary.contains(&f.key_id), "审计应记录外部 Key: {summary}");
        assert!(!summary.contains("PRIVATE KEY"));
    }

    // 凭证停用后保存被拒绝。
    let archived = json_request(
        f.app.clone(),
        "PUT",
        &format!("/api/v1/git-credentials/{}/status", f.credential_id),
        json!({"status":"archived","version":1}),
        &[("cookie", &f.cookie), ("x-csrf-token", &f.csrf)],
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    let mut revoked = f.save_body("agent_a");
    revoked["version"] = json!(3);
    let (status, _) = f.call("PUT", path, revoked).await;
    assert_ne!(status, StatusCode::OK, "已停用的凭证不得再被引用");
}

#[tokio::test]
async fn discovery_and_branch_flow_runs_end_to_end_with_simulated_agent() {
    let f = fixture().await;
    let saved = f.save_source("app_test").await;
    let version = saved["version"].as_i64().unwrap();
    let base = "/external/v1/applications/app_test/git-source";

    // 发现完成前设置分支：没有可用的发现结果。
    let (status, body) = f
        .call(
            "PUT",
            &format!("{base}/branch"),
            json!({"branch":"main","version":version}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::CONFLICT, &json!("git_ref_discovery_missing"))
    );

    let (status, discovery) = f
        .call("POST", &format!("{base}/refreshes"), json!({}))
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{discovery}");
    let discovery_id = discovery["id"].as_str().unwrap().to_owned();
    let (_, again) = f
        .call("POST", &format!("{base}/refreshes"), json!({}))
        .await;
    assert_eq!(
        again["id"],
        json!(discovery_id),
        "同一来源版本进行中的发现不得重复下发"
    );
    let tasks: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks WHERE kind='git_refs_query'")
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(tasks, 1);

    // 重复发起检查完成后再启动模拟 Agent：否则第一条发现可能先被应答完成，第二次发起就会合法地新建一条，用例随时序偶发失败。
    let completer = complete_pending_refs_query(
        AppState::new(f.pool.clone()),
        "agent_a",
        0,
        json!([
            {"name":"main","ref":"refs/heads/main","sha":SHA},
            {"name":"develop","ref":"refs/heads/develop","sha":SHA}
        ]),
    )
    .await;

    let mut finished = json!(null);
    for _ in 0..50 {
        let (status, shown) = f
            .call(
                "GET",
                &format!("{base}/refreshes/{discovery_id}"),
                json!({}),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        if shown["status"] == json!("succeeded") {
            finished = shown;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    completer.abort();
    assert_eq!(
        finished["refs"].as_array().map(Vec::len),
        Some(2),
        "{finished}"
    );
    assert_no_secrets(&finished);

    // 同一个 Key 绑定的另一个非生产应用，也不能借自己的路径读取这条发现记录。
    let (status, _) = f
        .call(
            "GET",
            &format!("/external/v1/applications/app_second/git-source/refreshes/{discovery_id}"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // 生产应用的来源详情与发现记录读取同样被拒绝（specs/005 FR-013）。
    for path in [
        "/external/v1/applications/app_prod/git-source".to_owned(),
        format!("/external/v1/applications/app_prod/git-source/refreshes/{discovery_id}"),
    ] {
        let (status, body) = f.call("GET", &path, json!({})).await;
        assert_eq!(
            (status, &body["code"]),
            (
                StatusCode::FORBIDDEN,
                &json!("external_production_application_forbidden")
            ),
            "{path}"
        );
    }

    // 不在发现结果里的分支被拒绝；版本过期被拒绝；成功后来源已验证。
    let (status, body) = f
        .call(
            "PUT",
            &format!("{base}/branch"),
            json!({"branch":"nope","version":version}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::CONFLICT, &json!("git_branch_not_found"))
    );
    let (status, body) = f
        .call(
            "PUT",
            &format!("{base}/branch"),
            json!({"branch":"main","version":version + 5}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::CONFLICT, &json!("resource_version_conflict"))
    );
    let (status, fixed) = f
        .call(
            "PUT",
            &format!("{base}/branch"),
            json!({"branch":"main","version":version}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{fixed}");
    assert_eq!(fixed["status"], json!("verified"));
    assert_eq!(fixed["deployment_branch"], json!("main"));

    // show-app 的来源摘要与之一致。
    let (_, shown) = f
        .call("GET", "/external/v1/applications/app_test", json!({}))
        .await;
    assert_eq!(shown["sources"]["git"]["deployment_branch"], json!("main"));
    assert_eq!(shown["sources"]["git"]["status"], json!("verified"));
}

#[tokio::test]
async fn refresh_and_branch_require_binding_and_non_production() {
    let f = fixture().await;
    f.save_source("app_test").await;
    let base = "/external/v1/applications/app_test/git-source";

    // 已配置来源的读取正常。
    let (status, _) = f.call("GET", base, json!({})).await;
    assert_eq!(status, StatusCode::OK);

    // 生产应用写入类请求返回既有生产禁止码；未绑定应用为 404。
    for (method, path, body) in [
        (
            "POST",
            "/external/v1/applications/app_prod/git-source/refreshes",
            json!({}),
        ),
        (
            "PUT",
            "/external/v1/applications/app_prod/git-source/branch",
            json!({"branch":"main","version":1}),
        ),
    ] {
        let (status, body) = f.call(method, path, body).await;
        assert_eq!(
            (status, &body["code"]),
            (
                StatusCode::FORBIDDEN,
                &json!("external_production_application_forbidden")
            ),
            "{method} {path}"
        );
    }
    let (status, _) = f
        .call(
            "POST",
            "/external/v1/applications/app_other/git-source/refreshes",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn external_repository_url_policy_does_not_affect_internal_saves_or_stored_sources() {
    let f = fixture().await;
    let ip_repo = "git@10.0.0.5:team/app.git";

    // 对外保存：内网 IP 字面量被拒绝，且没有写入来源。
    let path = "/external/v1/applications/app_test/git-source";
    for url in [
        ip_repo,
        "https://token@git.example.test/team/app.git",
        "https://169.254.169.254/latest/meta-data",
        "http://git.example.test/team/app.git",
        "ext::sh -c id",
        "git@localhost:team/app.git",
    ] {
        let mut body = f.save_body("agent_a");
        body["repository_url"] = json!(url);
        let (status, response) = f.call("PUT", path, body).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{url}: {response}"
        );
        assert!(
            !response.to_string().contains("token@"),
            "错误提示不得回显提交的地址: {response}"
        );
    }
    assert_eq!(f.count_sources().await, 0, "被拒绝的地址不得写入来源");

    // 管理面保存同一个地址不受影响（管理员在管理面维护内网 Git）。
    let internal = json_request(
        f.app.clone(),
        "PUT",
        "/api/v1/applications/app_test/source",
        json!({"repository_url": ip_repo, "build_agent_id": "agent_a", "source_policy": "branch"}),
        &[("cookie", &f.cookie), ("x-csrf-token", &f.csrf)],
    )
    .await;
    assert_eq!(internal.status(), StatusCode::CREATED);

    // 已存的内网地址来源：对外读取摘要、发起分支发现照常可用（地址只在保存时校验）。
    let (status, shown) = f
        .call("GET", "/external/v1/applications/app_test", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(shown["sources"]["git"]["status"], json!("draft"));
    let (status, discovery) = f
        .call("POST", &format!("{path}/refreshes"), json!({}))
        .await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "已存来源的发现不受新策略影响: {discovery}"
    );

    // 对外再次保存同一内网地址仍被拒绝（只影响对外这个新入口）。
    let mut resave = f.save_body("agent_a");
    resave["repository_url"] = json!(ip_repo);
    resave["version"] = json!(1);
    let (status, _) = f.call("PUT", path, resave).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn external_save_accepts_hostname_based_and_public_repositories() {
    let f = fixture().await;
    let path = "/external/v1/applications/app_test/git-source";
    for (index, url) in [
        "git@git.corp.local:team/app.git",
        "https://gitlab.internal.example.com/team/app.git",
    ]
    .into_iter()
    .enumerate()
    {
        let mut body = f.save_body("agent_a");
        body["repository_url"] = json!(url);
        if index > 0 {
            body["version"] = json!(index as i64);
        }
        let (status, response) = f.call("PUT", path, body).await;
        assert!(status.is_success(), "{url} 应允许: {response}");
    }
}
