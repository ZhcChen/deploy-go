mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use serde_json::{Value, json};

async fn seed_agent(
    pool: &sqlx::SqlitePool,
    suffix: &str,
    protocol_version: i64,
    runtime_probe: bool,
) {
    sqlx::query(
        "INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?,? ,?, 'online')",
    )
    .bind(format!("node_{suffix}"))
    .bind(format!("Node {suffix}"))
    .bind(format!("/srv/{suffix}/apps"))
    .bind(format!("/srv/{suffix}/secrets"))
    .execute(pool)
    .await
    .unwrap();
    let mut capabilities = vec!["pty_terminal", "privileged_release"];
    if runtime_probe {
        capabilities.push("runtime_probe_v1");
    }
    sqlx::query(
        "INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,capabilities_json) VALUES(?,?, '2026-09-07T00:00:00Z','2026-09-07T00:00:00Z','0.2.0',?,?)",
    )
    .bind(format!("agent_{suffix}"))
    .bind(format!("node_{suffix}"))
    .bind(protocol_version)
    .bind(serde_json::to_string(&capabilities).unwrap())
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_application(
    pool: &sqlx::SqlitePool,
    id: &str,
    status: &str,
    verification_config: Value,
    target_count: usize,
    node_suffixes: &[&str],
) {
    sqlx::query("INSERT INTO applications(id,name,display_name,slug,description,environment,verification_config,status) VALUES(?,?,?,?,?,?,?,?)")
        .bind(id)
        .bind(id)
        .bind(id)
        .bind(id)
        .bind("")
        .bind("prod")
        .bind(verification_config.to_string())
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
    for (index, node_suffix) in node_suffixes.iter().enumerate() {
        let target_id = if target_count == 1 {
            format!("target_{id}")
        } else {
            format!("target_{id}_{index}")
        };
        sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,target_code,execution_mode,script_path,timeout_seconds,status) VALUES(?,?,?,?,?,?,?,?,'active')")
            .bind(target_id)
            .bind(id)
            .bind(format!("node_{node_suffix}"))
            .bind("prod")
            .bind("prod")
            .bind("two_stage")
            .bind("/unused")
            .bind(900)
            .execute(pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn runtime_probe_batch_dispatches_only_single_target_apps_with_probe_port() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    seed_agent(&pool, "ok", 15, true).await;
    seed_agent(&pool, "multi_1", 15, true).await;
    seed_agent(&pool, "multi_2", 15, true).await;
    seed_application(
        &pool,
        "app_probe_ok",
        "active",
        json!({"type":"http","port":24710,"path":"/healthz","expected_status":200,"timeout_ms":5000}),
        1,
        &["ok"],
    )
    .await;
    seed_application(
        &pool,
        "app_probe_multi",
        "active",
        json!({"type":"http","port":24710,"path":"/healthz","expected_status":200,"timeout_ms":5000}),
        2,
        &["multi_1", "multi_2"],
    )
    .await;
    seed_agent(&pool, "no_port", 15, true).await;
    seed_application(
        &pool,
        "app_probe_no_port",
        "active",
        json!({"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000}),
        1,
        &["no_port"],
    )
    .await;
    seed_application(
        &pool,
        "app_probe_archived",
        "archived",
        json!({"type":"http","port":24710,"path":"/healthz","expected_status":200,"timeout_ms":5000}),
        0,
        &[],
    )
    .await;

    let response = json_request(
        app,
        "POST",
        "/api/v1/applications/runtime-probes",
        json!({"application_ids":[
            "app_probe_ok",
            "app_probe_multi",
            "app_probe_no_port",
            "app_probe_archived"
        ]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let by_id = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| (item["application_id"].as_str().unwrap(), item.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(by_id["app_probe_ok"]["status"], "queued");
    assert_eq!(by_id["app_probe_multi"]["status"], "skipped");
    assert_eq!(
        by_id["app_probe_multi"]["error_code"],
        "runtime_probe_multi_target"
    );
    assert_eq!(by_id["app_probe_no_port"]["status"], "skipped");
    assert_eq!(
        by_id["app_probe_no_port"]["error_code"],
        "runtime_probe_config_missing_port"
    );
    assert_eq!(by_id["app_probe_archived"]["status"], "skipped");

    let pending: (String, String) = sqlx::query_as(
        "SELECT runtime_status_id,status FROM application_runtime_statuses WHERE application_id='app_probe_ok'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pending.1, "pending");
    let task: (String, String, String) = sqlx::query_as(
        "SELECT kind,payload_json,runtime_status_id FROM agent_tasks WHERE runtime_status_id=?",
    )
    .bind(&pending.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(task.0, "system_inspect");
    assert_eq!(task.2, pending.0);
    let payload: Value = serde_json::from_str(&task.1).unwrap();
    assert_eq!(payload["kind"], "runtime_probe");
    assert_eq!(payload["payload"]["probe_type"], "http");
    assert_eq!(payload["payload"]["port"], 24710);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM application_runtime_statuses")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn old_agent_incompatibility_keeps_deployment_derived_list_state_with_probe_reason() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    seed_agent(&pool, "old", 14, false).await;
    seed_application(
        &pool,
        "app_probe_old_agent",
        "active",
        json!({"type":"http","port":24710,"path":"/healthz","expected_status":200,"timeout_ms":5000}),
        1,
        &["old"],
    )
    .await;
    let admin_id: String = sqlx::query_scalar("SELECT id FROM users WHERE username='admin'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,finished_at,created_at) VALUES('dep_probe_old','app_probe_old_agent','target_app_probe_old_agent',?,'succeeded','succeeded','idem-old','hash','snapshot','2026-09-07T00:00:00Z','2026-09-07T00:00:00Z')")
        .bind(&admin_id)
        .execute(&pool)
        .await
        .unwrap();

    let response = json_request(
        app.clone(),
        "POST",
        "/api/v1/applications/runtime-probes",
        json!({"application_ids":["app_probe_old_agent"]}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["items"][0]["status"], "failed");
    assert_eq!(
        body["items"][0]["error_code"],
        "runtime_probe_protocol_unsupported"
    );

    let list = json_request(
        app,
        "GET",
        "/api/v1/applications?limit=20",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let listed = response_json(list).await;
    let item = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "app_probe_old_agent")
        .unwrap()
        .clone();
    assert_eq!(item["runtime_state"], "running");
    assert_eq!(item["runtime_probe_status"], "failed");
    assert_eq!(
        item["runtime_probe_error_code"],
        "runtime_probe_protocol_unsupported"
    );
    assert_eq!(item["runtime_checked_at"], "2026-09-07T00:00:00Z");
}

#[tokio::test]
async fn successful_runtime_status_snapshot_overrides_deployment_state() {
    let (app, pool) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    seed_agent(&pool, "snapshot", 15, true).await;
    seed_application(
        &pool,
        "app_probe_snapshot",
        "active",
        json!({"type":"http","port":24710,"path":"/healthz","expected_status":200,"timeout_ms":5000}),
        1,
        &["snapshot"],
    )
    .await;
    let admin_id: String = sqlx::query_scalar("SELECT id FROM users WHERE username='admin'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,finished_at,created_at) VALUES('dep_snapshot','app_probe_snapshot','target_app_probe_snapshot',?,'failed','failed','idem-snapshot','hash','snapshot','2026-09-07T00:00:00Z','2026-09-07T00:00:00Z')")
        .bind(&admin_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_runtime_statuses(runtime_status_id,application_id,target_id,status,payload_json,observed_at,requested_at) VALUES('runtime_status_snapshot_ok','app_probe_snapshot','target_app_probe_snapshot','succeeded',?,'2026-09-07T00:05:00Z','2026-09-07T00:04:00Z')")
        .bind(serde_json::json!({"checked_at":"2026-09-07T00:05:00Z","http_status":200,"port":24710}).to_string())
        .execute(&pool)
        .await
        .unwrap();

    let response = json_request(
        app,
        "GET",
        "/api/v1/applications?limit=20",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let item = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "app_probe_snapshot")
        .unwrap()
        .clone();
    assert_eq!(item["runtime_state"], "running");
    assert_eq!(item["runtime_probe_status"], "succeeded");
    assert_eq!(item["runtime_checked_at"], "2026-09-07T00:05:00Z");
}
