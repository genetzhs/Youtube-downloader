//! Starts yt-dlp processes and streams what they print back over a channel.
//!
//! One thread per output pipe plus one that waits for the exit, so the UI
//! thread never blocks. Every message also calls `wake`, which the app uses
//! to request a repaint.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ytdl_core::queue::ItemId;

/// How long a cancelled process gets to exit before it is killed outright.
const KILL_GRACE: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerEvent {
    /// One line of output, from stdout or stderr.
    Line {
        id: ItemId,
        line: String,
    },
    Exited {
        id: ItemId,
        success: bool,
    },
    SpawnFailed {
        id: ItemId,
        error: String,
    },
    /// Result of an info lookup: yt-dlp's JSON, or `None` if it failed.
    Info {
        id: ItemId,
        json: Option<String>,
    },
}

type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct Runner {
    tx: Sender<RunnerEvent>,
    pids: Arc<Mutex<HashMap<ItemId, u32>>>,
    wake: Wake,
}

impl Runner {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> (Runner, Receiver<RunnerEvent>) {
        let (tx, rx) = channel();
        let runner = Runner {
            tx,
            pids: Arc::default(),
            wake: Arc::new(wake),
        };
        (runner, rx)
    }

    fn send(&self, ev: RunnerEvent) {
        // The receiver only goes away when the app shuts down.
        let _ = self.tx.send(ev);
        (self.wake)();
    }

    /// Starts `program args…` for item `id`.
    pub fn start(&self, id: ItemId, program: &Path, args: &[String]) {
        let mut cmd = command(program);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = match spawn(&mut cmd) {
            Ok(c) => c,
            Err(e) => {
                self.send(RunnerEvent::SpawnFailed {
                    id,
                    error: format!("could not start {}: {e}", program.display()),
                });
                return;
            }
        };
        self.pids.lock().unwrap().insert(id, child.id());

        let readers: Vec<_> = [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        .map(|pipe| {
            let this = self.clone();
            thread::spawn(move || this.forward_lines(id, pipe))
        })
        .collect();

        let this = self.clone();
        thread::spawn(move || {
            for r in readers {
                let _ = r.join();
            }
            let success = child.wait().map(|s| s.success()).unwrap_or(false);
            this.pids.lock().unwrap().remove(&id);
            this.send(RunnerEvent::Exited { id, success });
        });
    }

    fn forward_lines(&self, id: ItemId, pipe: Box<dyn Read + Send>) {
        let mut reader = BufReader::new(pipe);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buf);
                    let line = line.trim_end_matches(['\r', '\n']);
                    if !line.is_empty() {
                        self.send(RunnerEvent::Line {
                            id,
                            line: line.to_string(),
                        });
                    }
                }
            }
        }
    }

    /// Runs an info lookup in the background and reports its JSON.
    pub fn lookup_info(&self, id: ItemId, program: &Path, args: &[String]) {
        let this = self.clone();
        let program: PathBuf = program.to_path_buf();
        let args = args.to_vec();
        thread::spawn(move || {
            let mut cmd = command(&program);
            cmd.args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            let json = spawn(&mut cmd)
                .and_then(|c| c.wait_with_output())
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
            this.send(RunnerEvent::Info { id, json });
        });
    }

    pub fn is_running(&self, id: ItemId) -> bool {
        self.pids.lock().unwrap().contains_key(&id)
    }

    /// Asks the process (and the ffmpeg it may have started) to stop, then
    /// kills it if it is still there after a short grace period.
    pub fn cancel(&self, id: ItemId) {
        let Some(pid) = self.pids.lock().unwrap().get(&id).copied() else {
            return;
        };
        signal_group(pid, Signal::Terminate);
        let pids = self.pids.clone();
        thread::spawn(move || {
            thread::sleep(KILL_GRACE);
            if pids.lock().unwrap().get(&id) == Some(&pid) {
                signal_group(pid, Signal::Kill);
            }
        });
    }
}

/// A command whose output is unbuffered UTF-8 and which runs in its own
/// process group, so cancelling also stops the ffmpeg children.
/// Starts `cmd`, retrying briefly while the program file is still open for
/// writing somewhere ("text file busy"). That happens right after yt-dlp is
/// updated, when another thread forks while the new file is being written.
fn spawn(cmd: &mut Command) -> std::io::Result<std::process::Child> {
    const ETXTBSY: i32 = 26;
    let mut tries = 0;
    loop {
        match cmd.spawn() {
            Err(e) if cfg!(unix) && e.raw_os_error() == Some(ETXTBSY) && tries < 20 => {
                tries += 1;
                thread::sleep(std::time::Duration::from_millis(25));
            }
            result => return result,
        }
    }
}

fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.env("PYTHONUNBUFFERED", "1")
        .env("PYTHONIOENCODING", "utf-8");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd
}

enum Signal {
    Terminate,
    Kill,
}

#[cfg(unix)]
fn signal_group(pid: u32, sig: Signal) {
    let sig = match sig {
        Signal::Terminate => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
    };
    // The child leads its own group, so the group id equals its pid.
    unsafe {
        libc::kill(-(pid as libc::pid_t), sig);
    }
}

#[cfg(not(unix))]
fn signal_group(_pid: u32, _sig: Signal) {
    // Only Linux and macOS are supported.
}
