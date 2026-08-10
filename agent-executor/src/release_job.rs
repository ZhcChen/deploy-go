use crate::{
    protocol::{ReleaseJobState, ReleaseOutputFrame, ReleaseOutputStream},
    release::{ReleaseAdmissionError, SealedRelease},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    os::unix::{fs::OpenOptionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const STATE_FILE: &str = "state.json";
const OUTPUT_FILE: &str = "output.jsonl";
pub const DEFAULT_JOB_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
pub const DEFAULT_MAX_OUTPUT_FRAMES: u16 = 256;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseJobSnapshot {
    pub job_id: String,
    pub task_payload_digest: String,
    pub state: ReleaseJobState,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub reason: Option<String>,
    pub last_sequence: u64,
    pub output_truncated: bool,
    pub deadline_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseOutputBatch {
    pub frames: Vec<ReleaseOutputFrame>,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReleaseJobError {
    #[error("release job does not exist")]
    NotFound,
    #[error("release job payload conflicts with durable state")]
    Conflict,
    #[error("release job storage unavailable")]
    Storage,
    #[error("release process failed to start")]
    Spawn,
    #[error("release job is not recoverable")]
    RecoveryBlocked,
    #[error("release request is invalid")]
    Invalid,
}

#[derive(Clone)]
pub struct ReleaseJobManager {
    jobs_root: PathBuf,
    controls: Arc<Mutex<HashMap<String, Arc<JobControl>>>>,
    output_limit: u64,
    close_grace: Duration,
}

struct JobControl {
    child: Mutex<Child>,
    cancel_requested: AtomicBool,
    output_overflowed: AtomicBool,
    #[cfg(target_os = "linux")]
    cgroup: crate::cgroup::ReleaseCgroup,
}

struct OutputJournal {
    file: File,
    sequence: u64,
    bytes: u64,
    limit: u64,
}

impl ReleaseJobManager {
    pub fn new(jobs_root: PathBuf) -> Self {
        Self {
            jobs_root,
            controls: Arc::new(Mutex::new(HashMap::new())),
            output_limit: DEFAULT_JOB_OUTPUT_BYTES,
            close_grace: Duration::from_secs(2),
        }
    }

    pub fn with_limits(mut self, output_limit: u64, close_grace: Duration) -> Self {
        self.output_limit = output_limit;
        self.close_grace = close_grace;
        self
    }

    pub fn start(
        &self,
        sealed: SealedRelease,
        target_code: &str,
    ) -> Result<ReleaseJobSnapshot, ReleaseJobError> {
        let job_id = sealed
            .job_dir
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(ReleaseJobError::Invalid)?
            .to_owned();
        let state_path = sealed.job_dir.join(STATE_FILE);
        if state_path.exists() {
            let existing = read_state(&state_path)?;
            if existing.task_payload_digest == sealed.claims.task_payload_digest {
                return Ok(existing);
            }
            return Err(ReleaseJobError::Conflict);
        }
        let now = unix_time();
        let initial = ReleaseJobSnapshot {
            job_id: job_id.clone(),
            task_payload_digest: sealed.claims.task_payload_digest.clone(),
            state: ReleaseJobState::Sealing,
            pid: None,
            exit_code: None,
            reason: None,
            last_sequence: 0,
            output_truncated: false,
            deadline_at: sealed.claims.deadline_at,
            updated_at: now,
        };
        write_state(&state_path, &initial)?;

        #[cfg(target_os = "linux")]
        let cgroup =
            crate::cgroup::ReleaseCgroup::create(&job_id).map_err(|_| ReleaseJobError::Spawn)?;
        #[cfg(target_os = "linux")]
        let (launcher, launcher_arguments) = cgroup.launcher_command();
        #[cfg(target_os = "linux")]
        let mut command = sealed
            .command_for(&launcher, &launcher_arguments, target_code)
            .map_err(map_admission_error)?;
        #[cfg(not(target_os = "linux"))]
        let mut command = sealed.command(target_code).map_err(map_admission_error)?;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // SAFETY: setpgid is async-signal-safe and does not allocate in the child before exec.
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().map_err(|_| ReleaseJobError::Spawn)?;
        let stdout = child.stdout.take().ok_or(ReleaseJobError::Spawn)?;
        let stderr = child.stderr.take().ok_or(ReleaseJobError::Spawn)?;
        let pid = child.id();
        let running = ReleaseJobSnapshot {
            state: ReleaseJobState::Running,
            pid: Some(pid),
            updated_at: unix_time(),
            ..initial
        };
        write_state(&state_path, &running)?;
        let control = Arc::new(JobControl {
            child: Mutex::new(child),
            cancel_requested: AtomicBool::new(false),
            output_overflowed: AtomicBool::new(false),
            #[cfg(target_os = "linux")]
            cgroup,
        });
        self.controls
            .lock()
            .map_err(|_| ReleaseJobError::Storage)?
            .insert(job_id.clone(), Arc::clone(&control));
        let journal = Arc::new(Mutex::new(OutputJournal::open(
            &sealed.job_dir.join(OUTPUT_FILE),
            self.output_limit,
        )?));
        let stdout_thread = spawn_output_reader(
            stdout,
            ReleaseOutputStream::Stdout,
            Arc::clone(&journal),
            Arc::clone(&control),
        );
        let stderr_thread = spawn_output_reader(
            stderr,
            ReleaseOutputStream::Stderr,
            Arc::clone(&journal),
            Arc::clone(&control),
        );
        let controls = Arc::clone(&self.controls);
        let close_grace = self.close_grace;
        thread::spawn(move || {
            monitor_job(
                &sealed.job_dir,
                &job_id,
                control,
                journal,
                stdout_thread,
                stderr_thread,
                close_grace,
                controls,
            );
        });
        Ok(running)
    }

    pub fn status(
        &self,
        job_id: &str,
        task_payload_digest: &str,
    ) -> Result<ReleaseJobSnapshot, ReleaseJobError> {
        let state = read_state(&self.job_path(job_id)?.join(STATE_FILE))?;
        if state.task_payload_digest != task_payload_digest {
            return Err(ReleaseJobError::Conflict);
        }
        Ok(state)
    }

    pub fn output(
        &self,
        job_id: &str,
        task_payload_digest: &str,
        after_sequence: u64,
        max_frames: u16,
    ) -> Result<ReleaseOutputBatch, ReleaseJobError> {
        let state = self.status(job_id, task_payload_digest)?;
        let maximum = max_frames.clamp(1, DEFAULT_MAX_OUTPUT_FRAMES) as usize;
        let path = self.job_path(job_id)?.join(OUTPUT_FILE);
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ReleaseOutputBatch {
                    frames: Vec::new(),
                    truncated: state.output_truncated,
                });
            }
            Err(_) => return Err(ReleaseJobError::Storage),
        };
        let mut frames = Vec::new();
        for line in BufReader::new(file).lines() {
            let frame: ReleaseOutputFrame =
                serde_json::from_str(&line.map_err(|_| ReleaseJobError::Storage)?)
                    .map_err(|_| ReleaseJobError::Storage)?;
            if frame.sequence > after_sequence {
                frames.push(frame);
                if frames.len() == maximum {
                    break;
                }
            }
        }
        Ok(ReleaseOutputBatch {
            frames,
            truncated: state.output_truncated,
        })
    }

    pub fn cancel(
        &self,
        job_id: &str,
        task_payload_digest: &str,
    ) -> Result<ReleaseJobSnapshot, ReleaseJobError> {
        let state = self.status(job_id, task_payload_digest)?;
        if terminal(state.state) {
            return Ok(state);
        }
        let controls = self.controls.lock().map_err(|_| ReleaseJobError::Storage)?;
        let control = controls
            .get(job_id)
            .ok_or(ReleaseJobError::RecoveryBlocked)?;
        control.cancel_requested.store(true, Ordering::Release);
        Ok(state)
    }

    pub fn reconcile_after_restart(&self) -> Result<(), ReleaseJobError> {
        if !self.jobs_root.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.jobs_root).map_err(|_| ReleaseJobError::Storage)? {
            let entry = entry.map_err(|_| ReleaseJobError::Storage)?;
            if entry.file_name().to_string_lossy().starts_with('.') || !entry.path().is_dir() {
                continue;
            }
            let state_path = entry.path().join(STATE_FILE);
            if !state_path.exists() {
                continue;
            }
            let mut state = read_state(&state_path)?;
            if terminal(state.state) {
                continue;
            }
            if state.pid.is_some_and(process_exists) {
                return Err(ReleaseJobError::RecoveryBlocked);
            }
            state.state = ReleaseJobState::Failed;
            state.reason = Some("executor_restarted".into());
            state.updated_at = unix_time();
            write_state(&state_path, &state)?;
        }
        Ok(())
    }

    fn job_path(&self, job_id: &str) -> Result<PathBuf, ReleaseJobError> {
        if !job_id.starts_with("release_")
            || job_id.len() > 128
            || !job_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(ReleaseJobError::Invalid);
        }
        let path = self.jobs_root.join(job_id);
        if !path.is_dir() {
            return Err(ReleaseJobError::NotFound);
        }
        Ok(path)
    }
}

