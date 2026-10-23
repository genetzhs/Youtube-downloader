//! Reads yt-dlp's output one line at a time and turns it into typed events.

use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;

use crate::command::PROGRESS_MARKER;

#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub downloaded: u64,
    /// Exact size when known, otherwise yt-dlp's estimate.
    pub total: Option<u64>,
    /// Bytes per second.
    pub speed: Option<f64>,
    /// Seconds left.
    pub eta: Option<u64>,
}

impl Progress {
    /// 0.0..=1.0, or `None` when the size is unknown.
    pub fn fraction(&self) -> Option<f32> {
        let total = self.total.filter(|&t| t > 0)?;
        Some((self.downloaded as f64 / total as f64).clamp(0.0, 1.0) as f32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostStep {
    Merging,
    ExtractingAudio,
    /// Thumbnail, metadata, subtitles and other fix-ups.
    Finishing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// "Sign in to confirm you're not a bot": retry with browser cookies.
    BotCheck,
    /// Age-restricted or members-only content that needs a signed-in account.
    SignInRequired,
    Private,
    Unavailable,
    UnsupportedUrl,
    MissingFfmpeg,
    /// yt-dlp itself could not be found or started.
    MissingYtDlp,
    Network,
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Progress(Progress),
    /// A file is being written; for video this may be one of several streams.
    Destination(PathBuf),
    AlreadyDownloaded(PathBuf),
    PostProcess {
        step: PostStep,
        path: Option<PathBuf>,
    },
    PlaylistItem {
        index: u32,
        count: u32,
    },
    Error(ErrorKind),
}

impl Event {
    /// The path this event says the finished file will have, if any.
    pub fn final_path(&self) -> Option<&PathBuf> {
        match self {
            Event::AlreadyDownloaded(p) => Some(p),
            Event::PostProcess {
                step: PostStep::Merging | PostStep::ExtractingAudio,
                path: Some(p),
            } => Some(p),
            Event::Destination(p) if !is_partial_stream(p) => Some(p),
            _ => None,
        }
    }
}

static PARTIAL_STREAM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.f\d+(-\d+)?\.\w+$").unwrap());
static PLAYLIST_ITEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[download\] Downloading (?:item|video) (\d+) of (\d+)").unwrap()
});

/// Separate audio/video streams before merging look like `title.f137.mp4`.
fn is_partial_stream(p: &std::path::Path) -> bool {
    PARTIAL_STREAM.is_match(&p.to_string_lossy())
}

pub fn parse_line(line: &str) -> Option<Event> {
    let line = line.trim_end();

    if let Some(rest) = line.strip_prefix(PROGRESS_MARKER) {
        return parse_progress(rest.trim()).map(Event::Progress);
    }
    if let Some(msg) = line.strip_prefix("ERROR:") {
        return Some(Event::Error(classify_error(msg.trim())));
    }
    if let Some(p) = line.strip_prefix("[download] Destination: ") {
        return Some(Event::Destination(PathBuf::from(p)));
    }
    if let Some(rest) = line.strip_prefix("[download] ")
        && let Some(p) = rest.strip_suffix(" has already been downloaded")
    {
        return Some(Event::AlreadyDownloaded(PathBuf::from(p)));
    }
    if let Some(c) = PLAYLIST_ITEM.captures(line) {
        return Some(Event::PlaylistItem {
            index: c[1].parse().ok()?,
            count: c[2].parse().ok()?,
        });
    }
    if let Some(p) = line.strip_prefix("[Merger] Merging formats into ") {
        return Some(Event::PostProcess {
            step: PostStep::Merging,
            path: Some(PathBuf::from(p.trim_matches('"'))),
        });
    }
    if let Some(p) = line.strip_prefix("[ExtractAudio] Destination: ") {
        return Some(Event::PostProcess {
            step: PostStep::ExtractingAudio,
            path: Some(PathBuf::from(p)),
        });
    }
    if let Some(rest) = line.strip_prefix("[ExtractAudio] Not converting audio ")
        && let Some((p, _)) = rest.rsplit_once("; file is already in target format")
    {
        return Some(Event::PostProcess {
            step: PostStep::ExtractingAudio,
            path: Some(PathBuf::from(p)),
        });
    }
    const FINISHING: [&str; 6] = [
        "[EmbedThumbnail]",
        "[Metadata]",
        "[EmbedSubtitle]",
        "[VideoConvertor]",
        "[VideoRemuxer]",
        "[ThumbnailsConvertor]",
    ];
    if FINISHING.iter().any(|p| line.starts_with(p)) || line.starts_with("[Fixup") {
        return Some(Event::PostProcess {
            step: PostStep::Finishing,
            path: None,
        });
    }
    None
}

fn parse_progress(s: &str) -> Option<Progress> {
    let f: Vec<&str> = s.split('/').collect();
    let [downloaded, total, estimate, speed, eta] = f[..] else {
        return None;
    };
    Some(Progress {
        downloaded: num(downloaded)? as u64,
        total: num(total).or_else(|| num(estimate)).map(|t| t as u64),
        speed: num(speed),
        eta: num(eta).map(|e| e as u64),
    })
}

/// yt-dlp prints `NA` (or `None`) for missing values and may print floats.
fn num(s: &str) -> Option<f64> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
}

