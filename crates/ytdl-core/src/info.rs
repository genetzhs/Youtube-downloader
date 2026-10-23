//! Title, channel and thumbnail, read from `yt-dlp -J --flat-playlist`.

use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaInfo {
    pub title: Option<String>,
    pub uploader: Option<String>,
    /// Seconds.
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    /// Number of videos when the link is a playlist.
    pub playlist_count: Option<usize>,
}

/// Parses the JSON yt-dlp prints for one video or one (flat) playlist.
pub fn parse_info(json: &str) -> Option<MediaInfo> {
    let v: Value = serde_json::from_str(json).ok()?;
    let text = |key: &str| {
        v.get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let is_playlist = v.get("_type").and_then(Value::as_str) == Some("playlist");
    let playlist_count = is_playlist.then(|| {
        v.get("playlist_count")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .or_else(|| v.get("entries").and_then(Value::as_array).map(Vec::len))
            .unwrap_or(0)
    });
    // Playlists often lack a thumbnail of their own; use the first entry's.
    let thumbnail = text("thumbnail")
        .or_else(|| last_thumbnail(&v))
        .or_else(|| {
            v.get("entries")
                .and_then(Value::as_array)
                .and_then(|e| e.first())
                .and_then(last_thumbnail)
        });
    Some(MediaInfo {
        title: text("title"),
        uploader: text("uploader").or_else(|| text("channel")),
        duration: v.get("duration").and_then(Value::as_f64),
        thumbnail,
        playlist_count,
    })
}

/// yt-dlp lists thumbnails from worst to best.
fn last_thumbnail(v: &Value) -> Option<String> {
    v.get("thumbnails")?
        .as_array()?
        .iter()
        .rev()
        .find_map(|t| t.get("url")?.as_str().map(str::to_string))
}

/// Duration as `4:12` or `1:02:03`.
pub fn human_duration(secs: f64) -> String {
    let s = secs.round() as u64;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video() {
        let json = r#"{"_type":"video","title":"A song","uploader":"Someone","duration":252.0,
            "thumbnail":"https://i.ytimg.com/vi/x/maxresdefault.jpg"}"#;
        let i = parse_info(json).unwrap();
        assert_eq!(i.title.as_deref(), Some("A song"));
        assert_eq!(i.uploader.as_deref(), Some("Someone"));
        assert_eq!(i.duration, Some(252.0));
        assert_eq!(
            i.thumbnail.as_deref(),
            Some("https://i.ytimg.com/vi/x/maxresdefault.jpg")
        );
        assert_eq!(i.playlist_count, None);
    }

    #[test]
    fn flat_playlist() {
        let json = r#"{"_type":"playlist","title":"Mix","channel":"Chan","entries":[
            {"title":"one","thumbnails":[{"url":"small"},{"url":"big"}]},{"title":"two"}]}"#;
        let i = parse_info(json).unwrap();
        assert_eq!(i.title.as_deref(), Some("Mix"));
        assert_eq!(i.uploader.as_deref(), Some("Chan"));
        assert_eq!(i.playlist_count, Some(2));
        assert_eq!(i.thumbnail.as_deref(), Some("big"));
    }

    #[test]
    fn junk() {
        assert_eq!(parse_info("not json"), None);
        assert_eq!(parse_info("{}"), Some(MediaInfo::default()));
    }

    #[test]
    fn durations() {
        assert_eq!(human_duration(252.0), "4:12");
        assert_eq!(human_duration(3723.4), "1:02:03");
        assert_eq!(human_duration(5.0), "0:05");
    }
}
