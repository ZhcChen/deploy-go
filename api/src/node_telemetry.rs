use std::{collections::BTreeMap, sync::Mutex, time::Instant};

use chrono::{DateTime, Duration, SecondsFormat, Timelike, Utc};
use deploy_go_agent_protocol::{NodeTelemetry, TelemetryMetricStatus};
use serde::Serialize;
use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};
use tokio::sync::{Semaphore, SemaphorePermit};
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};

const CLOCK_SKEW_SECONDS: i64 = 300;
const NODE_HISTORY_LIMIT: i64 = 3_600;
const GLOBAL_HISTORY_LIMIT: i64 = 360_000;
const HISTORY_HOURS: i64 = 24;
const GLOBAL_RATE_PER_SECOND: f64 = 20.0;
const GLOBAL_BURST: f64 = 100.0;
const DIAGNOSTIC_RATE_PER_SECOND: f64 = 1.0 / 60.0;
const DIAGNOSTIC_BURST: f64 = 10.0;
const STORE_CONCURRENCY_LIMIT: usize = 4;
const RETENTION_BATCH_SIZE: i64 = 20_000;
const RETENTION_MAX_BATCHES: usize = 10;

#[derive(Debug)]
pub enum StoreOutcome {
    Stored,
    CurrentOnly,
    Dropped,
}

pub struct TelemetryBudget {
    state: Mutex<BudgetState>,
    store_permits: Semaphore,
}

struct BudgetState {
    tokens: f64,
    updated_at: Instant,
    diagnostic_tokens: f64,
    diagnostic_updated_at: Instant,
}

impl Default for TelemetryBudget {
    fn default() -> Self {
        Self {
            state: Mutex::new(BudgetState {
                tokens: GLOBAL_BURST,
                updated_at: Instant::now(),
                diagnostic_tokens: DIAGNOSTIC_BURST,
                diagnostic_updated_at: Instant::now(),
            }),
            store_permits: Semaphore::new(STORE_CONCURRENCY_LIMIT),
        }
    }
}

impl TelemetryBudget {
    pub fn try_acquire(&self) -> bool {
        let mut state = self.state.lock().expect("遥测预算锁未中毒");
        let now = Instant::now();
        state.tokens = (state.tokens
            + now.duration_since(state.updated_at).as_secs_f64() * GLOBAL_RATE_PER_SECOND)
            .min(GLOBAL_BURST);
        state.updated_at = now;
        if state.tokens < 1.0 {
            return false;
        }
        state.tokens -= 1.0;
        true
    }

    pub fn try_acquire_diagnostic(&self) -> bool {
        let mut state = self.state.lock().expect("遥测预算锁未中毒");
        let now = Instant::now();
        state.diagnostic_tokens = (state.diagnostic_tokens
            + now
                .duration_since(state.diagnostic_updated_at)
                .as_secs_f64()
                * DIAGNOSTIC_RATE_PER_SECOND)
            .min(DIAGNOSTIC_BURST);
        state.diagnostic_updated_at = now;
        if state.diagnostic_tokens < 1.0 {
            return false;
        }
        state.diagnostic_tokens -= 1.0;
        true
    }

    pub fn try_acquire_store(&self) -> Option<SemaphorePermit<'_>> {
        self.store_permits.try_acquire().ok()
    }
}

