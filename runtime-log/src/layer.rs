use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

use serde_json::Value;
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context};

use crate::{COMPONENTS, LogPolicy, LogRecord, SegmentStore};

pub type DiagnosticRecord = LogRecord;

enum Queued {
    Record(LogRecord),
    Barrier(mpsc::Sender<()>),
}

#[derive(Clone)]
pub struct DiagnosticLayer {
    sender: mpsc::SyncSender<Queued>,
    dropped: Arc<AtomicU64>,
}

impl DiagnosticLayer {
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn flush(&self) {
        let (sender, receiver) = mpsc::channel();
        if self.sender.try_send(Queued::Barrier(sender)).is_ok() {
            let _ = receiver.recv_timeout(Duration::from_millis(500));
        }
    }

    pub fn record(&self, record: LogRecord) {
        if record.fields.len() > 40
            || record.message.len() > 256
            || record.target.len() > 128
            || record.timestamp.len() > 64
            || record
                .request_id
                .as_ref()
                .is_some_and(|value| value.len() > 128)
            || record.fields.iter().any(|(name, value)| {
                name.len() > 64
                    || match value {
                        Value::String(value) => value.len() > 128,
                        Value::Bool(_) | Value::Number(_) | Value::Null => false,
                        _ => true,
                    }
            })
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if self.sender.try_send(Queued::Record(record)).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub fn node_layer(root: PathBuf, component: &str) -> std::io::Result<DiagnosticLayer> {
    if !COMPONENTS.contains(&component) {
        return Err(std::io::Error::other("runtime_log_component_invalid"));
    }
    let mut store = SegmentStore::open(root.join(component), LogPolicy::NODE)?;
    let (sender, receiver) = mpsc::sync_channel::<Queued>(128);
    let dropped = Arc::new(AtomicU64::new(0));
    let worker_dropped = dropped.clone();
    let component = component.to_owned();
    std::thread::Builder::new()
        .name(format!("diagnostic-{component}"))
        .stack_size(256 * 1024)
        .spawn(move || {
            while let Ok(item) = receiver.recv() {
                match item {
                    Queued::Record(mut record) => {
                        record
                            .fields
                            .insert("component".into(), Value::String(component.clone()));
                        record.fields.insert(
                            "collector_dropped".into(),
                            worker_dropped.load(Ordering::Relaxed).into(),
                        );
                        if store.append(record).is_err() {
                            worker_dropped.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Queued::Barrier(sender) => {
                        let _ = sender.send(());
                    }
                }
            }
        })?;
    Ok(DiagnosticLayer { sender, dropped })
}

pub fn allowed_field(name: &str) -> bool {
    matches!(
        name,
        "task_id"
            | "deployment_id"
            | "request_id"
            | "agent_id"
            | "node_id"
            | "job_id"
            | "lease_id"
            | "artifact_id"
            | "stage"
            | "phase"
            | "category"
            | "error_code"
            | "error_kind"
            | "operation"
            | "status"
            | "attempt"
            | "attempts"
            | "elapsed_ms"
            | "confirmed_offset"
            | "bytes"
            | "pending_bytes"
            | "acknowledged"
            | "sequence"
            | "succeeded"
            | "version"
            | "removed_task_dirs"
            | "removed_deployment_dirs"
            | "reclaimed_deployments"
            | "diagnostic_event"
            | "http_status"
            | "protocol_version"
            | "recoveries"
            | "collector_dropped"
    )
}

pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

#[derive(Default)]
struct Visitor {
    fields: BTreeMap<String, Value>,
}

impl Visit for Visitor {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if allowed_field(field.name()) {
            self.fields.insert(field.name().into(), value.into());
        }
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        if allowed_field(field.name()) {
            self.fields.insert(field.name().into(), value.into());
        }
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        if allowed_field(field.name()) {
            self.fields.insert(field.name().into(), value.into());
        }
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        if allowed_field(field.name()) && identifier(value) {
            self.fields.insert(field.name().into(), value.into());
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        // 只格式化已知关联 ID，且 formatter 遇到上限立即返回；错误链和正文不进入日志。
        if !matches!(
            field.name(),
            "task_id"
                | "deployment_id"
                | "request_id"
                | "agent_id"
                | "node_id"
                | "job_id"
                | "lease_id"
                | "artifact_id"
        ) {
            return;
        }
        struct Limited(String);
        impl std::fmt::Write for Limited {
            fn write_str(&mut self, value: &str) -> std::fmt::Result {
                if self.0.len() + value.len() > 130 {
                    return Err(std::fmt::Error);
                }
                self.0.push_str(value);
                Ok(())
            }
        }
        use std::fmt::Write;
        let mut output = Limited(String::new());
        if write!(&mut output, "{value:?}").is_ok() {
            self.record_str(field, output.0.trim_matches('"'));
        }
    }
}

impl<S: Subscriber> Layer<S> for DiagnosticLayer {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = Visitor::default();
        event.record(&mut visitor);
        let request_id = visitor
            .fields
            .remove("request_id")
            .and_then(|v| v.as_str().map(str::to_owned));
        self.record(LogRecord {
            sequence: 0,
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: metadata.level().as_str().into(),
            target: metadata.target().chars().take(128).collect(),
            message: visitor
                .fields
                .get("diagnostic_event")
                .and_then(Value::as_str)
                .unwrap_or(metadata.name())
                .chars()
                .take(256)
                .collect(),
            request_id,
            fields: visitor.fields,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tracing_subscriber::prelude::*;

    #[test]
    fn diagnostics_do_not_format_secret_message_or_error() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("agent")).unwrap();
        std::fs::set_permissions(root.join("agent"), std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let layer = node_layer(root.clone(), "agent").unwrap();
        tracing::subscriber::with_default(
            tracing_subscriber::registry().with(layer.clone()),
            || {
                tracing::error!(task_id = "task_fixture", category = "timeout", elapsed_ms = 12_u64,
                token = "SECRET_TOKEN", error = ?vec!["SECRET_ERROR"; 10000],
                "SECRET_MESSAGE https://secret.invalid/?password=VALUE");
            },
        );
        layer.flush();
        let page = crate::read_page(&root.join("agent"), 0, 64, 10000).unwrap();
        let encoded = serde_json::to_string(&page.entries).unwrap();
        assert_eq!(page.entries.len(), 1);
        assert!(encoded.contains("task_fixture"));
        assert!(encoded.contains("timeout"));
        assert!(!encoded.contains("SECRET"));
        assert!(!encoded.contains("password"));
    }
}
