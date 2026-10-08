use crate::{
    AppState, RequestId,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};
use axum::{
    Router,
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::sse::{Event as SseEvent, KeepAlive, Sse},
    routing::get,
};
use deploy_go_runtime_log::{
    LogPage, LogPolicy, LogRecord, MAX_RECORD_BYTES, SegmentStore, allowed_field, identifier,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    path::PathBuf,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
use tokio::sync::{Mutex, mpsc};
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context};
use utoipa::ToSchema;

const DEFAULT_CAPACITY: usize = 5_000;
const MEMORY_BYTES: usize = 8 * 1024 * 1024;
const QUEUE_BYTES: usize = 2 * 1024 * 1024;
const CHANNEL_CAPACITY: usize = 1_024;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RuntimeLogResponse {
    pub sequence: u64,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub request_id: Option<String>,
    pub fields: BTreeMap<String, Value>,
    pub node_id: Option<String>,
    pub component: Option<String>,
}
impl From<LogRecord> for RuntimeLogResponse {
    fn from(record: LogRecord) -> Self {
        Self {
            node_id: record
                .fields
                .get("node_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            component: record
                .fields
                .get("component")
                .and_then(Value::as_str)
                .map(str::to_owned),
            sequence: record.sequence,
            timestamp: record.timestamp,
            level: record.level,
            target: record.target,
            message: record.message,
            request_id: record.request_id,
            fields: record.fields,
        }
    }
}

enum FileCommand {
    Append(
        LogRecord,
        tokio::sync::oneshot::Sender<std::io::Result<LogRecord>>,
    ),
    Page(
        u64,
        usize,
        usize,
        tokio::sync::oneshot::Sender<std::io::Result<LogPage>>,
    ),
}

pub(crate) struct DurableStore {
    sender: std::sync::mpsc::SyncSender<FileCommand>,
    maximum: u64,
}

impl DurableStore {
    fn start(mut store: SegmentStore) -> std::io::Result<Self> {
        let maximum = store.maximum();
        let (sender, receiver) = std::sync::mpsc::sync_channel::<FileCommand>(8);
        std::thread::Builder::new()
            .name("runtime-log-files".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        FileCommand::Append(record, sender) => {
                            let _ = sender.send(store.append(record));
                        }
                        FileCommand::Page(after, limit, bytes, sender) => {
                            let _ = sender.send(store.page(after, limit, bytes));
                        }
                    }
                }
            })?;
        Ok(Self { sender, maximum })
    }
    async fn append(&self, record: LogRecord) -> std::io::Result<LogRecord> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.sender
            .try_send(FileCommand::Append(record, sender))
            .map_err(|_| std::io::Error::other("runtime_log_worker_unavailable"))?;
        receiver
            .await
            .map_err(|_| std::io::Error::other("runtime_log_worker_unavailable"))?
    }
    pub(crate) async fn page(
        &self,
        after: u64,
        limit: usize,
        bytes: usize,
    ) -> std::io::Result<LogPage> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.sender
            .try_send(FileCommand::Page(after, limit, bytes, sender))
            .map_err(|_| std::io::Error::other("runtime_log_worker_unavailable"))?;
        receiver
            .await
            .map_err(|_| std::io::Error::other("runtime_log_worker_unavailable"))?
    }
}

pub(crate) struct Storage {
    pub(crate) durable: Option<DurableStore>,
    pub(crate) needs_recovery: bool,
    entries: VecDeque<(RuntimeLogResponse, usize)>,
    bytes: usize,
    sequence: u64,
}

struct RuntimePage {
    entries: Vec<RuntimeLogResponse>,
    cursor: u64,
    minimum: Option<u64>,
    maximum: u64,
    gaps: Vec<(u64, u64)>,
}
#[derive(Clone)]
pub struct RuntimeLogStore {
    pub(crate) storage: Arc<Mutex<Storage>>,
    pub(crate) pool: Arc<OnceLock<sqlx::SqlitePool>>,
    capacity: usize,
    memory_bytes: usize,
    dropped: Arc<AtomicU64>,
    write_errors: Arc<AtomicU64>,
    queued_bytes: Arc<AtomicUsize>,
}
impl RuntimeLogStore {
    pub fn start() -> (Self, RuntimeLogLayer) {
        Self::launch(None, DEFAULT_CAPACITY, MEMORY_BYTES)
    }
    pub fn open(root: PathBuf, policy: LogPolicy) -> std::io::Result<(Self, RuntimeLogLayer)> {
        Ok(Self::launch(
            Some(DurableStore::start(SegmentStore::open(root, policy)?)?),
            DEFAULT_CAPACITY,
            MEMORY_BYTES,
        ))
    }

