//! Ties the queue to running processes: the one object the UI talks to.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use ytdl_core::command::{build_args, info_args, shell_quote};
use ytdl_core::info::parse_info;
use ytdl_core::options::DownloadOptions;
use ytdl_core::progress::{ErrorKind, parse_line};
use ytdl_core::queue::{ItemId, Queue, Status};

use crate::process::{Runner, RunnerEvent};

/// Lines of output kept per item for the log view.
const LOG_LINES: usize = 500;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tools {
    pub ytdlp: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
}

/// An item that reached Done or Failed, for history and notifications.
/// Cancelled items are not reported: the user already knows.
#[derive(Debug, Clone, PartialEq)]
pub struct Finished {
    pub id: ItemId,
    pub status: Status,
}

pub struct Downloader {
    queue: Queue,
    runner: Runner,
    rx: Receiver<RunnerEvent>,
    pub tools: Tools,
    logs: HashMap<ItemId, VecDeque<String>>,
    finished: Vec<Finished>,
}

impl Downloader {
    /// `wake` is called from background threads whenever something changed.
    pub fn new(max_parallel: usize, tools: Tools, wake: impl Fn() + Send + Sync + 'static) -> Self {
        let (runner, rx) = Runner::new(wake);
        Self {
            queue: Queue::new(max_parallel),
            runner,
            rx,
            tools,
            logs: HashMap::new(),
            finished: Vec::new(),
        }
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn set_max_parallel(&mut self, n: usize) {
        self.queue.max_parallel = n.max(1);
        self.start_ready();
    }

    /// Adds every link in `text`, looks up their titles and starts what fits.
    pub fn add_text(
        &mut self,
        text: &str,
        options: &DownloadOptions,
        out_dir: &Path,
    ) -> Vec<ItemId> {
        let ids = self.queue.add_text(text, options, out_dir);
        if let Some(ytdlp) = &self.tools.ytdlp {
            for &id in &ids {
                let item = self.queue.get(id).expect("just added");
                self.runner
                    .lookup_info(id, ytdlp, &info_args(&item.options, &item.url));
            }
        }
        self.start_ready();
        ids
    }

    /// Applies everything the background threads reported since the last call.
    /// Call it every frame; returns the items that finished.
    pub fn tick(&mut self) -> Vec<Finished> {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                RunnerEvent::Line { id, line } => {
                    if let Some(event) = parse_line(&line) {
                        self.queue.apply(id, &event);
                    }
                    self.log_line(id, line);
                }
                RunnerEvent::Exited { id, success } => {
                    let was_active = self.status(id).is_some_and(|s| s.is_active());
                    self.queue.finish(id, success);
                    if was_active {
                        self.report(id);
                    }
                }
                RunnerEvent::SpawnFailed { id, error } => {
                    self.log_line(id, error);
                    self.queue.fail(id, ErrorKind::MissingYtDlp);
                    self.report(id);
                }
                RunnerEvent::Info { id, json } => {
                    if let Some(info) = json.as_deref().and_then(parse_info) {
                        self.queue.set_info(id, info);
                    }
                }
            }
        }
        self.start_ready();
        std::mem::take(&mut self.finished)
    }

    pub fn cancel(&mut self, id: ItemId) {
        if self.queue.cancel(id) {
            self.runner.cancel(id);
        }
        self.start_ready();
    }

    pub fn cancel_all(&mut self) {
        for id in self.queue.cancel_all() {
            self.runner.cancel(id);
        }
    }

    /// Queues a finished item again, optionally with new options (e.g. cookies).
    pub fn retry(&mut self, id: ItemId, options: Option<DownloadOptions>) {
        self.queue.retry(id, options);
        self.logs.remove(&id);
        self.start_ready();
    }

    pub fn remove(&mut self, id: ItemId) {
        self.queue.remove(id);
        if self.queue.get(id).is_none() {
            self.logs.remove(&id);
        }
    }

    pub fn clear_finished(&mut self) {
        self.queue.clear_finished();
        let queue = &self.queue;
        self.logs.retain(|id, _| queue.get(*id).is_some());
    }

    pub fn log(&self, id: ItemId) -> impl Iterator<Item = &String> {
        self.logs.get(&id).into_iter().flatten()
    }

    /// Whether anything is running or waiting.
    pub fn is_busy(&self) -> bool {
        let c = self.queue.counts();
        c.active + c.queued > 0
    }

    fn status(&self, id: ItemId) -> Option<&Status> {
        self.queue.get(id).map(|i| &i.status)
    }

    fn report(&mut self, id: ItemId) {
        if let Some(status) = self.status(id).cloned() {
            self.finished.push(Finished { id, status });
        }
    }

    fn log_line(&mut self, id: ItemId, line: String) {
        let log = self.logs.entry(id).or_default();
        if log.len() == LOG_LINES {
            log.pop_front();
        }
        log.push_back(line);
    }

    fn start_ready(&mut self) {
        for id in self.queue.start_ready() {
            let item = self.queue.get(id).expect("started item exists");
            let (url, options, out_dir) =
                (item.url.clone(), item.options.clone(), item.out_dir.clone());

            let Some(ytdlp) = self.tools.ytdlp.clone() else {
                self.log_line(id, "yt-dlp was not found".into());
                self.queue.fail(id, ErrorKind::MissingYtDlp);
                self.report(id);
                continue;
            };
            if let Err(e) = std::fs::create_dir_all(&out_dir) {
                let msg = format!("could not create {}: {e}", out_dir.display());
                self.log_line(id, msg.clone());
                self.queue.fail(id, ErrorKind::Other(msg));
                self.report(id);
                continue;
            }

            let mut args = Vec::new();
            if let Some(dir) = self.tools.ffmpeg.as_deref().and_then(Path::parent) {
                args.push("--ffmpeg-location".to_string());
                args.push(dir.display().to_string());
            }
            args.extend(build_args(&options, &out_dir, &url));

            let shown: Vec<String> = std::iter::once(ytdlp.display().to_string())
                .chain(args.iter().cloned())
                .map(|a| shell_quote(&a))
                .collect();
            self.log_line(id, format!("$ {}", shown.join(" ")));
            self.runner.start(id, &ytdlp, &args);
        }
    }
}
