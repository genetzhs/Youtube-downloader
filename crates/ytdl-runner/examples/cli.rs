//! Downloads the links given on the command line and prints progress.
//! Useful for trying the runner without the UI:
//! `cargo run -p ytdl-runner --example cli -- <url>... [--out DIR] [--update]`

use std::path::PathBuf;
use std::time::Duration;

use ytdl_core::progress::{human_bytes, human_eta};
use ytdl_core::tools::{Locations, Tool, locate};
use ytdl_runner::{Downloader, Tools, update};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let out = match args.iter().position(|a| a == "--out") {
        Some(i) => {
            args.remove(i);
            PathBuf::from(args.remove(i))
        }
        None => std::env::temp_dir().join("ytdl-cli"),
    };
    let data = std::env::temp_dir().join("ytdl-cli-data");
    let exe = std::env::current_exe().unwrap();

    if let Some(i) = args.iter().position(|a| a == "--update") {
        args.remove(i);
        let loc = Locations::for_app(&exe, &data, None);
        let current = locate(Tool::YtDlp, &loc).map(|f| f.path);
        match update::update(current.as_deref(), &data.join("bin")) {
            Ok(o) => println!("update: {o:?}"),
            Err(e) => println!("update failed: {e}"),
        }
    }

    let loc = Locations::for_app(&exe, &data, None);
    let tools = Tools {
        ytdlp: locate(Tool::YtDlp, &loc).map(|f| f.path),
        ffmpeg: locate(Tool::Ffmpeg, &loc).map(|f| f.path),
    };
    println!("tools: {tools:?}");
    let mut d = Downloader::new(2, tools, || {});
    d.add_text(&args.join("\n"), &Default::default(), &out);

    while d.is_busy() {
        for f in d.tick() {
            println!("finished #{}: {:?}", f.id, f.status);
        }
        for item in d.queue().items().iter().filter(|i| i.status.is_active()) {
            let p = item.progress.as_ref();
            println!(
                "#{} {:?} {} {} {}",
                item.id,
                item.status,
                p.and_then(|p| p.fraction())
                    .map(|f| format!("{:.0}%", f * 100.0))
                    .unwrap_or_default(),
                p.and_then(|p| p.speed)
                    .map(|s| format!("{}/s", human_bytes(s)))
                    .unwrap_or_default(),
                p.and_then(|p| p.eta).map(human_eta).unwrap_or_default(),
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    for f in d.tick() {
        println!("finished #{}: {:?}", f.id, f.status);
    }
    for item in d.queue().items() {
        println!(
            "#{} {:?} title={:?} file={:?}",
            item.id,
            item.status,
            item.info.as_ref().and_then(|i| i.title.clone()),
            item.final_path
        );
    }
}
