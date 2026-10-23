//! Finding yt-dlp, ffmpeg and ffprobe.
//!
//! The packages (.deb, AppImage, macOS .app) ship all three. yt-dlp also
//! updates itself into the app's data folder, because YouTube changes often
//! and a bundled copy goes stale. Lookup order:
//! user override → updated copy → bundled copy → PATH → common install folders.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    YtDlp,
    Ffmpeg,
    Ffprobe,
}

impl Tool {
    pub fn exe_name(self) -> &'static str {
        match self {
            Tool::YtDlp => "yt-dlp",
            Tool::Ffmpeg => "ffmpeg",
            Tool::Ffprobe => "ffprobe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Override,
    Updated,
    Bundled,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub path: PathBuf,
    pub source: Source,
}

#[derive(Debug, Clone, Default)]
pub struct Locations {
    pub override_path: Option<PathBuf>,
    /// Where updated yt-dlp copies are stored (the app's data folder `bin/`).
    pub updated_dir: Option<PathBuf>,
    pub bundled_dirs: Vec<PathBuf>,
    /// The `PATH` variable.
    pub path_env: Option<OsString>,
    /// Folders that GUI apps often miss on PATH, such as `/opt/homebrew/bin`.
    pub extra_dirs: Vec<PathBuf>,
}

impl Locations {
    /// The usual setup for the running app.
    pub fn for_app(exe: &Path, data_dir: &Path, override_path: Option<PathBuf>) -> Self {
        Self {
            override_path,
            updated_dir: Some(data_dir.join("bin")),
            bundled_dirs: bundled_dirs(exe),
            path_env: std::env::var_os("PATH"),
            extra_dirs: ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"]
                .iter()
                .map(PathBuf::from)
                .collect(),
        }
    }
}

/// Where packages put the bundled tools, relative to the app's executable:
/// `.deb` and AppImage use `<prefix>/bin/app` + `<prefix>/lib/youtube-downloader/bin`,
/// macOS uses `Contents/MacOS/app` + `Contents/Resources/bin`.
pub fn bundled_dirs(exe: &Path) -> Vec<PathBuf> {
    let Some(dir) = exe.parent() else {
        return Vec::new();
    };
    vec![
        dir.join("../lib/youtube-downloader/bin"),
        dir.join("../Resources/bin"),
        dir.join("bin"),
    ]
}

pub fn locate(tool: Tool, loc: &Locations) -> Option<Found> {
    if let Some(p) = &loc.override_path
        && is_executable(p)
    {
        return Some(Found {
            path: p.clone(),
            source: Source::Override,
        });
    }
    let name = tool.exe_name();
    let in_dir = |d: &Path| Some(d.join(name)).filter(|p| is_executable(p));

    if tool == Tool::YtDlp
        && let Some(p) = loc.updated_dir.as_deref().and_then(in_dir)
    {
        return Some(Found {
            path: p,
            source: Source::Updated,
        });
    }
    if let Some(p) = loc.bundled_dirs.iter().find_map(|d| in_dir(d)) {
        return Some(Found {
            path: p,
            source: Source::Bundled,
        });
    }
    let path_dirs = loc
        .path_env
        .as_ref()
        .map(|p| std::env::split_paths(p).collect::<Vec<_>>())
        .unwrap_or_default();
    path_dirs
        .iter()
        .chain(&loc.extra_dirs)
        .find_map(|d| in_dir(d))
        .map(|p| Found {
            path: p,
            source: Source::System,
        })
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// yt-dlp self-update: the official standalone builds from GitHub releases.
pub mod ytdlp_release {
    use super::*;

    /// Redirects to `…/releases/download/<tag>/SHA2-256SUMS`, which tells us the
    /// latest version without the rate-limited GitHub API.
    pub const LATEST_CHECKSUMS_URL: &str =
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";
    const DOWNLOAD_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/download/";

    /// The release file for this platform (`std::env::consts::{OS, ARCH}`).
    pub fn asset_name(os: &str, arch: &str) -> Option<&'static str> {
        match (os, arch) {
            ("linux", "x86_64") => Some("yt-dlp_linux"),
            ("linux", "aarch64") => Some("yt-dlp_linux_aarch64"),
            // Universal binary for Intel and Apple Silicon.
            ("macos", _) => Some("yt-dlp_macos"),
            _ => None,
        }
    }

    /// A file of release `tag`, so the checksum list and the binary always
    /// come from the same release.
    pub fn file_url(tag: &str, file: &str) -> String {
        format!("{DOWNLOAD_BASE}{tag}/{file}")
    }

    /// Reads the tag from the redirect target of [`LATEST_CHECKSUMS_URL`].
    pub fn tag_from_redirect(location: &str) -> Option<String> {
        let rest = location.split("/releases/download/").nth(1)?;
        let tag = rest.split('/').next()?;
        let valid = tag.starts_with(|c: char| c.is_ascii_digit())
            && !tag.contains("..")
            && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '.');
        valid.then(|| tag.to_string())
    }

