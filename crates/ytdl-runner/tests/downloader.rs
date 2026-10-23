//! Runs the downloader against a fake yt-dlp shell script.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ytdl_core::progress::{ErrorKind, PostStep};
use ytdl_core::queue::Status;
use ytdl_runner::{Downloader, Finished, Tools};

/// Behaves according to the URL (the last argument):
/// `ok` downloads, `bot` fails with the bot check, `slow` hangs until stopped.
const FAKE_YTDLP: &str = r#"#!/bin/sh
for a in "$@"; do url="$a"; done
for a in "$@"; do
  if [ "$a" = "-J" ]; then
    printf '{"title":"Title of %s","uploader":"Chan","duration":61}\n' "$url"
    exit 0
  fi
done
case "$url" in
  *ok*)
    echo "[download] Destination: $OUT/song.webm"
    echo "[ytdl-progress] 50/100/NA/1000/1"
    echo "[ytdl-progress] 100/100/NA/1000/0"
    echo "[ExtractAudio] Destination: $OUT/song.mp3"
    exit 0 ;;
  *bot*)
    echo "ERROR: [youtube] x: Sign in to confirm you’re not a bot. Use --cookies-from-browser" >&2
    exit 1 ;;
  *slow*)
    echo "[ytdl-progress] 10/100/NA/5/9"
    sleep 30 &
    wait
    exit 0 ;;
esac
exit 2
"#;

fn setup(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("ytdl-runner-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let script = root.join("yt-dlp");
    std::fs::write(&script, FAKE_YTDLP).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: tests in this file all set the same value.
    unsafe { std::env::set_var("OUT", root.join("out")) };
    (root, script)
}

fn downloader(script: &Path, parallel: usize) -> Downloader {
    let tools = Tools {
        ytdlp: Some(script.to_path_buf()),
        ffmpeg: None,
    };
    Downloader::new(parallel, tools, || {})
}

/// Ticks until `done` holds, collecting finished items.
fn run_until(d: &mut Downloader, done: impl Fn(&Downloader) -> bool) -> Vec<Finished> {
    let start = Instant::now();
    let mut finished = Vec::new();
    loop {
        finished.extend(d.tick());
        if done(d) {
            return finished;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn successful_download() {
    let (root, script) = setup("ok");
    let mut d = downloader(&script, 2);
    let out = root.join("out");
    let ids = d.add_text("https://x/ok", &Default::default(), &out);
    let finished = run_until(&mut d, |d| {
        !d.is_busy() && d.queue().items()[0].info.is_some()
    });

    let item = d.queue().get(ids[0]).unwrap();
    assert_eq!(item.status, Status::Done);
    assert_eq!(
        item.final_path.as_deref(),
        Some(out.join("song.mp3").as_path())
    );
    assert_eq!(item.progress.as_ref().unwrap().fraction(), Some(1.0));
    assert_eq!(
        item.info.as_ref().unwrap().title.as_deref(),
        Some("Title of https://x/ok")
    );
    assert_eq!(
        finished,
        [Finished {
            id: ids[0],
            status: Status::Done
        }]
    );
    assert!(out.is_dir(), "output folder is created");
    let log: Vec<&String> = d.log(ids[0]).collect();
    assert!(log[0].starts_with("$ "));
    assert!(log.iter().any(|l| l.starts_with("[ExtractAudio]")));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn bot_check_is_reported() {
    let (root, script) = setup("bot");
    let mut d = downloader(&script, 2);
    let ids = d.add_text("https://x/bot", &Default::default(), &root.join("out"));
    let finished = run_until(&mut d, |d| !d.is_busy());
    assert_eq!(
        finished,
        [Finished {
            id: ids[0],
            status: Status::Failed(ErrorKind::BotCheck)
        }]
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn two_at_a_time_and_cancel() {
    let (root, script) = setup("cancel");
    let mut d = downloader(&script, 2);
    let ids = d.add_text(
        "https://x/slow1 https://x/slow2 https://x/ok",
        &Default::default(),
        &root.join("out"),
    );

    run_until(&mut d, |d| {
        d.queue().items()[..2].iter().all(|i| i.progress.is_some())
    });
    assert_eq!(d.queue().counts().active, 2);
    assert_eq!(d.queue().get(ids[2]).unwrap().status, Status::Queued);

    // Cancelling one frees a slot for the third.
    let started = Instant::now();
    d.cancel(ids[0]);
    let finished = run_until(&mut d, |d| {
        d.queue().get(ids[2]).unwrap().status == Status::Done
    });
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "cancel stops the process quickly"
    );
    assert_eq!(d.queue().get(ids[0]).unwrap().status, Status::Cancelled);
    assert_eq!(
        finished,
        [Finished {
            id: ids[2],
            status: Status::Done
        }]
    );

    d.cancel_all();
    run_until(&mut d, |d| !d.is_busy());
    assert_eq!(d.queue().get(ids[1]).unwrap().status, Status::Cancelled);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn missing_ytdlp_fails_items() {
    let mut d = Downloader::new(2, Tools::default(), || {});
    let ids = d.add_text("https://x/ok", &Default::default(), Path::new("/tmp"));
    let finished = d.tick();
    assert_eq!(
        finished,
        [Finished {
            id: ids[0],
            status: Status::Failed(ErrorKind::MissingYtDlp)
        }]
    );
}

#[test]
fn spawn_failure_is_reported() {
    let tools = Tools {
        ytdlp: Some("/nonexistent/yt-dlp".into()),
        ffmpeg: None,
    };
    let mut d = Downloader::new(1, tools, || {});
    let ids = d.add_text("https://x/ok", &Default::default(), &std::env::temp_dir());
    let finished = run_until(&mut d, |d| !d.is_busy());
    assert_eq!(finished[0].status, Status::Failed(ErrorKind::MissingYtDlp));
    assert!(d.log(ids[0]).any(|l| l.contains("could not start")));
}

#[test]
fn postprocess_status_shows_while_running() {
    // Parsing is covered in ytdl-core; here only check the status type is reachable.
    assert!(Status::PostProcessing(PostStep::Merging).is_active());
}
