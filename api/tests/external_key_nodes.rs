//! 外部 API Key 节点白名单：管理员为某个 Key 配置了可用节点后，写 Git/工作区来源与部署目标时
//! 只能使用这些节点；**没有配置的 Key 不受限制，保持对外 API 的既有行为**。
//! 读取、已有配置的部署与停用目标不受影响。
mod common;

use axum::{Router, http::StatusCode};
use common::{admin_session, json_request, response_json, test_app};
use serde_json::json;
use sqlx::SqlitePool;

const NODE_FORBIDDEN: &str = "external_node_not_allowed";

async fn seed_node(pool: &SqlitePool, node_id: &str, agent_id: &str) {
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?,'/srv/apps','/srv/secrets','online')",
    )
    .bind(node_id)
    .bind(format!("节点 {node_id}"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json) VALUES(?,?,'test','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')",
    )
    .bind(agent_id)
    .bind(node_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_application(pool: &SqlitePool, id: &str, environment: &str) {
    sqlx::query(
        "INSERT INTO applications(id,name,slug,description,status,environment) VALUES(?,?,?,'','active',?)",
    )
    .bind(id)
    .bind(id)
    .bind(id.replace('_', "-"))
    .bind(environment)
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
}

/// 非生产应用 `app_test`（已有位于 node_b 的目标 `target_b`）、生产应用 `app_prod`；
/// node_a 与 node_b 各有一个在线 Agent。Key 绑定两个应用，初始没有任何节点授权。
async fn fixture() -> Fixture {
    let (app, pool) = test_app().await;
    seed_node(&pool, "node_a", "agent_a").await;
    seed_node(&pool, "node_b", "agent_b").await;
    seed_application(&pool, "app_test", "test").await;
    seed_application(&pool, "app_prod", "prod").await;
    sqlx::query(
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_b','app_test','node_b','test','/srv/deploy.sh',60,'active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"节点白名单 Key","application_ids":["app_test","app_prod"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    Fixture {
        app,
        pool,
        cookie,
        csrf,
        key_id: created["id"].as_str().unwrap().to_owned(),
        auth: format!("Bearer {}", created["token"].as_str().unwrap()),
    }
}

impl Fixture {
    async fn grant(&self, node_ids: &[&str]) {
        let response = json_request(
            self.app.clone(),
            "PUT",
            &format!("/api/v1/external-api-keys/{}/nodes", self.key_id),
            json!({"node_ids": node_ids}),
            &[("cookie", &self.cookie), ("x-csrf-token", &self.csrf)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    async fn call(
        &self,
        method: &str,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let response = json_request(
            self.app.clone(),
            method,
            path,
            body,
            &[
                ("authorization", &self.auth),
                ("idempotency-key", "external-key-nodes-0001"),
            ],
        )
        .await;
        let status = response.status();
        let body = if status == StatusCode::NO_CONTENT {
            json!(null)
        } else {
            response_json(response).await
        };
        (status, body)
    }
}

fn target_body(node_id: &str) -> serde_json::Value {
    json!({"node_id": node_id, "script_path": "/srv/apps/deploy.sh", "timeout_seconds": 60})
}

#[tokio::test]
async fn unconfigured_key_keeps_the_existing_unrestricted_behavior() {
    let f = fixture().await;
    // 没有为 Key 配置任何节点：与加入白名单之前完全一致，任意节点都可以写。
    // 同一应用同一节点只能有一个目标，所以两个节点分别落在两个应用上。
    sqlx::query("INSERT INTO applications(id,name,slug,description,status,environment) VALUES('app_test2','app_test2','app-test2','','active','test')")
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO external_api_key_applications(api_key_id,application_id,granted_by) SELECT id,'app_test2',created_by FROM external_api_keys")
        .execute(&f.pool)
        .await
        .unwrap();
    for (application, node) in [("app_test", "node_a"), ("app_test2", "node_b")] {
        let (status, body) = f
            .call(
                "POST",
                &format!("/external/v1/applications/{application}/targets"),
                target_body(node),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{application}/{node}: {body}");
    }
    let (status, body) = f
        .call(
            "PUT",
            "/external/v1/applications/app_test/workspace-source",
            json!({"build_agent_id":"agent_b","workspace_path":"/srv/workspace"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn granted_node_allows_writes_and_other_nodes_stay_forbidden() {
    let f = fixture().await;
    f.grant(&["node_a"]).await;

    let (status, created) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/targets",
            target_body("node_a"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let target_id = created["id"].as_str().unwrap().to_owned();
    let version = created["version"].as_i64().unwrap();

    let (status, body) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/targets",
            target_body("node_b"),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(NODE_FORBIDDEN))
    );

    // 编辑：改到未授权节点被拒，保持授权节点可以。
    let mut moved = target_body("node_b");
    moved["version"] = json!(version);
    let (status, body) = f
        .call(
            "PATCH",
            &format!("/external/v1/deployment-targets/{target_id}"),
            moved,
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(NODE_FORBIDDEN))
    );
    let node: String = sqlx::query_scalar("SELECT node_id FROM deployment_targets WHERE id=?")
        .bind(&target_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(node, "node_a");

    let mut kept = target_body("node_a");
    kept["version"] = json!(version);
    let (status, _) = f
        .call(
            "PATCH",
            &format!("/external/v1/deployment-targets/{target_id}"),
            kept,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 工作区来源：授权节点的 Agent 可以，未授权节点的 Agent 被拒，不存在的 Agent 为 404。
    let (status, _) = f
        .call(
            "PUT",
            "/external/v1/applications/app_test/workspace-source",
            json!({"build_agent_id":"agent_a","workspace_path":"/srv/workspace"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, body) = f
        .call(
            "PUT",
            "/external/v1/applications/app_test/workspace-source",
            json!({"build_agent_id":"agent_b","workspace_path":"/srv/workspace","version":1}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(NODE_FORBIDDEN))
    );
    let (status, _) = f
        .call(
            "PUT",
            "/external/v1/applications/app_test/workspace-source",
            json!({"build_agent_id":"agent_missing","workspace_path":"/srv/workspace","version":1}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn existing_configuration_on_unlisted_nodes_still_reads_toggles_and_deploys() {
    let f = fixture().await;
    f.grant(&["node_a"]).await;
    // Key 没有任何节点授权，而 target_b 已位于 node_b。
    let (status, shown) = f
        .call("GET", "/external/v1/applications/app_test", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(shown["targets"][0]["id"], json!("target_b"));

    let (status, listed) = f
        .call(
            "GET",
            "/external/v1/applications/app_test/targets",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let version = listed["items"][0]["version"].as_i64().unwrap();

    let (status, body) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/deployments",
            json!({"parameters":{}}),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "白名单上线不改变已有目标的部署：{body}"
    );

    // 状态切换不新增或更换节点，不受白名单限制，保证随时可以停用。
    let (status, _) = f
        .call(
            "PUT",
            "/external/v1/deployment-targets/target_b/status",
            json!({"status":"disabled","version":version}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn archived_node_is_not_usable_even_if_previously_granted() {
    let f = fixture().await;
    f.grant(&["node_a"]).await;
    sqlx::query("UPDATE nodes SET archived_at=datetime('now') WHERE id='node_a'")
        .execute(&f.pool)
        .await
        .unwrap();
    let (status, body) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/targets",
            target_body("node_a"),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(NODE_FORBIDDEN))
    );
}

#[tokio::test]
async fn production_guard_runs_before_node_allowlist() {
    let f = fixture().await;
    // 没有任何授权，生产应用仍返回既有的生产禁止错误码，而不是节点错误码。
    for (method, path, body) in [
        (
            "POST",
            "/external/v1/applications/app_prod/targets",
            target_body("node_a"),
        ),
        (
            "PUT",
            "/external/v1/applications/app_prod/workspace-source",
            json!({"build_agent_id":"agent_a","workspace_path":"/srv/workspace"}),
        ),
    ] {
        let (status, body) = f.call(method, path, body).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(
            body["code"],
            json!("external_production_application_forbidden")
        );
    }
}

#[tokio::test]
async fn reenabling_a_disabled_target_on_an_unlisted_node_requires_node_grant() {
    let f = fixture().await;
    f.grant(&["node_a"]).await;
    // target_b 位于 node_b，Key 没有 node_b 授权：停用随时允许，重新启用等同于重新使用该节点。
    let version: i64 =
        sqlx::query_scalar("SELECT version FROM deployment_targets WHERE id='target_b'")
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let (status, disabled) = f
        .call(
            "PUT",
            "/external/v1/deployment-targets/target_b/status",
            json!({"status":"disabled","version":version}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{disabled}");
    let (status, body) = f
        .call(
            "PUT",
            "/external/v1/deployment-targets/target_b/status",
            json!({"status":"active","version":version + 1}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(NODE_FORBIDDEN)),
        "{body}"
    );
    let state: String =
        sqlx::query_scalar("SELECT status FROM deployment_targets WHERE id='target_b'")
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(state, "disabled");

    // 授权 node_b 之后可以重新启用。
    f.grant(&["node_b"]).await;
    let (status, _) = f
        .call(
            "PUT",
            "/external/v1/deployment-targets/target_b/status",
            json!({"status":"active","version":version + 1}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}
