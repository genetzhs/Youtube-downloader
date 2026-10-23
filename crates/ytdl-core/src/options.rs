//! What the user asks for: audio or video, which quality, and the extra flags.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    M4a,
    Opus,
    Flac,
    Wav,
    Vorbis,
    /// Keep whatever the source provides, no re-encoding.
    Best,
}

impl AudioFormat {
    pub const ALL: [AudioFormat; 7] = [
        Self::Mp3,
        Self::M4a,
        Self::Opus,
        Self::Flac,
        Self::Wav,
        Self::Vorbis,
        Self::Best,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Opus => "opus",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Vorbis => "vorbis",
            Self::Best => "best",
        }
    }

    /// Lossless formats ignore the bitrate setting.
    pub fn is_lossless(self) -> bool {
        matches!(self, Self::Flac | Self::Wav)
    }
}

/// Audio bitrate in kbps, or `None` for yt-dlp's best VBR quality (`0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioQuality(pub Option<u16>);

impl AudioQuality {
    pub const CHOICES: [AudioQuality; 8] = [
        AudioQuality(None),
        AudioQuality(Some(320)),
        AudioQuality(Some(256)),
        AudioQuality(Some(192)),
        AudioQuality(Some(160)),
        AudioQuality(Some(128)),
        AudioQuality(Some(96)),
        AudioQuality(Some(64)),
    ];

    pub fn arg(self) -> String {
        match self.0 {
            None => "0".into(),
            Some(kbps) => kbps.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    Mp4,
    Mkv,
    Webm,
}

impl Container {
    pub const ALL: [Container; 3] = [Self::Mp4, Self::Mkv, Self::Webm];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Webm => "webm",
        }
    }
}

/// Maximum video height in pixels, or `None` for the best available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution(pub Option<u16>);

impl Resolution {
    pub const CHOICES: [Resolution; 6] = [
        Resolution(None),
        Resolution(Some(2160)),
        Resolution(Some(1440)),
        Resolution(Some(1080)),
        Resolution(Some(720)),
        Resolution(Some(480)),
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Mode {
    Audio {
        format: AudioFormat,
        quality: AudioQuality,
    },
    Video {
        resolution: Resolution,
        container: Container,
    },
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Audio {
            format: AudioFormat::Mp3,
            quality: AudioQuality(Some(320)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsRuntime {
    Node,
    Deno,
    Bun,
    Quickjs,
}

impl JsRuntime {
    pub const ALL: [JsRuntime; 4] = [Self::Node, Self::Deno, Self::Bun, Self::Quickjs];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Deno => "deno",
            Self::Bun => "bun",
            Self::Quickjs => "quickjs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    Chrome,
    Chromium,
    Brave,
    Firefox,
    Edge,
    Opera,
    Vivaldi,
    Safari,
}

impl Browser {
    pub const ALL: [Browser; 8] = [
        Self::Chrome,
        Self::Chromium,
        Self::Brave,
        Self::Firefox,
        Self::Edge,
        Self::Opera,
        Self::Vivaldi,
        Self::Safari,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Chromium => "chromium",
            Self::Brave => "brave",
            Self::Firefox => "firefox",
            Self::Edge => "edge",
            Self::Opera => "opera",
            Self::Vivaldi => "vivaldi",
            Self::Safari => "safari",
        }
    }
}

/// Everything that shapes one yt-dlp invocation, apart from the URL and folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadOptions {
    pub mode: Mode,
    pub no_playlist: bool,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub embed_subs: bool,
    pub write_info_json: bool,
    pub overwrite: bool,
    pub js_runtime: Option<JsRuntime>,
    pub cookies_from: Option<Browser>,
    pub filename_template: String,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            no_playlist: true,
            embed_thumbnail: true,
            embed_metadata: true,
            embed_subs: false,
            write_info_json: false,
            overwrite: true,
            js_runtime: Some(JsRuntime::Node),
            cookies_from: None,
            filename_template: "%(title)s.%(ext)s".into(),
        }
    }
}
