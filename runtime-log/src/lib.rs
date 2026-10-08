mod layer;
mod layout;
mod store;

pub use layer::{DiagnosticLayer, DiagnosticRecord, node_layer};
pub use layer::{allowed_field, identifier};
pub use layout::prepare_node_layout;
pub use store::{LogPage, LogPolicy, LogRecord, SegmentStore, read_page};

pub const NODE_LOG_ROOT: &str = "/var/log/deploy-go-agent";
pub const COMPONENTS: [&str; 4] = ["agent", "runner", "executor", "updater"];
pub const MAX_RECORD_BYTES: usize = 16 * 1024;
pub const MAX_BATCH_RECORDS: usize = 64;
pub const MAX_BATCH_BYTES: usize = 512 * 1024;
