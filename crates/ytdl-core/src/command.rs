//! Turns [`DownloadOptions`] into yt-dlp arguments.

use std::path::Path;

use crate::options::{AudioFormat, Container, DownloadOptions, Mode};

/// Prefix of the machine-readable progress lines we ask yt-dlp to print.
pub const PROGRESS_MARKER: &str = "[ytdl-progress]";

/// Fields are separated by `/` because none of them can contain one.
const PROGRESS_TEMPLATE: &str = "download:[ytdl-progress] %(progress.downloaded_bytes)s/%(progress.total_bytes)s/%(progress.total_bytes_estimate)s/%(progress.speed)s/%(progress.eta)s";

/// Arguments for one download, without the program name.
pub fn build_args(opts: &DownloadOptions, out_dir: &Path, url: &str) -> Vec<String> {
    let mut args = option_args(opts);
    args.push("--newline".into());
    args.push("--progress-template".into());
    args.push(PROGRESS_TEMPLATE.into());
    args.push("-P".into());
    args.push(out_dir.display().to_string());
    args.push("-o".into());
    args.push(opts.filename_template.clone());
    // `--` keeps a URL that starts with `-` from being read as an option.
    args.push("--".into());
    args.push(url.into());
    args
}

/// Arguments that look up title, channel and thumbnail without downloading.
/// Uses the same cookies and JS runtime as the download, so a link that needs
/// them does not fail here first.
pub fn info_args(opts: &DownloadOptions, url: &str) -> Vec<String> {
    let mut args: Vec<String> = ["-J", "--flat-playlist", "--no-warnings"]
        .map(String::from)
        .to_vec();
    if opts.no_playlist {
        args.push("--no-playlist".into());
    }
    if let Some(rt) = opts.js_runtime {
        args.push("--js-runtimes".into());
        args.push(rt.as_str().into());
    }
    if let Some(b) = opts.cookies_from {
        args.push("--cookies-from-browser".into());
        args.push(b.as_str().into());
    }
    args.push("--".into());
    args.push(url.into());
    args
}

/// The command as the user sees it in the preview: only the options they control.
pub fn preview(opts: &DownloadOptions) -> String {
    let mut parts = vec!["yt-dlp".to_string()];
    parts.extend(option_args(opts));
    parts.push("-o".into());
    parts.push(opts.filename_template.clone());
    parts.push("<url>".into());
    parts
        .iter()
        .map(|p| shell_quote(p))
        .collect::<Vec<_>>()
        .join(" ")
}

fn option_args(opts: &DownloadOptions) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    let mut push = |s: &str| a.push(s.to_string());

    if opts.no_playlist {
        push("--no-playlist");
    }

    let mut thumbnail_ok = true;
    match opts.mode {
        Mode::Audio { format, quality } => {
            push("-x");
            if format != AudioFormat::Best {
                push("--audio-format");
                push(format.as_str());
            }
            if !format.is_lossless() {
                push("--audio-quality");
                push(&quality.arg());
            }
        }
        Mode::Video {
            resolution,
            container,
        } => {
            let mut sort: Vec<String> = Vec::new();
            if let Some(h) = resolution.0 {
                sort.push(format!("res:{h}"));
            }
            match container {
                Container::Mp4 => sort.push("ext:mp4:m4a".into()),
                Container::Webm => {
                    sort.push("ext:webm:webm".into());
                    // yt-dlp cannot embed cover art into webm.
                    thumbnail_ok = false;
                }
                Container::Mkv => {}
            }
            if !sort.is_empty() {
                push("-S");
                push(&sort.join(","));
            }
            push("--merge-output-format");
            push(container.as_str());
        }
    }

    if opts.embed_thumbnail && thumbnail_ok {
        push("--embed-thumbnail");
    }
    if opts.embed_metadata {
        push("--embed-metadata");
    }
    if opts.embed_subs {
        push("--embed-subs");
    }
    if opts.write_info_json {
        push("--write-info-json");
    }
    if opts.overwrite {
        push("--force-overwrites");
    }
    if let Some(rt) = opts.js_runtime {
        push("--js-runtimes");
        push(rt.as_str());
    }
    if let Some(b) = opts.cookies_from {
        push("--cookies-from-browser");
        push(b.as_str());
    }
    a
}