pub async fn store(
    pool: &SqlitePool,
    agent_id: &str,
    generation: i64,
    sample: &NodeTelemetry,
) -> Result<StoreOutcome, sqlx::Error> {
    if sample.validate().is_err()
        || sample.connection_generation != generation as u64
        || i64::try_from(sample.sample_sequence).is_err()
        || sample
            .snapshot
            .memory
            .total_bytes
            .is_some_and(|v| i64::try_from(v).is_err())
        || sample
            .snapshot
            .memory
            .used_bytes
            .is_some_and(|v| i64::try_from(v).is_err())
        || sample
            .snapshot
            .work_root_disk
            .total_bytes
            .is_some_and(|v| i64::try_from(v).is_err())
        || sample
            .snapshot
            .work_root_disk
            .used_bytes
            .is_some_and(|v| i64::try_from(v).is_err())
    {
        return Ok(StoreOutcome::Dropped);
    }
    let received = Utc::now();
    let Ok(captured) = DateTime::parse_from_rfc3339(&sample.captured_at) else {
        return Ok(StoreOutcome::Dropped);
    };
    if (captured.with_timezone(&Utc) - received)
        .num_seconds()
        .abs()
        > CLOCK_SKEW_SECONDS
    {
        return Ok(StoreOutcome::Dropped);
    }
    let gpus_json = serde_json::to_string(&sample.snapshot.gpus).unwrap_or_default();
    if gpus_json.len() > 4096 {
        return Ok(StoreOutcome::Dropped);
    }
    let received_at = received.to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut tx = pool.begin().await?;
    let node_id: Option<String> = sqlx::query_scalar(
        "SELECT node_id FROM agents WHERE id=? AND connection_generation=? AND revoked_at IS NULL AND archived_at IS NULL",
    )
    .bind(agent_id)
    .bind(generation)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(node_id) = node_id else {
        return Ok(StoreOutcome::Dropped);
    };
    let previous: Option<i64> = sqlx::query_scalar(
        "SELECT sample_sequence FROM node_telemetry_current WHERE node_id=? AND agent_id=? AND connection_generation=?",
    )
    .bind(&node_id)
    .bind(agent_id)
    .bind(generation)
    .fetch_optional(&mut *tx)
    .await?;
    if previous.is_some_and(|sequence| sequence >= sample.sample_sequence as i64) {
        return Ok(StoreOutcome::Dropped);
    }
    upsert_current(
        &mut tx,
        &node_id,
        agent_id,
        generation,
        sample,
        &received_at,
        &gpus_json,
    )
    .await?;
    let node_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM node_telemetry_history WHERE node_id=?")
            .bind(&node_id)
            .fetch_one(&mut *tx)
            .await?;
    let global_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM node_telemetry_history")
        .fetch_one(&mut *tx)
        .await?;
    if node_count >= NODE_HISTORY_LIMIT || global_count >= GLOBAL_HISTORY_LIMIT {
        tx.commit().await?;
        return Ok(StoreOutcome::CurrentOnly);
    }
    insert_history(
        &mut tx,
        &node_id,
        agent_id,
        generation,
        sample,
        &received_at,
        &gpus_json,
    )
    .await?;
    tx.commit().await?;
    Ok(StoreOutcome::Stored)
}

async fn upsert_current(
    tx: &mut Transaction<'_, Sqlite>,
    node_id: &str,
    agent_id: &str,
    generation: i64,
    sample: &NodeTelemetry,
    received_at: &str,
    gpus_json: &str,
) -> Result<(), sqlx::Error> {
    let s = &sample.snapshot;
    sqlx::query("INSERT INTO node_telemetry_current (node_id,agent_id,connection_generation,sample_sequence,captured_at,received_at,cpu_status,cpu_usage_percent,memory_status,memory_total_bytes,memory_used_bytes,memory_usage_percent,work_root_status,work_root_total_bytes,work_root_used_bytes,work_root_usage_percent,disk_io_status,disk_read_bytes_per_second,disk_write_bytes_per_second,disk_busy_percent,network_status,network_receive_bytes_per_second,network_transmit_bytes_per_second,gpu_status,gpu_reason,gpus_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(node_id) DO UPDATE SET agent_id=excluded.agent_id,connection_generation=excluded.connection_generation,sample_sequence=excluded.sample_sequence,captured_at=excluded.captured_at,received_at=excluded.received_at,cpu_status=excluded.cpu_status,cpu_usage_percent=excluded.cpu_usage_percent,memory_status=excluded.memory_status,memory_total_bytes=excluded.memory_total_bytes,memory_used_bytes=excluded.memory_used_bytes,memory_usage_percent=excluded.memory_usage_percent,work_root_status=excluded.work_root_status,work_root_total_bytes=excluded.work_root_total_bytes,work_root_used_bytes=excluded.work_root_used_bytes,work_root_usage_percent=excluded.work_root_usage_percent,disk_io_status=excluded.disk_io_status,disk_read_bytes_per_second=excluded.disk_read_bytes_per_second,disk_write_bytes_per_second=excluded.disk_write_bytes_per_second,disk_busy_percent=excluded.disk_busy_percent,network_status=excluded.network_status,network_receive_bytes_per_second=excluded.network_receive_bytes_per_second,network_transmit_bytes_per_second=excluded.network_transmit_bytes_per_second,gpu_status=excluded.gpu_status,gpu_reason=excluded.gpu_reason,gpus_json=excluded.gpus_json")
        .bind(node_id).bind(agent_id).bind(generation).bind(sample.sample_sequence as i64)
        .bind(&sample.captured_at).bind(received_at)
        .bind(status(s.cpu.status)).bind(s.cpu.usage_percent)
        .bind(status(s.memory.status)).bind(to_i64(s.memory.total_bytes)).bind(to_i64(s.memory.used_bytes)).bind(s.memory.usage_percent)
        .bind(status(s.work_root_disk.status)).bind(to_i64(s.work_root_disk.total_bytes)).bind(to_i64(s.work_root_disk.used_bytes)).bind(s.work_root_disk.usage_percent)
        .bind(status(s.disk_io.status)).bind(s.disk_io.read_bytes_per_second).bind(s.disk_io.write_bytes_per_second).bind(s.disk_io.busy_percent)
        .bind(status(s.network.status)).bind(s.network.receive_bytes_per_second).bind(s.network.transmit_bytes_per_second)
        .bind(status(s.gpu_status)).bind(s.gpu_reason.map(reason)).bind(gpus_json).execute(&mut **tx).await?;
    Ok(())
}