    pub fn start_from_env(default_root: PathBuf) -> std::io::Result<(Self, RuntimeLogLayer)> {
        use std::os::unix::fs::DirBuilderExt;
        let root = std::env::var_os("DEPLOY_GO_RUNTIME_LOG_DIR")
            .map(PathBuf::from)
            .unwrap_or(default_root);
        if !root.is_absolute() {
            return Err(std::io::Error::other("runtime_log_directory_invalid"));
        }
        // 先检查所有已有祖先，避免 create_dir_all 穿过符号链接创建目录。
        for parent in root.ancestors() {
            match std::fs::symlink_metadata(parent) {
                Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(std::io::Error::other("runtime_log_directory_invalid")),
            }
        }
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&root)?;
        Self::open(root, LogPolicy::CENTRAL)
    }
    fn launch(
        durable: Option<DurableStore>,
        capacity: usize,
        memory_bytes: usize,
    ) -> (Self, RuntimeLogLayer) {
        let (sender, mut receiver) = mpsc::channel::<(LogRecord, usize)>(CHANNEL_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let queued_bytes = Arc::new(AtomicUsize::new(0));
        let sequence = durable.as_ref().map_or(0, |store| store.maximum);
        let store = Self {
            storage: Arc::new(Mutex::new(Storage {
                needs_recovery: durable.is_some(),
                durable,
                entries: VecDeque::new(),
                bytes: 0,
                sequence,
            })),
            pool: Arc::new(OnceLock::new()),
            capacity,
            memory_bytes,
            dropped: dropped.clone(),
            write_errors: Arc::new(AtomicU64::new(0)),
            queued_bytes: queued_bytes.clone(),
        };
        let worker = store.clone();
        tokio::spawn(async move {
            while let Some((record, bytes)) = receiver.recv().await {
                let mut storage = worker.storage.lock().await;
                let ready = if storage.needs_recovery {
                    match worker.pool.get() {
                        Some(pool) => crate::runtime_log_ingest::recover_locked(&mut storage, pool)
                            .await
                            .is_ok(),
                        None => false,
                    }
                } else {
                    true
                };
                if !ready || worker.append_locked(&mut storage, record).await.is_err() {
                    worker.dropped.fetch_add(1, Ordering::Relaxed);
                }
                worker.queued_bytes.fetch_sub(bytes, Ordering::Relaxed);
            }
        });
        (
            store,
            RuntimeLogLayer {
                sender,
                dropped,
                queued_bytes,
            },
        )
    }
    pub(crate) fn attach_pool(&self, pool: sqlx::SqlitePool) {
        let _ = self.pool.set(pool);
    }
    pub(crate) async fn append_locked(
        &self,
        storage: &mut Storage,
        mut record: LogRecord,
    ) -> std::io::Result<LogRecord> {
        record = if let Some(durable) = storage.durable.as_mut() {
            match durable.append(record).await {
                Ok(record) => record,
                Err(error) => {
                    self.write_errors.fetch_add(1, Ordering::Relaxed);
                    return Err(error);
                }
            }
        } else {
            storage.sequence += 1;
            record.sequence = storage.sequence;
            record
        };
        storage.sequence = record.sequence;
        let response = RuntimeLogResponse::from(record.clone());
        let bytes = serde_json::to_vec(&response)?.len();
        while !storage.entries.is_empty()
            && (storage.entries.len() >= self.capacity || storage.bytes + bytes > self.memory_bytes)
        {
            let (_, old_bytes) = storage.entries.pop_front().unwrap();
            storage.bytes -= old_bytes;
        }
        if bytes <= self.memory_bytes && self.capacity > 0 {
            storage.bytes += bytes;
            storage.entries.push_back((response, bytes));
        }
        Ok(record)
    }
    async fn page(&self, after: u64, filter: &RuntimeLogQuery) -> std::io::Result<RuntimePage> {
        let storage = self.storage.lock().await;
        if let Some(durable) = &storage.durable {
            let page = durable.page(after, 500, MEMORY_BYTES).await?;
            let cursor = page.entries.last().map_or(after, |entry| entry.sequence);
            let mut previous = after;
            let mut gaps = Vec::new();
            for record in &page.entries {
                if record.sequence > previous.saturating_add(1) {
                    gaps.push((previous + 1, record.sequence - 1));
                }
                previous = record.sequence;
            }
            Ok(RuntimePage {
                entries: page
                    .entries
                    .into_iter()
                    .map(RuntimeLogResponse::from)
                    .filter(|entry| filter.matches(entry))
                    .collect(),
                cursor,
                minimum: page.minimum,
                maximum: page.maximum,
                gaps,
            })
        } else {
            let scanned: Vec<_> = storage
                .entries
                .iter()
                .filter(|(entry, _)| entry.sequence > after)
                .take(500)
                .collect();
            let cursor = scanned.last().map_or(after, |(entry, _)| entry.sequence);
            let minimum = scanned.first().map(|(entry, _)| entry.sequence);
            let gaps = minimum
                .filter(|v| *v > after.saturating_add(1))
                .map(|v| vec![(after + 1, v - 1)])
                .unwrap_or_default();
            Ok(RuntimePage {
                entries: scanned
                    .into_iter()
                    .filter(|(entry, _)| filter.matches(entry))
                    .map(|(entry, _)| entry.clone())
                    .collect(),
                cursor,
                minimum: storage.entries.front().map(|(entry, _)| entry.sequence),
                maximum: storage.sequence,
                gaps,
            })
        }
    }
    async fn bounds(&self) -> std::io::Result<(Option<u64>, u64)> {
        let storage = self.storage.lock().await;
        if let Some(durable) = &storage.durable {
            let page = durable.page(u64::MAX, 0, 0).await?;
            Ok((page.minimum, page.maximum))
        } else {
            Ok((
                storage.entries.front().map(|(entry, _)| entry.sequence),
                storage.sequence,
            ))
        }
    }
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
    pub async fn fallback_to_memory(&self) {
        let mut storage = self.storage.lock().await;
        storage.durable = None;
        storage.needs_recovery = false;
    }
    pub async fn recover(&self, pool: &sqlx::SqlitePool) -> ApiResult<()> {
        self.attach_pool(pool.clone());
        let mut storage = self.storage.lock().await;
        crate::runtime_log_ingest::recover_locked(&mut storage, pool).await
    }
}
#[derive(Clone)]
pub struct RuntimeLogLayer {
    sender: mpsc::Sender<(LogRecord, usize)>,
    dropped: Arc<AtomicU64>,
    queued_bytes: Arc<AtomicUsize>,
}
impl<S: Subscriber> Layer<S> for RuntimeLogLayer {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        let metadata = event.metadata();
        visitor.fields.insert("component".into(), "api".into());
        let pending = LogRecord {
            sequence: 0,
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: metadata.level().as_str().to_owned(),
            target: metadata.target().chars().take(128).collect(),
            message: visitor
                .fields
                .get("diagnostic_event")
                .and_then(Value::as_str)
                .unwrap_or(metadata.name())
                .chars()
                .take(256)
                .collect(),
            request_id: visitor.request_id,
            fields: visitor.fields,
        };
        let bytes =
            serde_json::to_vec(&pending).map_or(MAX_RECORD_BYTES + 1, |bytes| bytes.len() + 1);
        if bytes > MAX_RECORD_BYTES
            || self
                .queued_bytes
                .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |used| {
                    (used + bytes <= QUEUE_BYTES).then_some(used + bytes)
                })
                .is_err()
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if self.sender.try_send((pending, bytes)).is_err() {
            self.queued_bytes.fetch_sub(bytes, Ordering::Relaxed);
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}
#[derive(Default)]
struct EventVisitor {
    request_id: Option<String>,
    fields: BTreeMap<String, Value>,
}
impl EventVisitor {
    fn record_value(&mut self, field: &Field, value: Value) {
        if field.name() == "request_id" {
            self.request_id = value.as_str().filter(|v| identifier(v)).map(str::to_owned);
        } else if allowed_field(field.name()) && value.as_str().is_none_or(identifier) {
            self.fields.insert(field.name().to_owned(), value);
        } else if sensitive_field(field.name()) {
            self.fields
                .insert(field.name().into(), Value::String("[REDACTED]".into()));
        }
    }
}
fn sensitive_field(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "password",
        "token",
        "secret",
        "authorization",
        "cookie",
        "csrf",
        "private_key",
        "master_key",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}
