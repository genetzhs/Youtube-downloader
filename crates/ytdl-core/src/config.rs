//! Settings saved to `config.toml`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::options::{AudioFormat, AudioQuality, Container, DownloadOptions, Mode, Resolution};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The options used for the next download; `options.mode` is the current tab.
    pub options: DownloadOptions,
    /// Remembered so switching between Audio and Video keeps each choice.
    pub last_audio: Mode,
    pub last_video: Mode,
    /// `None` means the platform default (see [`default_dirs`]).
    pub audio_dir: Option<PathBuf>,
    pub video_dir: Option<PathBuf>,
    pub max_parallel: usize,
    pub theme: Theme,
    pub notify_when_done: bool,
    pub auto_update_ytdlp: bool,
    /// Use this yt-dlp instead of the bundled or updated one.
    pub ytdlp_override: Option<PathBuf>,
    pub ffmpeg_override: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            options: DownloadOptions::default(),
            last_audio: Mode::Audio {
                format: AudioFormat::Mp3,
                quality: AudioQuality(Some(320)),
            },
            last_video: Mode::Video {
                resolution: Resolution(Some(1080)),
                container: Container::Mp4,
            },
            audio_dir: None,
            video_dir: None,
            max_parallel: 2,
            theme: Theme::System,
            notify_when_done: true,
            auto_update_ytdlp: true,
            ytdlp_override: None,
            ffmpeg_override: None,
        }
    }
}

impl Config {
    pub fn is_video(&self) -> bool {
        matches!(self.options.mode, Mode::Video { .. })
    }

    /// Switches the current tab, remembering the choice made in the other one.
    pub fn set_video(&mut self, video: bool) {
        if video == self.is_video() {
            return;
        }
        if video {
            self.last_audio = self.options.mode;
            self.options.mode = self.last_video;
        } else {
            self.last_video = self.options.mode;
            self.options.mode = self.last_audio;
        }
    }

    /// The folder the current tab downloads into.
    pub fn out_dir(&self, defaults: &(PathBuf, PathBuf)) -> PathBuf {
        if self.is_video() {
            self.video_dir.clone().unwrap_or_else(|| defaults.1.clone())
        } else {
            self.audio_dir.clone().unwrap_or_else(|| defaults.0.clone())
        }
    }

    /// Reads the file; a missing file gives the defaults.
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(toml::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let text = toml::to_string_pretty(self)?;
        write_atomic(path, text.as_bytes())?;
        Ok(())
    }
}

/// Default (audio, video) folders: `~/Music/YouTube` and `~/Movies/YouTube` on
/// macOS or `~/Videos/YouTube` on Linux.
pub fn default_dirs(home: &Path, macos: bool) -> (PathBuf, PathBuf) {
    let video = if macos { "Movies" } else { "Videos" };
    (home.join("Music/YouTube"), home.join(video).join("YouTube"))
}

/// Writes to a temporary file and renames it, so a crash never leaves half a file.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read or write the settings file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the settings file is not valid: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("could not save settings: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ytdl-core-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("config.toml")
    }

    #[test]
    fn missing_file_gives_defaults() {
        assert_eq!(Config::load(&tmp("missing")).unwrap(), Config::default());
    }

    #[test]
    fn round_trip() {
        let path = tmp("roundtrip");
        let mut c = Config::default();
        c.set_video(true);
        c.max_parallel = 3;
        c.theme = Theme::Dark;
        c.audio_dir = Some("/music".into());
        c.options.cookies_from = Some(crate::options::Browser::Brave);
        c.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), c);
    }

    #[test]
    fn partial_file_fills_in_defaults() {
        let path = tmp("partial");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "max_parallel = 4\n").unwrap();
        let c = Config::load(&path).unwrap();
        assert_eq!(c.max_parallel, 4);
        assert_eq!(c.options, DownloadOptions::default());
    }

    #[test]
    fn switching_tabs_remembers_choices() {
        let mut c = Config::default();
        c.options.mode = Mode::Audio {
            format: AudioFormat::Flac,
            quality: AudioQuality(None),
        };
        c.set_video(true);
        assert_eq!(c.options.mode, Config::default().last_video);
        c.set_video(false);
        assert!(matches!(
            c.options.mode,
            Mode::Audio {
                format: AudioFormat::Flac,
                ..
            }
        ));
    }

    #[test]
    fn out_dir_per_tab() {
        let defaults = default_dirs(Path::new("/home/u"), false);
        let mut c = Config::default();
        assert_eq!(c.out_dir(&defaults), PathBuf::from("/home/u/Music/YouTube"));
        c.set_video(true);
        assert_eq!(
            c.out_dir(&defaults),
            PathBuf::from("/home/u/Videos/YouTube")
        );
        assert_eq!(
            default_dirs(Path::new("/Users/u"), true).1,
            PathBuf::from("/Users/u/Movies/YouTube")
        );
    }
}
