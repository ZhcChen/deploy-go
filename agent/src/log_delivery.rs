use crate::journal::TaskJournal;
use deploy_go_agent_protocol::{Message, TaskEventReceipt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

pub const STATE_FILE: &str = "log-delivery-v1.json";
pub const SEGMENT_BYTES: u64 = 4 * 1024 * 1024;
const NODE_OUTBOX_LIMIT: u64 = 512 * 1024 * 1024;
const TASK_OUTBOX_LIMIT: u64 = 128 * 1024 * 1024;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeliveryState {
    schema_version: u16,
    acknowledged: u64,
    pending_bytes: u64,
    finalized: bool,
}

#[derive(Serialize, Deserialize)]
struct PendingEvent {
    message: Message,
    checkpoint: TaskJournal,
}

pub fn initialize(task_dir: &Path) -> io::Result<()> {
    let existing = enabled(task_dir);
    if existing && !is_uninitialized_task(task_dir)? {
        return read_state(task_dir).map(|_| ());
    }
    let root = task_dir
        .parent()
        .ok_or_else(|| io::Error::other("missing task root"))?;
    let mut reserved = 0u64;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && enabled(&entry.path())
            && !finalized(&entry.path())
            && !is_uninitialized_task(&entry.path())?
        {
            reserved = reserved.saturating_add(TASK_OUTBOX_LIMIT);
        }
    }
    if node_pending_bytes(root)?
        .max(reserved)
        .saturating_add(TASK_OUTBOX_LIMIT)
        > NODE_OUTBOX_LIMIT
    {
        return Err(io::Error::other("node_log_spool_budget_exceeded"));
    }
    if existing {
        return read_state(task_dir).map(|_| ());
    }
    atomic_json(
        &task_dir.join(STATE_FILE),
        &DeliveryState {
            schema_version: 1,
            ..Default::default()
        },
    )
}

pub fn enabled(task_dir: &Path) -> bool {
    fs::symlink_metadata(task_dir.join(STATE_FILE)).is_ok()
}

// 初始化未提交 journal 时没有任务可恢复；仅识别空目录或零状态 sidecar，保留全部文件。
pub fn is_uninitialized_task(task_dir: &Path) -> io::Result<bool> {
    use std::os::unix::{fs::MetadataExt, fs::OpenOptionsExt};
    reject_symlink(task_dir)?;
    let metadata = fs::symlink_metadata(task_dir)?;
    let parent = fs::metadata(
        task_dir
            .parent()
            .ok_or_else(|| io::Error::other("missing task root"))?,
    )?;
    if !metadata.is_dir()
        || !matches!(metadata.mode() & 0o7777, 0o2770 | 0o3700)
        || metadata.uid() != parent.uid()
        || metadata.gid() != parent.gid()
    {
        return Ok(false);
    }
    for entry in fs::read_dir(task_dir)? {
        let entry = entry?;
        if entry.file_name() != STATE_FILE || !entry.file_type()?.is_file() {
            return Ok(false);
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(entry.path())?;
        let file_metadata = file.metadata()?;
        if !file_metadata.is_file()
            || file_metadata.nlink() != 1
            || file_metadata.uid() != metadata.uid()
            || file_metadata.len() > 4096
        {
            return Ok(false);
        }
        let mut bytes = Vec::new();
        file.take(4097).read_to_end(&mut bytes)?;
        if bytes.len() > 4096 {
            return Ok(false);
        }
        let Ok(state) = serde_json::from_slice::<DeliveryState>(&bytes) else {
            return Ok(false);
        };
        if state.schema_version != 1 {
            return Ok(false);
        }
        if state.acknowledged != 0 || state.pending_bytes != 0 || state.finalized {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn finalized(task_dir: &Path) -> bool {
    read_state(task_dir).is_ok_and(|state| state.finalized)
}

pub fn event_identity(message: &Message) -> Option<(&str, u64)> {
    match message {
        Message::TaskOutput(v) => Some((&v.task_id, v.sequence)),
        Message::TaskProgress(v) => Some((&v.task_id, v.sequence)),
        Message::TaskState(v) => Some((&v.task_id, v.sequence)),
        Message::TaskResult(v) => Some((&v.task_id, v.sequence)),
        _ => None,
    }
}

pub fn message_digest(message: &Message) -> io::Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(message)?)
    ))
}

