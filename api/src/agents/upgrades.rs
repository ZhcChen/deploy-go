use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::{Duration, Utc};
use deploy_go_agent_protocol::{
    AgentUpgradeAck, AgentUpgradeCommand, AgentUpgradeErrorCode, AgentUpgradePhase,
    AgentUpgradeProgress, AgentUpgradeReport, AgentUpgradeReportStatus, Message,
};
use deploy_go_release_authorization::{AgentUpgradeClaims, SCHEMA_VERSION, UPGRADE_AUDIENCE};
use serde::Serialize;
use sqlx::{FromRow, Sqlite, SqliteConnection, SqlitePool, Transaction};
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};

pub const UPGRADE_CAPABILITY: &str = "agent_upgrade_v1";
pub const UPGRADE_ARCHITECTURE: &str = "x86_64";

#[derive(FromRow)]
struct UpgradeCandidate {
    id: String,
    node_id: String,
    agent_version: Option<String>,
    architecture: Option<String>,
}

pub async fn run_worker(state: crate::AppState, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
    interval.tick().await;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(error) = scan(&state).await {
                    tracing::warn!(error = ?error, "Agent 自动升级扫描失败");
                }
            }
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() { break; }
            }
        }
    }
}

#[allow(dead_code)] // Agent 下载与 updater 闭环完成后再由 worker 打开实际下发
async fn dispatch_one(state: &crate::AppState) -> Result<(), String> {
    let signer = state
        .release_signer()
        .ok_or_else(|| "升级签名器不可用".to_owned())?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let Some(claim) = claim_ready_job(state.pool(), &now_text, 900)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let deadline = now.timestamp().saturating_add(900);
    let authorization = match signer.sign_agent_upgrade(&AgentUpgradeClaims {
        schema_version: SCHEMA_VERSION,
        audience: UPGRADE_AUDIENCE.to_owned(),
        job_id: claim.job_id.clone(),
        nonce: format!("upgrade_nonce_{}", Ulid::new()),
        node_id: claim.node_id.clone(),
        agent_id: claim.agent_id.clone(),
        target_version: claim.target_version.clone(),
        manifest_digest: claim.manifest_digest.clone(),
        architecture: UPGRADE_ARCHITECTURE.to_owned(),
        issued_at: now.timestamp(),
        expires_at: deadline,
        deadline_at: deadline,
    }) {
        Ok(value) => value,
        Err(error) => {
            let _ = release_claim(
                state.pool(),
                &claim.job_id,
                &claim.node_id,
                &claim.lease_token,
                claim.lock_epoch,
                "failed",
                Some("upgrade_authorization_failed"),
                &now_text,
            )
            .await;
            return Err(error.to_string());
        }
    };
    let deadline_at = chrono::DateTime::from_timestamp(deadline, 0)
        .ok_or_else(|| "升级截止时间无效".to_owned())?
        .to_rfc3339();
    let command = Message::AgentUpgradeCommand(AgentUpgradeCommand {
        job_id: claim.job_id.clone(),
        target_version: claim.target_version,
        manifest_digest: claim.manifest_digest,
        authorization,
        deadline_at,
        connection_generation: claim.connection_generation as u64,
    });
    if state
        .agent_connections()
        .send_generation(&claim.agent_id, claim.connection_generation, command)
        .await
        .is_err()
    {
        release_claim(
            state.pool(),
            &claim.job_id,
            &claim.node_id,
            &claim.lease_token,
            claim.lock_epoch,
            "failed",
            Some("upgrade_command_delivery_failed"),
            &now_text,
        )
        .await
        .map_err(|error| error.to_string())?;
        return Err("当前连接无法接收升级命令".to_owned());
    }
    Ok(())
}

