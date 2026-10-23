//! Keeps yt-dlp current by installing the latest official build into the
//! app's data folder. The bundled copy stays untouched as a fallback.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use ytdl_core::tools::ytdlp_release as release;

/// Generous: the standalone builds are ~35 MB.
const MAX_DOWNLOAD: u64 = 200 * 1024 * 1024;
pub(crate) const USER_AGENT: &str = concat!("youtube-downloader/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("no yt-dlp build is published for this system")]
    Unsupported,
    #[error("download failed: {0}")]
    Http(String),
    #[error("the downloaded file did not match its published checksum")]
    Checksum,
    #[error("the checksum list does not mention {0}")]
    NoChecksum(String),
    #[error("could not save the download: {0}")]
    Io(#[from] std::io::Error),
}

impl From<ureq::Error> for UpdateError {
    fn from(e: ureq::Error) -> Self {
        UpdateError::Http(e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    UpToDate(String),
    Updated { from: Option<String>, to: String },
}

/// What `yt-dlp --version` prints, e.g. `2026.10.21`.
pub fn installed_version(ytdlp: &Path) -> Option<String> {
    let out = Command::new(ytdlp)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!v.is_empty()).then_some(v)
}

/// The system's certificates, so downloads also work behind proxies that the
/// user's machine trusts (common on company networks).
pub(crate) fn tls() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build()
}

pub(crate) fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(tls())
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .new_agent()
}

pub(crate) fn get_text(agent: &ureq::Agent, url: &str) -> Result<String, UpdateError> {
    Ok(agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()?
        .body_mut()
        .read_to_string()?)
}

/// The newest release tag, read from where GitHub redirects the
/// "latest" checksum list. No API call, so no rate limit.
pub fn latest_version() -> Result<String, UpdateError> {
    let no_redirects = ureq::Agent::config_builder()
        .tls_config(tls())
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .max_redirects_will_error(false)
        .build()
        .new_agent();
    let resp = no_redirects
        .get(release::LATEST_CHECKSUMS_URL)
        .header("User-Agent", USER_AGENT)
        .call()?;
    resp.headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .and_then(release::tag_from_redirect)
        .ok_or_else(|| UpdateError::Http("GitHub did not report the latest version".into()))
}

/// Downloads release `tag` for this system into `dest_dir/yt-dlp`.
pub fn install(tag: &str, dest_dir: &Path) -> Result<PathBuf, UpdateError> {
    let asset = release::asset_name(std::env::consts::OS, std::env::consts::ARCH)
        .ok_or(UpdateError::Unsupported)?;
    let agent = agent();
    let sums = get_text(&agent, &release::file_url(tag, "SHA2-256SUMS"))?;
    let bytes = agent
        .get(&release::file_url(tag, asset))
        .header("User-Agent", USER_AGENT)
        .call()?
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()?;
    install_bytes(&bytes, &sums, asset, dest_dir)
}

/// Verifies `bytes` against the checksum list and installs them as
/// `dest_dir/yt-dlp`, replacing any earlier copy in one step.
pub fn install_bytes(
    bytes: &[u8],
    sums: &str,
    asset: &str,
    dest_dir: &Path,
) -> Result<PathBuf, UpdateError> {
    let expected = release::expected_sha256(sums, asset)
        .ok_or_else(|| UpdateError::NoChecksum(asset.into()))?;
    if !release::sha256_matches(bytes, &expected) {
        return Err(UpdateError::Checksum);
    }
    std::fs::create_dir_all(dest_dir)?;
    let target = dest_dir.join("yt-dlp");
    let tmp = dest_dir.join("yt-dlp.download");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, &target)?;
    Ok(target)
}

/// Installs the latest yt-dlp into `dest_dir` unless `current` is already that version.
/// Blocking: run it on a background thread.
pub fn update(current: Option<&Path>, dest_dir: &Path) -> Result<UpdateOutcome, UpdateError> {
    let latest = latest_version()?;
    let installed = current.and_then(installed_version);
    if let Some(v) = &installed
        && !release::is_newer(&latest, v)
    {
        return Ok(UpdateOutcome::UpToDate(v.clone()));
    }
    install(&latest, dest_dir)?;
    Ok(UpdateOutcome::Updated {
        from: installed,
        to: latest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("ytdl-runner-update-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    const HELLO_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    #[test]
    fn installs_verified_file() {
        let dir = tmp("ok");
        let sums = format!("{HELLO_SHA}  yt-dlp_linux\n");
        let path = install_bytes(b"hello", &sums, "yt-dlp_linux", &dir).unwrap();
        assert_eq!(path, dir.join("yt-dlp"));
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o755
            );
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_bad_checksum_and_keeps_old_copy() {
        let dir = tmp("bad");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("yt-dlp"), "old").unwrap();
        let sums = format!("{HELLO_SHA}  yt-dlp_linux\n");
        assert!(matches!(
            install_bytes(b"tampered", &sums, "yt-dlp_linux", &dir),
            Err(UpdateError::Checksum)
        ));
        assert!(matches!(
            install_bytes(b"hello", &sums, "yt-dlp_macos", &dir),
            Err(UpdateError::NoChecksum(_))
        ));
        assert_eq!(std::fs::read(dir.join("yt-dlp")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Talks to GitHub: `cargo test -p ytdl-runner -- --ignored`.
    #[test]
    #[ignore]
    fn live_install() {
        let dir = tmp("live");
        let tag = latest_version().unwrap();
        let path = install(&tag, &dir).unwrap();
        assert_eq!(installed_version(&path).as_deref(), Some(tag.as_str()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
