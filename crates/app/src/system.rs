//! Talking to the operating system: tools, yt-dlp updates, opening files
//! and finding which browsers are installed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{Receiver, channel};

use ytdl_core::options::Browser;
use ytdl_core::tools::{Locations, Source, Tool, locate};
use ytdl_runner::Tools;
use ytdl_runner::update::{self, UpdateOutcome};

use crate::paths;

/// What the footer and Settings show about yt-dlp and ffmpeg.
#[derive(Debug, Clone, Default)]
pub struct ToolStatus {
    pub ytdlp: Option<PathBuf>,
    pub ytdlp_source: Option<Source>,
    pub ytdlp_version: Option<String>,
    pub ffmpeg: Option<PathBuf>,
    pub updating: bool,
    pub update_error: Option<String>,
}

impl ToolStatus {
    pub fn tools(&self) -> Tools {
        Tools {
            ytdlp: self.ytdlp.clone(),
            ffmpeg: self.ffmpeg.clone(),
        }
    }
}

fn locations(ytdlp_override: Option<PathBuf>) -> Locations {
    let exe = std::env::current_exe().unwrap_or_default();
    let data = paths::data_dir().unwrap_or_else(std::env::temp_dir);
    Locations::for_app(&exe, &data, ytdlp_override)
}

/// Finds the tools now (fast: only looks at files).
pub fn find_tools(ytdlp_override: Option<PathBuf>, ffmpeg_override: Option<PathBuf>) -> ToolStatus {
    let loc = locations(ytdlp_override);
    let ytdlp = locate(Tool::YtDlp, &loc);
    let ffmpeg_loc = Locations {
        override_path: ffmpeg_override,
        ..loc
    };
    ToolStatus {
        ytdlp_source: ytdlp.as_ref().map(|f| f.source),
        ytdlp: ytdlp.map(|f| f.path),
        ffmpeg: locate(Tool::Ffmpeg, &ffmpeg_loc).map(|f| f.path),
        ..Default::default()
    }
}

pub enum ToolEvent {
    Version(Option<String>),
    Updated(Result<UpdateOutcome, String>),
}

/// Reads the yt-dlp version and, if asked, updates it, on a background thread.
pub fn check_ytdlp(
    current: Option<PathBuf>,
    update_too: bool,
    wake: impl Fn() + Send + 'static,
) -> Receiver<ToolEvent> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let version = current.as_deref().and_then(update::installed_version);
        let _ = tx.send(ToolEvent::Version(version));
        wake();
        if update_too {
            let dest = paths::data_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("bin");
            let result = update::update(current.as_deref(), &dest).map_err(|e| e.to_string());
            let _ = tx.send(ToolEvent::Updated(result));
            wake();
        }
    });
    rx
}

/// Shows a file selected in the file manager, or opens a folder.
pub fn reveal(path: &Path) {
    let result = if cfg!(target_os = "macos") {
        if path.is_file() {
            Command::new("open").arg("-R").arg(path).spawn()
        } else {
            Command::new("open").arg(path).spawn()
        }
    } else {
        let dir = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        let _ = std::fs::create_dir_all(dir);
        Command::new("xdg-open").arg(dir).spawn()
    };
    if let Err(e) = result {
        eprintln!("could not open {}: {e}", path.display());
    }
}

/// Opens a file with its default app.
pub fn open(path: &Path) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = Command::new(program).arg(path).spawn() {
        eprintln!("could not open {}: {e}", path.display());
    }
}

/// Browsers whose profile folder exists, most common first.
pub fn installed_browsers() -> Vec<Browser> {
    let home = paths::home();
    let candidates: &[(Browser, &[&str])] = if cfg!(target_os = "macos") {
        &[
            (
                Browser::Chrome,
                &["Library/Application Support/Google/Chrome"],
            ),
            (Browser::Firefox, &["Library/Application Support/Firefox"]),
            (
                Browser::Brave,
                &["Library/Application Support/BraveSoftware/Brave-Browser"],
            ),
            (
                Browser::Edge,
                &["Library/Application Support/Microsoft Edge"],
            ),
            (Browser::Vivaldi, &["Library/Application Support/Vivaldi"]),
            (
                Browser::Opera,
                &["Library/Application Support/com.operasoftware.Opera"],
            ),
            (Browser::Chromium, &["Library/Application Support/Chromium"]),
            (Browser::Safari, &["Library/Safari"]),
        ]
    } else {
        &[
            (
                Browser::Firefox,
                &[".mozilla/firefox", "snap/firefox/common/.mozilla/firefox"],
            ),
            (Browser::Chrome, &[".config/google-chrome"]),
            (Browser::Brave, &[".config/BraveSoftware/Brave-Browser"]),
            (
                Browser::Chromium,
                &[".config/chromium", "snap/chromium/common/chromium"],
            ),
            (Browser::Edge, &[".config/microsoft-edge"]),
            (Browser::Vivaldi, &[".config/vivaldi"]),
            (Browser::Opera, &[".config/opera"]),
        ]
    };
    candidates
        .iter()
        .filter(|(_, dirs)| dirs.iter().any(|d| home.join(d).is_dir()))
        .map(|(b, _)| *b)
        .collect()
}

pub fn browser_name(b: Browser) -> &'static str {
    match b {
        Browser::Chrome => "Chrome",
        Browser::Chromium => "Chromium",
        Browser::Brave => "Brave",
        Browser::Firefox => "Firefox",
        Browser::Edge => "Edge",
        Browser::Opera => "Opera",
        Browser::Vivaldi => "Vivaldi",
        Browser::Safari => "Safari",
    }
}
