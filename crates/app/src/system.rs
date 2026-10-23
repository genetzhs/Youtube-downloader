//! Talking to the operating system: tools, yt-dlp updates, opening files
//! and finding which browsers are installed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{Receiver, channel};
use std::time::{SystemTime, UNIX_EPOCH};

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
    installed_browsers_in(&paths::home())
}

fn installed_browsers_in(home: &Path) -> Vec<Browser> {
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
    // Most recently used first: the browser whose cookie store changed last is
    // almost always the one the user is signed in with.
    let mut found: Vec<(Browser, SystemTime)> = candidates
        .iter()
        .filter_map(|(b, dirs)| {
            let roots: Vec<PathBuf> = dirs
                .iter()
                .map(|d| home.join(d))
                .filter(|p| p.is_dir())
                .collect();
            if roots.is_empty() {
                return None;
            }
            let newest = roots
                .iter()
                .filter_map(|r| newest_cookie_store(r, 3))
                .max()
                .unwrap_or(UNIX_EPOCH);
            Some((*b, newest))
        })
        .collect();
    found.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
    found.into_iter().map(|(b, _)| b).collect()
}

/// Modification time of the newest cookie database under `dir`
/// (`Cookies` for Chromium browsers, `cookies.sqlite` for Firefox).
fn newest_cookie_store(dir: &Path, depth: u32) -> Option<SystemTime> {
    let mut newest = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let is_store =
            name == "Cookies" || name == "cookies.sqlite" || name == "Cookies.binarycookies";
        let time = if is_store {
            entry.metadata().and_then(|m| m.modified()).ok()
        } else if depth > 0 && entry.file_type().is_ok_and(|t| t.is_dir()) {
            newest_cookie_store(&path, depth - 1)
        } else {
            None
        };
        newest = newest.max(time);
    }
    newest
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn touch(path: &Path, age: Duration) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let f = std::fs::File::create(path).unwrap();
        f.set_modified(SystemTime::now() - age).unwrap();
    }

    /// Mint ships Firefox, but people often sign in to YouTube in Chrome:
    /// the browser used most recently must come first.
    #[test]
    #[cfg(not(target_os = "macos"))]
    fn most_recently_used_browser_first() {
        let home = std::env::temp_dir().join(format!("ytdl-app-browsers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        touch(
            &home.join(".mozilla/firefox/abc.default/cookies.sqlite"),
            Duration::from_secs(86_400 * 30),
        );
        touch(
            &home.join(".config/google-chrome/Default/Cookies"),
            Duration::from_secs(60),
        );
        std::fs::create_dir_all(home.join(".config/chromium")).unwrap(); // installed, never used
        assert_eq!(
            installed_browsers_in(&home),
            [Browser::Chrome, Browser::Firefox, Browser::Chromium]
        );

        // Newer Chrome versions keep cookies under Network/.
        touch(
            &home.join(".mozilla/firefox/abc.default/cookies.sqlite"),
            Duration::from_secs(1),
        );
        assert_eq!(installed_browsers_in(&home)[0], Browser::Firefox);
        touch(
            &home.join(".config/google-chrome/Profile 1/Network/Cookies"),
            Duration::ZERO,
        );
        assert_eq!(installed_browsers_in(&home)[0], Browser::Chrome);
        let _ = std::fs::remove_dir_all(home);
    }
}
