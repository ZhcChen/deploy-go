use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::MAX_RECORD_BYTES;

const METADATA_RESERVE: u64 = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogRecord {
    pub sequence: u64,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub request_id: Option<String>,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Clone, Copy)]
pub struct LogPolicy {
    pub segment_bytes: u64,
    pub total_bytes: u64,
}

impl LogPolicy {
    pub const NODE: Self = Self {
        segment_bytes: 16 * 1024 * 1024,
        total_bytes: 32 * 1024 * 1024,
    };
    pub const CENTRAL: Self = Self {
        segment_bytes: 32 * 1024 * 1024,
        total_bytes: 512 * 1024 * 1024,
    };
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreState {
    epoch: String,
    next_sequence: u64,
    evicted_bytes: u64,
}

pub struct SegmentStore {
    root: PathBuf,
    policy: LogPolicy,
    state: StoreState,
    poisoned: bool,
    tail_next_sequence: Option<u64>,
    _lock: File,
}

pub struct LogPage {
    pub epoch: String,
    pub entries: Vec<LogRecord>,
    pub minimum: Option<u64>,
    pub maximum: u64,
    pub evicted_bytes: u64,
}

fn invalid() -> io::Error {
    io::Error::other("runtime_log_storage_invalid")
}

fn check_directory(root: &Path) -> io::Result<()> {
    if !root.is_absolute() {
        return Err(invalid());
    }
    let owner = fs::symlink_metadata(root)?.uid();
    for parent in root.ancestors() {
        let meta = fs::symlink_metadata(parent)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(invalid());
        }
        if meta.uid() != 0 && meta.uid() != owner {
            return Err(invalid());
        }
        // 测试临时目录允许 root 拥有的 sticky 父目录；生产专属树仍禁止他人写入。
        if meta.mode() & 0o022 != 0
            && !(meta.uid() == 0 && meta.mode() & 0o1000 != 0 && parent != root)
        {
            return Err(invalid());
        }
    }
    let meta = fs::metadata(root)?;
    if meta.mode() & 0o022 != 0 {
        return Err(invalid());
    }
    Ok(())
}

fn open_file(path: &Path, write: bool, create: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    let directory = fs::metadata(path.parent().ok_or_else(invalid)?)?;
    let mode = if directory.mode() & 0o070 == 0 {
        0o600
    } else {
        0o640
    };
    options
        .read(true)
        .write(write)
        .create(create)
        .mode(mode)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.nlink() != 1
        || meta.uid() != directory.uid()
        || meta.mode() & 0o027 != 0
    {
        return Err(invalid());
    }
    if write
        && meta.gid() != directory.gid()
        && unsafe { libc::fchown(file.as_raw_fd(), u32::MAX, directory.gid()) } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}

fn state_at(root: &Path) -> io::Result<StoreState> {
    let file = open_file(&root.join("state.json"), false, false)?;
    if file.metadata()?.len() > METADATA_RESERVE {
        return Err(invalid());
    }
    let state: StoreState = serde_json::from_reader(file).map_err(|_| invalid())?;
    if state.next_sequence == 0 || state.epoch.parse::<ulid::Ulid>().is_err() {
        return Err(invalid());
    }
    Ok(state)
}

fn segments(root: &Path) -> io::Result<Vec<(u64, PathBuf, u64)>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(invalid());
        };
        if let Some(start) = name.strip_suffix(".jsonl") {
            if start.len() != 20 || !start.bytes().all(|v| v.is_ascii_digit()) {
                return Err(invalid());
            }
            let start = start.parse::<u64>().map_err(|_| invalid())?;
            if start == 0 || result.len() >= 1024 {
                return Err(invalid());
            }
            let file = open_file(&entry.path(), false, false)?;
            result.push((start, entry.path(), file.metadata()?.len()));
        } else if !matches!(name, "state.json" | "state.next" | ".lock") {
            return Err(invalid());
        }
    }
    result.sort_by_key(|entry| entry.0);
    Ok(result)
}

