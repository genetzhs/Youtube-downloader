//! UI-independent logic for YouTube Downloader: building yt-dlp commands,
//! reading its output, the download queue, settings, history and finding tools.

pub mod app_release;
pub mod command;
pub mod config;
pub mod history;
pub mod info;
pub mod options;
pub mod progress;
pub mod queue;
pub mod tools;