pub async fn scan(state: &crate::AppState) -> Result<u64, sqlx::Error> {
    let Some(installation) = state.agent_installation() else {
        return Ok(0);
    };
    let Some((target_version, manifest_digest)) = installation
        .current_upgrade_target()
        .map_err(|_| sqlx::Error::Protocol("Agent 发布物不可用".into()))?
    else {
        return Ok(0);
    };
    let candidates: Vec<UpgradeCandidate> = sqlx::query_as(
        "SELECT id,node_id,agent_version,architecture FROM agents WHERE revoked_at IS NULL AND archived_at IS NULL",
    )
    .fetch_all(state.pool())
    .await?;
    let mut created = 0;
    for candidate in candidates {
        if enqueue(
            state.pool(),
            &candidate.id,
            &candidate.node_id,
            candidate.agent_version.as_deref(),
            &target_version,
            &manifest_digest,
            candidate.architecture.as_deref(),
        )
        .await?
        .is_some()
        {
            created += 1;
        }
    }
    refresh_waiting_state(state.pool(), &Utc::now().to_rfc3339()).await?;
    Ok(created)
}

pub async fn is_node_maintained(pool: &SqlitePool, node_id: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_maintenance_locks WHERE node_id=?)")
        .bind(node_id)
        .fetch_one(pool)
        .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeClaim {
    pub job_id: String,
    pub agent_id: String,
    pub node_id: String,
    pub target_version: String,
    pub manifest_digest: String,
    pub lease_token: String,
    pub lock_epoch: i64,
    pub connection_generation: i64,
}

#[derive(Debug, Serialize, ToSchema, FromRow)]
pub struct AgentUpgradeResponse {
    pub id: String,
    pub agent_id: String,
    pub node_id: String,
    pub target_version: String,
    pub target_architecture: String,
    pub status: String,
    pub phase: Option<String>,
    pub current_version: Option<String>,
    pub attempt_count: i64,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub updated_at: String,
}

#[utoipa::path(
    operation_id = "agent_upgrades_list",
    get,
    path = "/api/v1/agent-upgrades",
    responses((status = 200, body = [AgentUpgradeResponse]), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse))
)]
pub async fn list_api(
    State(state): State<AppState>,
    axum::extract::Extension(request_id): axum::extract::Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<Vec<AgentUpgradeResponse>>> {
    actor.require_administrator(request_id.as_str())?;
    let items = sqlx::query_as::<_, AgentUpgradeResponse>("SELECT id,agent_id,node_id,target_version,target_architecture,status,phase,current_version,attempt_count,error_code,error_summary,queued_at,started_at,finished_at,updated_at FROM agent_upgrade_jobs ORDER BY queued_at DESC,id DESC LIMIT 200")
        .fetch_all(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(items))
}

#[utoipa::path(
    operation_id = "agent_upgrades_show",
    get,
    path = "/api/v1/agent-upgrades/{id}",
    params(("id" = String, Path)),
    responses((status = 200, body = AgentUpgradeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse))
)]
pub async fn show_api(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Extension(request_id): axum::extract::Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<AgentUpgradeResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let item = sqlx::query_as::<_, AgentUpgradeResponse>("SELECT id,agent_id,node_id,target_version,target_architecture,status,phase,current_version,attempt_count,error_code,error_summary,queued_at,started_at,finished_at,updated_at FROM agent_upgrade_jobs WHERE id=?")
        .bind(id).fetch_optional(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?
        .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    Ok(Json(item))
}

#[utoipa::path(
    operation_id = "agent_upgrades_retry",
    post,
    path = "/api/v1/agent-upgrades/{id}/retry",
    params(("id" = String, Path), ("X-CSRF-Token" = String, Header)),
    responses((status = 200, body = AgentUpgradeResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse))
)]
pub async fn retry_api(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Extension(request_id): axum::extract::Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Json<AgentUpgradeResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let now = Utc::now().to_rfc3339();
    let updated = sqlx::query("UPDATE agent_upgrade_jobs SET status='queued',phase=NULL,lease_token=NULL,lease_expires_at=NULL,error_code=NULL,error_summary=NULL,started_at=NULL,finished_at=NULL,updated_at=?,version=version+1 WHERE id=? AND status='failed'")
        .bind(&now).bind(&id).execute(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "upgrade_retry_unavailable",
            "只有失败的升级任务可以重试",
            request_id.as_str(),
        ));
    }
    show_api(
        State(state),
        Path(id),
        axum::extract::Extension(request_id),
        actor,
    )
    .await
}

