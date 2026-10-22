//! 对外部署预览、运行探测与运行状态（specs/005 阶段 E）。
//! 预览/探测都会写入记录，必须经过 Key 绑定与非生产守卫；预览响应是独立的脱敏结构。
mod common;

use axum::{Router, http::StatusCode};
use common::{admin_session, complete_pending_refs_query, json_request, response_json, test_app};
use deploy_go_api::AppState;
use serde_json::{Value, json};
use sqlx::SqlitePool;

const PROD_DEPLOY_FORBIDDEN: &str = "external_production_deployment_forbidden";
const SHA: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

struct Fixture {
    app: Router,
    pool: SqlitePool,
    auth: String,
}

async fn seed_agent_node(pool: &SqlitePool, node: &str, agent: &str) {
    sqlx::query(
        "INSERT INTO nodes(id,name,host,port,username,work_root,secrets_root,status) VALUES(?,?,'10.9.8.7',22,'deploy-user','/srv/apps','/srv/secrets','online')",
    )
    .bind(node)
    .bind(format!("节点 {node}"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json,connection_generation) VALUES(?,?,'test','0.2.0',15,'[\"pty_terminal\",\"privileged_release\",\"runtime_probe_v1\"]',0)",
    )
    .bind(agent)
    .bind(node)
    .execute(pool)
    .await
    .unwrap();
}

/// 两阶段 Git 应用 `app_test`（已验证来源，分支 production）、
/// 另有生产应用 `app_prod`（含生产目标）、带生产目标的非生产应用 `app_drift`、未绑定应用 `app_other`。
async fn fixture() -> Fixture {
    let (app, pool) = test_app().await;
    seed_agent_node(&pool, "node_a", "agent_a").await;
    sqlx::query("INSERT INTO applications(id,name,slug,description,status,environment,parameter_schema,verification_config) VALUES('app_test','app_test','app-test','','active','test',?,?)")
        .bind(json!({"type":"object","properties":{"release-version":{"type":"string","maxLength":32},"modules":{"type":"string","maxLength":512}},"required":["release-version","modules"],"additionalProperties":false}).to_string())
        .bind(json!({"type":"http","port":24710,"path":"/healthz","expected_status":200,"timeout_ms":5000}).to_string())
        .execute(&pool).await.unwrap();
    for (id, environment) in [
        ("app_prod", "prod"),
        // 生产应用但还没有任何目标：单靠“应用没有生产目标”的检查拦不住，必须看应用自身环境。
        ("app_prod_no_target", "prod"),
        ("app_drift", "test"),
        ("app_other", "test"),
    ] {
        sqlx::query("INSERT INTO applications(id,name,slug,description,status,environment) VALUES(?,?,?,'','active',?)")
            .bind(id).bind(id).bind(id.replace('_', "-")).bind(environment)
            .execute(&pool).await.unwrap();
    }
    for (target, application, environment, mode) in [
        ("target_test", "app_test", "test", "two_stage"),
        ("target_prod", "app_prod", "prod", "script"),
        ("target_drift", "app_drift", "prod", "script"),
    ] {
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES(?,?,'node_a',?,?,'/srv/apps/deploy.sh',900,'active')")
            .bind(target).bind(application).bind(environment).bind(mode)
            .execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO application_sources(id,application_id,repository_url,build_agent_id,source_policy,deployment_branch,source_version,status,version) VALUES('source_test','app_test','git@git.example.test:deploy-go/example.git','agent_a','branch','production',1,'verified',1)")
        .execute(&pool).await.unwrap();
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/external-api-keys",
            json!({"name":"预览探测 Key","application_ids":["app_test","app_prod","app_prod_no_target","app_drift"]}),
            &[("cookie", &cookie), ("x-csrf-token", &csrf)],
        )
        .await,
    )
    .await;
    Fixture {
        app,
        pool,
        auth: format!("Bearer {}", created["token"].as_str().unwrap()),
    }
}

impl Fixture {
    async fn call(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        let response = json_request(
            self.app.clone(),
            method,
            path,
            body,
            &[
                ("authorization", &self.auth),
                ("idempotency-key", "external-preview-0001"),
            ],
        )
        .await;
        let status = response.status();
        (status, response_json(response).await)
    }

