//! Checks for newer releases of this app and, when it runs as an AppImage,
//! replaces the AppImage file with the new one.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ytdl_core::app_release as release;

use crate::update::{USER_AGENT, UpdateError, agent, get_text, tls};

/// The AppImage is ~120 MB with yt-dlp and ffmpeg inside.
const MAX_DOWNLOAD: u64 = 600 * 1024 * 1024;

/// The newest published version of the app, e.g. `0.1.4`.
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
        .and_then(release::version_from_redirect)
        .ok_or_else(|| UpdateError::Http("GitHub did not report the latest version".into()))
}

/// The AppImage this process runs from, if any. The AppImage runtime sets
/// `APPIMAGE` to its path.
pub fn running_appimage() -> Option<PathBuf> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let path = PathBuf::from(std::env::var_os("APPIMAGE")?);
    path.is_file().then_some(path)
}

/// Downloads the AppImage of `version` and puts it in place of `target`.
/// Blocking: run it on a background thread.
pub fn install_appimage(version: &str, target: &Path) -> Result<(), UpdateError> {
    let asset = release::appimage_name(version);
    let agent = agent();
    let sums = get_text(&agent, &release::file_url(version, "SHA256SUMS"))?;
    let bytes = agent
        .get(&release::file_url(version, &asset))
        .header("User-Agent", USER_AGENT)
        .call()?
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()?;
    replace_verified(&bytes, &sums, &asset, target)
}

/// Verifies `bytes` and swaps them in for `target` in one step. The running
/// AppImage keeps working: Linux keeps the old file alive until it exits.
pub fn replace_verified(
    bytes: &[u8],
    sums: &str,
    asset: &str,
    target: &Path,
) -> Result<(), UpdateError> {
    let expected = release::expected_sha256(sums, asset)
        .ok_or_else(|| UpdateError::NoChecksum(asset.into()))?;
    if !release::sha256_matches(bytes, &expected) {
        return Err(UpdateError::Checksum);
    }
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(".download");
    let tmp = PathBuf::from(tmp);
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
    if let Err(e) = std::fs::rename(&tmp, target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    #[test]
    fn replaces_only_verified_file() {
        let dir = std::env::temp_dir().join(format!("ytdl-self-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("App.AppImage");
        std::fs::write(&target, "old").unwrap();
        let sums = format!("{HELLO_SHA}  App-1.AppImage\n");
        assert!(matches!(
            replace_verified(b"tampered", &sums, "App-1.AppImage", &target),
            Err(UpdateError::Checksum)
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        replace_verified(b"hello", &sums, "App-1.AppImage", &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"hello");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&target).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Talks to GitHub: `cargo test -p ytdl-runner -- --ignored`.
    #[test]
    #[ignore]
    fn live_latest_version() {
        let v = latest_version().unwrap();
        assert!(v.starts_with(|c: char| c.is_ascii_digit()));
    }
}
