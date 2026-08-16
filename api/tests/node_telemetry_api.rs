mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{admin_session, json_request, response_json};
use deploy_go_agent_protocol::{Message, NodeTelemetry};
use deploy_go_api::{AppState, app, db, node_telemetry};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

fn sample(sequence: u64) -> NodeTelemetry {
    let message: Message = serde_json::from_value(json!({
        "type":"node_telemetry","connection_generation":7,"sample_sequence":sequence,
        "captured_at":Utc::now().to_rfc3339(),"snapshot":{
            "cpu":{"status":"available","usage_percent":25.0},
            "memory":{"status":"available","total_bytes":1000,"used_bytes":400,"usage_percent":40.0},
            "work_root_disk":{"status":"available","total_bytes":2000,"used_bytes":1000,"usage_percent":50.0},
            "disk_io":{"status":"available","read_bytes_per_second":10.0,"write_bytes_per_second":20.0,"busy_percent":5.0},
            "network":{"status":"available","receive_bytes_per_second":30.0,"transmit_bytes_per_second":40.0},
            "gpu_status":"unsupported","gpus":[]
        }
    })).unwrap();
    let Message::NodeTelemetry(sample) = message else {
        unreachable!()
    };
    sample
}

async fn authenticated_get(
    app: axum::Router,
    cookie: &str,
    path: &str,
) -> axum::response::Response {
    json_request(app, "GET", path, json!({}), &[("cookie", cookie)]).await
}

#[tokio::test]
async fn valid_sample_updates_current_and_history_while_replay_is_ignored() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-1','Node One','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,connection_generation) VALUES('agent-1','node-1',12,7)").execute(&pool).await.unwrap();
    assert!(matches!(
        node_telemetry::store(&pool, "agent-1", 7, &sample(1))
            .await
            .unwrap(),
        node_telemetry::StoreOutcome::Stored
    ));
    assert!(matches!(
        node_telemetry::store(&pool, "agent-1", 7, &sample(1))
            .await
            .unwrap(),
        node_telemetry::StoreOutcome::Dropped
    ));
    let facts: (i64,i64) = sqlx::query_as("SELECT (SELECT sample_sequence FROM node_telemetry_current WHERE node_id='node-1'),(SELECT COUNT(*) FROM node_telemetry_history WHERE node_id='node-1')").fetch_one(&pool).await.unwrap();
    assert_eq!(facts, (1, 1));
}

#[tokio::test]
async fn stale_generation_and_clock_skew_are_dropped() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-1','Node One','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,connection_generation) VALUES('agent-1','node-1',12,7)").execute(&pool).await.unwrap();
    assert!(matches!(
        node_telemetry::store(&pool, "agent-1", 6, &sample(1))
            .await
            .unwrap(),
        node_telemetry::StoreOutcome::Dropped
    ));
    let mut skewed = sample(2);
    skewed.captured_at = (Utc::now() + Duration::minutes(6)).to_rfc3339();
    assert!(matches!(
        node_telemetry::store(&pool, "agent-1", 7, &skewed)
            .await
            .unwrap(),
        node_telemetry::StoreOutcome::Dropped
    ));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM node_telemetry_history")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn telemetry_api_exposes_supported_snapshot_and_hides_it_after_v11_downgrade() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let app = app(AppState::new(pool.clone()));
    let (cookie, _) = admin_session(app.clone()).await;
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-1','Node One','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,connection_generation) VALUES('agent-1','node-1',12,7)").execute(&pool).await.unwrap();
    node_telemetry::store(&pool, "agent-1", 7, &sample(1))
        .await
        .unwrap();
    let response = authenticated_get(app.clone(), &cookie, "/api/v1/nodes/node-1/telemetry").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["capability"], "supported");
    assert_eq!(body["freshness"], "fresh");
    assert_eq!(body["latest"]["cpu_usage_ratio"]["value"], 0.25);
    sqlx::query("UPDATE agents SET protocol_version=11 WHERE id='agent-1'")
        .execute(&pool)
        .await
        .unwrap();
    let body =
        response_json(authenticated_get(app, &cookie, "/api/v1/nodes/node-1/telemetry").await)
            .await;
    assert_eq!(body["capability"], "unsupported");
    assert!(body["latest"].is_null());
    assert_eq!(body["history"], json!([]));
}

#[tokio::test]
async fn reconnect_hides_old_current_until_the_new_generation_has_a_sample() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-1','Node One','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,connection_generation) VALUES('agent-1','node-1',12,7)").execute(&pool).await.unwrap();
    node_telemetry::store(&pool, "agent-1", 7, &sample(1))
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=8 WHERE id='agent-1'")
        .execute(&pool)
        .await
        .unwrap();
    let empty = node_telemetry::query(&pool, "node-1", "test")
        .await
        .unwrap();
    assert_eq!(empty.freshness, "empty");
    assert!(empty.latest.is_none());
    let mut next = sample(1);
    next.connection_generation = 8;
    node_telemetry::store(&pool, "agent-1", 8, &next)
        .await
        .unwrap();
    let current = node_telemetry::query(&pool, "node-1", "test")
        .await
        .unwrap();
    assert_eq!(current.freshness, "fresh");
    assert_eq!(current.history.len(), 1);
    let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM node_telemetry_history")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, 2);
}

#[tokio::test]
async fn regular_user_only_reads_telemetry_for_an_application_granted_node() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let app =
        app(AppState::new(pool.clone()).with_allowed_origins(vec!["http://localhost".into()]));
    let (admin_cookie, csrf) = admin_session(app.clone()).await;
    let created = json_request(
        app.clone(),
        "POST",
        "/api/v1/users",
        json!({"username":"operator","password":"operator-password-long"}),
        &[("cookie", &admin_cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let user = response_json(created).await;
    let user_id = user["id"].as_str().unwrap();
    let (user_cookie, _) = common::login(app.clone(), "operator", "operator-password-long").await;
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-1','Node One','offline')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,connection_generation) VALUES('agent-1','node-1',12,7)").execute(&pool).await.unwrap();
    let hidden =
        authenticated_get(app.clone(), &user_cookie, "/api/v1/nodes/node-1/telemetry").await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app-1','App One','app-one','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,target_code,script_path,timeout_seconds,status) VALUES('target-1','app-1','node-1','test','test','/srv/deploy.sh',60,'active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO user_application_grants(user_id,application_id,granted_by) VALUES(?,'app-1',(SELECT id FROM users WHERE identity='administrator'))").bind(user_id).execute(&pool).await.unwrap();
    let visible = authenticated_get(app, &user_cookie, "/api/v1/nodes/node-1/telemetry").await;
    assert_eq!(visible.status(), StatusCode::OK);
}

#[tokio::test]
async fn retention_deletes_only_samples_older_than_24_hours() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-1','Node One','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,connection_generation) VALUES('agent-1','node-1',12,7)").execute(&pool).await.unwrap();
    for sequence in 1..=2 {
        node_telemetry::store(&pool, "agent-1", 7, &sample(sequence))
            .await
            .unwrap();
    }
    sqlx::query("UPDATE node_telemetry_history SET received_at=? WHERE sample_sequence=1")
        .bind((Utc::now() - Duration::hours(25)).to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(node_telemetry::purge_expired(&pool).await.unwrap(), 1);
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM node_telemetry_history")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 1);
}
