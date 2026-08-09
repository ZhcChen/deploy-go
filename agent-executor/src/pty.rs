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
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    child: Option<Box<dyn Child + Send + Sync>>,
    output: mpsc::Receiver<Vec<u8>>,
    reader_thread: Option<thread::JoinHandle<()>>,
    close_grace: Duration,
    output_overflowed: Arc<AtomicBool>,
    closed: bool,
    #[cfg(target_os = "linux")]
    cgroup: Option<crate::cgroup::SessionCgroup>,
    cleanup_gate: Option<Arc<crate::session_claim::SessionRegistry>>,
}

impl PtySession {
    pub fn spawn(
        shell: &Path,
        home: &Path,
        rows: u16,
        cols: u16,
        output_buffer_frames: usize,
        close_grace: Duration,
        #[cfg(target_os = "linux")] cgroup: Option<crate::cgroup::SessionCgroup>,
        cleanup_gate: Option<Arc<crate::session_claim::SessionRegistry>>,
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
        #[cfg(target_os = "linux")]
        let (program, arguments) = if let Some(cgroup) = cgroup.as_ref() {
            cgroup.launcher_command(shell)?
        } else {
            (shell.to_path_buf(), vec!["-l".to_owned()])
        };
        #[cfg(not(target_os = "linux"))]
        let (program, arguments) = (shell.to_path_buf(), vec!["-l".to_owned()]);
        let mut command = CommandBuilder::new(program);
        command.args(arguments);
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
            master: Some(pair.master),
            writer: Some(writer),
            child: Some(child),
            output,
            reader_thread: Some(reader_thread),
            close_grace,
            output_overflowed,
            closed: false,
            #[cfg(target_os = "linux")]
            cgroup,
            cleanup_gate,
        })
    }

    pub fn input(&self, data: &[u8]) -> anyhow::Result<()> {
        if data.len() > MAX_INPUT_BYTES {
            anyhow::bail!("input frame exceeds limit");
        }
        let mut writer = self
            .writer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("PTY is closed"))?
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
        self.master
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("PTY is closed"))?
            .resize(PtySize {
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
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("PTY is closed"))?;
        Ok(child.try_wait()?.map(|status| status.exit_code()))
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let mut failure = None;
        let process_group = self
            .master
            .as_ref()
            .and_then(|master| master.process_group_leader())
            .or_else(|| {
                self.child
                    .as_ref()
                    .and_then(|child| child.process_id())
                    .map(|pid| pid as i32)
            });
        if let Some(process_group) = process_group {
            unsafe { libc::kill(-process_group, libc::SIGTERM) };
            let deadline = std::time::Instant::now() + self.close_grace;
            while std::time::Instant::now() < deadline {
                match self.child.as_mut().expect("child exists").try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => thread::sleep(Duration::from_millis(10)),
                    Err(error) => {
                        remember_error(&mut failure, error);
                        break;
                    }
                }
            }
        }
        #[cfg(target_os = "linux")]
        if let Some(cgroup) = self.cgroup.as_ref() {
            if let Err(error) = cgroup.kill_all() {
                remember_error(&mut failure, error);
            }
        }
        match self.child.as_mut().expect("child exists").try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                if let Err(error) = self.child.as_mut().expect("child exists").kill() {
                    remember_error(&mut failure, error);
                }
            }
            Err(error) => remember_error(&mut failure, error),
        }
        let reap_deadline = std::time::Instant::now() + self.close_grace;
        while std::time::Instant::now() < reap_deadline {
            match self.child.as_mut().expect("child exists").try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(error) => {
                    remember_error(&mut failure, error);
                    break;
                }
            }
        }
        match self.child.as_mut().expect("child exists").try_wait() {
            Ok(Some(_)) => {
                self.child.take();
            }
            Ok(None) => {
                remember_error(
                    &mut failure,
                    anyhow::anyhow!("PTY child was not reaped before cleanup deadline"),
                );
                let mut child = self.child.take().expect("child exists");
                thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(error) => remember_error(&mut failure, error),
        }
        #[cfg(target_os = "linux")]
        if let Some(cgroup) = self.cgroup.take() {
            if let Err(error) = cgroup.wait_empty_and_remove(self.close_grace) {
                remember_error(&mut failure, error);
            }
        }
        self.writer.take();
        self.master.take();
        if let Some(thread) = self.reader_thread.take() {
            let reader_deadline = std::time::Instant::now() + self.close_grace;
            while !thread.is_finished() && std::time::Instant::now() < reader_deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            if thread.is_finished() {
                let _ = thread.join();
            }
        }
        if let Some(error) = failure {
            if let Some(gate) = self.cleanup_gate.as_ref() {
                gate.block_after_cleanup_failure();
            }
            Err(error)
        } else {
            Ok(())
        }
    }
}

fn remember_error<E>(failure: &mut Option<anyhow::Error>, error: E)
where
    E: Into<anyhow::Error>,
{
    if failure.is_none() {
        *failure = Some(error.into());
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
