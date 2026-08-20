use chrono::Utc;
use deploy_go_agent_protocol::{MIN_SUPPORTED_PROTOCOL_VERSION, PROTOCOL_VERSION};
use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};

const ACTIVE_STATUSES: &str = "'opening','active','closing'";
const UNATTACHED_OPENING_LEASE_SECONDS: i64 = 30;

#[derive(Clone, Debug, FromRow, PartialEq)]
pub struct TerminalSessionRecord {
    pub id: String,
    pub node_id: String,
    pub agent_id: String,
    pub actor_id: String,
    pub request_id: String,
    pub status: String,
    pub started_at: String,
    pub opened_at: Option<String>,
    pub close_requested_at: Option<String>,
    pub finished_at: Option<String>,
    pub exit_reason: Option<String>,
    pub exit_code: Option<i64>,
    pub input_bytes: i64,
    pub output_bytes: i64,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Debug)]
pub enum CreateSessionError {
    ActiveSessionConflict,
    GateRejected,
    Database(sqlx::Error),
}

pub async fn create_session(
    pool: &SqlitePool,
    id: &str,
    node_id: &str,
    agent_id: &str,
    actor_id: &str,
    request_id: &str,
) -> Result<TerminalSessionRecord, CreateSessionError> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(CreateSessionError::Database)?;
    let session = create_session_in(
        &mut transaction,
        id,
        node_id,
        agent_id,
        actor_id,
        request_id,
    )
    .await?;
    transaction
        .commit()
        .await
        .map_err(CreateSessionError::Database)?;
    Ok(session)
}

pub async fn create_session_in(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    node_id: &str,
    agent_id: &str,
    actor_id: &str,
    request_id: &str,
) -> Result<TerminalSessionRecord, CreateSessionError> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE terminal_sessions SET status='interrupted',finished_at=?,exit_reason='attach_timeout',updated_at=?,version=version+1 WHERE node_id=? AND status='opening' AND opened_at IS NULL AND datetime(started_at)<=datetime('now',?)")
        .bind(&now)
        .bind(&now)
        .bind(node_id)
        .bind(format!("-{UNATTACHED_OPENING_LEASE_SECONDS} seconds"))
        .execute(&mut **transaction)
        .await
        .map_err(CreateSessionError::Database)?;
    let result = sqlx::query("INSERT INTO terminal_sessions(id,node_id,agent_id,actor_id,request_id,status,started_at,created_at,updated_at) SELECT ?,n.id,a.id,?,?,'opening',?,?,? FROM nodes n JOIN agents a ON a.node_id=n.id WHERE n.id=? AND a.id=? AND n.status='online' AND n.archived_at IS NULL AND a.revoked_at IS NULL AND a.archived_at IS NULL AND a.protocol_version>=? AND a.protocol_version<=? AND EXISTS(SELECT 1 FROM json_each(a.capabilities_json) WHERE value='pty_terminal') AND EXISTS(SELECT 1 FROM json_each(a.capabilities_json) WHERE value='privileged_release')")
        .bind(id)
        .bind(actor_id)
        .bind(request_id)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .bind(node_id)
        .bind(agent_id)
        .bind(i64::from(MIN_SUPPORTED_PROTOCOL_VERSION))
        .bind(i64::from(PROTOCOL_VERSION))
        .execute(&mut **transaction)
        .await
        .map_err(map_create_error)?;
    if result.rows_affected() != 1 {
        return Err(CreateSessionError::GateRejected);
    }
    find_session_in(transaction, id)
        .await
        .map_err(CreateSessionError::Database)?
        .ok_or_else(|| CreateSessionError::Database(sqlx::Error::RowNotFound))
}

async fn find_session_in(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> sqlx::Result<Option<TerminalSessionRecord>> {
    sqlx::query_as("SELECT id,node_id,agent_id,actor_id,request_id,status,started_at,opened_at,close_requested_at,finished_at,exit_reason,exit_code,input_bytes,output_bytes,created_at,updated_at,version FROM terminal_sessions WHERE id=?")
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
}

fn map_create_error(error: sqlx::Error) -> CreateSessionError {
    if let sqlx::Error::Database(database) = &error
        && (database.constraint() == Some("terminal_sessions_one_active_per_node")
            || database.message().contains("terminal_sessions.node_id"))
    {
        return CreateSessionError::ActiveSessionConflict;
    }
    CreateSessionError::Database(error)
}

pub async fn find_session(
    pool: &SqlitePool,
    id: &str,
) -> sqlx::Result<Option<TerminalSessionRecord>> {
    sqlx::query_as("SELECT id,node_id,agent_id,actor_id,request_id,status,started_at,opened_at,close_requested_at,finished_at,exit_reason,exit_code,input_bytes,output_bytes,created_at,updated_at,version FROM terminal_sessions WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn close_session(
    pool: &SqlitePool,
    id: &str,
    reason: &str,
) -> sqlx::Result<Option<TerminalSessionRecord>> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(&format!("UPDATE terminal_sessions SET status='closed',close_requested_at=COALESCE(close_requested_at,?),finished_at=?,exit_reason=COALESCE(exit_reason,?),updated_at=?,version=version+1 WHERE id=? AND status IN ({ACTIVE_STATUSES})"))
        .bind(&now)
        .bind(&now)
        .bind(reason)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    find_session(pool, id).await
}

pub async fn request_close(
    pool: &SqlitePool,
    id: &str,
    reason: &str,
) -> sqlx::Result<Option<TerminalSessionRecord>> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE terminal_sessions SET status='closing',close_requested_at=COALESCE(close_requested_at,?),exit_reason=COALESCE(exit_reason,?),updated_at=?,version=version+1 WHERE id=? AND status IN ('opening','active')")
        .bind(&now)
        .bind(reason)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    find_session(pool, id).await
}