pub async fn enqueue(
    pool: &SqlitePool,
    agent_id: &str,
    node_id: &str,
    current_version: Option<&str>,
    target_version: &str,
    manifest_digest: &str,
    architecture: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    let status = if architecture != Some(UPGRADE_ARCHITECTURE) {
        "blocked_unsupported_architecture"
    } else if current_version == Some(target_version) {
        return Ok(None);
    } else {
        "queued"
    };
    let id = format!("upgrade_{}", Ulid::new());
    let result = sqlx::query(
        "INSERT INTO agent_upgrade_jobs (id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,current_version) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT DO NOTHING",
    )
    .bind(&id)
    .bind(agent_id)
    .bind(node_id)
    .bind(target_version)
    .bind(manifest_digest)
    .bind(UPGRADE_ARCHITECTURE)
    .bind(status)
    .bind(current_version)
    .execute(pool)
    .await?;
    Ok((result.rows_affected() == 1).then_some(id))
}

pub async fn refresh_waiting_state(pool: &SqlitePool, now: &str) -> Result<u64, sqlx::Error> {
    let cutoff = (chrono::DateTime::parse_from_rfc3339(now)
        .unwrap_or_else(|_| Utc::now().fixed_offset())
        - Duration::seconds(45))
    .to_rfc3339();
    let result = sqlx::query(
        "UPDATE agent_upgrade_jobs SET status=CASE WHEN NOT EXISTS (SELECT 1 FROM agents a WHERE a.id=agent_upgrade_jobs.agent_id AND a.revoked_at IS NULL AND a.archived_at IS NULL AND a.last_seen_at>=? AND a.protocol_version>=17 AND json_valid(a.capabilities_json) AND EXISTS (SELECT 1 FROM json_each(a.capabilities_json) WHERE value=?) AND a.architecture='x86_64') THEN 'waiting_for_online' WHEN EXISTS (SELECT 1 FROM agent_tasks t WHERE t.agent_id=agent_upgrade_jobs.agent_id AND t.status IN ('queued','delivered','accepted','running','canceling')) THEN 'waiting_for_idle' ELSE 'queued' END,updated_at=? WHERE status IN ('queued','waiting_for_online','waiting_for_idle')",
    )
    .bind(cutoff)
    .bind(UPGRADE_CAPABILITY)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn claim_ready_job(
    pool: &SqlitePool,
    now: &str,
    lease_seconds: i64,
) -> Result<Option<UpgradeClaim>, sqlx::Error> {
    let mut connection = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *connection)
        .await?;
    let result = claim_ready_job_inner(&mut connection, now, lease_seconds).await;
    match result {
        Ok(claim) => {
            sqlx::query("COMMIT").execute(&mut *connection).await?;
            Ok(claim)
        }
        Err(error) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
            Err(error)
        }
    }
}

