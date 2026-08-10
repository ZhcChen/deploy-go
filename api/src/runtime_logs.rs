use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use axum::{
    Router,
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::sse::{Event as SseEvent, KeepAlive, Sse},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{RwLock, mpsc};
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context};
use utoipa::ToSchema;

use crate::{
    AppState, RequestId,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};

const DEFAULT_CAPACITY: usize = 5_000;
const CHANNEL_CAPACITY: usize = 1_024;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct RuntimeLogResponse {
    pub sequence: u64,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub request_id: Option<String>,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug)]
struct PendingLog {
    timestamp: String,
    level: String,
    target: String,
    message: String,
    request_id: Option<String>,
    fields: BTreeMap<String, Value>,
}

#[derive(Clone)]
pub struct RuntimeLogStore {
    entries: Arc<RwLock<VecDeque<RuntimeLogResponse>>>,
    capacity: usize,
    dropped: Arc<AtomicU64>,
}

impl RuntimeLogStore {
    pub fn start() -> (Self, RuntimeLogLayer) {
        Self::start_with_capacity(DEFAULT_CAPACITY)
    }

    fn start_with_capacity(capacity: usize) -> (Self, RuntimeLogLayer) {
        let (sender, mut receiver) = mpsc::channel::<PendingLog>(CHANNEL_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let store = Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            capacity,
            dropped: dropped.clone(),
        };
        let worker_store = store.clone();
        tokio::spawn(async move {
            let mut sequence = 0_u64;
            while let Some(log) = receiver.recv().await {
                sequence += 1;
                let mut entries = worker_store.entries.write().await;
                if entries.len() == worker_store.capacity {
                    entries.pop_front();
                }
                entries.push_back(RuntimeLogResponse {
                    sequence,
                    timestamp: log.timestamp,
                    level: log.level,
                    target: log.target,
                    message: log.message,
                    request_id: log.request_id,
                    fields: log.fields,
                });
            }
        });
        (store, RuntimeLogLayer { sender, dropped })
    }

    async fn after(&self, after: u64, filter: &RuntimeLogQuery) -> Vec<RuntimeLogResponse> {
        self.entries
            .read()
            .await
            .iter()
            .filter(|entry| entry.sequence > after && filter.matches(entry))
            .take(500)
            .cloned()
            .collect()
    }

    async fn bounds(&self) -> (Option<u64>, Option<u64>) {
        let entries = self.entries.read().await;
        (
            entries.front().map(|entry| entry.sequence),
            entries.back().map(|entry| entry.sequence),
        )
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

#[derive(Clone)]
pub struct RuntimeLogLayer {
    sender: mpsc::Sender<PendingLog>,
    dropped: Arc<AtomicU64>,
}

impl<S> Layer<S> for RuntimeLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        let metadata = event.metadata();
        let pending = PendingLog {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: metadata.level().as_str().to_owned(),
            target: metadata.target().to_owned(),
            message: visitor
                .message
                .unwrap_or_else(|| metadata.name().to_owned()),
            request_id: visitor.request_id,
            fields: visitor.fields,
        };
        if self.sender.try_send(pending).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[derive(Default)]
struct EventVisitor {
    message: Option<String>,
    request_id: Option<String>,
    fields: BTreeMap<String, Value>,
}

impl EventVisitor {
    fn record_value(&mut self, field: &Field, value: Value) {
        if sensitive_field(field.name()) {
            self.fields.insert(
                field.name().to_owned(),
                Value::String("[REDACTED]".to_owned()),
            );
            return;
        }
        match field.name() {
            "message" => {
                self.message = value
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| Some(value.to_string()))
            }
            "request_id" => {
                self.request_id = value
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| Some(value.to_string()))
            }
            name => {
                self.fields.insert(name.to_owned(), value);
            }
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
        self.record_value(field, value.into());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_value(field, format!("{value:?}").into());
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeLogQuery {
    after: Option<u64>,
    level: Option<String>,
    request_id: Option<String>,
    target: Option<String>,
}

impl RuntimeLogQuery {
    fn matches(&self, entry: &RuntimeLogResponse) -> bool {
        self.level
            .as_ref()
            .is_none_or(|value| entry.level.eq_ignore_ascii_case(value))
            && self.request_id.as_ref().is_none_or(|value| {
                entry
                    .request_id
                    .as_deref()
                    .is_some_and(|id| id.contains(value))
            })
            && self
                .target
                .as_ref()
                .is_none_or(|value| entry.target.contains(value))
    }

    fn validate(&self, request_id: &str) -> ApiResult<()> {
        if self.level.as_ref().is_some_and(|level| {
            !matches!(
                level.to_ascii_uppercase().as_str(),
                "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"
            )
        }) {
            return Err(ApiError::validation(
                "level 必须是 TRACE、DEBUG、INFO、WARN 或 ERROR",
                request_id,
            ));
        }
        if self
            .request_id
            .as_ref()
            .is_some_and(|value| value.len() > 128)
            || self.target.as_ref().is_some_and(|value| value.len() > 128)
        {
            return Err(ApiError::validation(
                "日志筛选条件不能超过 128 个字符",
                request_id,
            ));
        }
        Ok(())
    }
}

pub fn router() -> Router<AppState> {
    Router::new().route("/runtime-logs", get(stream))
}

#[utoipa::path(operation_id = "runtime_logs_stream", get, path = "/api/v1/runtime-logs", params(("after" = Option<u64>, Query), ("level" = Option<String>, Query), ("request_id" = Option<String>, Query), ("target" = Option<String>, Query)), responses((status = 200, content_type = "text/event-stream"), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
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
        .and_then(|value| value.to_str().ok())
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
    let bounds = state.runtime_logs().bounds().await;
    if bounds.1.is_some_and(|maximum| after > maximum)
        || bounds
            .0
            .is_some_and(|minimum| after > 0 && after < minimum.saturating_sub(1))
    {
        return Err(ApiError::validation(
            "运行日志游标无效或已经过期",
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
            if !active {
                yield Ok(SseEvent::default().event("authorization-revoked").data("运行日志访问权限已经失效"));
                break;
            }
            for entry in store.after(after, &query).await {
                after = after.max(entry.sequence);
                let data = serde_json::to_string(&entry).unwrap_or_else(|_| "{}".to_owned());
                yield Ok(SseEvent::default().id(entry.sequence.to_string()).event("log").data(data));
            }
            yield Ok(SseEvent::default().event("stats").data(serde_json::json!({"dropped": store.dropped()}).to_string()));
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
    async fn layer_captures_structured_request_log() {
        let (store, layer) = RuntimeLogStore::start_with_capacity(2);
        tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
            tracing::info!(
                request_id = "req_01TEST",
                status = 200_u64,
                "request completed"
            );
        });
        tokio::task::yield_now().await;
        let logs = store
            .after(
                0,
                &RuntimeLogQuery {
                    after: None,
                    level: None,
                    request_id: None,
                    target: None,
                },
            )
            .await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].request_id.as_deref(), Some("req_01TEST"));
        assert_eq!(logs[0].fields.get("status"), Some(&Value::from(200_u64)));
    }

    #[tokio::test]
    async fn layer_redacts_sensitive_fields() {
        let (store, layer) = RuntimeLogStore::start_with_capacity(2);
        tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
            tracing::warn!(access_token = "do-not-store", "credential rejected");
        });
        tokio::task::yield_now().await;
        let logs = store
            .after(
                0,
                &RuntimeLogQuery {
                    after: None,
                    level: None,
                    request_id: None,
                    target: None,
                },
            )
            .await;
        assert_eq!(logs[0].fields["access_token"], "[REDACTED]");
    }
}
