//! Finished downloads, kept in `history.json`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::write_atomic;

/// Older entries are dropped beyond this many.
pub const MAX_ENTRIES: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub url: String,
    pub title: Option<String>,
    pub path: Option<PathBuf>,
    /// Short description such as `MP3 · 320k` or `1080p · MP4`.
    pub summary: String,
    /// Unix time in seconds.
    pub finished_at: u64,
    pub success: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    pub entries: Vec<Entry>,
}

impl History {
    /// Reads the file; a missing or damaged file gives an empty history.
    pub fn load(path: &Path) -> History {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        write_atomic(path, &json)
    }

    /// Newest first.
    pub fn push(&mut self, entry: Entry) {
        self.entries.insert(0, entry);
        self.entries.truncate(MAX_ENTRIES);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: u64) -> Entry {
        Entry {
            url: format!("https://x/{n}"),
            title: Some(format!("Title {n}")),
            path: Some(format!("/m/{n}.mp3").into()),
            summary: "MP3 · 320k".into(),
            finished_at: n,
            success: true,
        }
    }

    #[test]
    fn round_trip_and_order() {
        let dir = std::env::temp_dir().join(format!("ytdl-core-history-{}", std::process::id()));
        let path = dir.join("history.json");
        let mut h = History::default();
        h.push(entry(1));
        h.push(entry(2));
        h.save(&path).unwrap();
        let loaded = History::load(&path);
        assert_eq!(loaded, h);
        assert_eq!(loaded.entries[0].finished_at, 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_file_is_empty() {
        let dir =
            std::env::temp_dir().join(format!("ytdl-core-history-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(History::load(&path), History::default());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn capped() {
        let mut h = History::default();
        for n in 0..(MAX_ENTRIES as u64 + 5) {
            h.push(entry(n));
        }
        assert_eq!(h.entries.len(), MAX_ENTRIES);
        assert_eq!(h.entries[0].finished_at, MAX_ENTRIES as u64 + 4);
    }
}