fn read_record(reader: &mut BufReader<File>) -> io::Result<Option<(LogRecord, u64)>> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_RECORD_BYTES + 1) as u64)
        .read_until(b'\n', &mut bytes)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(invalid());
    }
    if bytes.last() != Some(&b'\n') {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "runtime_log_partial_tail",
        ));
    }
    let record: LogRecord = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if record.sequence == 0 {
        return Err(invalid());
    }
    Ok(Some((record, bytes.len() as u64)))
}

impl SegmentStore {
    pub fn open(root: PathBuf, policy: LogPolicy) -> io::Result<Self> {
        check_directory(&root)?;
        if policy.segment_bytes < 256
            || policy.total_bytes < policy.segment_bytes + METADATA_RESERVE
        {
            return Err(invalid());
        }
        let lock = open_file(&root.join(".lock"), true, true)?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::other("runtime_log_writer_busy"));
        }
        let paths = segments(&root)?;
        let mut state = match state_at(&root) {
            Ok(state) => state,
            Err(error) if error.kind() == io::ErrorKind::NotFound && paths.is_empty() => {
                StoreState {
                    epoch: ulid::Ulid::new().to_string(),
                    next_sequence: 1,
                    evicted_bytes: 0,
                }
            }
            Err(error) => return Err(error),
        };
        let mut tail_next_sequence = None;
        if let Some((start, path, _)) = paths.last() {
            let mut reader = BufReader::new(open_file(path, false, false)?);
            let mut length = 0;
            let mut expected = *start;
            loop {
                match read_record(&mut reader) {
                    Ok(Some((record, bytes))) if record.sequence == expected => {
                        length += bytes;
                        expected = expected.checked_add(1).ok_or_else(invalid)?;
                    }
                    Ok(None) => break,
                    Ok(Some(_)) => return Err(invalid()),
                    Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                        let file = open_file(path, true, false)?;
                        state.evicted_bytes = state
                            .evicted_bytes
                            .saturating_add(file.metadata()?.len().saturating_sub(length));
                        file.set_len(length)?;
                        file.sync_all()?;
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }
            tail_next_sequence = Some(expected);
            state.next_sequence = state.next_sequence.max(expected);
        }
        let mut store = Self {
            root,
            policy,
            state,
            poisoned: false,
            tail_next_sequence,
            _lock: lock,
        };
        store.reclaim(0, None)?;
        store.save_state()?;
        Ok(store)
    }

    pub fn epoch(&self) -> &str {
        &self.state.epoch
    }
    pub fn evicted_bytes(&self) -> u64 {
        self.state.evicted_bytes
    }
    pub fn maximum(&self) -> u64 {
        self.state.next_sequence - 1
    }

    fn save_state(&self) -> io::Result<()> {
        let mut file = open_file(&self.root.join("state.next"), true, true)?;
        file.set_len(0)?;
        serde_json::to_writer(&mut file, &self.state).map_err(|_| invalid())?;
        file.sync_all()?;
        fs::rename(self.root.join("state.next"), self.root.join("state.json"))?;
        File::open(&self.root)?.sync_all()
    }

    fn reclaim(&mut self, additional: u64, protected: Option<&Path>) -> io::Result<()> {
        let paths = segments(&self.root)?;
        let mut total = paths.iter().map(|v| v.2).sum::<u64>();
        let max_segments = (self.policy.total_bytes - METADATA_RESERVE)
            .div_ceil(self.policy.segment_bytes)
            .min(1024) as usize;
        let extra_segment =
            usize::from(protected.is_some_and(|path| !paths.iter().any(|entry| entry.1 == path)));
        let mut count = paths.len() + extra_segment;
        for (_, path, length) in paths {
            if count <= max_segments
                && total.saturating_add(additional) <= self.policy.total_bytes - METADATA_RESERVE
            {
                break;
            }
            if protected == Some(path.as_path()) {
                continue;
            }
            // 日志目录只有固定 writer 可写，采集器只读；删除前再次核对普通文件。
            drop(open_file(&path, false, false)?);
            fs::remove_file(path)?;
            count = count.saturating_sub(1);
            total = total.saturating_sub(length);
            self.state.evicted_bytes = self.state.evicted_bytes.saturating_add(length);
        }
        if total.saturating_add(additional) > self.policy.total_bytes - METADATA_RESERVE {
            return Err(invalid());
        }
        Ok(())
    }

    pub fn append(&mut self, mut record: LogRecord) -> io::Result<LogRecord> {
        if self.poisoned {
            return Err(io::Error::other("runtime_log_writer_failed"));
        }
        record.sequence = self.state.next_sequence;
        let mut bytes = serde_json::to_vec(&record).map_err(|_| invalid())?;
        bytes.push(b'\n');
        if bytes.len() > MAX_RECORD_BYTES || bytes.len() as u64 > self.policy.segment_bytes {
            return Err(invalid());
        }
        let paths = segments(&self.root)?;
        let path = match paths.last() {
            Some((_, path, size))
                if self.tail_next_sequence == Some(record.sequence)
                    && size + bytes.len() as u64 <= self.policy.segment_bytes =>
            {
                path.clone()
            }
            _ => self.root.join(format!("{:020}.jsonl", record.sequence)),
        };
        // 发生部分写入或持久状态失败后关闭本实例；重启从最后完整记录恢复，避免复用序号。
        let result = (|| {
            self.reclaim(bytes.len() as u64, Some(&path))?;
            let mut file = open_file(&path, true, true)?;
            use std::io::Seek;
            file.seek(std::io::SeekFrom::End(0))?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            self.state.next_sequence = self
                .state
                .next_sequence
                .checked_add(1)
                .ok_or_else(invalid)?;
            self.tail_next_sequence = Some(self.state.next_sequence);
            self.save_state()
        })();
        if let Err(error) = result {
            self.poisoned = true;
            return Err(error);
        }
        Ok(record)
    }

    pub fn page(&self, after: u64, limit: usize, byte_limit: usize) -> io::Result<LogPage> {
        read_page(&self.root, after, limit, byte_limit)
    }
}