async fn claim_ready_job_inner(
    connection: &mut SqliteConnection,
    now: &str,
    lease_seconds: i64,
) -> Result<Option<UpgradeClaim>, sqlx::Error> {
    let job: Option<(String, String, String, String, i64)> = sqlx::query_as(
        "SELECT j.id,j.agent_id,j.node_id,j.target_version,a.connection_generation FROM agent_upgrade_jobs j JOIN agents a ON a.id=j.agent_id WHERE j.status='queued' AND a.revoked_at IS NULL AND a.archived_at IS NULL AND a.connection_generation>0 AND a.last_seen_at>=strftime('%Y-%m-%dT%H:%M:%fZ', ?, '-45 seconds') AND a.protocol_version>=17 AND a.architecture='x86_64' AND json_valid(a.capabilities_json) AND EXISTS (SELECT 1 FROM json_each(a.capabilities_json) WHERE value='agent_upgrade_v1') AND NOT EXISTS (SELECT 1 FROM agent_tasks t WHERE t.agent_id=j.agent_id AND t.status IN ('queued','delivered','accepted','running','canceling')) AND NOT EXISTS (SELECT 1 FROM agent_maintenance_locks l WHERE l.node_id=j.node_id) AND NOT EXISTS (SELECT 1 FROM agent_upgrade_leases l WHERE l.lease_key='global' AND l.expires_at>?) ORDER BY j.queued_at,j.id LIMIT 1",
    )
    .bind(now)
    .bind(now)
    .fetch_optional(&mut *connection)
    .await?;
    let Some((job_id, agent_id, node_id, target_version, connection_generation)) = job else {
        return Ok(None);
    };
    let manifest_digest: String = sqlx::query_scalar(
        "SELECT manifest_digest FROM agent_upgrade_jobs WHERE id=? AND status='queued'",
    )
    .bind(&job_id)
    .fetch_one(&mut *connection)
    .await?;
    let lease_token = format!("lease_{}", Ulid::new());
    let lock_epoch: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(lock_epoch),0)+1 FROM agent_maintenance_locks WHERE node_id=?",
    )
    .bind(&node_id)
    .fetch_one(&mut *connection)
    .await?;
    let expires_at = (chrono::DateTime::parse_from_rfc3339(now)
        .unwrap_or_else(|_| Utc::now().fixed_offset())
        + Duration::seconds(lease_seconds))
    .to_rfc3339();
    sqlx::query("INSERT INTO agent_maintenance_locks (node_id,agent_id,job_id,lease_token,lock_epoch,reason) VALUES (?,?,?,?,?,'agent_upgrade')")
        .bind(&node_id).bind(&agent_id).bind(&job_id).bind(&lease_token).bind(lock_epoch)
        .execute(&mut *connection).await?;
    sqlx::query("INSERT INTO agent_upgrade_leases (lease_key,job_id,lease_token,expires_at) VALUES ('global',?,?,?)")
        .bind(&job_id).bind(&lease_token).bind(&expires_at)
        .execute(&mut *connection).await?;
    sqlx::query("UPDATE agent_upgrade_jobs SET status='downloading',phase='validating',attempt_count=attempt_count+1,connection_generation=?,lease_token=?,lease_expires_at=?,updated_at=?,version=version+1 WHERE id=? AND status='queued'")
        .bind(connection_generation).bind(&lease_token).bind(&expires_at).bind(now).bind(&job_id)
        .execute(&mut *connection).await?;
    Ok(Some(UpgradeClaim {
        job_id,
        agent_id,
        node_id,
        target_version,
        manifest_digest,
        lease_token,
        lock_epoch,
        connection_generation,
    }))
}

pub async fn release_claim(
    pool: &SqlitePool,
    job_id: &str,
    node_id: &str,
    lease_token: &str,
    lock_epoch: i64,
    status: &str,
    error_code: Option<&str>,
    now: &str,
) -> Result<bool, sqlx::Error> {
    let mut transaction: Transaction<'_, Sqlite> = pool.begin().await?;
    let updated = sqlx::query("UPDATE agent_upgrade_jobs SET status=?,phase=NULL,error_code=?,finished_at=?,updated_at=?,version=version+1 WHERE id=? AND lease_token=? AND status IN ('downloading','installing','reconnecting')")
        .bind(status).bind(error_code).bind(now).bind(now).bind(job_id).bind(lease_token)
        .execute(&mut *transaction).await?;
    if updated.rows_affected() != 1 {
        transaction.rollback().await?;
        return Ok(false);
    }
    sqlx::query(
        "DELETE FROM agent_upgrade_leases WHERE lease_key='global' AND job_id=? AND lease_token=?",
    )
    .bind(job_id)
    .bind(lease_token)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("DELETE FROM agent_maintenance_locks WHERE node_id=? AND job_id=? AND lease_token=? AND lock_epoch=?")
        .bind(node_id).bind(job_id).bind(lease_token).bind(lock_epoch).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(true)
}

fn upgrade_error_code(error: Option<AgentUpgradeErrorCode>) -> Option<String> {
    error.map(|value| {
        serde_json::to_string(&value)
            .unwrap_or_default()
            .trim_matches('"')
            .to_owned()
    })
}

