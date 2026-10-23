//! Sample queue rows for `--demo`, used for screenshots until the real
//! queue is connected (M4). Titles are placeholders on purpose.

use crate::app::{Row, RowState};

pub fn rows() -> Vec<Row> {
    vec![
        Row {
            title: "[Video title]".into(),
            meta: "1080p · MP4 · 154 of 248 MB · 12.4 MB/s · 8s left".into(),
            state: RowState::Downloading {
                fraction: 0.62,
                percent: "62%".into(),
            },
        },
        Row {
            title: "[Video title]".into(),
            meta: "Audio · MP3 320k · 2.6 of 9.4 MB · 3.1 MB/s".into(),
            state: RowState::Downloading {
                fraction: 0.28,
                percent: "28%".into(),
            },
        },
        Row {
            title: "[Playlist title] · 12 videos".into(),
            meta: "Waiting for a free slot".into(),
            state: RowState::Waiting,
        },
        Row {
            title: "[Video title]".into(),
            meta: "720p · MKV · 96 MB · finished".into(),
            state: RowState::Done,
        },
        Row {
            title: "YouTube wants you signed in".into(),
            meta: "Use your browser's cookies to continue.".into(),
            state: RowState::NeedsCookies,
        },
    ]
}
