use std::time::Duration;

use chrono::Utc;
use sqlx::SqlitePool;

use crate::{
    AppState,
    agents::dispatcher,
    error::{ApiError, ApiResult},
    settings,
};

pub async fn recover(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let requeued = sqlx::query("UPDATE agent_tasks SET status='queued',lease_expires_at=NULL,updated_at=? WHERE status='delivered' AND lease_expires_at IS NOT NULL AND lease_expires_at<=?")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?
        .rows_affected();
    let interrupted = sqlx::query("UPDATE deployments SET status='interrupted',phase='interrupted',result_summary='API 重启时无法关联活动 Agent 任务',finished_at=?,updated_at=?,version=version+1 WHERE status IN ('running','canceling') AND NOT EXISTS (SELECT 1 FROM agent_tasks t WHERE t.deployment_id=deployments.id)")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?
        .rows_affected();
    let terminalized = sqlx::query("UPDATE deployments SET status=(SELECT CASE WHEN t.status='canceled' THEN 'canceled' WHEN t.status='interrupted' THEN 'interrupted' WHEN t.status='succeeded' THEN 'succeeded' ELSE 'failed' END FROM agent_tasks t WHERE t.deployment_id=deployments.id AND t.status IN ('succeeded','failed','canceled','interrupted') AND NOT (t.stage='prepare' AND t.status='succeeded') ORDER BY t.created_at DESC,t.id DESC LIMIT 1),phase=(SELECT CASE WHEN t.status='canceled' THEN 'canceled' WHEN t.status='interrupted' THEN 'interrupted' WHEN t.status='succeeded' THEN 'succeeded' ELSE 'failed' END FROM agent_tasks t WHERE t.deployment_id=deployments.id AND t.status IN ('succeeded','failed','canceled','interrupted') AND NOT (t.stage='prepare' AND t.status='succeeded') ORDER BY t.created_at DESC,t.id DESC LIMIT 1),result_summary='API 重启后按持久化任务终态收敛',protocol_complete=1,finished_at=?,updated_at=?,version=version+1 WHERE deployments.status='running' AND EXISTS (SELECT 1 FROM agent_tasks t WHERE t.deployment_id=deployments.id AND t.status IN ('succeeded','failed','canceled','interrupted') AND NOT (t.stage='prepare' AND t.status='succeeded'))")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(requeued + interrupted + terminalized)
}

pub async fn process_one(state: &AppState) -> ApiResult<Option<String>> {
    // 不兼容 Agent 的遗留活动任务不能持续占用 worker 并发额度。
    dispatcher::requeue_expired_deliveries(state).await?;
    dispatcher::terminalize_runs_for_terminal_deployments(state).await?;
    let limit = settings::load(state.pool(), "worker")
        .await?
        .max_concurrent_deployments;
    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks WHERE status IN ('delivered','accepted','running','canceling')")
        .fetch_one(state.pool())
        .await
        .map_err(|_| ApiError::internal("worker"))?;
    if active >= i64::from(limit) {
        return Ok(None);
    }
    let dispatched = dispatcher::dispatch_next_deployment(state).await?;
    if dispatched.is_some() && dispatcher::has_queued_deployment_waiting_for_agent(state).await? {
        return Ok(None);
    }
    Ok(dispatched)
}

pub async fn run_worker(state: AppState, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    if let Err(error) = recover(state.pool()).await {
        tracing::error!(error = %error, "部署恢复失败");
        return;
    }
    if let Some(store) = state.artifact_store()
        && let Err(error) = crate::artifacts::reconcile_and_cleanup(state.pool(), store).await
    {
        tracing::warn!(error = %error, "制品存储恢复失败，将在清理周期重试");
    }
    let mut last_retention = tokio::time::Instant::now() - Duration::from_secs(3600);
    let mut last_capacity = tokio::time::Instant::now() - Duration::from_secs(60);
    loop {
        if *shutdown.borrow() {
            tracing::info!("部署 worker 已停止");
            return;
        }
        if last_capacity.elapsed() >= Duration::from_secs(60) {
            if let Err(error) = purge_over_capacity_output(&state).await {
                tracing::warn!(error = ?error, "部署日志容量清理失败");
            }
            last_capacity = tokio::time::Instant::now();
        }
        if last_retention.elapsed() >= Duration::from_secs(3600) {
            if let Err(error) = purge_expired_output(&state).await {
                tracing::warn!(error = ?error, "部署日志保留清理失败");
            }
            if let Err(error) = crate::node_telemetry::purge_expired(state.pool()).await {
                tracing::warn!(error = %error, "节点遥测保留清理失败");
            }
            if let Some(store) = state.artifact_store()
                && let Err(error) =
                    crate::artifacts::reconcile_and_cleanup(state.pool(), store).await
            {
                tracing::warn!(error = %error, "制品清理失败");
            }
            last_retention = tokio::time::Instant::now();
        }
        match process_one(&state).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                if wait_or_shutdown(&mut shutdown, Duration::from_millis(500)).await {
                    tracing::info!("部署 worker 已停止");
                    return;
                }
            }
            Err(error) => {
                tracing::warn!(error = ?error, "Agent 部署任务调度失败");
                if wait_or_shutdown(&mut shutdown, Duration::from_millis(500)).await {
                    tracing::info!("部署 worker 已停止");
                    return;
                }
            }
        }
    }
}