pub async fn handle_ack(
    pool: &SqlitePool,
    agent_id: &str,
    generation: i64,
    ack: &AgentUpgradeAck,
) -> Result<(), sqlx::Error> {
    let Some((lease_token, node_id, lock_epoch)): Option<(String, String, i64)> = sqlx::query_as(
        "SELECT j.lease_token,j.node_id,l.lock_epoch FROM agent_upgrade_jobs j JOIN agent_maintenance_locks l ON l.job_id=j.id WHERE j.id=? AND j.agent_id=? AND j.connection_generation=? AND j.status='downloading'",
    )
    .bind(&ack.job_id)
    .bind(agent_id)
    .bind(generation)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(());
    };
    if ack.accepted {
        sqlx::query("UPDATE agent_upgrade_jobs SET phase='downloading',updated_at=?,version=version+1 WHERE id=? AND lease_token=? AND status='downloading'")
            .bind(Utc::now().to_rfc3339()).bind(&ack.job_id).bind(&lease_token).execute(pool).await?;
    } else {
        let code =
            upgrade_error_code(ack.error_code).unwrap_or_else(|| "upgrade_rejected".to_owned());
        release_claim(
            pool,
            &ack.job_id,
            agent_id,
            &lease_token,
            lock_epoch,
            "failed",
            Some(&code),
            &Utc::now().to_rfc3339(),
        )
        .await?;
        let _ = node_id;
    }
    Ok(())
}

pub async fn handle_progress(
    pool: &SqlitePool,
    agent_id: &str,
    generation: i64,
    progress: &AgentUpgradeProgress,
) -> Result<(), sqlx::Error> {
    if progress.sequence == 0 || progress.sequence > i64::MAX as u64 {
        return Ok(());
    }
    let phase = match progress.phase {
        AgentUpgradePhase::Validating => "validating",
        AgentUpgradePhase::Downloading => "downloading",
        AgentUpgradePhase::Staged => "staged",
        AgentUpgradePhase::Installing => "installing",
        AgentUpgradePhase::ExecutorRestart => "executor_restart",
        AgentUpgradePhase::Reconnecting => "reconnecting",
    };
    let now = Utc::now().to_rfc3339();
    let inserted = sqlx::query("INSERT INTO agent_upgrade_events(job_id,sequence,kind,phase,summary) SELECT ?,?,'progress',?,? WHERE EXISTS (SELECT 1 FROM agent_upgrade_jobs WHERE id=? AND agent_id=? AND connection_generation=? AND status IN ('downloading','installing','reconnecting')) ON CONFLICT(job_id,sequence) DO NOTHING")
        .bind(&progress.job_id).bind(progress.sequence as i64).bind(phase)
        .bind(format!("升级阶段：{phase}"))
        .bind(&progress.job_id).bind(agent_id).bind(generation).execute(pool).await?;
    if inserted.rows_affected() == 1 {
        sqlx::query("UPDATE agent_upgrade_jobs SET status=CASE WHEN ? IN ('installing','executor_restart') THEN 'installing' WHEN ?='reconnecting' THEN 'reconnecting' ELSE 'downloading' END,phase=?,updated_at=?,version=version+1 WHERE id=? AND agent_id=? AND connection_generation=? AND status IN ('downloading','installing','reconnecting')")
            .bind(phase).bind(phase).bind(phase).bind(&now).bind(&progress.job_id).bind(agent_id).bind(generation).execute(pool).await?;
    }
    Ok(())
}

pub async fn handle_report(
    pool: &SqlitePool,
    agent_id: &str,
    generation: i64,
    report: &AgentUpgradeReport,
) -> Result<(), sqlx::Error> {
    let Some((node_id, lease_token, lock_epoch, target_version, manifest_digest)): Option<(String, String, i64, String, String)> = sqlx::query_as(
        "SELECT j.node_id,j.lease_token,l.lock_epoch,j.target_version,j.manifest_digest FROM agent_upgrade_jobs j JOIN agent_maintenance_locks l ON l.job_id=j.id WHERE j.id=? AND j.agent_id=? AND j.connection_generation=? AND j.status IN ('downloading','installing','reconnecting')",
    ).bind(&report.job_id).bind(agent_id).bind(generation).fetch_optional(pool).await? else { return Ok(()); };
    if report.target_version != target_version || report.manifest_digest != manifest_digest {
        return Ok(());
    }
    let status = match report.status {
        AgentUpgradeReportStatus::Succeeded => "succeeded",
        AgentUpgradeReportStatus::Failed => "failed",
    };
    let error = upgrade_error_code(report.error_code);
    release_claim(
        pool,
        &report.job_id,
        &node_id,
        &lease_token,
        lock_epoch,
        status,
        error.as_deref(),
        &Utc::now().to_rfc3339(),
    )
    .await?;
    Ok(())
}
