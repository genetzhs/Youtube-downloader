//! The download queue: which items wait, which run, and how they ended.
//!
//! Pure state. The runner reports events and exit codes; the queue decides
//! what each item's status is and which items may start next.

use std::path::PathBuf;

use crate::info::MediaInfo;
use crate::options::DownloadOptions;
use crate::progress::{ErrorKind, Event, PostStep, Progress};

pub type ItemId = u64;

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Queued,
    Running,
    PostProcessing(PostStep),
    Done,
    Failed(ErrorKind),
    Cancelled,
}

impl Status {
    pub fn is_active(&self) -> bool {
        matches!(self, Status::Running | Status::PostProcessing(_))
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Status::Done | Status::Failed(_) | Status::Cancelled)
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: ItemId,
    pub url: String,
    /// Snapshot of the options when the item was added, so later changes
    /// in the UI do not affect what is already queued.
    pub options: DownloadOptions,
    pub out_dir: PathBuf,
    pub status: Status,
    pub progress: Option<Progress>,
    pub playlist: Option<(u32, u32)>,
    pub final_path: Option<PathBuf>,
    /// Title, channel and thumbnail, once yt-dlp has looked them up.
    pub info: Option<MediaInfo>,
    /// What yt-dlp reported about the browser cookies on the last attempt.
    pub cookies: Option<CookieReport>,
    last_error: Option<ErrorKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieReport {
    pub browser: String,
    pub count: u32,
    pub undecrypted: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub queued: usize,
    pub active: usize,
    pub done: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[derive(Debug)]
pub struct Queue {
    items: Vec<Item>,
    next_id: ItemId,
    pub max_parallel: usize,
}

impl Queue {
    pub fn new(max_parallel: usize) -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            max_parallel: max_parallel.max(1),
        }
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    fn get_mut(&mut self, id: ItemId) -> Option<&mut Item> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    pub fn add(&mut self, url: &str, options: DownloadOptions, out_dir: PathBuf) -> ItemId {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Item {
            id,
            url: url.to_string(),
            options,
            out_dir,
            status: Status::Queued,
            progress: None,
            playlist: None,
            final_path: None,
            info: None,
            cookies: None,
            last_error: None,
        });
        id
    }

    /// Adds every link found in pasted text (one per line or separated by spaces).
    pub fn add_text(
        &mut self,
        text: &str,
        options: &DownloadOptions,
        out_dir: &std::path::Path,
    ) -> Vec<ItemId> {
        extract_urls(text)
            .into_iter()
            .map(|u| self.add(&u, options.clone(), out_dir.to_path_buf()))
            .collect()
    }

    /// Marks as many queued items as running as there are free slots and returns them.
    pub fn start_ready(&mut self) -> Vec<ItemId> {
        let free = self.max_parallel.saturating_sub(self.counts().active);
        let mut started = Vec::new();
        for item in self
            .items
            .iter_mut()
            .filter(|i| i.status == Status::Queued)
            .take(free)
        {
            item.status = Status::Running;
            started.push(item.id);
        }
        started
    }

    pub fn set_info(&mut self, id: ItemId, info: MediaInfo) {
        if let Some(item) = self.get_mut(id) {
            item.info = Some(info);
        }
    }

    /// Fails a running item without waiting for a process, e.g. when it could not start.
    pub fn fail(&mut self, id: ItemId, kind: ErrorKind) {
        if let Some(item) = self.get_mut(id)
            && item.status.is_active()
        {
            item.status = Status::Failed(kind);
        }
    }

    pub fn apply(&mut self, id: ItemId, event: &Event) {
        let Some(item) = self.get_mut(id) else { return };
        if !item.status.is_active() {
            return;
        }
        if let Some(p) = event.final_path() {
            item.final_path = Some(p.clone());
        }
        match event {
            Event::Progress(p) => {
                item.status = Status::Running;
                item.progress = Some(p.clone());
            }
            Event::PostProcess { step, .. } => item.status = Status::PostProcessing(*step),
            Event::PlaylistItem { index, count } => {
                item.playlist = Some((*index, *count));
                item.progress = None;
            }
            Event::Error(kind) => item.last_error = Some(kind.clone()),
            Event::CookiesRead {
                browser,
                count,
                undecrypted,
            } => {
                item.cookies = Some(CookieReport {
                    browser: browser.clone(),
                    count: *count,
                    undecrypted: *undecrypted,
                });
            }
            Event::Destination(_) | Event::AlreadyDownloaded(_) => {}
        }
    }

    /// Called when the yt-dlp process exits.
    pub fn finish(&mut self, id: ItemId, success: bool) {
        let Some(item) = self.get_mut(id) else { return };
        if !item.status.is_active() {
            // Already cancelled: the exit is just the process going away.
            return;
        }
        item.status = if success {
            Status::Done
        } else {
            Status::Failed(
                item.last_error
                    .clone()
                    .unwrap_or_else(|| ErrorKind::Other("yt-dlp stopped with an error".into())),
            )
        };
    }

    /// Returns true when the item was running, so the caller must stop its process.
    pub fn cancel(&mut self, id: ItemId) -> bool {
        let Some(item) = self.get_mut(id) else {
            return false;
        };
        let was_active = item.status.is_active();
        if !item.status.is_finished() {
            item.status = Status::Cancelled;
        }
        was_active
    }

    /// Cancels everything not finished and returns the items whose process must stop.
    pub fn cancel_all(&mut self) -> Vec<ItemId> {
        let ids: Vec<ItemId> = self.items.iter().map(|i| i.id).collect();
        ids.into_iter().filter(|&id| self.cancel(id)).collect()
    }