pub fn classify_error(msg: &str) -> ErrorKind {
    let m = msg.to_lowercase();
    if m.contains("confirm you") && m.contains("not a bot") {
        ErrorKind::BotCheck
    } else if m.contains("sign in to confirm your age")
        || m.contains("members-only")
        || m.contains("join this channel")
        || m.contains("use --cookies")
    {
        ErrorKind::SignInRequired
    } else if m.contains("private video") {
        ErrorKind::Private
    } else if m.contains("video unavailable") || m.contains("this video is not available") {
        ErrorKind::Unavailable
    } else if m.contains("unsupported url") {
        ErrorKind::UnsupportedUrl
    } else if m.contains("ffmpeg") && (m.contains("not found") || m.contains("not installed")) {
        ErrorKind::MissingFfmpeg
    } else if m.contains("unable to download")
        || m.contains("timed out")
        || m.contains("connection")
        || m.contains("network is unreachable")
    {
        ErrorKind::Network
    } else {
        ErrorKind::Other(msg.to_string())
    }
}

/// Bytes as `12.4 MB` (decimal units, like file managers on macOS and GNOME).
pub fn human_bytes(b: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b;
    let mut i = 0;
    while v >= 1000.0 && i < UNITS.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 || v >= 100.0 {
        format!("{v:.0} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

/// Seconds as `8s`, `3:07` or `1:02:03`.
pub fn human_eta(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else if m > 0 {
        format!("{m}:{s:02}")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_line() {
        let e = parse_line("[ytdl-progress] 1048576/4194304/NA/524288.5/6").unwrap();
        let Event::Progress(p) = e else { panic!() };
        assert_eq!(p.downloaded, 1_048_576);
        assert_eq!(p.total, Some(4_194_304));
        assert_eq!(p.speed, Some(524_288.5));
        assert_eq!(p.eta, Some(6));
        assert_eq!(p.fraction(), Some(0.25));
    }

    #[test]
    fn progress_falls_back_to_estimate() {
        let Some(Event::Progress(p)) = parse_line("[ytdl-progress] 500/NA/1000.0/NA/NA") else {
            panic!()
        };
        assert_eq!(p.total, Some(1000));
        assert_eq!(p.speed, None);
        assert_eq!(p.eta, None);
        assert_eq!(p.fraction(), Some(0.5));
    }

    #[test]
    fn progress_unknown_size() {
        let Some(Event::Progress(p)) = parse_line("[ytdl-progress] 500/NA/NA/None/NA") else {
            panic!()
        };
        assert_eq!(p.total, None);
        assert_eq!(p.fraction(), None);
    }

    #[test]
    fn destinations_and_final_path() {
        let part = parse_line("[download] Destination: /v/Song.f137.mp4").unwrap();
        assert_eq!(part, Event::Destination("/v/Song.f137.mp4".into()));
        assert_eq!(part.final_path(), None);

        let merged = parse_line(r#"[Merger] Merging formats into "/v/Song.mp4""#).unwrap();
        assert_eq!(merged.final_path(), Some(&PathBuf::from("/v/Song.mp4")));

        let audio = parse_line("[ExtractAudio] Destination: /m/Song name.mp3").unwrap();
        assert_eq!(audio.final_path(), Some(&PathBuf::from("/m/Song name.mp3")));

        let again = parse_line("[download] /m/Song.mp3 has already been downloaded").unwrap();
        assert_eq!(again, Event::AlreadyDownloaded("/m/Song.mp3".into()));
    }

    /// Output captured from yt-dlp 2026.10.21 with the arguments `build_args` produces.
    const REAL_RUN: &str = "\
[generic] 1-second-of-silence: Downloading webpage
[info] 1-second-of-silence: Downloading 1 format(s): mpeg
[download] Destination: /tmp/x/1-second-of-silence.mp3
[ytdl-progress] 1024/37206/NA/NA/NA
[ytdl-progress] 7168/37206/NA/422816.23311675526/0
[ytdl-progress] 37206/37206/NA/97684.64678961967/NA
[ExtractAudio] Not converting audio /tmp/x/1-second-of-silence.mp3; file is already in target format mp3
[Metadata] Adding metadata to \"/tmp/x/1-second-of-silence.mp3\"";

    #[test]
    fn real_run() {
        let events: Vec<Event> = REAL_RUN.lines().filter_map(parse_line).collect();
        assert_eq!(events.len(), 6);
        let Event::Progress(p) = &events[2] else {
            panic!()
        };
        assert_eq!(p.speed, Some(422816.23311675526));
        assert_eq!(p.eta, Some(0));
        let Event::Progress(last) = &events[3] else {
            panic!()
        };
        assert_eq!(last.fraction(), Some(1.0));
        assert_eq!(
            events[4].final_path(),
            Some(&PathBuf::from("/tmp/x/1-second-of-silence.mp3"))
        );
    }

    #[test]
    fn postprocess_steps() {
        for l in [
            "[EmbedThumbnail] ffmpeg: Adding thumbnail to \"/m/a.mp3\"",
            "[Metadata] Adding metadata to \"/m/a.mp3\"",
            "[FixupM3u8] Fixing MPEG-TS in MP4 container",
        ] {
            assert!(matches!(
                parse_line(l),
                Some(Event::PostProcess {
                    step: PostStep::Finishing,
                    ..
                })
            ));
        }
    }

    #[test]
    fn playlist_item() {
        assert_eq!(
            parse_line("[download] Downloading item 3 of 12"),
            Some(Event::PlaylistItem {
                index: 3,
                count: 12
            })
        );
    }

    #[test]
    fn errors() {
        let bot = "ERROR: [youtube] abc: Sign in to confirm you’re not a bot. Use --cookies-from-browser or --cookies for the authentication.";
        assert_eq!(parse_line(bot), Some(Event::Error(ErrorKind::BotCheck)));
        assert_eq!(
            parse_line(
                "ERROR: [youtube] abc: Private video. Sign in if you've been granted access"
            ),
            Some(Event::Error(ErrorKind::Private))
        );
        assert_eq!(
            parse_line("ERROR: [youtube] abc: Video unavailable"),
            Some(Event::Error(ErrorKind::Unavailable))
        );
        assert_eq!(
            parse_line("ERROR: Unsupported URL: https://example.com"),
            Some(Event::Error(ErrorKind::UnsupportedUrl))
        );
        assert_eq!(
            parse_line(
                "ERROR: Postprocessing: ffprobe and ffmpeg not found. Please install or provide the path using --ffmpeg-location"
            ),
            Some(Event::Error(ErrorKind::MissingFfmpeg))
        );
        assert_eq!(
            parse_line("ERROR: something odd"),
            Some(Event::Error(ErrorKind::Other("something odd".into())))
        );
    }

    #[test]
    fn noise_is_ignored() {
        assert_eq!(parse_line("[youtube] Extracting URL: https://x"), None);
        assert_eq!(parse_line("[info] abc: Downloading 1 format(s): 251"), None);
        assert_eq!(parse_line(""), None);
    }

    #[test]
    fn formatting() {
        assert_eq!(human_bytes(512.0), "512 B");
        assert_eq!(human_bytes(12_400_000.0), "12.4 MB");
        assert_eq!(human_bytes(248_000_000.0), "248 MB");
        assert_eq!(human_eta(8), "8s");
        assert_eq!(human_eta(187), "3:07");
        assert_eq!(human_eta(3723), "1:02:03");
    }
}
