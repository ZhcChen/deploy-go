use crate::{
    AppState, RequestId,
    agents::auth::authenticate_access,
    error::{ApiError, ApiResult},
    runtime_logs::Storage,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Extension, State},
    http::HeaderMap,
    routing::post,
};
use deploy_go_runtime_log::{
    COMPONENTS, LogRecord, MAX_BATCH_BYTES, MAX_BATCH_RECORDS, MAX_RECORD_BYTES, allowed_field,
    identifier,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLogBatch {
    pub component: String,
    pub epoch: String,
    pub after: u64,
    pub minimum: u64,
    pub evicted_bytes: u64,
    #[schema(value_type = Vec<RuntimeLogEntrySchema>)]
    pub entries: Vec<LogRecord>,
}

#[derive(ToSchema)]
pub struct RuntimeLogEntrySchema {
    pub sequence: u64,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub request_id: Option<String>,
    pub fields: BTreeMap<String, Value>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RuntimeLogBatchResponse {
    pub epoch: String,
    pub acknowledged_sequence: u64,
}

#[derive(sqlx::FromRow)]
struct Watermark {
    epoch: String,
    sequence: i64,
    source_digest: String,
}

fn conflict() -> ApiError {
    ApiError::conflict(
        "runtime_log_source_conflict",
        "运行日志来源 epoch 或序号冲突",
        "runtime-log-ingest",
    )
}
fn unavailable() -> ApiError {
    ApiError::service_not_ready("runtime-log-ingest")
}
fn digest(record: &LogRecord) -> ApiResult<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(record).map_err(|_| unavailable())?)
    ))
}
fn valid_epoch(epoch: &str) -> bool {
    epoch
        .parse::<ulid::Ulid>()
        .is_ok_and(|value| value.to_string() == epoch)
}

fn safe_message(entry: &LogRecord, fields: &BTreeMap<String, Value>) -> String {
    if let Some(event) = fields.get("diagnostic_event").and_then(Value::as_str) {
        return event.to_owned();
    }
    // 只保留受管组件的编译事件位置，不接收任意 message/error/URL 正文。
    if let Some(site) = entry.message.strip_prefix("event ")
        && let Some((file, line)) = site.rsplit_once(':')
        && ["agent/src/", "agent-executor/src/", "agent-updater/src/"]
            .iter()
            .any(|prefix| file.starts_with(prefix))
        && file.ends_with(".rs")
        && line.parse::<u32>().is_ok()
        && entry.message.len() <= 256
        && file
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || matches!(v, b'/' | b'_' | b'-' | b'.'))
    {
        return entry.message.clone();
    }
    "node_runtime_diagnostic".into()
}
fn validate(batch: &RuntimeLogBatch) -> ApiResult<()> {
    let invalid = || ApiError::validation("运行日志批次无效或超出上限", "runtime-log-ingest");
    if !COMPONENTS.contains(&batch.component.as_str())
        || !valid_epoch(&batch.epoch)
        || batch.after > i64::MAX as u64
        || batch.minimum == 0
        || batch.minimum > i64::MAX as u64
        || batch.entries.len() > MAX_BATCH_RECORDS
    {
        return Err(invalid());
    }
    let mut bytes = 0;
    for (index, entry) in batch.entries.iter().enumerate() {
        let size = serde_json::to_vec(entry).map_err(|_| invalid())?.len() + 1;
        bytes += size;
        if size > MAX_RECORD_BYTES
            || entry.sequence == 0
            || entry.sequence > i64::MAX as u64
            || entry.sequence < batch.minimum
            || chrono::DateTime::parse_from_rfc3339(&entry.timestamp).is_err()
            || entry.timestamp.len() > 64
            || !matches!(
                entry.level.as_str(),
                "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"
            )
            || (index > 0 && entry.sequence <= batch.entries[index - 1].sequence)
        {
            return Err(invalid());
        }
    }
    if bytes > MAX_BATCH_BYTES {
        return Err(invalid());
    }
    if let Some(first) = batch.entries.first()
        && first.sequence <= batch.after
    {
        return Err(invalid());
    }
    Ok(())
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agent/runtime-logs", post(ingest))
        .layer(DefaultBodyLimit::max(MAX_BATCH_BYTES))
}