pub fn read_page(root: &Path, after: u64, limit: usize, byte_limit: usize) -> io::Result<LogPage> {
    check_directory(root)?;
    let state = state_at(root)?;
    let paths = segments(root)?;
    let mut page = LogPage {
        epoch: state.epoch,
        entries: Vec::new(),
        minimum: paths.first().map(|v| v.0),
        maximum: state.next_sequence - 1,
        evicted_bytes: state.evicted_bytes,
    };
    // 空闲采集与 SSE 边界查询只读元数据，避免反复扫描当前整个分段。
    if after >= page.maximum || limit == 0 || byte_limit == 0 {
        return Ok(page);
    }
    let mut used = 0;
    for (index, (_, path, _)) in paths.iter().enumerate() {
        if paths
            .get(index + 1)
            .is_some_and(|next| next.0 <= after.saturating_add(1))
        {
            continue;
        }
        let mut reader = BufReader::new(open_file(path, false, false)?);
        while let Some((record, bytes)) = read_record(&mut reader)? {
            if record.sequence <= after {
                continue;
            }
            if record.sequence > page.maximum {
                break;
            }
            if page.entries.len() >= limit.min(500)
                || used + bytes as usize > byte_limit.min(8 * 1024 * 1024)
            {
                return Ok(page);
            }
            used += bytes as usize;
            page.entries.push(record);
        }
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    fn record() -> LogRecord {
        LogRecord {
            sequence: 0,
            timestamp: "2026-10-08T00:00:00Z".into(),
            level: "INFO".into(),
            target: "fixture".into(),
            message: "diagnostic".into(),
            request_id: None,
            fields: BTreeMap::new(),
        }
    }
    fn directory() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        root
    }

    #[test]
    fn rotation_keeps_budget_and_sequence_after_restart() {
        let temp = directory();
        let root = temp.path().canonicalize().unwrap();
        let policy = LogPolicy {
            segment_bytes: 600,
            total_bytes: METADATA_RESERVE + 1200,
        };
        let mut store = SegmentStore::open(root.clone(), policy).unwrap();
        let epoch = store.epoch().to_owned();
        for _ in 0..40 {
            store.append(record()).unwrap();
        }
        let page = store.page(0, 100, 10000).unwrap();
        assert!(page.minimum.unwrap() > 1);
        assert_eq!(page.maximum, 40);
        assert!(page.evicted_bytes > 0);
        let size: u64 = fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().metadata().unwrap().len())
            .sum();
        assert!(size <= policy.total_bytes);
        drop(store);
        let mut store = SegmentStore::open(root, policy).unwrap();
        assert_eq!(store.epoch(), epoch);
        assert_eq!(store.append(record()).unwrap().sequence, 41);
    }

    #[test]
    fn partial_tail_is_recovered_and_links_are_rejected() {
        let temp = directory();
        let root = temp.path().canonicalize().unwrap();
        let mut store = SegmentStore::open(root.clone(), LogPolicy::NODE).unwrap();
        store.append(record()).unwrap();
        drop(store);
        let path = segments(&root).unwrap().pop().unwrap().1;
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"{partial")
            .unwrap();
        let mut store = SegmentStore::open(root.clone(), LogPolicy::NODE).unwrap();
        assert_eq!(store.append(record()).unwrap().sequence, 2);
        drop(store);
        fs::remove_file(&path).unwrap();
        symlink("/etc/passwd", &path).unwrap();
        assert!(SegmentStore::open(root, LogPolicy::NODE).is_err());
    }

    #[test]
    fn huge_record_and_second_writer_are_rejected() {
        let temp = directory();
        let root = temp.path().canonicalize().unwrap();
        let mut store = SegmentStore::open(root.clone(), LogPolicy::NODE).unwrap();
        assert!(SegmentStore::open(root, LogPolicy::NODE).is_err());
        let mut oversized = record();
        oversized.message = "x".repeat(MAX_RECORD_BYTES);
        assert!(store.append(oversized).is_err());
        assert_eq!(store.maximum(), 0);
    }

    #[test]
    fn complete_corruption_never_truncates_later_records() {
        let temp = directory();
        let root = temp.path().canonicalize().unwrap();
        let mut store = SegmentStore::open(root.clone(), LogPolicy::NODE).unwrap();
        for _ in 0..3 {
            store.append(record()).unwrap();
        }
        drop(store);
        let path = segments(&root).unwrap()[0].1.clone();
        let mut bytes = fs::read(&path).unwrap();
        bytes[0] = b'!';
        fs::write(&path, &bytes).unwrap();
        assert!(SegmentStore::open(root, LogPolicy::NODE).is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn truncated_acknowledged_tail_creates_new_segment_without_reusing_sequence() {
        let temp = directory();
        let root = temp.path().canonicalize().unwrap();
        let mut store = SegmentStore::open(root.clone(), LogPolicy::NODE).unwrap();
        store.append(record()).unwrap();
        store.append(record()).unwrap();
        let first_length = serde_json::to_vec(&store.page(0, 1, 10000).unwrap().entries[0])
            .unwrap()
            .len()
            + 1;
        drop(store);
        let path = segments(&root).unwrap()[0].1.clone();
        open_file(&path, true, false)
            .unwrap()
            .set_len(first_length as u64 + 10)
            .unwrap();
        let mut store = SegmentStore::open(root.clone(), LogPolicy::NODE).unwrap();
        assert_eq!(store.append(record()).unwrap().sequence, 3);
        assert_eq!(segments(&root).unwrap().len(), 2);
        assert!(store.evicted_bytes() > 0);
        drop(store);
        let mut store = SegmentStore::open(root, LogPolicy::NODE).unwrap();
        assert_eq!(store.append(record()).unwrap().sequence, 4);
        assert_eq!(
            store
                .page(0, 10, 10000)
                .unwrap()
                .entries
                .iter()
                .map(|record| record.sequence)
                .collect::<Vec<_>>(),
            vec![1, 3, 4]
        );
    }
}