async fn insert_history(
    tx: &mut Transaction<'_, Sqlite>,
    node_id: &str,
    agent_id: &str,
    generation: i64,
    sample: &NodeTelemetry,
    received_at: &str,
    gpus_json: &str,
) -> Result<(), sqlx::Error> {
    let s = &sample.snapshot;
    sqlx::query("INSERT OR IGNORE INTO node_telemetry_history (node_id,agent_id,connection_generation,sample_sequence,captured_at,received_at,cpu_status,cpu_usage_percent,memory_status,memory_total_bytes,memory_used_bytes,work_root_status,work_root_total_bytes,work_root_used_bytes,disk_io_status,disk_read_bytes_per_second,disk_write_bytes_per_second,disk_busy_percent,network_status,network_receive_bytes_per_second,network_transmit_bytes_per_second,gpu_status,gpu_reason,gpus_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(node_id).bind(agent_id).bind(generation).bind(sample.sample_sequence as i64).bind(&sample.captured_at).bind(received_at)
        .bind(status(s.cpu.status)).bind(s.cpu.usage_percent)
        .bind(status(s.memory.status)).bind(to_i64(s.memory.total_bytes)).bind(to_i64(s.memory.used_bytes))
        .bind(status(s.work_root_disk.status)).bind(to_i64(s.work_root_disk.total_bytes)).bind(to_i64(s.work_root_disk.used_bytes))
        .bind(status(s.disk_io.status)).bind(s.disk_io.read_bytes_per_second).bind(s.disk_io.write_bytes_per_second).bind(s.disk_io.busy_percent)
        .bind(status(s.network.status)).bind(s.network.receive_bytes_per_second).bind(s.network.transmit_bytes_per_second)
        .bind(status(s.gpu_status)).bind(s.gpu_reason.map(reason)).bind(gpus_json)
        .execute(&mut **tx).await?;
    Ok(())
}

fn to_i64(value: Option<u64>) -> Option<i64> {
    value.map(|value| i64::try_from(value).expect("样本已通过 SQLite 整数范围校验"))
}
fn status(value: TelemetryMetricStatus) -> &'static str {
    match value {
        TelemetryMetricStatus::Available => "available",
        TelemetryMetricStatus::WarmingUp => "warming_up",
        TelemetryMetricStatus::Unsupported => "unsupported",
        TelemetryMetricStatus::CollectionError => "collection_error",
    }
}
fn reason(value: deploy_go_agent_protocol::TelemetryMetricReason) -> &'static str {
    use deploy_go_agent_protocol::TelemetryMetricReason::*;
    match value {
        HardwareNotPresent => "hardware_not_present",
        UnsupportedPlatform => "unsupported_platform",
        BackendUnavailable => "backend_unavailable",
        PermissionDenied => "permission_denied",
        Timeout => "timeout",
        ParseError => "parse_error",
        SourceUnavailable => "source_unavailable",
    }
}