impl OutputJournal {
    fn open(path: &Path, limit: u64) -> Result<Self, ReleaseJobError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)
            .map_err(|_| ReleaseJobError::Storage)?;
        Ok(Self {
            file,
            sequence: 0,
            bytes: 0,
            limit,
        })
    }

    fn append(
        &mut self,
        stream: ReleaseOutputStream,
        data: Vec<u8>,
    ) -> Result<bool, ReleaseJobError> {
        let next_sequence = self.sequence.saturating_add(1);
        let frame = ReleaseOutputFrame {
            sequence: next_sequence,
            stream,
            data,
        };
        let mut encoded = serde_json::to_vec(&frame).map_err(|_| ReleaseJobError::Storage)?;
        encoded.push(b'\n');
        if self.bytes.saturating_add(encoded.len() as u64) > self.limit {
            return Ok(false);
        }
        self.file
            .write_all(&encoded)
            .map_err(|_| ReleaseJobError::Storage)?;
        self.file.flush().map_err(|_| ReleaseJobError::Storage)?;
        self.sequence = next_sequence;
        self.bytes += encoded.len() as u64;
        Ok(true)
    }
}

fn spawn_output_reader<R: Read + Send + 'static>(
    mut reader: R,
    stream: ReleaseOutputStream,
    journal: Arc<Mutex<OutputJournal>>,
    control: Arc<JobControl>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8 * 1024];
        loop {
            let read = match reader.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(read) => read,
            };
            let accepted = journal
                .lock()
                .ok()
                .and_then(|mut journal| journal.append(stream, buffer[..read].to_vec()).ok())
                .unwrap_or(false);
            if !accepted {
                control.output_overflowed.store(true, Ordering::Release);
                return;
            }
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn monitor_job(
    job_dir: &Path,
    job_id: &str,
    control: Arc<JobControl>,
    journal: Arc<Mutex<OutputJournal>>,
    stdout_thread: thread::JoinHandle<()>,
    stderr_thread: thread::JoinHandle<()>,
    close_grace: Duration,
    controls: Arc<Mutex<HashMap<String, Arc<JobControl>>>>,
) {
    let state_path = job_dir.join(STATE_FILE);
    let mut state = match read_state(&state_path) {
        Ok(state) => state,
        Err(_) => return,
    };
    let (job_state, reason) = loop {
        let cancel = control.cancel_requested.load(Ordering::Acquire);
        let overflow = control.output_overflowed.load(Ordering::Acquire);
        let timed_out = unix_time() >= state.deadline_at;
        if cancel || overflow || timed_out {
            terminate_process_group(&control, close_grace);
            break if timed_out {
                (ReleaseJobState::TimedOut, "deadline_exceeded")
            } else if overflow {
                (ReleaseJobState::Failed, "output_limit_exceeded")
            } else {
                (ReleaseJobState::Canceled, "cancel_requested")
            };
        }
        let result = control
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok().flatten());
        if let Some(result) = result {
            let terminal = if result.success() {
                (ReleaseJobState::Succeeded, "process_exited")
            } else {
                (ReleaseJobState::Failed, "process_exited")
            };
            state.exit_code = result.code();
            break terminal;
        }
        thread::sleep(Duration::from_millis(20));
    };
    #[cfg(target_os = "linux")]
    let _ = control.cgroup.kill_all();
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    if let Ok(journal) = journal.lock() {
        state.last_sequence = journal.sequence;
        state.output_truncated = control.output_overflowed.load(Ordering::Acquire);
    }
    state.state = job_state;
    state.reason = Some(reason.into());
    state.updated_at = unix_time();
    let _ = write_state(&state_path, &state);
    if let Ok(mut controls) = controls.lock() {
        controls.remove(job_id);
    }
}

