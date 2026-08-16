use std::time::Duration;

use chrono::Utc;
use deploy_go_agent_protocol::{Message, NodeTelemetry};
use deploy_go_api::{db, node_telemetry};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

fn sample(sequence: u64) -> NodeTelemetry {
    let Message::NodeTelemetry(sample) = serde_json::from_value(json!({
        "type":"node_telemetry","connection_generation":1,"sample_sequence":sequence,
        "captured_at":Utc::now().to_rfc3339(),"snapshot":{
            "cpu":{"status":"warming_up","usage_percent":null},
            "memory":{"status":"available","total_bytes":1000,"used_bytes":400,"usage_percent":40.0},
            "work_root_disk":{"status":"available","total_bytes":2000,"used_bytes":1000,"usage_percent":50.0},
            "disk_io":{"status":"warming_up","read_bytes_per_second":null,"write_bytes_per_second":null,"busy_percent":null},
            "network":{"status":"warming_up","receive_bytes_per_second":null,"transmit_bytes_per_second":null},
            "gpu_status":"unsupported","gpu_reason":"hardware_not_present","gpus":[]
        }
    })).unwrap() else { unreachable!() };
    sample
}

#[tokio::test]
#[ignore = "容量基线，仅在发布准备阶段显式执行"]
async fn one_hundred_nodes_store_without_blocking_and_overload_budget_is_bounded() {
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let started = tokio::time::Instant::now();
    for index in 0..100 {
        let node = format!("node-{index}");
        let agent = format!("agent-{index}");
        sqlx::query("INSERT INTO nodes(id,name,status) VALUES(?,?, 'online')")
            .bind(&node)
            .bind(&node)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,protocol_version,connection_generation) VALUES(?,?,12,1)").bind(&agent).bind(&node).execute(&pool).await.unwrap();
        assert!(matches!(
            node_telemetry::store(&pool, &agent, 1, &sample(1))
                .await
                .unwrap(),
            node_telemetry::StoreOutcome::Stored
        ));
    }
    assert!(started.elapsed() < Duration::from_secs(5));
    let budget = node_telemetry::TelemetryBudget::default();
    assert_eq!((0..1_000).filter(|_| budget.try_acquire()).count(), 100);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM node_telemetry_history")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 100);
}