pub fn persist(task_dir: &Path, message: &Message, checkpoint: &TaskJournal) -> io::Result<()> {
    if !enabled(task_dir) {
        return Ok(());
    }
    let (id, sequence) =
        event_identity(message).ok_or_else(|| io::Error::other("invalid log message"))?;
    if id != checkpoint.task_id || sequence != checkpoint.last_sequence {
        return Err(io::Error::other("log checkpoint mismatch"));
    }
    let outbox = task_dir.join("log-outbox-v1");
    reject_symlink(&outbox)?;
    fs::create_dir_all(&outbox)?;
    let path = outbox.join(format!("{sequence:020}.json"));
    let event = PendingEvent {
        message: message.clone(),
        checkpoint: checkpoint.clone(),
    };
    let bytes = serde_json::to_vec(&event)?;
    if path.exists() {
        if fs::read(path)? != bytes {
            return Err(io::Error::other("log sequence conflict"));
        }
        return Ok(());
    }
    let mut state = read_state(task_dir)?;
    state.pending_bytes = actual_pending_bytes(task_dir)?;
    if sequence <= state.acknowledged {
        return Ok(());
    }
    let last = pending_paths(task_dir)?
        .last()
        .and_then(|path| path.file_stem()?.to_str()?.parse::<u64>().ok())
        .unwrap_or(state.acknowledged)
        .max(state.acknowledged);
    if sequence != last.saturating_add(1) {
        return Err(io::Error::other("outbox sequence gap"));
    }
    let node_bytes = node_pending_bytes(
        task_dir
            .parent()
            .ok_or_else(|| io::Error::other("missing task root"))?,
    )?;
    if state.pending_bytes.saturating_add(bytes.len() as u64) > TASK_OUTBOX_LIMIT
        || node_bytes.saturating_add(bytes.len() as u64) > NODE_OUTBOX_LIMIT
    {
        return Err(io::Error::other("node_log_spool_budget_exceeded"));
    }
    atomic_json(&path, &event)?;
    state.pending_bytes = state.pending_bytes.saturating_add(bytes.len() as u64);
    atomic_json(&task_dir.join(STATE_FILE), &state)
}

pub fn recover_checkpoint(task_dir: &Path, journal: &mut TaskJournal) -> io::Result<()> {
    if !enabled(task_dir) {
        return Ok(());
    }
    let state = read_state(task_dir)?;
    let paths = pending_paths(task_dir)?;
    if paths
        .last()
        .and_then(|path| path.file_stem()?.to_str()?.parse::<u64>().ok())
        .is_none_or(|sequence| sequence <= journal.last_sequence)
    {
        return Ok(());
    }
    let mut expected = state.acknowledged.saturating_add(1);
    for path in paths {
        let event: PendingEvent = read_event(&path)?;
        if event.checkpoint.task_id != journal.task_id
            || event.checkpoint.payload_digest != journal.payload_digest
        {
            return Err(io::Error::other("log checkpoint identity mismatch"));
        }
        if event_identity(&event.message)
            != Some((journal.task_id.as_str(), event.checkpoint.last_sequence))
        {
            return Err(io::Error::other("outbox sequence identity mismatch"));
        }
        if event.checkpoint.last_sequence <= state.acknowledged {
            continue;
        }
        if event.checkpoint.last_sequence != expected {
            return Err(io::Error::other("outbox sequence gap"));
        }
        expected = expected.saturating_add(1);
        if event.checkpoint.last_sequence > journal.last_sequence {
            journal.last_sequence = event.checkpoint.last_sequence;
            journal.stdout_offset = journal.stdout_offset.max(event.checkpoint.stdout_offset);
            journal.stderr_offset = journal.stderr_offset.max(event.checkpoint.stderr_offset);
            journal.events_offset = journal.events_offset.max(event.checkpoint.events_offset);
            journal.events_sent = journal.events_sent.max(event.checkpoint.events_sent);
            journal.result_sequence = journal.result_sequence.or(event.checkpoint.result_sequence);
        }
    }
    Ok(())
}

pub fn pending_messages(task_dir: &Path) -> io::Result<Vec<PathBuf>> {
    pending_paths(task_dir)
}

pub fn read_pending(path: &Path) -> io::Result<Message> {
    Ok(read_event(path)?.message)
}

fn read_event(path: &Path) -> io::Result<PendingEvent> {
    if let Some(parent) = path.parent() {
        reject_symlink(parent)?;
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err(io::Error::other("invalid outbox file"));
    }
    let event: PendingEvent = serde_json::from_slice(&fs::read(path)?)?;
    let (_, sequence) =
        event_identity(&event.message).ok_or_else(|| io::Error::other("invalid outbox message"))?;
    if path.file_name().and_then(|name| name.to_str())
        != Some(format!("{sequence:020}.json").as_str())
    {
        return Err(io::Error::other("outbox filename sequence mismatch"));
    }
    Ok(event)
}