fn terminate_process_group(control: &JobControl, grace: Duration) {
    let pid = match control.child.lock() {
        Ok(child) => child.id(),
        Err(_) => return,
    };
    let _ = unsafe { libc::kill(-(pid as i32), libc::SIGTERM) };
    let deadline = std::time::Instant::now() + grace;
    loop {
        let exited = control
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok().flatten())
            .is_some();
        if exited {
            return;
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let _ = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
    #[cfg(target_os = "linux")]
    let _ = control.cgroup.kill_all();
    if let Ok(mut child) = control.child.lock() {
        let _ = child.wait();
    }
}

fn write_state(path: &Path, state: &ReleaseJobSnapshot) -> Result<(), ReleaseJobError> {
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec(state).map_err(|_| ReleaseJobError::Storage)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| ReleaseJobError::Storage)?;
    file.write_all(&bytes)
        .map_err(|_| ReleaseJobError::Storage)?;
    file.sync_all().map_err(|_| ReleaseJobError::Storage)?;
    fs::rename(temporary, path).map_err(|_| ReleaseJobError::Storage)
}

fn read_state(path: &Path) -> Result<ReleaseJobSnapshot, ReleaseJobError> {
    serde_json::from_slice(&fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ReleaseJobError::NotFound
        } else {
            ReleaseJobError::Storage
        }
    })?)
    .map_err(|_| ReleaseJobError::Storage)
}

fn terminal(state: ReleaseJobState) -> bool {
    matches!(
        state,
        ReleaseJobState::Succeeded
            | ReleaseJobState::Failed
            | ReleaseJobState::Canceled
            | ReleaseJobState::TimedOut
    )
}

fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn process_exists(pid: u32) -> bool {
    (unsafe { libc::kill(pid as i32, 0) }) == 0
        || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn map_admission_error(_: ReleaseAdmissionError) -> ReleaseJobError {
    ReleaseJobError::Invalid
}
