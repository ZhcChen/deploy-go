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

    /// 顺序重放 `after` 之后的全部记录，见 [`replay`]。
    pub fn replay(
        &self,
        after: u64,
        batch_records: usize,
        batch_bytes: usize,
        on_batch: impl FnMut(Vec<LogRecord>) -> bool,
    ) -> io::Result<()> {
        replay(&self.root, after, batch_records, batch_bytes, on_batch)
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

/// 顺序重放 `after` 之后的全部记录：每个段只打开一次，每条记录只解析一次，
/// 按条数与字节数分批交给 `on_batch`；`on_batch` 返回 `false` 时提前结束。
///
/// 为什么不反复调用 [`read_page`]：它每次都要从所在段的开头解析到 `after` 才能继续，
/// 逐页读完一个段的复杂度是二次的（一个 32MiB 段约 6 万条记录，要解析数百万次）。
/// 中心日志启动恢复曾因此在 360MB 日志上耗时超过 3 分钟，超出安装器的健康检查而被判失败并回滚。
/// 批次边界与 `read_page` 一致（至多 `batch_records` 条、至多 `batch_bytes` 字节），
/// 但单条记录超过字节预算时仍独占一批，保证重放总能前进、不会静默中止。
pub fn replay(
    root: &Path,
    after: u64,
    batch_records: usize,
    batch_bytes: usize,
    mut on_batch: impl FnMut(Vec<LogRecord>) -> bool,
) -> io::Result<()> {
    check_directory(root)?;
    let state = state_at(root)?;
    let maximum = state.next_sequence - 1;
    if after >= maximum || batch_records == 0 || batch_bytes == 0 {
        return Ok(());
    }
    let paths = segments(root)?;
    let mut batch: Vec<LogRecord> = Vec::new();
    let mut used = 0_usize;
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
            if record.sequence > maximum {
                break;
            }
            let bytes = bytes as usize;
            if !batch.is_empty() && (batch.len() >= batch_records || used + bytes > batch_bytes) {
                used = 0;
                if !on_batch(std::mem::take(&mut batch)) {
                    return Ok(());
                }
            }
            used += bytes;
            batch.push(record);
        }
    }
    if !batch.is_empty() {
        on_batch(batch);
    }
    Ok(())
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

    /// 多段、有淘汰的日志：重放结果必须与逐页 `read_page` 读出的完全一致。
    fn multi_segment_store() -> (tempfile::TempDir, PathBuf, LogPolicy, u64) {
        let temp = directory();
        let root = temp.path().canonicalize().unwrap();
        let policy = LogPolicy {
            segment_bytes: 600,
            total_bytes: METADATA_RESERVE + 2400,
        };
        let mut store = SegmentStore::open(root.clone(), policy).unwrap();
        for index in 0..60 {
            let mut item = record();
            item.message = format!("diagnostic-{index}");
            store.append(item).unwrap();
        }
        let maximum = store.maximum();
        (temp, root, policy, maximum)
    }

    fn paged(root: &Path, after: u64) -> Vec<u64> {
        let (mut cursor, mut all) = (after, Vec::new());
        loop {
            let page = read_page(root, cursor, 7, 1_000_000).unwrap();
            let Some(last) = page.entries.last() else {
                return all;
            };
            cursor = last.sequence;
            all.extend(page.entries.iter().map(|entry| entry.sequence));
        }
    }

    fn replayed(root: &Path, after: u64, records: usize, bytes: usize) -> (Vec<u64>, Vec<usize>) {
        let (mut all, mut sizes) = (Vec::new(), Vec::new());
        replay(root, after, records, bytes, |batch| {
            sizes.push(batch.len());
            all.extend(batch.iter().map(|entry| entry.sequence));
            true
        })
        .unwrap();
        (all, sizes)
    }

    #[test]
    fn replay_matches_paged_reads_across_segments_and_cursors() {
        let (_temp, root, _policy, maximum) = multi_segment_store();
        let first = read_page(&root, 0, 1, 1_000_000).unwrap().entries[0].sequence;
        assert!(first > 1, "夹具应已淘汰最早的段");
        // 游标：从头、落在段中间、恰为某段最后一条、最后一条、已越过末尾。
        for after in [
            0,
            first,
            first + 3,
            first + 9,
            maximum - 1,
            maximum,
            maximum + 5,
        ] {
            let (all, _) = replayed(&root, after, 7, 1_000_000);
            assert_eq!(all, paged(&root, after), "after={after}");
            let expected: Vec<u64> = (after.max(first - 1) + 1..=maximum).collect();
            assert_eq!(all, expected, "after={after}");
        }
    }

    #[test]
    fn replay_batches_respect_record_and_byte_limits_and_always_progress() {
        let (_temp, root, _policy, maximum) = multi_segment_store();
        let (all, sizes) = replayed(&root, 0, 7, 1_000_000);
        assert_eq!(*all.last().unwrap(), maximum);
        assert!(sizes.iter().all(|size| *size <= 7));
        assert_eq!(sizes.iter().sum::<usize>(), all.len());
        // 字节预算小于一条记录：每批仍至少一条，不会死循环或静默中止。
        let (tiny, tiny_sizes) = replayed(&root, 0, 100, 1);
        assert_eq!(tiny, all);
        assert!(tiny_sizes.iter().all(|size| *size == 1));
        // 字节预算只容得下两三条。
        let one = serde_json::to_vec(&record()).unwrap().len() + 40;
        let (limited, limited_sizes) = replayed(&root, 0, 100, one * 2);
        assert_eq!(limited, all);
        assert!(limited_sizes.iter().all(|size| *size <= 3));
        // 回调返回 false 即停止，后续记录不再交付。
        let mut seen = 0;
        replay(&root, 0, 5, 1_000_000, |batch| {
            seen += batch.len();
            false
        })
        .unwrap();
        assert_eq!(seen, 5);
        // 0 预算与空范围不触发回调。
        let mut calls = 0;
        replay(&root, 0, 0, 1_000_000, |_| {
            calls += 1;
            true
        })
        .unwrap();
        replay(&root, maximum, 7, 1_000_000, |_| {
            calls += 1;
            true
        })
        .unwrap();
        assert_eq!(calls, 0);
    }

    #[test]
    fn replay_reports_a_partial_tail_instead_of_skipping_it() {
        let temp = directory();
        let root = temp.path().canonicalize().unwrap();
        let mut store = SegmentStore::open(root.clone(), LogPolicy::NODE).unwrap();
        for _ in 0..3 {
            store.append(record()).unwrap();
        }
        let segment = root.join(format!("{:020}.jsonl", 1));
        let mut file = fs::OpenOptions::new().append(true).open(&segment).unwrap();
        file.write_all(b"{\"sequence\":4,\"timestamp\"").unwrap();
        drop(file);
        // 部分写入的尾部必须暴露为错误（与 read_page 一致），不能被当作“读完了”。
        let Err(replay_error) = replay(&root, 0, 10, 1_000_000, |_| true) else {
            panic!("部分写入的尾部被 replay 当作读完了");
        };
        let Err(paged_error) = read_page(&root, 0, 10, 1_000_000) else {
            panic!("部分写入的尾部被 read_page 当作读完了");
        };
        assert_eq!(replay_error.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(replay_error.kind(), paged_error.kind());
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