pub fn acknowledge(
    task_dir: &Path,
    journal: &TaskJournal,
    receipt: &TaskEventReceipt,
) -> io::Result<()> {
    if !enabled(task_dir)
        || receipt.task_id != journal.task_id
        || receipt.payload_digest != journal.payload_digest
    {
        return Err(io::Error::other("log receipt identity mismatch"));
    }
    let mut state = read_state(task_dir)?;
    if receipt.sequence <= state.acknowledged {
        for path in pending_paths(task_dir)? {
            let event = read_event(&path)?;
            if event.checkpoint.last_sequence <= state.acknowledged {
                fs::remove_file(path)?;
            }
        }
        if state.finalized {
            remove_log_files(task_dir)?;
        }
        return Ok(());
    }
    if receipt.sequence != state.acknowledged + 1 || receipt.sequence > journal.last_sequence {
        return Err(io::Error::other("log receipt sequence mismatch"));
    }
    let path = task_dir
        .join("log-outbox-v1")
        .join(format!("{:020}.json", receipt.sequence));
    let event = read_event(&path)?;
    if message_digest(&event.message)? != receipt.message_digest {
        return Err(io::Error::other("log receipt digest mismatch"));
    }
    let size = fs::metadata(&path)?.len();
    state.acknowledged = receipt.sequence;
    state.pending_bytes = actual_pending_bytes(task_dir)?.saturating_sub(size);
    state.finalized = journal.result_sequence == Some(receipt.sequence)
        && matches!(event.message, Message::TaskResult(_));
    atomic_json(&task_dir.join(STATE_FILE), &state)?;
    fs::remove_file(path)?;
    if state.finalized {
        remove_log_files(task_dir)?;
        let frames = task_dir.join("executor-output-frames");
        reject_symlink(&frames)?;
        if frames.is_dir() {
            fs::remove_dir_all(frames)?;
        }
    } else {
        reclaim_segments(task_dir, &event.checkpoint)?;
    }
    Ok(())
}

fn node_pending_bytes(root: &Path) -> io::Result<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && enabled(&entry.path()) {
            total = total.saturating_add(actual_pending_bytes(&entry.path())?);
            for file in fs::read_dir(entry.path())? {
                let file = file?;
                if is_log_file(&file.file_name().to_string_lossy()) && file.file_type()?.is_file() {
                    total = total.saturating_add(file.metadata()?.len());
                }
            }
            let frames = entry.path().join("executor-output-frames");
            reject_symlink(&frames)?;
            if frames.is_dir() {
                for frame in fs::read_dir(frames)? {
                    let frame = frame?;
                    if frame.file_type()?.is_file() {
                        total = total.saturating_add(frame.metadata()?.len());
                    }
                }
            }
        }
    }
    Ok(total)
}

pub fn ensure_node_capacity(task_dir: &Path, additional: u64) -> io::Result<()> {
    if enabled(task_dir)
        && node_pending_bytes(
            task_dir
                .parent()
                .ok_or_else(|| io::Error::other("missing task root"))?,
        )?
        .saturating_add(additional)
            > NODE_OUTBOX_LIMIT
    {
        return Err(io::Error::other("node_log_spool_budget_exceeded"));
    }
    Ok(())
}

fn read_state(task_dir: &Path) -> io::Result<DeliveryState> {
    reject_symlink(task_dir)?;
    let metadata = fs::symlink_metadata(task_dir.join(STATE_FILE))?;
    if !metadata.is_file() || metadata.len() > 4096 {
        return Err(io::Error::other("invalid delivery state"));
    }
    let state: DeliveryState = serde_json::from_slice(&fs::read(task_dir.join(STATE_FILE))?)?;
    if state.schema_version != 1 {
        return Err(io::Error::other("unsupported log storage"));
    }
    Ok(state)
}

