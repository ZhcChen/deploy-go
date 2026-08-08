use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

pub const MAX_INPUT_BYTES: usize = 12 * 1024;

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn Child + Send + Sync>,
    output: mpsc::Receiver<Vec<u8>>,
    reader_thread: Option<thread::JoinHandle<()>>,
    close_grace: Duration,
    output_overflowed: Arc<AtomicBool>,
    closed: bool,
}

impl PtySession {
    pub fn spawn(
        shell: &Path,
        home: &Path,
        rows: u16,
        cols: u16,
        output_buffer_frames: usize,
        close_grace: Duration,
    ) -> anyhow::Result<Self> {
        if !crate::protocol::validate_dimensions(rows, cols) {
            anyhow::bail!("invalid terminal dimensions");
        }
        let pair = native_pty_system().openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut command = CommandBuilder::new(shell);
        command.arg("-l");
        command.env_clear();
        command.env(
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        );
        command.env("HOME", home);
        command.env("USER", "root");
        command.env("LOGNAME", "root");
        command.env("SHELL", shell);
        command.env("TERM", "xterm-256color");
        command.cwd(home);
        let child = pair.slave.spawn_command(command)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader()?;
        let writer = Arc::new(Mutex::new(pair.master.take_writer()?));
        let (output_tx, output) = mpsc::sync_channel(output_buffer_frames.max(1));
        let output_overflowed = Arc::new(AtomicBool::new(false));
        let reader_overflowed = Arc::clone(&output_overflowed);
        let reader_thread = thread::spawn(move || {
            let mut buffer = vec![0; 8192];
            while let Ok(length) = reader.read(&mut buffer) {
                if length == 0 {
                    break;
                }
                if output_tx.try_send(buffer[..length].to_vec()).is_err() {
                    reader_overflowed.store(true, Ordering::Release);
                    break;
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            child,
            output,
            reader_thread: Some(reader_thread),
            close_grace,
            output_overflowed,
            closed: false,
        })
    }

    pub fn input(&self, data: &[u8]) -> anyhow::Result<()> {
        if data.len() > MAX_INPUT_BYTES {
            anyhow::bail!("input frame exceeds limit");
        }
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY writer poisoned"))?;
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, rows: u16, cols: u16) -> anyhow::Result<()> {
        if !crate::protocol::validate_dimensions(rows, cols) {
            anyhow::bail!("invalid terminal dimensions");
        }
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    pub fn recv_output_timeout(&self, timeout: Duration) -> Option<Vec<u8>> {
        self.output.recv_timeout(timeout).ok()
    }

    pub fn output_overflowed(&self) -> bool {
        self.output_overflowed.load(Ordering::Acquire)
    }

    pub fn try_wait(&mut self) -> anyhow::Result<Option<u32>> {
        Ok(self.child.try_wait()?.map(|status| status.exit_code()))
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let process_group = self
            .master
            .process_group_leader()
            .or_else(|| self.child.process_id().map(|pid| pid as i32));
        #[cfg(target_os = "linux")]
        let session_id = self
            .child
            .process_id()
            .and_then(|pid| linux_session_id(pid as i32));
        if let Some(process_group) = process_group {
            unsafe { libc::kill(-process_group, libc::SIGTERM) };
            let deadline = std::time::Instant::now() + self.close_grace;
            while std::time::Instant::now() < deadline {
                if self.child.try_wait()?.is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            if self.child.try_wait()?.is_none() {
                // A descendant may ignore TERM while retaining the PTY. Kill the complete
                // process group before waiting for EOF from the reader thread.
                unsafe { libc::kill(-process_group, libc::SIGKILL) };
            }
        }
        #[cfg(target_os = "linux")]
        if let Some(session_id) = session_id {
            kill_linux_session(session_id, libc::SIGKILL);
        }
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        let _ = self.child.wait();
        if let Some(thread) = self.reader_thread.take() {
            let _ = thread.join();
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn linux_session_id(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields = stat
        .rsplit_once(") ")?
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    fields.get(3)?.parse().ok()
}

#[cfg(target_os = "linux")]
fn kill_linux_session(session_id: i32, signal: i32) {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<i32>().ok())
        else {
            continue;
        };
        if linux_session_id(pid) == Some(session_id) {
            unsafe { libc::kill(pid, signal) };
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
