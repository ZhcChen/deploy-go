//! 对外 API 对生产（`prod`）应用的行为基线。
//!
//! 对外 API 只允许操作非生产应用：所有写操作对 `prod` 返回 403，只读与 `cancel` 保持现状。
//! 本文件把“现有每个对外接口对 prod 的预期”钉成表，后续新增对外接口（见
//! `specs/005-external-nonprod-management`）不得改变这些行为；新增写接口须追加进 `PROD_WRITE_CASES`。
mod common;

use axum::{Router, http::StatusCode};
use common::{admin_session, json_request, response_json, test_app};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::collections::BTreeSet;

const APPLICATION_FORBIDDEN: &str = "external_production_application_forbidden";
const ENVIRONMENT_FORBIDDEN: &str = "external_production_environment_forbidden";
const DEPLOYMENT_FORBIDDEN: &str = "external_production_deployment_forbidden";

/// 一个对 `prod` 必须被拒绝的写请求：方法、路径、请求体、预期 403 错误码。
struct WriteCase {
    method: &'static str,
    /// OpenAPI 路径模板，用于核对覆盖是否完整。
    template: &'static str,
    path: &'static str,
    body: fn() -> Value,
    code: &'static str,
}

fn none() -> Value {
    json!({})
}

const PROD_WRITE_CASES: &[WriteCase] = &[
    WriteCase {
        method: "POST",
        template: "/external/v1/applications",
        path: "/external/v1/applications",
        body: || json!({"name":"新正式应用","slug":"new-prod-app","environment":"prod"}),
        code: ENVIRONMENT_FORBIDDEN,
    },
    WriteCase {
        method: "PATCH",
        template: "/external/v1/applications/{id}",
        path: "/external/v1/applications/app_prod",
        body: || json!({"version":1,"name":"改名"}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/applications/{id}/deployments",
        path: "/external/v1/applications/app_prod/deployments",
        body: || json!({"parameters":{}}),
        code: DEPLOYMENT_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/applications/{id}/env-files",
        path: "/external/v1/applications/app_prod/env-files",
        body: || json!({"files":[{"file_name":"prod.env","module":"api","format":"dotenv-v1","content":"A=1\n"}]}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "PUT",
        template: "/external/v1/applications/{id}/env-files/{env_file_id}",
        path: "/external/v1/applications/app_prod/env-files/env_missing",
        body: || json!({"content":"A=2\n","expected_version":1}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "DELETE",
        template: "/external/v1/applications/{id}/env-files/{env_file_id}",
        path: "/external/v1/applications/app_prod/env-files/env_missing",
        body: || json!({"confirm_file_name":"prod.env","expected_version":1}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/applications/{id}/env-files/{env_file_id}/inspect",
        path: "/external/v1/applications/app_prod/env-files/env_missing/inspect",
        body: || json!({"keys":["A"]}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/applications/{id}/targets",
        path: "/external/v1/applications/app_prod/targets",
        body: || json!({"node_id":"node_prod","script_path":"/srv/deploy.sh","timeout_seconds":60}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "PUT",
        template: "/external/v1/applications/{id}/workspace-source",
        path: "/external/v1/applications/app_prod/workspace-source",
        body: || json!({"build_agent_id":"agent_prod","workspace_path":"/srv/workspace"}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "PATCH",
        template: "/external/v1/deployment-targets/{target_id}",
        path: "/external/v1/deployment-targets/target_prod",
        body: || json!({"node_id":"node_prod","script_path":"/srv/deploy.sh","timeout_seconds":60,"version":1}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "PUT",
        template: "/external/v1/deployment-targets/{target_id}/status",
        path: "/external/v1/deployment-targets/target_prod/status",
        body: || json!({"status":"disabled","version":1}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "PUT",
        template: "/external/v1/applications/{id}/git-source",
        path: "/external/v1/applications/app_prod/git-source",
        body: || json!({"repository_url":"git@git.example.test:deploy-go/example.git","build_agent_id":"agent_prod","source_policy":"branch"}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/applications/{id}/git-source/refreshes",
        path: "/external/v1/applications/app_prod/git-source/refreshes",
        body: none,
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "PUT",
        template: "/external/v1/applications/{id}/git-source/branch",
        path: "/external/v1/applications/app_prod/git-source/branch",
        body: || json!({"branch":"main","version":1}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "PUT",
        template: "/external/v1/applications/{id}/status",
        path: "/external/v1/applications/app_prod/status",
        body: || json!({"status":"archived","version":1}),
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/applications/{id}/env-files/{env_file_id}/sync-retry",
        path: "/external/v1/applications/app_prod/env-files/env_missing/sync-retry",
        body: none,
        code: APPLICATION_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/applications/{id}/deployment-preview",
        path: "/external/v1/applications/app_prod/deployment-preview",
        body: || json!({"parameters":{}}),
        code: DEPLOYMENT_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/deployment-targets/{target_id}/deployment-preview",
        path: "/external/v1/deployment-targets/target_prod/deployment-preview",
        body: || json!({"parameters":{}}),
        code: DEPLOYMENT_FORBIDDEN,
    },
    WriteCase {
        method: "POST",
        template: "/external/v1/applications/{id}/runtime-probe",
        path: "/external/v1/applications/app_prod/runtime-probe",
        body: none,
        code: DEPLOYMENT_FORBIDDEN,
    },
];

/// 对 prod 只读请求：现状是 Key 绑定即可读取。第三项是预期状态码：
/// 生产应用没有配置 Git 来源时读取来源与发现记录为 404（可读，只是没有内容）。
const PROD_READ_CASES: &[(&str, &str, StatusCode)] = &[
    (
        "/external/v1/applications",
        "/external/v1/applications",
        StatusCode::OK,
    ),
    (
        "/external/v1/applications/{id}",
        "/external/v1/applications/app_prod",
        StatusCode::OK,
    ),
    (
        "/external/v1/applications/{id}/env-files",
        "/external/v1/applications/app_prod/env-files",
        StatusCode::OK,
    ),
    (
        "/external/v1/applications/{id}/targets",
        "/external/v1/applications/app_prod/targets",
        StatusCode::OK,
    ),
    (
        "/external/v1/deployments/{id}",
        "/external/v1/deployments/DEPLOYMENT",
        StatusCode::OK,
    ),
    (
        "/external/v1/deployments/{id}/diagnostics",
        "/external/v1/deployments/DEPLOYMENT/diagnostics",
        StatusCode::OK,
    ),
    (
        "/external/v1/deployments/{id}/logs",
        "/external/v1/deployments/DEPLOYMENT/logs",
        StatusCode::OK,
    ),
    (
        "/external/v1/applications/{id}/runtime-status",
        "/external/v1/applications/app_prod/runtime-status",
        StatusCode::OK,
    ),
    (
        "/external/v1/build-agents",
        "/external/v1/build-agents",
        StatusCode::OK,
    ),
    (
        "/external/v1/git-credentials",
        "/external/v1/git-credentials",
        StatusCode::OK,
    ),
    (
        "/external/v1/applications/{id}/git-source",
        "/external/v1/applications/app_prod/git-source",
        StatusCode::NOT_FOUND,
    ),
    (
        "/external/v1/applications/{id}/git-source/refreshes/{refs_query_id}",
        "/external/v1/applications/app_prod/git-source/refreshes/refs_missing",
        StatusCode::NOT_FOUND,
    ),
];

/// 新增的只读接口：生产应用同样被拒绝（配置结构本身也不经对外 API 暴露）。
/// 既有只读接口保持“Key 绑定即可读”的现状，见 `PROD_READ_CASES`。
const PROD_READ_FORBIDDEN_CASES: &[(&str, &str)] = &[(
    "/external/v1/applications/{id}/config-files",
    "/external/v1/applications/app_prod/config-files",
)];

async fn seed(pool: &SqlitePool) {
    for sql in [
        "INSERT INTO applications(id,name,slug,description,status,environment) VALUES('app_prod','Prod App','prod-app','','active','prod')",
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_prod','正式节点','/srv/apps','/srv/secrets','online')",
        "INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,capabilities_json) VALUES('agent_prod','node_prod','prod','0.2.0',14,'[\"pty_terminal\",\"privileged_release\"]')",
        "INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('target_prod','app_prod','node_prod','prod','/srv/deploy.sh',60,'active')",
    ] {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

/// 生产应用本身不能经对外 API 创建部署，这里先把它临时当作 test 环境创建一次部署，
/// 再还原为 prod，得到一个属于生产应用的真实部署记录用来验证只读与取消的现状。
async fn seed_prod_deployment(app: &Router, pool: &SqlitePool, auth: &str) -> String {
    sqlx::query("UPDATE applications SET environment='test' WHERE id='app_prod'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_targets SET environment='test' WHERE id='target_prod'")
        .execute(pool)
        .await
        .unwrap();
    let created = json_request(
        app.clone(),
        "POST",
        "/external/v1/applications/app_prod/deployments",
        json!({"parameters":{}}),
        &[
            ("authorization", auth),
            ("idempotency-key", "prod-guard-seed-0001"),
        ],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    sqlx::query("UPDATE applications SET environment='prod' WHERE id='app_prod'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_targets SET environment='prod' WHERE id='target_prod'")
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn guarded_app() -> (Router, SqlitePool, String, String) {
    let (app, pool) = test_app().await;
    seed(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/external-api-keys",
        json!({"name":"生产护栏 Key","application_ids":["app_prod"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let token = response_json(created).await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let auth = format!("Bearer {token}");
    let deployment_id = seed_prod_deployment(&app, &pool, &auth).await;
    (app, pool, auth, deployment_id)
}

#[tokio::test]
async fn every_external_write_on_production_is_forbidden_with_stable_code() {
    let (app, pool, auth, _) = guarded_app().await;
    let before = snapshot(&pool).await;

    for case in PROD_WRITE_CASES {
        let response = json_request(
            app.clone(),
            case.method,
            case.path,
            (case.body)(),
            &[
                ("authorization", &auth),
                ("idempotency-key", "prod-guard-0001"),
            ],
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{} {} 应对 prod 返回 403",
            case.method,
            case.path
        );
        assert_eq!(
            response_json(response).await["code"],
            json!(case.code),
            "{} {} 的错误码",
            case.method,
            case.path
        );
    }

    assert_eq!(
        snapshot(&pool).await,
        before,
        "被拒绝的写操作不得改变任何数据"
    );
}

#[tokio::test]
async fn new_read_endpoints_are_forbidden_for_production() {
    let (app, _pool, auth, _) = guarded_app().await;
    for (_, path) in PROD_READ_FORBIDDEN_CASES {
        let response = json_request(
            app.clone(),
            "GET",
            path,
            none(),
            &[("authorization", &auth)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "GET {path}");
        assert_eq!(
            response_json(response).await["code"],
            json!(APPLICATION_FORBIDDEN),
            "GET {path}"
        );
    }
}

#[tokio::test]
async fn production_reads_keep_current_visibility() {
    let (app, _pool, auth, deployment_id) = guarded_app().await;
    for (_, path, expected) in PROD_READ_CASES {
        let path = path.replace("DEPLOYMENT", &deployment_id);
        let response = json_request(
            app.clone(),
            "GET",
            &path,
            none(),
            &[("authorization", &auth)],
        )
        .await;
        assert_eq!(response.status(), *expected, "GET {path} 应保持现状");
    }

    let listed = response_json(
        json_request(
            app.clone(),
            "GET",
            "/external/v1/applications",
            none(),
            &[("authorization", &auth)],
        )
        .await,
    )
    .await;
    assert_eq!(listed["items"][0]["environment"], json!("prod"));
}

#[tokio::test]
async fn production_deployment_cancel_keeps_current_behavior() {
    let (app, _pool, auth, deployment_id) = guarded_app().await;
    let response = json_request(
        app,
        "POST",
        &format!("/external/v1/deployments/{deployment_id}/cancel"),
        none(),
        &[("authorization", &auth)],
    )
    .await;
    // 现状：cancel 不受生产守卫限制，Key 可取消自己可见应用的部署。
    assert!(
        response.status().is_success(),
        "cancel 应保持可用，实际 {}",
        response.status()
    );
}

#[tokio::test]
async fn guard_table_covers_every_external_openapi_operation() {
    let (app, _) = test_app().await;
    let spec =
        response_json(json_request(app, "GET", "/external/v1/openapi.json", none(), &[]).await)
            .await;
    let mut documented = BTreeSet::new();
    for (path, item) in spec["paths"].as_object().unwrap() {
        for method in item.as_object().unwrap().keys() {
            documented.insert(format!("{} {path}", method.to_uppercase()));
        }
    }

    let mut covered: BTreeSet<String> = PROD_WRITE_CASES
        .iter()
        .map(|case| format!("{} {}", case.method, case.template))
        .collect();
    for (template, _, _) in PROD_READ_CASES {
        covered.insert(format!("GET {template}"));
    }
    for (template, _) in PROD_READ_FORBIDDEN_CASES {
        covered.insert(format!("GET {template}"));
    }
    covered.insert("POST /external/v1/deployments/{id}/cancel".to_owned());
    // 目标列表与工作区来源读取没有 prod 守卫，随读取清单之外的现状一并列出。
    covered.insert("GET /external/v1/applications/{id}/workspace-source".to_owned());

    assert_eq!(
        documented, covered,
        "对外 OpenAPI 操作集合与生产护栏清单不一致：新增接口必须补进清单"
    );
}

/// 与外部写入相关的表行数快照，用于证明被拒绝的请求没有副作用。
async fn snapshot(pool: &SqlitePool) -> Vec<i64> {
    let mut counts = Vec::new();
    for table in [
        "applications",
        "deployment_targets",
        "deployments",
        "application_env_files",
        "application_workspace_sources",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(pool)
            .await
            .unwrap();
        counts.push(count);
    }
    counts
}