    /// 让发现分支的任务由模拟 Agent 应答，预览会实时解析远端 HEAD。
    async fn simulate_refs(&self) -> tokio::task::JoinHandle<()> {
        complete_pending_refs_query(
            AppState::new(self.pool.clone()),
            "agent_a",
            0,
            json!([{"name":"production","ref":"refs/heads/production","sha":SHA}]),
        )
        .await
    }

    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

fn params() -> Value {
    json!({"parameters":{"release-version":"20261022120000","modules":"api,admin"}})
}

fn assert_redacted(value: &Value) {
    let text = value.to_string();
    for forbidden in [
        "agent_a",
        "agent_id",
        "env_gate",
        "image_spec",
        "workspace_path",
        "10.9.8.7",
        "deploy-user",
        "/srv/secrets",
        "lease_id",
        "PRIVATE KEY",
    ] {
        assert!(
            !text.contains(forbidden),
            "对外预览不应包含 {forbidden}: {text}"
        );
    }
}

#[tokio::test]
async fn application_preview_then_external_deploy_with_the_returned_snapshot_hash() {
    let f = fixture().await;
    let refs = f.simulate_refs().await;
    let (status, preview) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/deployment-preview",
            params(),
        )
        .await;
    refs.abort();
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["application_id"], json!("app_test"));
    assert_eq!(preview["execution_mode"], json!("two_stage"));
    assert_eq!(preview["deployment_branch"], json!("production"));
    assert_eq!(preview["resolved_commit_sha"], json!(SHA));
    assert_eq!(preview["release_version"], json!("20261022120000"));
    assert!(preview["preview_expires_at"].is_string());
    assert_eq!(preview["targets"][0]["target_id"], json!("target_test"));
    assert_redacted(&preview);
    let hash = preview["snapshot_hash"].as_str().unwrap().to_owned();
    assert_eq!(f.count("deployment_previews").await, 1, "预览已持久化");

    // 用预览固定的 hash 确认部署：部署固定在预览解析出的提交上。
    let mut confirm = params();
    confirm["snapshot_hash"] = json!(hash);
    let (status, deployment) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/deployments",
            confirm,
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{deployment}");
    assert_eq!(deployment["resolved_commit_sha"], json!(SHA));
    assert_eq!(deployment["deployment_branch"], json!("production"));

    // 不存在的 hash 不能确认部署。
    let mut bogus = params();
    bogus["snapshot_hash"] = json!("sha256:0000");
    let (status, _) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/deployments",
            bogus,
        )
        .await;
    assert_ne!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn target_preview_is_scoped_to_the_target_and_hides_internal_fields() {
    let f = fixture().await;
    let refs = f.simulate_refs().await;
    let (status, preview) = f
        .call(
            "POST",
            "/external/v1/deployment-targets/target_test/deployment-preview",
            params(),
        )
        .await;
    refs.abort();
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["targets"].as_array().unwrap().len(), 1);
    assert_eq!(preview["targets"][0]["target_id"], json!("target_test"));
    assert!(
        preview["targets"][0].get("agent_online").is_none(),
        "单目标预览不检查 Agent 在线，不返回该字段"
    );
    assert_redacted(&preview);
}

#[tokio::test]
async fn previews_reject_production_unbound_and_invalid_requests_without_side_effects() {
    let f = fixture().await;
    let previews = f.count("deployment_previews").await;

    for (path, code) in [
        (
            "/external/v1/applications/app_prod/deployment-preview",
            PROD_DEPLOY_FORBIDDEN,
        ),
        (
            "/external/v1/applications/app_drift/deployment-preview",
            PROD_DEPLOY_FORBIDDEN,
        ),
        (
            "/external/v1/applications/app_prod_no_target/deployment-preview",
            PROD_DEPLOY_FORBIDDEN,
        ),
        (
            "/external/v1/deployment-targets/target_prod/deployment-preview",
            PROD_DEPLOY_FORBIDDEN,
        ),
        (
            "/external/v1/deployment-targets/target_drift/deployment-preview",
            PROD_DEPLOY_FORBIDDEN,
        ),
    ] {
        let (status, body) = f.call("POST", path, params()).await;
        assert_eq!(
            (status, &body["code"]),
            (StatusCode::FORBIDDEN, &json!(code)),
            "{path}"
        );
    }
    for path in [
        "/external/v1/applications/app_other/deployment-preview",
        "/external/v1/deployment-targets/target_missing/deployment-preview",
    ] {
        let (status, _) = f.call("POST", path, params()).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
    let (status, _) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/deployment-preview",
            json!({"parameters":{},"unknown":1}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        f.count("deployment_previews").await,
        previews,
        "被拒绝的预览不得写入记录"
    );
}

#[tokio::test]
async fn runtime_probe_queues_for_non_production_and_is_forbidden_for_production() {
    let f = fixture().await;
    let before = f.count("application_runtime_statuses").await;
    let (status, probe) = f
        .call(
            "POST",
            "/external/v1/applications/app_test/runtime-probe",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{probe}");
    assert_eq!(probe["application_id"], json!("app_test"));
    assert!(
        matches!(probe["status"].as_str(), Some("queued" | "in_progress")),
        "{probe}"
    );
    assert_eq!(f.count("application_runtime_statuses").await, before + 1);
    assert_redacted(&probe);
    let audit: String = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE action='application.runtime_probe.request' ORDER BY created_at DESC,id DESC LIMIT 1",
    )
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert!(audit.contains("target_test"));

    let after = f.count("application_runtime_statuses").await;
    let (status, body) = f
        .call(
            "POST",
            "/external/v1/applications/app_prod/runtime-probe",
            json!({}),
        )
        .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!(PROD_DEPLOY_FORBIDDEN))
    );
    let (status, _) = f
        .call(
            "POST",
            "/external/v1/applications/app_other/runtime-probe",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(f.count("application_runtime_statuses").await, after);
}

#[tokio::test]
async fn runtime_status_reports_sanitized_summary_and_keeps_production_readable() {
    let f = fixture().await;
    f.call(
        "POST",
        "/external/v1/applications/app_test/runtime-probe",
        json!({}),
    )
    .await;
    sqlx::query("UPDATE application_runtime_statuses SET status='succeeded',observed_at='2026-10-22T00:00:00Z'")
        .execute(&f.pool)
        .await
        .unwrap();
    let (status, runtime) = f
        .call(
            "GET",
            "/external/v1/applications/app_test/runtime-status",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{runtime}");
    assert_eq!(runtime["runtime_state"], json!("running"));
    assert_eq!(runtime["probe_status"], json!("succeeded"));
    assert_eq!(runtime["checked_at"], json!("2026-10-22T00:00:00Z"));
    let mut keys: Vec<&str> = runtime
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "application_id",
            "checked_at",
            "probe_status",
            "runtime_state"
        ]
    );

    // 只读：生产应用保持“Key 绑定即可读”的现状，未绑定为 404。
    let (status, prod) = f
        .call(
            "GET",
            "/external/v1/applications/app_prod/runtime-status",
            json!({}),
        )
        .await;
    assert_eq!(
        (status, &prod["runtime_state"]),
        (StatusCode::OK, &json!("unknown"))
    );
    let (status, _) = f
        .call(
            "GET",
            "/external/v1/applications/app_other/runtime-status",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