/// Quotes a token for display in a POSIX shell. Only used for previews and logs.
pub fn shell_quote(s: &str) -> String {
    let safe = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=,@%+<>".contains(c));
    if safe && !s.contains("%(") {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::*;

    fn audio(format: AudioFormat, kbps: Option<u16>) -> DownloadOptions {
        DownloadOptions {
            mode: Mode::Audio {
                format,
                quality: AudioQuality(kbps),
            },
            ..Default::default()
        }
    }

    fn video(h: Option<u16>, container: Container) -> DownloadOptions {
        DownloadOptions {
            mode: Mode::Video {
                resolution: Resolution(h),
                container,
            },
            ..Default::default()
        }
    }

    fn has_pair(args: &[String], flag: &str, value: &str) -> bool {
        args.windows(2).any(|w| w[0] == flag && w[1] == value)
    }

    #[test]
    fn default_audio_matches_python_app() {
        let args = build_args(
            &DownloadOptions::default(),
            Path::new("/music"),
            "https://x",
        );
        assert_eq!(
            &args[..12],
            [
                "--no-playlist",
                "-x",
                "--audio-format",
                "mp3",
                "--audio-quality",
                "320",
                "--embed-thumbnail",
                "--embed-metadata",
                "--force-overwrites",
                "--js-runtimes",
                "node",
                "--newline",
            ]
        );
        assert!(has_pair(&args, "-P", "/music"));
        assert!(has_pair(&args, "-o", "%(title)s.%(ext)s"));
        assert_eq!(&args[args.len() - 2..], ["--", "https://x"]);
    }

    #[test]
    fn best_audio_skips_format_and_uses_quality_zero() {
        let args = build_args(&audio(AudioFormat::Best, None), Path::new("/m"), "u");
        assert!(!args.iter().any(|a| a == "--audio-format"));
        assert!(has_pair(&args, "--audio-quality", "0"));
    }

    #[test]
    fn lossless_audio_skips_bitrate() {
        let args = build_args(&audio(AudioFormat::Flac, Some(320)), Path::new("/m"), "u");
        assert!(has_pair(&args, "--audio-format", "flac"));
        assert!(!args.iter().any(|a| a == "--audio-quality"));
    }

    #[test]
    fn video_1080_mp4() {
        let args = build_args(&video(Some(1080), Container::Mp4), Path::new("/v"), "u");
        assert!(!args.iter().any(|a| a == "-x"));
        assert!(has_pair(&args, "-S", "res:1080,ext:mp4:m4a"));
        assert!(has_pair(&args, "--merge-output-format", "mp4"));
        assert!(args.iter().any(|a| a == "--embed-thumbnail"));
    }

    #[test]
    fn video_best_mkv_has_no_sort() {
        let args = build_args(&video(None, Container::Mkv), Path::new("/v"), "u");
        assert!(!args.iter().any(|a| a == "-S"));
        assert!(has_pair(&args, "--merge-output-format", "mkv"));
    }

    #[test]
    fn webm_never_embeds_thumbnail() {
        let args = build_args(&video(Some(720), Container::Webm), Path::new("/v"), "u");
        assert!(has_pair(&args, "-S", "res:720,ext:webm:webm"));
        assert!(!args.iter().any(|a| a == "--embed-thumbnail"));
    }

    #[test]
    fn optional_flags() {
        let opts = DownloadOptions {
            no_playlist: false,
            embed_subs: true,
            write_info_json: true,
            overwrite: false,
            js_runtime: None,
            cookies_from: Some(Browser::Firefox),
            ..Default::default()
        };
        let args = build_args(&opts, Path::new("/m"), "u");
        assert!(
            !args
                .iter()
                .any(|a| a == "--no-playlist" || a == "--force-overwrites")
        );
        assert!(!args.iter().any(|a| a == "--js-runtimes"));
        assert!(args.iter().any(|a| a == "--embed-subs"));
        assert!(args.iter().any(|a| a == "--write-info-json"));
        assert!(has_pair(&args, "--cookies-from-browser", "firefox"));
    }

    #[test]
    fn preview_is_readable() {
        assert_eq!(
            preview(&DownloadOptions::default()),
            "yt-dlp --no-playlist -x --audio-format mp3 --audio-quality 320 --embed-thumbnail \
             --embed-metadata --force-overwrites --js-runtimes node -o '%(title)s.%(ext)s' <url>"
        );
    }

    #[test]
    fn shell_quote_escapes() {
        assert_eq!(shell_quote("plain"), "plain");
        assert_eq!(shell_quote("has space"), "'has space'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_quote(""), "''");
    }
}
