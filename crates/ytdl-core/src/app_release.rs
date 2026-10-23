//! Finding newer releases of this app and the AppImage to update to.

pub use crate::tools::ytdlp_release::{expected_sha256, sha256_matches};

/// Redirects to `…/releases/download/<tag>/SHA256SUMS`, which tells us the
/// latest version without the rate-limited GitHub API.
pub const LATEST_CHECKSUMS_URL: &str =
    "https://github.com/genetzhs/Youtube-downloader/releases/latest/download/SHA256SUMS";
pub const RELEASES_PAGE: &str = "https://github.com/genetzhs/Youtube-downloader/releases/latest";
const DOWNLOAD_BASE: &str = "https://github.com/genetzhs/Youtube-downloader/releases/download/";

/// Reads the version (`0.1.4`) from the redirect target of [`LATEST_CHECKSUMS_URL`].
pub fn version_from_redirect(location: &str) -> Option<String> {
    let rest = location.split("/releases/download/").nth(1)?;
    let version = rest.split('/').next()?.strip_prefix('v')?;
    let valid = version.starts_with(|c: char| c.is_ascii_digit())
        && !version.contains("..")
        && version.chars().all(|c| c.is_ascii_digit() || c == '.');
    valid.then(|| version.to_string())
}

/// A file of release `version`, so the checksum list and the AppImage always
/// come from the same release.
pub fn file_url(version: &str, file: &str) -> String {
    format!("{DOWNLOAD_BASE}v{version}/{file}")
}

/// The AppImage published with each release (x86_64 only).
pub fn appimage_name(version: &str) -> String {
    format!("YouTube_Downloader-{version}-x86_64.AppImage")
}

/// Compares `1.2.3`-style versions as number lists.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.trim()
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parse(candidate) > parse(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_version_from_redirect() {
        assert_eq!(
            version_from_redirect(
                "https://github.com/genetzhs/Youtube-downloader/releases/download/v0.1.4/SHA256SUMS"
            )
            .as_deref(),
            Some("0.1.4")
        );
        assert_eq!(
            version_from_redirect("https://github.com/x/releases/download/0.1.4/S"),
            None
        );
        assert_eq!(
            version_from_redirect("https://github.com/x/releases/download/v../S"),
            None
        );
        assert_eq!(version_from_redirect("https://github.com/x/releases"), None);
    }

    #[test]
    fn version_order() {
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(is_newer("0.2.0", "0.1.3"));
        assert!(!is_newer("0.1.3", "0.1.3"));
        assert!(!is_newer("0.1.2", "0.1.3"));
    }

    #[test]
    fn urls() {
        assert_eq!(
            file_url("0.1.4", &appimage_name("0.1.4")),
            "https://github.com/genetzhs/Youtube-downloader/releases/download/v0.1.4/YouTube_Downloader-0.1.4-x86_64.AppImage"
        );
    }
}