#[utoipa::path(operation_id = "agent_runtime_logs_ingest", post, path = "/api/v1/agent/runtime-logs", request_body = RuntimeLogBatch, responses((status = 200, body = RuntimeLogBatchResponse), (status = 401, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse), (status = 503, body = crate::error::ErrorResponse)))]
pub(crate) async fn ingest(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    crate::http::ApiJson(batch): crate::http::ApiJson<RuntimeLogBatch>,
) -> ApiResult<Json<RuntimeLogBatchResponse>> {
    let identity = authenticate_access(state.pool(), &headers, request_id.as_str()).await?;
    validate(&batch)?;
    let node_id: String = sqlx::query_scalar("SELECT node_id FROM agents WHERE id=?")
        .bind(&identity.agent_id)
        .fetch_one(state.pool())
        .await
        .map_err(|_| unavailable())?;
    let store = state.runtime_logs();
    let mut storage = store.storage.lock().await;
    if storage.durable.is_none() {
        return Err(unavailable());
    }
    recover_locked(&mut storage, state.pool()).await?;
    let current: Option<Watermark> = sqlx::query_as("SELECT epoch,sequence,source_digest FROM runtime_log_source_watermarks WHERE agent_id=? AND component=?")
        .bind(&identity.agent_id).bind(&batch.component).fetch_optional(state.pool()).await.map_err(|_| unavailable())?;
    let (mut acknowledged, last_digest) = match current {
        Some(current) if current.epoch == batch.epoch => {
            (current.sequence as u64, current.source_digest)
        }
        Some(current) if batch.epoch > current.epoch && batch.after == 0 => (0, String::new()),
        None if batch.after == 0 => (0, String::new()),
        _ => return Err(conflict()),
    };
    if batch.after > acknowledged {
        return Err(conflict());
    }
    // 在任何写入之前验证最新序号的重传摘要，冲突批次不产生副作用。
    for entry in &batch.entries {
        if entry.sequence == acknowledged && acknowledged > 0 && digest(entry)? != last_digest {
            return Err(conflict());
        }
    }
    if batch.entries.is_empty() {
        return Ok(Json(RuntimeLogBatchResponse {
            epoch: batch.epoch,
            acknowledged_sequence: acknowledged,
        }));
    }
    for entry in &batch.entries {
        if entry.sequence <= acknowledged {
            continue;
        }
        let source_digest = digest(entry)?;
        let mut fields: BTreeMap<String, Value> = entry
            .fields
            .iter()
            .filter(|(name, value)| {
                allowed_field(name)
                    && (value.is_boolean()
                        || value.is_i64()
                        || value.is_u64()
                        || value.as_str().is_some_and(identifier))
            })
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        fields.insert("agent_id".into(), identity.agent_id.clone().into());
        fields.insert("node_id".into(), node_id.clone().into());
        fields.insert("component".into(), batch.component.clone().into());
        fields.insert("source_epoch".into(), batch.epoch.clone().into());
        fields.insert("source_sequence".into(), entry.sequence.into());
        fields.insert("source_digest".into(), source_digest.clone().into());
        fields.insert("source_evicted_bytes".into(), batch.evicted_bytes.into());
        fields.insert("source_minimum".into(), batch.minimum.into());
        if entry.sequence > acknowledged + 1 {
            fields.insert("source_gap_from".into(), (acknowledged + 1).into());
            fields.insert("source_gap_to".into(), (entry.sequence - 1).into());
            fields.insert(
                "source_gap_count".into(),
                (entry.sequence - acknowledged - 1).into(),
            );
        }
        let record =
            LogRecord {
                sequence: 0,
                timestamp: entry.timestamp.clone(),
                level: entry.level.clone(),
                target: if entry.target.len() <= 128
                    && entry.target.bytes().all(|v| {
                        v.is_ascii_alphanumeric() || matches!(v, b'_' | b':' | b'-' | b'.')
                    }) {
                    entry.target.clone()
                } else {
                    "node_runtime".into()
                },
                message: safe_message(entry, &fields),
                request_id: entry
                    .request_id
                    .as_deref()
                    .filter(|v| identifier(v))
                    .map(str::to_owned),
                fields,
            };
        // 跨文件/数据库的窗口在同一锁内；取消或 DB 失败时禁止其他 writer 轮转，先恢复。
        storage.needs_recovery = true;
        let persisted = store
            .append_locked(&mut storage, record)
            .await
            .map_err(|_| unavailable())?;
        save_watermark(
            state.pool(),
            &identity.agent_id,
            &batch.component,
            &batch.epoch,
            entry.sequence,
            &source_digest,
            persisted.sequence,
        )
        .await?;
        storage.needs_recovery = false;
        acknowledged = entry.sequence;
    }
    Ok(Json(RuntimeLogBatchResponse {
        epoch: batch.epoch,
        acknowledged_sequence: acknowledged,
    }))
}