pub async fn mark_opened(pool: &SqlitePool, id: &str) -> sqlx::Result<bool> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query("UPDATE terminal_sessions SET status='active',opened_at=COALESCE(opened_at,?),updated_at=?,version=version+1 WHERE id=? AND status='opening'")
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn add_input_bytes(
    pool: &SqlitePool,
    id: &str,
    bytes: i64,
    maximum: i64,
) -> sqlx::Result<bool> {
    let result = sqlx::query(&format!("UPDATE terminal_sessions SET input_bytes=input_bytes+?,updated_at=?,version=version+1 WHERE id=? AND status IN ({ACTIVE_STATUSES}) AND input_bytes+?<=?"))
        .bind(bytes)
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .bind(bytes)
        .bind(maximum)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn add_output_bytes(
    pool: &SqlitePool,
    id: &str,
    bytes: i64,
    maximum: i64,
) -> sqlx::Result<bool> {
    let result = sqlx::query(&format!("UPDATE terminal_sessions SET output_bytes=output_bytes+?,updated_at=?,version=version+1 WHERE id=? AND status IN ({ACTIVE_STATUSES}) AND output_bytes+?<=?"))
        .bind(bytes)
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .bind(bytes)
        .bind(maximum)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn finish_session(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    reason: &str,
    exit_code: Option<i32>,
) -> sqlx::Result<Option<TerminalSessionRecord>> {
    debug_assert!(matches!(status, "closed" | "failed" | "interrupted"));
    let now = Utc::now().to_rfc3339();
    sqlx::query(&format!("UPDATE terminal_sessions SET status=?,finished_at=?,exit_reason=?,exit_code=?,updated_at=?,version=version+1 WHERE id=? AND status IN ({ACTIVE_STATUSES})"))
        .bind(status)
        .bind(&now)
        .bind(reason)
        .bind(exit_code)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    find_session(pool, id).await
}

pub async fn interrupt_active_sessions(pool: &SqlitePool) -> sqlx::Result<u64> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(&format!("UPDATE terminal_sessions SET status='interrupted',finished_at=?,exit_reason='api_restarted',updated_at=?,version=version+1 WHERE status IN ({ACTIVE_STATUSES})"))
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn active_sessions(pool: &SqlitePool) -> sqlx::Result<Vec<TerminalSessionRecord>> {
    sqlx::query_as(&format!("SELECT id,node_id,agent_id,actor_id,request_id,status,started_at,opened_at,close_requested_at,finished_at,exit_reason,exit_code,input_bytes,output_bytes,created_at,updated_at,version FROM terminal_sessions WHERE status IN ({ACTIVE_STATUSES}) ORDER BY created_at,id"))
        .fetch_all(pool)
        .await
}

pub async fn close_sessions_for_agent(
    pool: &SqlitePool,
    agent_id: &str,
    reason: &str,
) -> sqlx::Result<u64> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let affected = close_sessions_for_agent_in(&mut transaction, agent_id, reason).await?;
    transaction.commit().await?;
    Ok(affected)
}

pub async fn close_sessions_for_agent_in(
    transaction: &mut Transaction<'_, Sqlite>,
    agent_id: &str,
    reason: &str,
) -> sqlx::Result<u64> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(&format!("UPDATE terminal_sessions SET status='closed',close_requested_at=COALESCE(close_requested_at,?),finished_at=?,exit_reason=COALESCE(exit_reason,?),updated_at=?,version=version+1 WHERE agent_id=? AND status IN ({ACTIVE_STATUSES})"))
        .bind(&now)
        .bind(&now)
        .bind(reason)
        .bind(&now)
        .bind(agent_id)
        .execute(&mut **transaction)
        .await?;
    Ok(result.rows_affected())
}
