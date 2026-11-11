//! 同节点并发部署的调度门禁：同一 Agent 上活动的部署类任务数受“有效并发上限”约束。
//!
//! 有效上限 = Agent 声明了 `concurrent_deployments_v1` ? 节点设置 : 1。
//! 这些测试用真实的 `try_dispatch`（单条原子 UPDATE）和真实的在线连接注册表，不是对纯函数的断言。
use super::*;
use crate::db;
use deploy_go_agent_protocol::{DeploymentExecuteTask, TaskPayload};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePoolOptions;

const SERIAL_CAPABILITIES: &str = r#"["pty_terminal","privileged_release"]"#;
const CONCURRENT_CAPABILITIES: &str =
    r#"["pty_terminal","privileged_release","concurrent_deployments_v1"]"#;

fn payload_json() -> (String, String) {
    let payload = TaskPayload::DeploymentExecute(DeploymentExecuteTask {
        deployment_id: "deployment_concurrency".to_owned(),
        work_root: "/srv/apps".to_owned(),
        script_path: "/srv/apps/deploy.sh".to_owned(),
        argument_tokens: Vec::new(),
        environment_file_references: Vec::new(),
        timeout_seconds: 60,
        wrapper_version: "1".to_owned(),
    });
    let json = serde_json::to_string(&payload).unwrap();
    let digest = format!("sha256:{:x}", Sha256::digest(json.as_bytes()));
    (json, digest)
}

/// 一个节点、一个 Agent（在线），`count` 个排队中的部署任务（各属于不同部署，互不冲突）。
async fn fixture(
    capabilities: &str,
    node_setting: i64,
    count: usize,
) -> (AppState, sqlx::SqlitePool, Vec<String>) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status,max_concurrent_deployments) VALUES('node','node','/srv/apps','/srv/secrets','online',?)")
        .bind(node_setting)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,capabilities_json,connection_generation) VALUES('agent','node',18,?,1)")
        .bind(capabilities)
        .execute(&pool)
        .await
        .unwrap();
    let (json, digest) = payload_json();
    let mut ids = Vec::new();
    for index in 0..count {
        let id = format!("task_{index}");
        sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,'agent','deployment_execute',?,?,?,'queued','2099-01-01T00:00:00Z')")
            .bind(&id)
            .bind(format!("idem-{index}-0123456789"))
            .bind(&digest)
            .bind(&json)
            .execute(&pool)
            .await
            .unwrap();
        ids.push(id);
    }
    let state = AppState::new(pool.clone());
    // 让 Agent 在线：`try_dispatch` 在更新前会检查连接，没有它所有断言都会因为“未连接”而假性通过。
    let (outbound, receiver) = tokio::sync::mpsc::channel(64);
    std::mem::forget(receiver);
    let _stop = state.agent_connections().register("agent", 1, outbound);
    (state, pool, ids)
}