fn pending_paths(task_dir: &Path) -> io::Result<Vec<PathBuf>> {
    reject_symlink(task_dir)?;
    let outbox = task_dir.join("log-outbox-v1");
    reject_symlink(&outbox)?;
    if !outbox.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(outbox)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.len() == 25
            && name.ends_with(".json")
            && name[..20].bytes().all(|b| b.is_ascii_digit())
        {
            if !entry.file_type()?.is_file() {
                return Err(io::Error::other("invalid outbox entry"));
            }
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn remove_log_files(task_dir: &Path) -> io::Result<()> {
    reject_symlink(task_dir)?;
    for entry in fs::read_dir(task_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_log_file(&name) && entry.file_type()?.is_file() {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn is_log_file(name: &str) -> bool {
    ["stdout.log", "stderr.log"].contains(&name)
        || ["stdout.", "stderr."].iter().any(|prefix| {
            name.strip_prefix(prefix)
                .and_then(|rest| rest.strip_suffix(".log"))
                .is_some_and(|number| {
                    number.len() == 8 && number.bytes().all(|b| b.is_ascii_digit())
                })
        })
}

pub fn read_chunk(task_dir: &Path, stream: &str, offset: u64, limit: usize) -> io::Result<Vec<u8>> {
    reject_symlink(task_dir)?;
    if !matches!(stream, "stdout" | "stderr") {
        return Err(io::Error::other("invalid log stream"));
    }
    let path = if enabled(task_dir) {
        task_dir.join(format!("{stream}.{:08}.log", offset / SEGMENT_BYTES))
    } else {
        task_dir.join(format!("{stream}.log"))
    };
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = match fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let local_offset = if enabled(task_dir) {
        offset % SEGMENT_BYTES
    } else {
        offset
    };
    file.seek(SeekFrom::Start(local_offset))?;
    let max = if enabled(task_dir) {
        limit.min((SEGMENT_BYTES - local_offset) as usize)
    } else {
        limit
    };
    let mut bytes = vec![0; max];
    let count = file.read(&mut bytes)?;
    bytes.truncate(count);
    Ok(bytes)
}

pub(crate) fn reject_symlink(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(io::Error::other("log path is symlink"))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn actual_pending_bytes(task_dir: &Path) -> io::Result<u64> {
    pending_paths(task_dir)?
        .into_iter()
        .try_fold(0u64, |sum, path| {
            Ok(sum.saturating_add(fs::symlink_metadata(path)?.len()))
        })
}

fn reclaim_segments(task_dir: &Path, checkpoint: &TaskJournal) -> io::Result<()> {
    // 特权输出仍需原始段重建事件；最终 ACK 后统一回收。
    if task_dir.join("executor-output-frames").exists() {
        return Ok(());
    }
    for (stream, offset) in [
        ("stdout", checkpoint.stdout_offset),
        ("stderr", checkpoint.stderr_offset),
    ] {
        for index in 0..offset / SEGMENT_BYTES {
            let path = task_dir.join(format!("{stream}.{index:08}.log"));
            if !task_dir
                .join(format!("{stream}.{:08}.log", index + 1))
                .exists()
            {
                continue;
            }
            reject_symlink(&path)?;
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

pub fn retain_legacy_logs(root: &Path) -> io::Result<()> {
    reject_symlink(root)?;
    if !root.exists() || root.join("legacy-log-migration-v1.json").exists() {
        return Ok(());
    }
    let mut retained = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || enabled(&entry.path()) {
            continue;
        }
        if entry.path().join("journal.json").is_file()
            && (entry.path().join("stdout.log").exists()
                || entry.path().join("stderr.log").exists())
        {
            atomic_json(
                &entry.path().join("legacy-log-retained-v1.json"),
                &serde_json::json!({"status":"retained","reason":"no_contiguous_durable_receipt"}),
            )?;
            retained.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    retained.sort();
    atomic_json(
        &root.join("legacy-log-migration-v1.json"),
        &serde_json::json!({"schema_version":1,"status":"completed_with_retention","retained":retained}),
    )
}

pub fn cleanup_protected(task_dir: &Path) -> bool {
    if enabled(task_dir) {
        !finalized(task_dir)
    } else {
        task_dir.join("stdout.log").exists()
            || task_dir.join("log-outbox-v1").exists()
            || task_dir.join("executor-output-frames").exists()
            || task_dir.join("stderr.log").exists()
            || task_dir.join("legacy-log-retained-v1.json").exists()
            || fs::read_dir(task_dir).is_ok_and(|entries| {
                entries
                    .flatten()
                    .any(|entry| is_log_file(&entry.file_name().to_string_lossy()))
            })
    }
}

pub fn atomic_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let suffix = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = path.with_extension(format!("{}.{}.part", std::process::id(), suffix));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&temporary)?;
    serde_json::to_writer(&mut file, value)?;
    file.flush()?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    if let Some(parent) = path.parent() {
        fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

pub struct LogWriter {
    task_dir: PathBuf,
    stream: String,
    segmented: bool,
    offset: u64,
    file: fs::File,
}

impl LogWriter {
    pub fn task_directory(&self) -> &Path {
        &self.task_dir
    }
    pub fn create(path: &Path) -> io::Result<Self> {
        let task_dir = path
            .parent()
            .ok_or_else(|| io::Error::other("missing log directory"))?
            .to_owned();
        let stream = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| io::Error::other("invalid stream"))?
            .to_owned();
        if !matches!(stream.as_str(), "stdout" | "stderr") {
            return Err(io::Error::other("invalid stream"));
        }
        let segmented = enabled(&task_dir);
        let target = if segmented {
            task_dir.join(format!("{stream}.00000000.log"))
        } else {
            path.to_owned()
        };
        let file = open_log(&target)?;
        Ok(Self {
            task_dir,
            stream,
            segmented,
            offset: 0,
            file,
        })
    }
    pub fn append(path: &Path) -> io::Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;
        let task_dir = path
            .parent()
            .ok_or_else(|| io::Error::other("missing log directory"))?
            .to_owned();
        let stream = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| io::Error::other("invalid stream"))?
            .to_owned();
        if !matches!(stream.as_str(), "stdout" | "stderr") {
            return Err(io::Error::other("invalid stream"));
        }
        let segmented = enabled(&task_dir);
        let mut index = 0;
        let mut target = path.to_owned();
        if segmented {
            for entry in fs::read_dir(&task_dir)? {
                let entry = entry?;
                let name = entry.file_name();
                if let Some(number) = name
                    .to_str()
                    .and_then(|name| name.strip_prefix(&format!("{stream}.")))
                    .and_then(|name| name.strip_suffix(".log"))
                    && number.len() == 8
                    && let Ok(value) = number.parse::<u64>()
                {
                    index = index.max(value);
                }
            }
            target = task_dir.join(format!("{stream}.{index:08}.log"));
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o640)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(target)?;
        let offset = index * SEGMENT_BYTES + file.metadata()?.len();
        Ok(Self {
            task_dir,
            stream,
            segmented,
            offset,
            file,
        })
    }
    pub fn write_all(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        while !bytes.is_empty() {
            if self.segmented
                && self.offset.is_multiple_of(SEGMENT_BYTES)
                && delivery_owner_is_current_user(&self.task_dir)?
            {
                let root = self
                    .task_dir
                    .parent()
                    .ok_or_else(|| io::Error::other("missing task root"))?;
                if node_pending_bytes(root)?.saturating_add(SEGMENT_BYTES) > NODE_OUTBOX_LIMIT {
                    atomic_json(
                        &self.task_dir.join("log-budget-exceeded.json"),
                        &serde_json::json!({"error_code":"node_log_spool_budget_exceeded"}),
                    )?;
                    return Err(io::Error::other("node_log_spool_budget_exceeded"));
                }
            }
            if self.segmented && self.offset > 0 && self.offset.is_multiple_of(SEGMENT_BYTES) {
                self.file.sync_all()?;
                self.file = open_log(&self.task_dir.join(format!(
                    "{}.{:08}.log",
                    self.stream,
                    self.offset / SEGMENT_BYTES
                )))?;
            }
            let count = if self.segmented {
                bytes
                    .len()
                    .min((SEGMENT_BYTES - self.offset % SEGMENT_BYTES) as usize)
            } else {
                bytes.len()
            };
            self.file.write_all(&bytes[..count])?;
            self.offset += count as u64;
            bytes = &bytes[count..];
        }
        Ok(())
    }
    pub fn sync_all(&self) -> io::Result<()> {
        self.file.sync_all()
    }
}

fn delivery_owner_is_current_user(task_dir: &Path) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    // 降权 Runner 无权枚举 tasks 根目录；节点预算由 Agent 在任务准入时预留。
    Ok(fs::symlink_metadata(task_dir.join(STATE_FILE))?.uid() == unsafe { libc::geteuid() })
}

fn open_log(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o640)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}

pub struct LogReader {
    task_dir: PathBuf,
    stream: String,
    offset: u64,
}
impl LogReader {
    pub fn new(task_dir: &Path, stream: &str) -> Self {
        Self {
            task_dir: task_dir.to_owned(),
            stream: stream.to_owned(),
            offset: 0,
        }
    }
}
impl Read for LogReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let bytes = read_chunk(&self.task_dir, &self.stream, self.offset, buffer.len())?;
        buffer[..bytes.len()].copy_from_slice(&bytes);
        self.offset += bytes.len() as u64;
        Ok(bytes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{JournalState, JournalStore};
    use deploy_go_agent_protocol::{OutputStream, TaskOutput, TaskResult, TaskTerminalStatus};

    #[test]
    fn failed_initializations_do_not_exhaust_node_reservations() {
        let fixture = tempfile::tempdir().unwrap();
        for i in 0..4 {
            let dir = fixture.path().join(format!("task_orphan_{i}"));
            fs::create_dir(&dir).unwrap();
            crate::dir_guard::ensure_directory_mode(&dir, 0o3700, &[0o3700]).unwrap();
            initialize(&dir).unwrap();
        }
        let next = fixture.path().join("task_next");
        fs::create_dir(&next).unwrap();
        initialize(&next).expect("没有 journal 或输出的初始化残留不应占满节点额度");
    }

    #[test]
    fn committed_journals_still_reserve_node_capacity() {
        let fixture = tempfile::tempdir().unwrap();
        let store = JournalStore::new(fixture.path().join("tasks"));
        store.enable_reliable_logs(true);
        for i in 0..4 {
            store
                .create(
                    &format!("task_committed_{i}"),
                    &format!("idem_0123456789abcdef{i}"),
                    "sha256:0123456789abcdef",
                )
                .unwrap();
        }
        assert!(
            store
                .create(
                    "task_over_budget",
                    "idem_0123456789abcdef_extra",
                    "sha256:0123456789abcdef"
                )
                .is_err()
        );
    }

    #[test]
    fn reused_pristine_sidecar_must_recheck_capacity() {
        let fixture = tempfile::tempdir().unwrap();
        let store = JournalStore::new(fixture.path().join("tasks"));
        store.enable_reliable_logs(true);
        let orphan = store.task_dir("task_reused");
        fs::create_dir_all(&orphan).unwrap();
        crate::dir_guard::ensure_directory_mode(&orphan, 0o3700, &[0o3700]).unwrap();
        initialize(&orphan).unwrap();
        for i in 0..4 {
            store
                .create(
                    &format!("task_full_{i}"),
                    &format!("idem_0123456789abcdef{i}"),
                    "sha256:0123456789abcdef",
                )
                .unwrap();
        }
        assert!(
            store
                .create(
                    "task_reused",
                    "idem_0123456789abcdef_reused",
                    "sha256:0123456789abcdef"
                )
                .is_err()
        );
        assert!(!orphan.join("journal.json").exists());
    }

    #[test]
    fn malformed_initialization_residue_is_reserved_without_blocking_scan() {
        let fixture = tempfile::tempdir().unwrap();
        let store = JournalStore::new(fixture.path().join("tasks"));
        store.enable_reliable_logs(true);
        let orphan = store.task_dir("task_malformed");
        fs::create_dir_all(&orphan).unwrap();
        crate::dir_guard::ensure_directory_mode(&orphan, 0o3700, &[0o3700]).unwrap();
        let original = b"{broken";
        fs::write(orphan.join(STATE_FILE), original).unwrap();
        assert!(!is_uninitialized_task(&orphan).unwrap());
        for i in 0..3 {
            store
                .create(
                    &format!("task_valid_{i}"),
                    &format!("idem_0123456789abcdef{i}"),
                    "sha256:0123456789abcdef",
                )
                .unwrap();
        }
        assert!(
            store
                .create(
                    "task_overflow",
                    "idem_0123456789abcdef_overflow",
                    "sha256:0123456789abcdef"
                )
                .is_err()
        );
        assert_eq!(fs::read(orphan.join(STATE_FILE)).unwrap(), original);
    }

    #[test]
    fn uninitialized_detection_protects_unknown_files_and_active_modes() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = tempfile::tempdir().unwrap();
        let dir = fixture.path().join("task_residue");
        fs::create_dir(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o3700)).unwrap();
        initialize(&dir).unwrap();
        assert!(is_uninitialized_task(&dir).unwrap());
        for name in [
            "journal.json",
            "process.json",
            "runner-spec.json",
            "stdout.log",
            "log-outbox-v1",
            "unknown.part",
        ] {
            fs::write(dir.join(name), b"").unwrap();
            assert!(!is_uninitialized_task(&dir).unwrap());
            fs::remove_file(dir.join(name)).unwrap();
        }
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o3770)).unwrap();
        assert!(!is_uninitialized_task(&dir).unwrap());
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o3700)).unwrap();
        fs::remove_file(dir.join(STATE_FILE)).unwrap();
        std::os::unix::fs::symlink("/dev/null", dir.join(STATE_FILE)).unwrap();
        assert!(!is_uninitialized_task(&dir).unwrap());
    }

    fn fixture() -> (tempfile::TempDir, JournalStore, TaskJournal) {
        let dir = tempfile::tempdir().unwrap();
        let store = JournalStore::new(dir.path().join("tasks"));
        store.enable_reliable_logs(true);
        let journal = store
            .create(
                "task_logs",
                "idem_0123456789abcdef",
                "sha256:0123456789abcdef",
            )
            .unwrap();
        (dir, store, journal)
    }
    #[test]
    fn recovering_delivery_checkpoint_preserves_later_terminal_and_executor_state() {
        let (_dir, store, mut journal) = fixture();
        let dir = store.task_dir(&journal.task_id);
        let message = output(&mut journal, "durable");
        persist(&dir, &message, &journal).unwrap();
        let mut later = store.load(&journal.task_id).unwrap();
        later.last_sequence = 0;
        later.stdout_offset = 0;
        later.state = JournalState::Succeeded;
        later.exit_code = Some(0);
        later.external_output_sequence = 99;
        recover_checkpoint(&dir, &mut later).unwrap();
        assert_eq!(later.state, JournalState::Succeeded);
        assert_eq!(later.exit_code, Some(0));
        assert_eq!(later.external_output_sequence, 99);
        assert_eq!(later.stdout_offset, 7);
        assert_eq!(later.last_sequence, 1);
    }
    fn output(journal: &mut TaskJournal, text: &str) -> Message {
        journal.last_sequence += 1;
        journal.stdout_offset += text.len() as u64;
        Message::TaskOutput(TaskOutput {
            task_id: journal.task_id.clone(),
            sequence: journal.last_sequence,
            stream: OutputStream::Stdout,
            text: text.to_owned(),
        })
    }
    fn receipt(journal: &TaskJournal, message: &Message) -> TaskEventReceipt {
        TaskEventReceipt {
            task_id: journal.task_id.clone(),
            payload_digest: journal.payload_digest.clone(),
            sequence: event_identity(message).unwrap().1,
            message_digest: message_digest(message).unwrap(),
        }
    }

    #[test]
    fn outbox_recovers_offsets_before_journal_commit_and_counts_actual_files() {
        let (_dir, store, mut journal) = fixture();
        let path = store.task_dir(&journal.task_id);
        let first = output(&mut journal, "first\n");
        persist(&path, &first, &journal).unwrap();
        let mut state = read_state(&path).unwrap();
        state.pending_bytes = TASK_OUTBOX_LIMIT;
        atomic_json(&path.join(STATE_FILE), &state).unwrap();
        let mut recovered = store.load(&journal.task_id).unwrap();
        assert_eq!(recovered.stdout_offset, 6);
        assert_eq!(recovered.last_sequence, 1);
        let second = output(&mut recovered, "second\n");
        persist(&path, &second, &recovered).unwrap();
        assert_eq!(
            read_state(&path).unwrap().pending_bytes,
            actual_pending_bytes(&path).unwrap()
        );
        assert_eq!(pending_paths(&path).unwrap().len(), 2);
    }

    #[test]
    fn acknowledgements_require_contiguous_matching_durable_events() {
        let (_dir, store, mut journal) = fixture();
        let path = store.task_dir(&journal.task_id);
        let first = output(&mut journal, "one");
        persist(&path, &first, &journal).unwrap();
        let second = output(&mut journal, "two");
        persist(&path, &second, &journal).unwrap();
        assert!(acknowledge(&path, &journal, &receipt(&journal, &second)).is_err());
        let mut forged = receipt(&journal, &first);
        forged.message_digest = "sha256:wrong".to_owned();
        assert!(acknowledge(&path, &journal, &forged).is_err());
        acknowledge(&path, &journal, &receipt(&journal, &first)).unwrap();
        acknowledge(&path, &journal, &receipt(&journal, &first)).unwrap();
        assert_eq!(pending_paths(&path).unwrap().len(), 1);
        acknowledge(&path, &journal, &receipt(&journal, &second)).unwrap();
        assert!(pending_paths(&path).unwrap().is_empty());
        assert!(!finalized(&path));
    }

    #[test]
    fn terminal_unacknowledged_task_is_replayed_and_protected() {
        let (_dir, store, mut journal) = fixture();
        let path = store.task_dir(&journal.task_id);
        let mut writer = LogWriter::create(&path.join("stdout.log")).unwrap();
        writer.write_all(b"raw").unwrap();
        let first = output(&mut journal, "raw");
        persist(&path, &first, &journal).unwrap();
        journal.state = JournalState::Succeeded;
        journal.last_sequence = 2;
        journal.result_sequence = Some(2);
        let result = Message::TaskResult(TaskResult {
            task_id: journal.task_id.clone(),
            sequence: 2,
            status: TaskTerminalStatus::Succeeded,
            exit_code: Some(0),
            error_code: None,
            summary: None,
            data: None,
        });
        persist(&path, &result, &journal).unwrap();
        store.store(&journal).unwrap();
        assert!(store.active_task_ids().unwrap().is_empty());
        assert_eq!(
            store.delivery_task_ids().unwrap(),
            vec![journal.task_id.clone()]
        );
        assert!(cleanup_protected(&path));
        acknowledge(&path, &journal, &receipt(&journal, &first)).unwrap();
        assert!(path.join("stdout.00000000.log").exists());
        acknowledge(&path, &journal, &receipt(&journal, &result)).unwrap();
        assert!(finalized(&path));
        assert!(!path.join("stdout.00000000.log").exists());
        assert!(path.join("journal.json").exists());
        assert!(store.delivery_task_ids().unwrap().is_empty());
    }

    #[test]
    fn segmented_writer_reader_and_reclamation_preserve_current_segment() {
        let (_dir, store, mut journal) = fixture();
        let path = store.task_dir(&journal.task_id);
        let mut writer = LogWriter::create(&path.join("stdout.log")).unwrap();
        writer
            .write_all(&vec![b'a'; SEGMENT_BYTES as usize])
            .unwrap();
        journal.stdout_offset = SEGMENT_BYTES;
        reclaim_segments(&path, &journal).unwrap();
        assert!(path.join("stdout.00000000.log").exists());
        writer.write_all(b"tail").unwrap();
        writer.sync_all().unwrap();
        assert_eq!(
            read_chunk(&path, "stdout", SEGMENT_BYTES - 1, 16).unwrap(),
            b"a"
        );
        assert_eq!(
            read_chunk(&path, "stdout", SEGMENT_BYTES, 16).unwrap(),
            b"tail"
        );
        reclaim_segments(&path, &journal).unwrap();
        assert!(!path.join("stdout.00000000.log").exists());
        let mut append = LogWriter::append(&path.join("stdout.log")).unwrap();
        append.write_all(b"!").unwrap();
        assert_eq!(
            read_chunk(&path, "stdout", SEGMENT_BYTES, 16).unwrap(),
            b"tail!"
        );
        assert!(path.join("stdout.00000001.log").exists());
    }

    #[test]
    fn legacy_migration_is_resumable_and_never_resets_offsets() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("tasks");
        let store = JournalStore::new(root.clone());
        let mut journal = store
            .create(
                "task_old",
                "idem_0123456789abcdef",
                "sha256:0123456789abcdef",
            )
            .unwrap();
        journal.stdout_offset = 123;
        journal.last_sequence = 71;
        store.store(&journal).unwrap();
        let path = store.task_dir(&journal.task_id);
        fs::write(path.join("stdout.log"), b"legacy").unwrap();
        atomic_json(
            &path.join("legacy-log-retained-v1.json"),
            &serde_json::json!({"status":"retained"}),
        )
        .unwrap();
        retain_legacy_logs(&root).unwrap();
        let marker = fs::read(root.join("legacy-log-migration-v1.json")).unwrap();
        retain_legacy_logs(&root).unwrap();
        assert_eq!(
            fs::read(root.join("legacy-log-migration-v1.json")).unwrap(),
            marker
        );
        assert_eq!(store.load(&journal.task_id).unwrap().stdout_offset, 123);
        assert_eq!(store.load(&journal.task_id).unwrap().last_sequence, 71);
        assert!(!enabled(&path));
        assert!(cleanup_protected(&path));
        assert_eq!(fs::read(path.join("stdout.log")).unwrap(), b"legacy");
    }

    #[test]
    fn symlinks_and_invalid_streams_cannot_escape_log_storage() {
        let (dir, store, _journal) = fixture();
        let path = store.task_dir("task_logs");
        let outside = dir.path().join("outside");
        fs::write(&outside, b"protected").unwrap();
        std::os::unix::fs::symlink(&outside, path.join("stdout.00000000.log")).unwrap();
        assert!(read_chunk(&path, "stdout", 0, 16).is_err());
        assert!(LogWriter::create(&path.join("stdout.log")).is_err());
        assert!(read_chunk(&path, "../outside", 0, 16).is_err());
        let outside_dir = dir.path().join("outside-dir");
        fs::create_dir(&outside_dir).unwrap();
        std::os::unix::fs::symlink(&outside_dir, path.join("log-outbox-v1")).unwrap();
        assert!(pending_paths(&path).is_err());
        assert_eq!(fs::read(outside).unwrap(), b"protected");
    }
}
