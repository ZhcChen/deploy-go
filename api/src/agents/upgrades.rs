use chrono::{Duration, Utc};
use sqlx::{Sqlite, SqliteConnection, SqlitePool, Transaction};
use ulid::Ulid;

pub const UPGRADE_CAPABILITY: &str = "agent_upgrade_v1";
pub const UPGRADE_ARCHITECTURE: &str = "x86_64";

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
        "SELECT j.id,j.agent_id,j.node_id,j.target_version,j.connection_generation FROM agent_upgrade_jobs j JOIN agents a ON a.id=j.agent_id WHERE j.status='queued' AND a.revoked_at IS NULL AND a.archived_at IS NULL AND a.last_seen_at>=strftime('%Y-%m-%dT%H:%M:%fZ', ?, '-45 seconds') AND a.protocol_version>=17 AND a.architecture='x86_64' AND json_valid(a.capabilities_json) AND EXISTS (SELECT 1 FROM json_each(a.capabilities_json) WHERE value='agent_upgrade_v1') AND NOT EXISTS (SELECT 1 FROM agent_tasks t WHERE t.agent_id=j.agent_id AND t.status IN ('queued','delivered','accepted','running','canceling')) AND NOT EXISTS (SELECT 1 FROM agent_maintenance_locks l WHERE l.node_id=j.node_id) AND NOT EXISTS (SELECT 1 FROM agent_upgrade_leases l WHERE l.lease_key='global' AND l.expires_at>?) ORDER BY j.queued_at,j.id LIMIT 1",
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
    sqlx::query("UPDATE agent_upgrade_jobs SET status='downloading',phase='validating',attempt_count=attempt_count+1,lease_token=?,lease_expires_at=?,updated_at=?,version=version+1 WHERE id=? AND status='queued'")
        .bind(&lease_token).bind(&expires_at).bind(now).bind(&job_id)
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