impl Visit for EventVisitor {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_value(field, value.into());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_value(field, value.into());
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_value(field, value.into());
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, Value::String(value.chars().take(129).collect()));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if matches!(
            field.name(),
            "task_id" | "deployment_id" | "request_id" | "agent_id" | "node_id" | "job_id"
        ) {
            struct Limited(String);
            impl fmt::Write for Limited {
                fn write_str(&mut self, value: &str) -> fmt::Result {
                    if self.0.len() + value.len() > 130 {
                        return Err(fmt::Error);
                    }
                    self.0.push_str(value);
                    Ok(())
                }
            }
            let mut output = Limited(String::with_capacity(130));
            if fmt::write(&mut output, format_args!("{value:?}")).is_ok() && identifier(&output.0) {
                self.record_value(field, output.0.into());
            }
            return;
        }
        if sensitive_field(field.name()) {
            self.fields
                .insert(field.name().into(), Value::String("[REDACTED]".into()));
        }
    }
}
#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeLogQuery {
    after: Option<u64>,
    level: Option<String>,
    request_id: Option<String>,
    target: Option<String>,
    node_id: Option<String>,
    component: Option<String>,
}
impl RuntimeLogQuery {
    fn matches(&self, entry: &RuntimeLogResponse) -> bool {
        self.level
            .as_ref()
            .is_none_or(|v| entry.level.eq_ignore_ascii_case(v))
            && self
                .request_id
                .as_ref()
                .is_none_or(|v| entry.request_id.as_ref().is_some_and(|id| id.contains(v)))
            && self
                .target
                .as_ref()
                .is_none_or(|v| entry.target.contains(v))
            && self
                .node_id
                .as_ref()
                .is_none_or(|v| entry.node_id.as_ref() == Some(v))
            && self
                .component
                .as_ref()
                .is_none_or(|v| entry.component.as_ref() == Some(v))
    }
    fn validate(&self, request_id: &str) -> ApiResult<()> {
        if self.level.as_ref().is_some_and(|v| {
            !matches!(
                v.to_ascii_uppercase().as_str(),
                "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"
            )
        }) || [&self.request_id, &self.target, &self.node_id]
            .iter()
            .any(|v| v.as_ref().is_some_and(|v| v.len() > 128))
            || self.component.as_ref().is_some_and(|v| {
                v != "api" && !deploy_go_runtime_log::COMPONENTS.contains(&v.as_str())
            })
        {
            return Err(ApiError::validation("运行日志筛选条件无效", request_id));
        }
        Ok(())
    }
}
pub fn router() -> Router<AppState> {
    Router::new().route("/runtime-logs", get(stream))
}
#[utoipa::path(operation_id = "runtime_logs_stream", get, path = "/api/v1/runtime-logs", params(("after" = Option<u64>, Query), ("level" = Option<String>, Query), ("request_id" = Option<String>, Query), ("target" = Option<String>, Query), ("node_id" = Option<String>, Query), ("component" = Option<String>, Query)), responses((status = 200, content_type = "text/event-stream"), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
async fn stream(
    State(state): State<AppState>,
    Query(query): Query<RuntimeLogQuery>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Sse<impl futures_core::Stream<Item = Result<SseEvent, std::convert::Infallible>>>> {
    actor.require_administrator(request_id.as_str())?;
    query.validate(request_id.as_str())?;
    let header_after = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .map(str::parse::<u64>)
        .transpose()
        .map_err(|_| ApiError::validation("Last-Event-ID 格式不正确", request_id.as_str()))?;
    if header_after.is_some() && query.after.is_some() && header_after != query.after {
        return Err(ApiError::validation(
            "运行日志游标不一致",
            request_id.as_str(),
        ));
    }
    let mut after = header_after.or(query.after).unwrap_or(0);
    let bounds = state
        .runtime_logs()
        .bounds()
        .await
        .map_err(|_| ApiError::service_not_ready(request_id.as_str()))?;
    if after > bounds.1 {
        return Err(ApiError::validation(
            "运行日志游标无效",
            request_id.as_str(),
        ));
    }
    let store = state.runtime_logs().clone();
    let pool = state.pool().clone();
    let actor_id = actor.id;
    let session_id = actor.session_id;
    let output = async_stream::stream! {
        loop {
            let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users u JOIN sessions s ON s.user_id=u.id WHERE u.id=? AND u.identity='administrator' AND u.status='active' AND u.system_account=0 AND s.id=? AND s.revoked_at IS NULL AND s.expires_at>strftime('%Y-%m-%dT%H:%M:%fZ','now'))")
                .bind(&actor_id).bind(&session_id).fetch_one(&pool).await.unwrap_or(false);
            if !active { yield Ok(SseEvent::default().event("authorization-revoked").data("运行日志访问权限已经失效")); break; }
            match store.page(after, &query).await {
                Ok(page) => {
                    for (from,to) in page.gaps {
                        yield Ok(SseEvent::default().event("gap").data(serde_json::json!({"kind":"central","from_sequence":from,"to_sequence":to,"minimum_sequence":page.minimum,"maximum_sequence":page.maximum}).to_string()));
                    }
                    for entry in page.entries {
                        if let (Some(from), Some(to)) = (entry.fields.get("source_gap_from"), entry.fields.get("source_gap_to")) {
                            yield Ok(SseEvent::default().event("gap").data(serde_json::json!({"kind":"source","node_id":entry.node_id,"component":entry.component,"epoch":entry.fields.get("source_epoch"),"from_sequence":from,"to_sequence":to}).to_string()));
                        }
                        yield Ok(SseEvent::default().id(entry.sequence.to_string()).event("log").data(serde_json::to_string(&entry).unwrap_or_else(|_| "{}".into())));
                    }
                    after = after.max(page.cursor);
                }
                Err(_) => { yield Ok(SseEvent::default().event("storage-unavailable").data("运行日志存储暂不可用")); break; }
            }
            yield Ok(SseEvent::default().event("stats").data(serde_json::json!({"dropped":store.dropped(),"write_errors":store.write_errors.load(Ordering::Relaxed),"queued_bytes":store.queued_bytes.load(Ordering::Relaxed)}).to_string()));
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    };
    Ok(Sse::new(output).keep_alive(KeepAlive::default()))
}
#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::prelude::*;
    #[tokio::test]
    async fn layer_captures_structured_request_log_and_never_formats_debug() {
        struct Secret;
        impl fmt::Debug for Secret {
            fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
                panic!("不能格式化秘密");
            }
        }
        let (store, layer) = RuntimeLogStore::launch(None, 2, MEMORY_BYTES);
        tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
            tracing::info!(request_id="req_01TEST", status=200_u64, error=?Secret, access_token="secret", "raw secret message");
        });
        tokio::task::yield_now().await;
        let logs = store
            .page(0, &RuntimeLogQuery::default())
            .await
            .unwrap()
            .entries;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].request_id.as_deref(), Some("req_01TEST"));
        assert_eq!(logs[0].fields["status"], 200);
        assert_eq!(logs[0].fields["access_token"], "[REDACTED]");
        assert!(!serde_json::to_string(&logs).unwrap().contains("raw secret"));
    }

    #[tokio::test]
    async fn display_identifiers_are_bounded_and_filterable() {
        let (store, layer) = RuntimeLogStore::launch(None, 2, MEMORY_BYTES);
        let oversized = "x".repeat(10000);
        tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
            tracing::info!(request_id=%"req_display",deployment_id=%"deployment_fixture",task_id=%oversized,"ignored");
        });
        tokio::task::yield_now().await;
        let query = RuntimeLogQuery {
            request_id: Some("req_display".into()),
            ..Default::default()
        };
        let page = store.page(0, &query).await.unwrap();
        assert_eq!(page.entries.len(), 1);
        assert_eq!(
            page.entries[0].fields["deployment_id"],
            "deployment_fixture"
        );
        assert!(!page.entries[0].fields.contains_key("task_id"));
    }

    #[tokio::test]
    async fn queue_and_memory_are_byte_bounded_and_collection_never_waits() {
        let (store, layer) = RuntimeLogStore::launch(None, 5000, 512);
        tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
            for _ in 0..3000 {
                tracing::info!(request_id = "req_bounded", status = 200_u64, "ignored");
            }
        });
        assert!(store.queued_bytes.load(Ordering::Relaxed) <= QUEUE_BYTES);
        assert!(store.dropped() > 0);
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        let storage = store.storage.lock().await;
        assert!(storage.bytes <= 512);
        assert!(storage.entries.len() < 3);
    }

    #[tokio::test]
    async fn disk_history_remains_queryable_when_cache_cannot_hold_records() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let durable =
            DurableStore::start(SegmentStore::open(root, LogPolicy::CENTRAL).unwrap()).unwrap();
        let (store, _layer) = RuntimeLogStore::launch(Some(durable), 1, 128);
        let mut storage = store.storage.lock().await;
        storage.needs_recovery = false;
        for _ in 0..5 {
            store
                .append_locked(
                    &mut storage,
                    LogRecord {
                        sequence: 0,
                        timestamp: "2026-10-08T00:00:00Z".into(),
                        level: "INFO".into(),
                        target: "api".into(),
                        message: "fixture".into(),
                        request_id: None,
                        fields: BTreeMap::new(),
                    },
                )
                .await
                .unwrap();
        }
        assert!(storage.entries.is_empty());
        drop(storage);
        let page = store.page(0, &RuntimeLogQuery::default()).await.unwrap();
        assert_eq!(page.entries.len(), 5);
        assert_eq!(page.cursor, 5);
    }
}
