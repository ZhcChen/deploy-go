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
    dispatcher::dispatch_next_deployment(state).await
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
    loop {
        if *shutdown.borrow() {
            tracing::info!("部署 worker 已停止");
            return;
        }
        if last_retention.elapsed() >= Duration::from_secs(3600) {
            if let Err(error) = purge_expired_output(&state).await {
                tracing::warn!(error = ?error, "部署日志保留清理失败");
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
        .begin()
        .await
        .map_err(|_| ApiError::internal("retention"))?;
    sqlx::query("DELETE FROM deployment_events WHERE deployment_id IN (SELECT id FROM deployments WHERE status IN ('succeeded','failed','canceled','interrupted') AND datetime(finished_at) < datetime('now', ?))")
        .bind(&modifier).execute(&mut *transaction).await.map_err(|_| ApiError::internal("retention"))?;
    let deleted = sqlx::query("DELETE FROM deployment_logs WHERE deployment_id IN (SELECT id FROM deployments WHERE status IN ('succeeded','failed','canceled','interrupted') AND datetime(finished_at) < datetime('now', ?))")
        .bind(&modifier).execute(&mut *transaction).await.map_err(|_| ApiError::internal("retention"))?.rows_affected();
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
