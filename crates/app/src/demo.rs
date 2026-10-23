//! Sample queue rows for `--demo`, used for screenshots and for trying the
//! layout without downloading anything. Titles are placeholders on purpose.

use ytdl_core::options::Browser;

use crate::views::{DetailsView, Fix, RowState, RowView, Stage};
use crate::widgets::Ring;

pub fn rows() -> Vec<RowView> {
    let row = |id, title: &str, meta: &str, state| RowView {
        id,
        title: title.into(),
        meta: meta.into(),
        state,
    };
    vec![
        row(
            1,
            "[Video title]",
            "1080p · MP4 · 154 MB of 248 MB · 12.4 MB/s · 8s left",
            RowState::Active {
                ring: Ring::Progress(0.62),
                percent: Some("62%".into()),
            },
        ),
        row(
            2,
            "[Video title]",
            "MP3 · 320k · 2.6 MB of 9.4 MB · 3.1 MB/s · 2s left",
            RowState::Active {
                ring: Ring::Progress(0.28),
                percent: Some("28%".into()),
            },
        ),
        row(
            3,
            "[Playlist title]",
            "1080p · MP4 · Waiting for a free slot",
            RowState::Waiting,
        ),
        row(
            4,
            "[Video title]",
            "720p · MKV · 96 MB · finished",
            RowState::Done,
        ),
        row(
            5,
            "YouTube wants you signed in",
            "Use your browser's cookies to continue · [Video title]",
            RowState::Problem {
                fix: Fix::UseCookies(Browser::Firefox),
            },
        ),
    ]
}

pub fn details() -> DetailsView {
    DetailsView {
        id: 1,
        title: "[Video title]".into(),
        channel: Some("[Channel]".into()),
        duration: Some("12:48".into()),
        thumbnail: None,
        fraction: Some(0.62),
        facts: vec![
            ("Format".into(), "1080p · MP4".into()),
            ("Size".into(), "248 MB".into()),
            ("Speed".into(), "12.4 MB/s".into()),
            ("Time left".into(), "8s".into()),
        ],
        path_label: "Saving to",
        path: "~/Videos/YouTube/[Video title].mp4".into(),
        stage: Stage::Working,
        audio: false,
    }
}