async fn wait_or_shutdown(
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
    duration: Duration,
) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => false,
        result = shutdown.changed() => result.is_err() || *shutdown.borrow(),
    }
}

pub async fn cancel_remote(state: &AppState, id: &str) -> ApiResult<()> {
    dispatcher::request_deployment_cancel(state, id).await?;
    Ok(())
}

pub async fn purge_expired_output(state: &AppState) -> ApiResult<u64> {
    let days = settings::load(state.pool(), "retention")
        .await?
        .log_retention_days;
    let modifier = format!("-{days} days");
    let mut transaction = state
        .pool()
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| ApiError::internal("retention"))?;
    let ids: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT d.id FROM deployments d WHERE {safe} AND datetime(d.finished_at) < datetime('now', ?) ORDER BY d.finished_at,d.id", safe = safe_output_deployment()
    )).bind(&modifier).fetch_all(&mut *transaction).await.map_err(|_| ApiError::internal("retention"))?;
    let mut deleted = 0;
    for id in ids {
        let Some(count) = delete_output(&mut transaction, &id).await? else {
            continue;
        };
        deleted += count;
        sqlx::query("DELETE FROM deployment_events WHERE deployment_id=? AND event_name NOT IN ('diagnostic','result','progress') AND diagnostic_code IS NULL")
            .bind(&id).execute(&mut *transaction).await.map_err(|_| ApiError::internal("retention"))?;
    }
    sqlx::query("DELETE FROM deployment_previews WHERE datetime(expires_at) < datetime('now') OR (status='confirmed' AND datetime(confirmed_at) < datetime('now', ?))")
        .bind(&modifier)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal("retention"))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("retention"))?;
    Ok(deleted)
}

// receipts 的主键和正序号约束保证区间计数能证明无缺口；旧任务不推断交付完整。
const UNSAFE_OUTPUT_TASK: &str = r#"
t.status NOT IN ('succeeded','failed','canceled','interrupted')
            OR t.last_sequence <= 0
            OR CASE WHEN json_valid(t.result_json) THEN
                COALESCE(json_type(t.result_json,'$.sequence') != 'integer'
                    OR json_extract(t.result_json,'$.sequence') != t.last_sequence
                    OR json_extract(t.result_json,'$.status') NOT IN ('succeeded','failed','canceled','interrupted'), 1)
                ELSE 1 END
            OR (SELECT COUNT(*) FROM agent_task_event_receipts r
                WHERE r.task_id=t.id AND r.sequence BETWEEN 1 AND t.last_sequence AND r.committed=1) != t.last_sequence
            OR EXISTS(SELECT 1 FROM agent_task_event_receipts r WHERE r.task_id=t.id AND r.sequence>t.last_sequence)
            OR EXISTS(SELECT 1 FROM agent_task_events e WHERE e.task_id=t.id AND e.sequence>t.last_sequence AND e.kind!='diagnostic')
            OR NOT EXISTS(SELECT 1 FROM agent_task_events e WHERE e.task_id=t.id AND e.sequence=t.last_sequence AND e.kind='result'
                AND CASE WHEN json_valid(e.payload_json) THEN
                    json_extract(e.payload_json,'$.sequence')=t.last_sequence AND json_extract(e.payload_json,'$.status')=t.status
                ELSE 0 END)
"#;

