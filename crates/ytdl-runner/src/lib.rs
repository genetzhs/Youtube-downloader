//! Runs yt-dlp for the queue in `ytdl-core`, and keeps yt-dlp up to date.

pub mod downloader;
pub mod process;
pub mod update;

pub use downloader::{Downloader, Finished, Tools};
