//! Where the app keeps its files (see the README's "Where settings are stored").

use std::path::PathBuf;

const APP_DIR: &str = "youtube-downloader";

/// `~/.config/youtube-downloader` on Linux, `~/Library/Application Support/youtube-downloader` on macOS.
pub fn config_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.config_dir().join(APP_DIR))
}

/// `~/.local/share/youtube-downloader` on Linux, the same folder as settings on macOS.
pub fn data_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.data_dir().join(APP_DIR))
}

pub fn history_file() -> Option<PathBuf> {
    data_dir().map(|d| d.join("history.json"))
}

pub fn config_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

pub fn home() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Shows `/home/me/Music` as `~/Music`.
pub fn display(path: &std::path::Path) -> String {
    match path.strip_prefix(home()) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}