fn safe_output_deployment() -> String {
    format!(
        "d.status IN ('succeeded','failed','canceled','interrupted') AND d.finished_at IS NOT NULL AND NOT EXISTS(SELECT 1 FROM agent_tasks t WHERE t.deployment_id=d.id AND ({UNSAFE_OUTPUT_TASK}))"
    )
}
async fn delete_output(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    deployment_id: &str,
) -> ApiResult<Option<u64>> {
    let results: Vec<(String, String)> = sqlx::query_as("SELECT t.result_json,e.payload_json FROM agent_tasks t JOIN agent_task_events e ON e.task_id=t.id AND e.sequence=t.last_sequence AND e.kind='result' WHERE t.deployment_id=?")
        .bind(deployment_id).fetch_all(&mut **transaction).await.map_err(|_| ApiError::internal("retention"))?;
    if results
        .iter()
        .any(|(result, event)| !matching_result(result, event))
    {
        return Ok(None);
    }
    let deleted = sqlx::query("DELETE FROM deployment_logs WHERE deployment_id=?")
        .bind(deployment_id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal("retention"))?
        .rows_affected();
    sqlx::query("DELETE FROM agent_task_events WHERE kind='output' AND task_id IN (SELECT id FROM agent_tasks WHERE deployment_id=?)")
        .bind(deployment_id).execute(&mut **transaction).await.map_err(|_| ApiError::internal("retention"))?;
    Ok(Some(deleted))
}

fn matching_result(result: &str, event: &str) -> bool {
    match (
        serde_json::from_str::<serde_json::Value>(result),
        serde_json::from_str::<serde_json::Value>(event),
    ) {
        (Ok(result), Ok(event)) => result == event,
        _ => false,
    }
}

pub async fn purge_over_capacity_output(state: &AppState) -> ApiResult<u64> {
    let budget = settings::load(state.pool(), "capacity")
        .await?
        .max_total_log_bytes as i64;
    // 先取得写锁，再读取计数和安全判据，避免并发接收改变待删除部署的状态。
    let mut transaction = state
        .pool()
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| ApiError::internal("capacity"))?;
    let mut total: i64 =
        sqlx::query_scalar("SELECT total_bytes FROM deployment_log_capacity WHERE id=1")
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal("capacity"))?;
    let mut deleted = 0;
    if total > budget {
        let ids: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT d.id FROM deployments d WHERE {safe} AND (EXISTS(SELECT 1 FROM deployment_logs l WHERE l.deployment_id=d.id) OR EXISTS(SELECT 1 FROM agent_task_events e JOIN agent_tasks t ON t.id=e.task_id WHERE t.deployment_id=d.id AND e.kind='output')) ORDER BY d.finished_at,d.id", safe = safe_output_deployment()
        )).fetch_all(&mut *transaction).await.map_err(|_| ApiError::internal("capacity"))?;
        for id in ids {
            let Some(count) = delete_output(&mut transaction, &id).await? else {
                continue;
            };
            deleted += count;
            total =
                sqlx::query_scalar("SELECT total_bytes FROM deployment_log_capacity WHERE id=1")
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(|_| ApiError::internal("capacity"))?;
            if total <= budget {
                break;
            }
        }
    }
    if total > budget {
        // 非部署任务的受管输出也计入预算，使用相同终态/receipt/结果判据回收。
        let rows: Vec<(String, String, String)> = sqlx::query_as(&format!("SELECT t.id,t.result_json,e.payload_json FROM agent_tasks t JOIN agent_task_events e ON e.task_id=t.id AND e.sequence=t.last_sequence AND e.kind='result' WHERE t.deployment_id IS NULL AND t.finished_at IS NOT NULL AND NOT ({UNSAFE_OUTPUT_TASK}) AND EXISTS(SELECT 1 FROM agent_task_events o WHERE o.task_id=t.id AND o.kind='output') ORDER BY t.finished_at,t.id"))
            .fetch_all(&mut *transaction).await.map_err(|_| ApiError::internal("capacity"))?;
        for (id, result, event) in rows {
            if !matching_result(&result, &event) {
                continue;
            }
            deleted +=
                sqlx::query("DELETE FROM agent_task_events WHERE task_id=? AND kind='output'")
                    .bind(&id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|_| ApiError::internal("capacity"))?
                    .rows_affected();
            total =
                sqlx::query_scalar("SELECT total_bytes FROM deployment_log_capacity WHERE id=1")
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(|_| ApiError::internal("capacity"))?;
            if total <= budget {
                break;
            }
        }
    }
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal("capacity"))?;
    if total > budget {
        tracing::warn!(
            total_bytes = total,
            max_total_log_bytes = budget,
            "部署日志超过容量目标，活动或未完整交付任务受到保护，保留输出且不改变任务结果"
        );
    }
    Ok(deleted)
}