async fn status(pool: &sqlx::SqlitePool, id: &str) -> String {
    sqlx::query_scalar("SELECT status FROM agent_tasks WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn delivered_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks WHERE status='delivered'")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[test]
fn effective_concurrency_requires_the_capability_and_falls_back_to_one() {
    // 没有声明能力：节点设置再大也是 1（生产、预发布与旧版本 Agent）。
    assert_eq!(
        effective_deployment_concurrency(Some(SERIAL_CAPABILITIES), 10),
        1
    );
    assert_eq!(
        effective_deployment_concurrency(Some(SERIAL_CAPABILITIES), 16),
        1
    );
    // 声明了能力：取节点设置，并按协议范围截断。
    assert_eq!(
        effective_deployment_concurrency(Some(CONCURRENT_CAPABILITIES), 1),
        1
    );
    assert_eq!(
        effective_deployment_concurrency(Some(CONCURRENT_CAPABILITIES), 10),
        10
    );
    assert_eq!(
        effective_deployment_concurrency(Some(CONCURRENT_CAPABILITIES), 16),
        16
    );
    assert_eq!(
        effective_deployment_concurrency(Some(CONCURRENT_CAPABILITIES), 99),
        16
    );
    assert_eq!(
        effective_deployment_concurrency(Some(CONCURRENT_CAPABILITIES), 0),
        1
    );
    assert_eq!(
        effective_deployment_concurrency(Some(CONCURRENT_CAPABILITIES), -5),
        1
    );
    // 缺失、空、损坏、含未知能力导致整体解析失败：一律安全地退回 1，不会误放开并发。
    for broken in [
        None,
        Some(""),
        Some("[]"),
        Some("not json"),
        Some(r#"["concurrent_deployments_v1","from_the_future"]"#),
        Some(r#"{"concurrent_deployments_v1":true}"#),
    ] {
        assert_eq!(
            effective_deployment_concurrency(broken, 10),
            1,
            "{broken:?}"
        );
    }
}

#[tokio::test]
async fn default_agent_still_runs_one_deployment_at_a_time() {
    // 默认（没有并发能力）：即使节点设置是 10，也是一次一个。这是对现状行为不变的保证。
    let (state, pool, ids) = fixture(SERIAL_CAPABILITIES, 10, 3).await;
    assert!(try_dispatch(&state, &ids[0]).await.unwrap());
    assert!(!try_dispatch(&state, &ids[1]).await.unwrap());
    assert!(!try_dispatch(&state, &ids[2]).await.unwrap());
    assert_eq!(delivered_count(&pool).await, 1);
    assert!(
        has_queued_deployment_waiting_for_agent(&state)
            .await
            .unwrap()
    );
    // 前一个结束后，下一个才可以投递。
    sqlx::query("UPDATE agent_tasks SET status='succeeded' WHERE id=?")
        .bind(&ids[0])
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !has_queued_deployment_waiting_for_agent(&state)
            .await
            .unwrap()
    );
    assert!(try_dispatch(&state, &ids[1]).await.unwrap());
    assert!(!try_dispatch(&state, &ids[2]).await.unwrap());
}

#[tokio::test]
async fn concurrent_agent_runs_up_to_the_node_setting_and_refills_slots() {
    let (state, pool, ids) = fixture(CONCURRENT_CAPABILITIES, 3, 5).await;
    // 前 3 个同时投递，第 4、5 个排队。
    for id in &ids[..3] {
        assert!(try_dispatch(&state, id).await.unwrap(), "{id}");
    }
    assert!(!try_dispatch(&state, &ids[3]).await.unwrap());
    assert!(!try_dispatch(&state, &ids[4]).await.unwrap());
    assert_eq!(delivered_count(&pool).await, 3);
    assert_eq!(status(&pool, &ids[3]).await, "queued");
    // 满载时 worker 应当等待（不要空转）。
    assert!(
        has_queued_deployment_waiting_for_agent(&state)
            .await
            .unwrap()
    );
    // 结束一个，补上一个；其余仍然排队。
    sqlx::query("UPDATE agent_tasks SET status='succeeded' WHERE id=?")
        .bind(&ids[1])
        .execute(&pool)
        .await
        .unwrap();
    assert!(try_dispatch(&state, &ids[3]).await.unwrap());
    assert!(!try_dispatch(&state, &ids[4]).await.unwrap());
    assert_eq!(delivered_count(&pool).await, 3);
    // 再结束两个，最后一个也能投递；此时没有任务在等 Agent。
    for id in [&ids[0], &ids[2]] {
        sqlx::query("UPDATE agent_tasks SET status='failed' WHERE id=?")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    assert!(try_dispatch(&state, &ids[4]).await.unwrap());
    assert!(
        !has_queued_deployment_waiting_for_agent(&state)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn node_setting_of_one_keeps_a_concurrent_capable_agent_serial() {
    // 本机开了并发，但控制面设置为 1（回退手段之一）：仍是一次一个。
    let (state, pool, ids) = fixture(CONCURRENT_CAPABILITIES, 1, 3).await;
    assert!(try_dispatch(&state, &ids[0]).await.unwrap());
    assert!(!try_dispatch(&state, &ids[1]).await.unwrap());
    assert_eq!(delivered_count(&pool).await, 1);
}

#[tokio::test]
async fn already_delivered_task_can_be_resent_without_counting_itself() {
    // 重新投递（连接重连后同一任务再次发送）时，任务不能把自己算作“另一个活动任务”。
    let (state, pool, ids) = fixture(CONCURRENT_CAPABILITIES, 2, 3).await;
    assert!(try_dispatch(&state, &ids[0]).await.unwrap());
    assert!(try_dispatch(&state, &ids[1]).await.unwrap());
    assert!(
        try_dispatch(&state, &ids[0]).await.unwrap(),
        "已投递的任务重发不应被自己占满"
    );
    assert!(try_dispatch(&state, &ids[1]).await.unwrap());
    assert!(!try_dispatch(&state, &ids[2]).await.unwrap());
    assert_eq!(delivered_count(&pool).await, 2);
}

#[tokio::test]
async fn running_accepted_and_canceling_tasks_all_hold_a_slot() {
    let (state, pool, ids) = fixture(CONCURRENT_CAPABILITIES, 3, 4).await;
    for (id, status) in [
        (&ids[0], "accepted"),
        (&ids[1], "running"),
        (&ids[2], "canceling"),
    ] {
        sqlx::query("UPDATE agent_tasks SET status=? WHERE id=?")
            .bind(status)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    // 三个活动状态都占用名额，第 4 个必须等待；其中一个到终态才释放。
    assert!(!try_dispatch(&state, &ids[3]).await.unwrap());
    sqlx::query("UPDATE agent_tasks SET status='interrupted' WHERE id=?")
        .bind(&ids[2])
        .execute(&pool)
        .await
        .unwrap();
    assert!(try_dispatch(&state, &ids[3]).await.unwrap());
}

#[tokio::test]
async fn limits_are_per_agent_and_overdue_tasks_release_their_slot() {
    let (state, pool, ids) = fixture(CONCURRENT_CAPABILITIES, 2, 3).await;
    // 另一个节点上的 Agent（没有并发能力）互不影响：它自己的一次一个。
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('other','other','/srv/apps','/srv/secrets','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,capabilities_json,connection_generation) VALUES('other_agent','other',18,?,1)")
        .bind(SERIAL_CAPABILITIES)
        .execute(&pool)
        .await
        .unwrap();
    let (json, digest) = payload_json();
    for index in 0..2 {
        sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,'other_agent','deployment_execute',?,?,?,'queued','2099-01-01T00:00:00Z')")
            .bind(format!("other_{index}"))
            .bind(format!("other-idem-{index}-0123456789"))
            .bind(&digest)
            .bind(&json)
            .execute(&pool)
            .await
            .unwrap();
    }
    let (outbound, receiver) = tokio::sync::mpsc::channel(64);
    std::mem::forget(receiver);
    let _stop = state
        .agent_connections()
        .register("other_agent", 1, outbound);
    assert!(try_dispatch(&state, &ids[0]).await.unwrap());
    assert!(try_dispatch(&state, &ids[1]).await.unwrap());
    assert!(!try_dispatch(&state, &ids[2]).await.unwrap());
    assert!(try_dispatch(&state, "other_0").await.unwrap());
    assert!(!try_dispatch(&state, "other_1").await.unwrap());
    // 015 的超期兜底把卡死的任务判为中断后，它占用的名额被释放，同节点其他部署继续。
    sqlx::query(
        "UPDATE agent_tasks SET status='running',deadline_at='2026-01-01T00:00:00Z' WHERE id=?",
    )
    .bind(&ids[0])
    .execute(&pool)
    .await
    .unwrap();
    assert!(!try_dispatch(&state, &ids[2]).await.unwrap());
    assert_eq!(interrupt_overdue_deployment_tasks(&state).await.unwrap(), 1);
    assert!(try_dispatch(&state, &ids[2]).await.unwrap());
}