#[derive(Serialize, ToSchema)]
pub struct MetricValue {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

#[derive(Serialize, ToSchema)]
pub struct LatestTelemetry {
    pub cpu_usage_ratio: MetricValue,
    pub memory_total_bytes: MetricValue,
    pub memory_used_bytes: MetricValue,
    pub work_root_total_bytes: MetricValue,
    pub work_root_used_bytes: MetricValue,
    pub disk_read_bytes_per_second: MetricValue,
    pub disk_write_bytes_per_second: MetricValue,
    pub disk_busy_ratio: MetricValue,
    pub network_receive_bytes_per_second: MetricValue,
    pub network_transmit_bytes_per_second: MetricValue,
    pub gpu_status: String,
    pub gpu_reason: Option<String>,
    pub gpus: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
pub struct HistoryPoint {
    pub received_at: String,
    pub cpu_usage_ratio: Option<f64>,
    pub memory_used_bytes: Option<f64>,
    pub work_root_used_bytes: Option<f64>,
    pub disk_read_bytes_per_second: Option<f64>,
    pub disk_write_bytes_per_second: Option<f64>,
    pub disk_busy_ratio: Option<f64>,
    pub network_receive_bytes_per_second: Option<f64>,
    pub network_transmit_bytes_per_second: Option<f64>,
}

#[derive(Serialize, ToSchema)]
pub struct TelemetryResponse {
    pub node_id: String,
    pub connectivity: String,
    pub capability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_reason: Option<String>,
    pub freshness: String,
    pub captured_at: Option<String>,
    pub received_at: Option<String>,
    pub latest: Option<LatestTelemetry>,
    pub history: Vec<HistoryPoint>,
}

#[derive(FromRow)]
struct AgentState {
    status: String,
    agent_id: Option<String>,
    protocol_version: Option<i64>,
    connection_generation: Option<i64>,
    last_seen_at: Option<String>,
    revoked_at: Option<String>,
    archived_at: Option<String>,
}

#[derive(FromRow)]
struct CurrentRow {
    captured_at: String,
    received_at: String,
    cpu_status: String,
    cpu_usage_percent: Option<f64>,
    memory_status: String,
    memory_total_bytes: Option<i64>,
    memory_used_bytes: Option<i64>,
    work_root_status: String,
    work_root_total_bytes: Option<i64>,
    work_root_used_bytes: Option<i64>,
    disk_io_status: String,
    disk_read_bytes_per_second: Option<f64>,
    disk_write_bytes_per_second: Option<f64>,
    disk_busy_percent: Option<f64>,
    network_status: String,
    network_receive_bytes_per_second: Option<f64>,
    network_transmit_bytes_per_second: Option<f64>,
    gpu_status: String,
    gpu_reason: Option<String>,
    gpus_json: String,
}

#[derive(FromRow)]
struct HistoryRow {
    received_at: String,
    cpu_usage_percent: Option<f64>,
    memory_used_bytes: Option<i64>,
    work_root_used_bytes: Option<i64>,
    disk_read_bytes_per_second: Option<f64>,
    disk_write_bytes_per_second: Option<f64>,
    disk_busy_percent: Option<f64>,
    network_receive_bytes_per_second: Option<f64>,
    network_transmit_bytes_per_second: Option<f64>,
}

pub async fn query(
    pool: &SqlitePool,
    node_id: &str,
    request_id: &str,
) -> ApiResult<TelemetryResponse> {
    let state = sqlx::query_as::<_, AgentState>("SELECT n.status,a.id AS agent_id,a.protocol_version,a.connection_generation,a.last_seen_at,a.revoked_at,a.archived_at FROM nodes n LEFT JOIN agents a ON a.node_id=n.id WHERE n.id=?")
        .bind(node_id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))?;
    let connectivity = match state.status.as_str() {
        "online" => "online",
        "offline" => "offline",
        "disabled" => "disabled",
        _ => "unknown",
    }
    .to_owned();
    let (capability, reason) = capability(&state);
    if capability != "supported" {
        return Ok(empty(node_id, connectivity, capability, reason));
    }
    let agent_id = state
        .agent_id
        .as_deref()
        .expect("supported capability has agent");
    let generation = state.connection_generation.unwrap_or_default();
    let current = sqlx::query_as::<_, CurrentRow>("SELECT captured_at,received_at,cpu_status,cpu_usage_percent,memory_status,memory_total_bytes,memory_used_bytes,work_root_status,work_root_total_bytes,work_root_used_bytes,disk_io_status,disk_read_bytes_per_second,disk_write_bytes_per_second,disk_busy_percent,network_status,network_receive_bytes_per_second,network_transmit_bytes_per_second,gpu_status,gpu_reason,gpus_json FROM node_telemetry_current WHERE node_id=? AND agent_id=? AND connection_generation=?")
        .bind(node_id).bind(agent_id).bind(generation).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?;
    let Some(current) = current else {
        return Ok(empty(node_id, connectivity, capability, None));
    };
    let received = DateTime::parse_from_rfc3339(&current.received_at)
        .map_err(|_| ApiError::internal(request_id))?
        .with_timezone(&Utc);
    let freshness = if Utc::now() - received > Duration::seconds(90) {
        "stale"
    } else {
        "fresh"
    }
    .to_owned();
    let cutoff =
        (Utc::now() - Duration::hours(HISTORY_HOURS)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let rows = sqlx::query_as::<_, HistoryRow>("SELECT received_at,cpu_usage_percent,memory_used_bytes,work_root_used_bytes,disk_read_bytes_per_second,disk_write_bytes_per_second,disk_busy_percent,network_receive_bytes_per_second,network_transmit_bytes_per_second FROM node_telemetry_history WHERE node_id=? AND agent_id=? AND received_at>=? ORDER BY received_at")
        .bind(node_id).bind(agent_id).bind(cutoff).fetch_all(pool).await.map_err(|_| ApiError::internal(request_id))?;
    Ok(TelemetryResponse {
        node_id: node_id.to_owned(),
        connectivity,
        capability,
        capability_reason: None,
        freshness,
        captured_at: Some(current.captured_at.clone()),
        received_at: Some(current.received_at.clone()),
        latest: Some(latest(current)),
        history: aggregate(rows),
    })
}

fn capability(state: &AgentState) -> (String, Option<String>) {
    if state.agent_id.is_none() {
        return ("unavailable".into(), Some("no_agent".into()));
    }
    if state.revoked_at.is_some() {
        return ("unavailable".into(), Some("revoked".into()));
    }
    if state.archived_at.is_some() {
        return ("unavailable".into(), Some("archived".into()));
    }
    if state.last_seen_at.is_none() {
        return ("unavailable".into(), Some("not_connected".into()));
    }
    match state.protocol_version {
        Some(version) if version >= 12 => ("supported".into(), None),
        Some(_) => ("unsupported".into(), Some("protocol_v11".into())),
        None => ("unavailable".into(), Some("not_connected".into())),
    }
}

fn empty(
    node_id: &str,
    connectivity: String,
    capability: String,
    reason: Option<String>,
) -> TelemetryResponse {
    TelemetryResponse {
        node_id: node_id.to_owned(),
        connectivity,
        capability,
        capability_reason: reason,
        freshness: "empty".into(),
        captured_at: None,
        received_at: None,
        latest: None,
        history: vec![],
    }
}

fn metric(status: String, value: Option<f64>) -> MetricValue {
    let reason = match status.as_str() {
        "warming_up" => Some("warming_up".into()),
        "unsupported" => Some("unsupported".into()),
        "collection_error" => Some("source_unavailable".into()),
        _ => None,
    };
    MetricValue {
        status,
        reason,
        value,
    }
}
fn latest(row: CurrentRow) -> LatestTelemetry {
    LatestTelemetry {
        cpu_usage_ratio: metric(row.cpu_status, row.cpu_usage_percent.map(|v| v / 100.0)),
        memory_total_bytes: metric(
            row.memory_status.clone(),
            row.memory_total_bytes.map(|v| v as f64),
        ),
        memory_used_bytes: metric(row.memory_status, row.memory_used_bytes.map(|v| v as f64)),
        work_root_total_bytes: metric(
            row.work_root_status.clone(),
            row.work_root_total_bytes.map(|v| v as f64),
        ),
        work_root_used_bytes: metric(
            row.work_root_status,
            row.work_root_used_bytes.map(|v| v as f64),
        ),
        disk_read_bytes_per_second: metric(
            row.disk_io_status.clone(),
            row.disk_read_bytes_per_second,
        ),
        disk_write_bytes_per_second: metric(
            row.disk_io_status.clone(),
            row.disk_write_bytes_per_second,
        ),
        disk_busy_ratio: metric(row.disk_io_status, row.disk_busy_percent.map(|v| v / 100.0)),
        network_receive_bytes_per_second: metric(
            row.network_status.clone(),
            row.network_receive_bytes_per_second,
        ),
        network_transmit_bytes_per_second: metric(
            row.network_status,
            row.network_transmit_bytes_per_second,
        ),
        gpu_status: row.gpu_status,
        gpu_reason: row.gpu_reason,
        gpus: serde_json::from_str(&row.gpus_json).unwrap_or_else(|_| serde_json::json!([])),
    }
}

#[derive(Default)]
struct Aggregate {
    cpu: f64,
    cpu_n: f64,
    memory: f64,
    memory_n: f64,
    work: f64,
    work_n: f64,
    read: f64,
    read_n: f64,
    write: f64,
    write_n: f64,
    busy: f64,
    busy_n: f64,
    receive: f64,
    receive_n: f64,
    transmit: f64,
    transmit_n: f64,
}
fn add(sum: &mut f64, count: &mut f64, value: Option<f64>) {
    if let Some(value) = value {
        *sum += value;
        *count += 1.0;
    }
}
fn avg(sum: f64, count: f64) -> Option<f64> {
    (count > 0.0).then_some(sum / count)
}
fn aggregate(rows: Vec<HistoryRow>) -> Vec<HistoryPoint> {
    let mut buckets = BTreeMap::<String, Aggregate>::new();
    for row in rows {
        let Ok(at) = DateTime::parse_from_rfc3339(&row.received_at) else {
            continue;
        };
        let minute = at.minute() / 2 * 2;
        let key = at
            .with_minute(minute)
            .and_then(|v| v.with_second(0))
            .and_then(|v| v.with_nanosecond(0))
            .expect("valid bucket")
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        let a = buckets.entry(key).or_default();
        add(
            &mut a.cpu,
            &mut a.cpu_n,
            row.cpu_usage_percent.map(|v| v / 100.0),
        );
        add(
            &mut a.memory,
            &mut a.memory_n,
            row.memory_used_bytes.map(|v| v as f64),
        );
        add(
            &mut a.work,
            &mut a.work_n,
            row.work_root_used_bytes.map(|v| v as f64),
        );
        add(&mut a.read, &mut a.read_n, row.disk_read_bytes_per_second);
        add(
            &mut a.write,
            &mut a.write_n,
            row.disk_write_bytes_per_second,
        );
        add(
            &mut a.busy,
            &mut a.busy_n,
            row.disk_busy_percent.map(|v| v / 100.0),
        );
        add(
            &mut a.receive,
            &mut a.receive_n,
            row.network_receive_bytes_per_second,
        );
        add(
            &mut a.transmit,
            &mut a.transmit_n,
            row.network_transmit_bytes_per_second,
        );
    }
    let skip = buckets.len().saturating_sub(720);
    buckets
        .into_iter()
        .skip(skip)
        .map(|(received_at, a)| HistoryPoint {
            received_at,
            cpu_usage_ratio: avg(a.cpu, a.cpu_n),
            memory_used_bytes: avg(a.memory, a.memory_n),
            work_root_used_bytes: avg(a.work, a.work_n),
            disk_read_bytes_per_second: avg(a.read, a.read_n),
            disk_write_bytes_per_second: avg(a.write, a.write_n),
            disk_busy_ratio: avg(a.busy, a.busy_n),
            network_receive_bytes_per_second: avg(a.receive, a.receive_n),
            network_transmit_bytes_per_second: avg(a.transmit, a.transmit_n),
        })
        .collect()
}

pub async fn purge_expired(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let cutoff =
        (Utc::now() - Duration::hours(HISTORY_HOURS)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut deleted = 0;
    for _ in 0..RETENTION_MAX_BATCHES {
        let rows = sqlx::query("DELETE FROM node_telemetry_history WHERE id IN (SELECT id FROM node_telemetry_history WHERE received_at<? ORDER BY received_at LIMIT ?)")
            .bind(&cutoff).bind(RETENTION_BATCH_SIZE).execute(pool).await?.rows_affected();
        deleted += rows;
        if rows < RETENTION_BATCH_SIZE as u64 {
            break;
        }
        tokio::task::yield_now().await;
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_concurrency_is_bounded() {
        let budget = TelemetryBudget::default();
        let permits: Vec<_> = (0..STORE_CONCURRENCY_LIMIT)
            .map(|_| budget.try_acquire_store().unwrap())
            .collect();
        assert!(budget.try_acquire_store().is_none());
        drop(permits);
        assert!(budget.try_acquire_store().is_some());
    }
}