async fn save_watermark(
    pool: &sqlx::SqlitePool,
    agent: &str,
    component: &str,
    epoch: &str,
    sequence: u64,
    digest: &str,
    central_sequence: u64,
) -> ApiResult<()> {
    sqlx::query(WATERMARK_UPSERT)
        .bind(agent)
        .bind(component)
        .bind(epoch)
        .bind(sequence as i64)
        .bind(digest)
        .bind(central_sequence as i64)
        .execute(pool)
        .await
        .map_err(|_| unavailable())?;
    Ok(())
}

const WATERMARK_UPSERT: &str = "INSERT INTO runtime_log_source_watermarks(agent_id,component,epoch,sequence,source_digest,central_sequence) VALUES(?,?,?,?,?,?) ON CONFLICT(agent_id,component) DO UPDATE SET epoch=excluded.epoch,sequence=excluded.sequence,source_digest=excluded.source_digest,central_sequence=excluded.central_sequence WHERE excluded.epoch>runtime_log_source_watermarks.epoch OR (excluded.epoch=runtime_log_source_watermarks.epoch AND excluded.sequence>runtime_log_source_watermarks.sequence)";

struct RecoveryWatermark {
    epoch: String,
    sequence: u64,
    digest: String,
    central_sequence: u64,
}
type RecoverySources = BTreeMap<(String, String), RecoveryWatermark>;
const RECOVERY_SOURCES: usize = 4096;

async fn flush_recovery(sources: &mut RecoverySources, pool: &sqlx::SqlitePool) -> ApiResult<()> {
    if sources.is_empty() {
        return Ok(());
    }
    let mut transaction = pool.begin().await.map_err(|_| unavailable())?;
    for ((agent, component), value) in sources.iter() {
        sqlx::query(WATERMARK_UPSERT)
            .bind(agent)
            .bind(component)
            .bind(&value.epoch)
            .bind(value.sequence as i64)
            .bind(&value.digest)
            .bind(value.central_sequence as i64)
            .execute(&mut *transaction)
            .await
            .map_err(|_| unavailable())?;
    }
    transaction.commit().await.map_err(|_| unavailable())?;
    sources.clear();
    Ok(())
}

pub(crate) async fn recover_locked(
    storage: &mut Storage,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    if !storage.needs_recovery {
        return Ok(());
    }
    let Some(durable) = &storage.durable else {
        storage.needs_recovery = false;
        return Ok(());
    };
    let mut cursor = 0;
    let mut sources = RecoverySources::new();
    loop {
        let page = durable
            .page(cursor, 500, 8 * 1024 * 1024)
            .await
            .map_err(|_| unavailable())?;
        if page.entries.is_empty() {
            break;
        }
        for record in page.entries {
            cursor = record.sequence;
            let fields = &record.fields;
            if let (Some(agent), Some(component), Some(epoch), Some(sequence), Some(digest)) = (
                fields.get("agent_id").and_then(Value::as_str),
                fields.get("component").and_then(Value::as_str),
                fields.get("source_epoch").and_then(Value::as_str),
                fields.get("source_sequence").and_then(Value::as_u64),
                fields.get("source_digest").and_then(Value::as_str),
            ) {
                if !COMPONENTS.contains(&component)
                    || !valid_epoch(epoch)
                    || sequence > i64::MAX as u64
                {
                    return Err(unavailable());
                }
                let key = (agent.to_owned(), component.to_owned());
                if !sources.contains_key(&key) && sources.len() >= RECOVERY_SOURCES {
                    flush_recovery(&mut sources, pool).await?;
                }
                if sources.get(&key).is_none_or(|previous| {
                    epoch > previous.epoch.as_str()
                        || (epoch == previous.epoch && sequence > previous.sequence)
                }) {
                    sources.insert(
                        key,
                        RecoveryWatermark {
                            epoch: epoch.to_owned(),
                            sequence,
                            digest: digest.to_owned(),
                            central_sequence: record.sequence,
                        },
                    );
                }
            }
        }
    }
    flush_recovery(&mut sources, pool).await?;
    storage.needs_recovery = false;
    Ok(())
}