    /// Finds the hash for `asset` in a `SHA2-256SUMS` file (`<hex>  <name>` lines).
    pub fn expected_sha256(sums: &str, asset: &str) -> Option<String> {
        sums.lines().find_map(|l| {
            let mut parts = l.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            (name == asset && hash.len() == 64).then(|| hash.to_ascii_lowercase())
        })
    }

    pub fn sha256_matches(bytes: &[u8], expected_hex: &str) -> bool {
        hex::encode(Sha256::digest(bytes)) == expected_hex.to_ascii_lowercase()
    }

    /// yt-dlp versions are dates (`2026.10.10`, sometimes `.1` suffixed), so
    /// comparing them as number lists gives the right order.
    pub fn is_newer(candidate: &str, installed: &str) -> bool {
        let parse = |s: &str| -> Vec<u32> {
            s.trim()
                .split('.')
                .map(|p| p.parse().unwrap_or(0))
                .collect()
        };
        parse(candidate) > parse(installed)
    }
}

#[cfg(test)]
mod tests {
    use super::ytdlp_release::*;
    use super::*;

    fn fake_exe(dir: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        p
    }

    #[test]
    fn lookup_order() {
        let root = std::env::temp_dir().join(format!("ytdl-core-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let system = fake_exe(&root.join("sys"), "yt-dlp");
        let mut loc = Locations {
            path_env: Some(root.join("sys").into_os_string()),
            updated_dir: Some(root.join("data/bin")),
            bundled_dirs: vec![root.join("bundle")],
            ..Default::default()
        };
        assert_eq!(
            locate(Tool::YtDlp, &loc),
            Some(Found {
                path: system,
                source: Source::System
            })
        );

        let bundled = fake_exe(&root.join("bundle"), "yt-dlp");
        assert_eq!(
            locate(Tool::YtDlp, &loc).unwrap(),
            Found {
                path: bundled,
                source: Source::Bundled
            }
        );

        let updated = fake_exe(&root.join("data/bin"), "yt-dlp");
        assert_eq!(
            locate(Tool::YtDlp, &loc).unwrap(),
            Found {
                path: updated,
                source: Source::Updated
            }
        );

        let own = fake_exe(&root.join("own"), "my-yt-dlp");
        loc.override_path = Some(own.clone());
        assert_eq!(
            locate(Tool::YtDlp, &loc).unwrap(),
            Found {
                path: own,
                source: Source::Override
            }
        );

        // ffmpeg is never taken from the yt-dlp update folder.
        fake_exe(&root.join("data/bin"), "ffmpeg");
        loc.override_path = None;
        assert_eq!(locate(Tool::Ffmpeg, &loc), None);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn bundled_dirs_cover_all_packages() {
        let deb = bundled_dirs(Path::new("/usr/bin/youtube-downloader"));
        assert!(deb.contains(&PathBuf::from("/usr/bin/../lib/youtube-downloader/bin")));
        let mac = bundled_dirs(Path::new(
            "/Applications/YouTube Downloader.app/Contents/MacOS/app",
        ));
        assert!(mac.contains(&PathBuf::from(
            "/Applications/YouTube Downloader.app/Contents/MacOS/../Resources/bin"
        )));
    }

    #[test]
    fn release_assets() {
        assert_eq!(asset_name("linux", "x86_64"), Some("yt-dlp_linux"));
        assert_eq!(asset_name("linux", "aarch64"), Some("yt-dlp_linux_aarch64"));
        assert_eq!(asset_name("macos", "aarch64"), Some("yt-dlp_macos"));
        assert_eq!(asset_name("windows", "x86_64"), None);
        assert_eq!(
            file_url("2026.10.21", "yt-dlp_linux"),
            "https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.21/yt-dlp_linux"
        );
    }

    #[test]
    fn checksums() {
        let data = b"hello";
        let hash = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        let sums = format!("{}  yt-dlp\n{hash}  yt-dlp_linux\n", "0".repeat(64));
        assert_eq!(
            expected_sha256(&sums, "yt-dlp_linux").as_deref(),
            Some(hash)
        );
        assert_eq!(expected_sha256(&sums, "yt-dlp_macos"), None);
        assert!(sha256_matches(data, hash));
        assert!(!sha256_matches(b"other", hash));
    }

    #[test]
    fn versions() {
        assert_eq!(
            tag_from_redirect(
                "https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.21/SHA2-256SUMS"
            )
            .as_deref(),
            Some("2026.10.21")
        );
        assert_eq!(tag_from_redirect("https://github.com/login"), None);
        assert_eq!(tag_from_redirect("/releases/download/../x"), None);
        assert!(is_newer("2026.10.10", "2026.09.30"));
        assert!(is_newer("2026.10.10.1", "2026.10.10"));
        assert!(!is_newer("2026.10.10", "2026.10.10"));
    }
}
