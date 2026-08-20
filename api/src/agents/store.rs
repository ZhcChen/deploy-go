use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};
use ulid::Ulid;

#[derive(Clone, Debug, FromRow, PartialEq)]
pub struct AgentRecord {
    pub id: String,
    pub node_id: String,
    pub name: String,
    pub environment: String,
    pub registered_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub revoked_at: Option<String>,
    pub archived_at: Option<String>,
    pub agent_version: Option<String>,
    pub protocol_version: Option<i64>,
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub architecture: Option<String>,
    pub capabilities_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Debug)]
pub enum CreateAgentError {
    InvalidName,
    NameConflict,
    NodeNotFound,
    NodeAlreadyBound,
    Database(sqlx::Error),
}

pub async fn create_with_node(
    pool: &SqlitePool,
    name: &str,
    environment: &str,
) -> Result<AgentRecord, CreateAgentError> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(CreateAgentError::Database)?;
    let (agent_id, _) = create_with_node_in(&mut transaction, name, environment).await?;

    transaction
        .commit()
        .await
        .map_err(CreateAgentError::Database)?;
    find(pool, &agent_id)
        .await
        .map_err(CreateAgentError::Database)?
        .ok_or(CreateAgentError::NodeNotFound)
}

pub async fn create_with_node_in(
    transaction: &mut Transaction<'_, Sqlite>,
    name: &str,
    environment: &str,
) -> Result<(String, String), CreateAgentError> {
    let name = validate_name(name)?;
    let node_id = format!("node_{}", Ulid::new());
    let agent_id = format!("agent_{}", Ulid::new());
    sqlx::query("INSERT INTO nodes (id, name, work_root, secrets_root, status) VALUES (?, ?, '/var/lib/deploy-go-agent/apps', '/var/lib/deploy-go-agent/secrets', 'offline')")
        .bind(&node_id)
        .bind(name)
        .execute(&mut **transaction)
        .await
        .map_err(map_create_error)?;
    sqlx::query("INSERT INTO agents (id, node_id, environment) VALUES (?, ?, ?)")
        .bind(&agent_id)
        .bind(&node_id)
        .bind(environment)
        .execute(&mut **transaction)
        .await
        .map_err(map_create_error)?;
    Ok((agent_id, node_id))
}

pub async fn bind_existing_node(
    pool: &SqlitePool,
    node_id: &str,
    environment: &str,
) -> Result<AgentRecord, CreateAgentError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM nodes WHERE id=?)")
        .bind(node_id)
        .fetch_one(pool)
        .await
        .map_err(CreateAgentError::Database)?;
    if !exists {
        return Err(CreateAgentError::NodeNotFound);
    }
    let agent_id = format!("agent_{}", Ulid::new());
    sqlx::query("INSERT INTO agents (id, node_id, environment) VALUES (?, ?, ?)")
        .bind(&agent_id)
        .bind(node_id)
        .bind(environment)
        .execute(pool)
        .await
        .map_err(map_create_error)?;
    find(pool, &agent_id)
        .await
        .map_err(CreateAgentError::Database)?
        .ok_or(CreateAgentError::NodeNotFound)
}

pub async fn bind_existing_node_in(
    transaction: &mut Transaction<'_, Sqlite>,
    node_id: &str,
    environment: &str,
) -> Result<(String, String), CreateAgentError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM nodes WHERE id=?)")
        .bind(node_id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(CreateAgentError::Database)?;
    if !exists {
        return Err(CreateAgentError::NodeNotFound);
    }
    let agent_id = format!("agent_{}", Ulid::new());
    sqlx::query("INSERT INTO agents (id, node_id, environment) VALUES (?, ?, ?)")
        .bind(&agent_id)
        .bind(node_id)
        .bind(environment)
        .execute(&mut **transaction)
        .await
        .map_err(map_create_error)?;
    Ok((agent_id, node_id.to_owned()))
}

pub async fn find(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<AgentRecord>> {
    sqlx::query_as("SELECT a.id,a.node_id,n.name,a.environment,a.registered_at,a.last_seen_at,a.revoked_at,a.archived_at,a.agent_version,a.protocol_version,a.hostname,a.os_name,a.architecture,a.capabilities_json,a.created_at,a.updated_at,a.version FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.id=?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

fn validate_name(name: &str) -> Result<&str, CreateAgentError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
        return Err(CreateAgentError::InvalidName);
    }
    Ok(name)
}

pub(crate) fn map_create_error(error: sqlx::Error) -> CreateAgentError {
    let message = error.to_string();
    if message.contains("nodes.name") {
        CreateAgentError::NameConflict
    } else if message.contains("agents.node_id") {
        CreateAgentError::NodeAlreadyBound
    } else {
        CreateAgentError::Database(error)
    }
}