    /// Puts a finished item back in the queue, optionally with new options
    /// (for example after choosing browser cookies).
    pub fn retry(&mut self, id: ItemId, options: Option<DownloadOptions>) {
        let Some(item) = self.get_mut(id) else { return };
        if !item.status.is_finished() {
            return;
        }
        if let Some(o) = options {
            item.options = o;
        }
        item.status = Status::Queued;
        item.progress = None;
        item.playlist = None;
        item.final_path = None;
        item.last_error = None;
        item.cookies = None;
    }

    pub fn remove(&mut self, id: ItemId) {
        self.items.retain(|i| i.id != id || i.status.is_active());
    }

    pub fn clear_finished(&mut self) {
        self.items.retain(|i| !i.status.is_finished());
    }

    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for i in &self.items {
            match i.status {
                Status::Queued => c.queued += 1,
                Status::Running | Status::PostProcessing(_) => c.active += 1,
                Status::Done => c.done += 1,
                Status::Failed(_) => c.failed += 1,
                Status::Cancelled => c.cancelled += 1,
            }
        }
        c
    }
}

/// Finds http(s) links in free text; `www.` links get `https://` added.
pub fn extract_urls(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|t| t.trim_matches(|c| matches!(c, '<' | '>' | '"' | '\'' | ',' | '(' | ')')))
        .filter_map(|t| {
            if t.starts_with("http://") || t.starts_with("https://") {
                Some(t.to_string())
            } else if t.starts_with("www.") {
                Some(format!("https://{t}"))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn queue_with(n: usize, parallel: usize) -> (Queue, Vec<ItemId>) {
        let mut q = Queue::new(parallel);
        let text: Vec<String> = (0..n).map(|i| format!("https://x/{i}")).collect();
        let ids = q.add_text(
            &text.join("\n"),
            &DownloadOptions::default(),
            Path::new("/m"),
        );
        (q, ids)
    }

    #[test]
    fn extracts_links() {
        let urls = extract_urls("https://a.com/1\n  www.b.com/2 nonsense <https://c.com/3>,\n");
        assert_eq!(
            urls,
            ["https://a.com/1", "https://www.b.com/2", "https://c.com/3"]
        );
    }

    #[test]
    fn respects_parallel_limit() {
        let (mut q, ids) = queue_with(5, 2);
        assert_eq!(q.start_ready(), ids[..2]);
        assert_eq!(q.start_ready(), Vec::<ItemId>::new());
        q.finish(ids[0], true);
        assert_eq!(q.start_ready(), [ids[2]]);
        assert_eq!(
            q.counts(),
            Counts {
                queued: 2,
                active: 2,
                done: 1,
                failed: 0,
                cancelled: 0
            }
        );
    }

    #[test]
    fn tracks_progress_and_final_path() {
        let (mut q, ids) = queue_with(1, 2);
        q.start_ready();
        let id = ids[0];
        q.apply(
            id,
            &crate::progress::parse_line("[ytdl-progress] 50/100/NA/10/5").unwrap(),
        );
        assert_eq!(
            q.get(id).unwrap().progress.as_ref().unwrap().fraction(),
            Some(0.5)
        );
        q.apply(
            id,
            &crate::progress::parse_line("[ExtractAudio] Destination: /m/a.mp3").unwrap(),
        );
        assert_eq!(
            q.get(id).unwrap().status,
            Status::PostProcessing(PostStep::ExtractingAudio)
        );
        q.finish(id, true);
        let item = q.get(id).unwrap();
        assert_eq!(item.status, Status::Done);
        assert_eq!(item.final_path.as_deref(), Some(Path::new("/m/a.mp3")));
    }

    #[test]
    fn failure_keeps_the_reported_error() {
        let (mut q, ids) = queue_with(1, 1);
        q.start_ready();
        q.apply(ids[0], &Event::Error(ErrorKind::BotCheck));
        q.finish(ids[0], false);
        assert_eq!(
            q.get(ids[0]).unwrap().status,
            Status::Failed(ErrorKind::BotCheck)
        );
    }

    #[test]
    fn cancel_wins_over_late_exit() {
        let (mut q, ids) = queue_with(2, 1);
        q.start_ready();
        assert!(q.cancel(ids[0]));
        assert!(!q.cancel(ids[1]));
        q.finish(ids[0], false);
        assert_eq!(q.get(ids[0]).unwrap().status, Status::Cancelled);
        assert_eq!(q.get(ids[1]).unwrap().status, Status::Cancelled);
        assert_eq!(q.start_ready(), Vec::<ItemId>::new());
    }

    #[test]
    fn retry_with_new_options() {
        let (mut q, ids) = queue_with(1, 1);
        q.start_ready();
        q.finish(ids[0], false);
        let opts = DownloadOptions {
            cookies_from: Some(crate::options::Browser::Firefox),
            ..Default::default()
        };
        q.retry(ids[0], Some(opts.clone()));
        let item = q.get(ids[0]).unwrap();
        assert_eq!(item.status, Status::Queued);
        assert_eq!(item.options, opts);
    }

    #[test]
    fn running_items_cannot_be_removed() {
        let (mut q, ids) = queue_with(2, 1);
        q.start_ready();
        q.remove(ids[0]);
        q.remove(ids[1]);
        assert_eq!(q.items().len(), 1);
        q.finish(ids[0], true);
        q.clear_finished();
        assert!(q.items().is_empty());
    }
}
