# YouTube Downloader

A native desktop app for [yt-dlp](https://github.com/yt-dlp/yt-dlp), written in Rust with
[egui](https://github.com/emilk/egui). It replaces the earlier Python/CustomTkinter app
([genetzhs/youtubeDL](https://github.com/genetzhs/youtubeDL)).

- Audio (mp3, m4a, opus, flac, wav, vorbis) and video (up to 2160p, MP4/MKV/WebM)
- A queue that runs two downloads at a time
- Browser cookies for YouTube's "confirm you're not a bot" check
- Ships with yt-dlp, ffmpeg and ffprobe inside the package; yt-dlp keeps itself up to date
- Linux (`.deb`, AppImage) and macOS (`.app` / `.dmg`)

## Status

Work in progress. Milestones:

| | Milestone | State |
|---|---|---|
| M1 | `ytdl-core`: command builder, progress parser, queue, settings, history, tool lookup | done |
| M2 | Runner: spawn yt-dlp, stream output, cancel, two at a time, yt-dlp self-update | next |
| M3 | Theme and custom egui widgets (dark design) | |
| M4 | Main window: link field, Audio/Video, quality chips, queue, details panel | |
| M5 | Settings, history, first-run setup | |
| M6 | Packaging: `.deb`, AppImage, macOS `.app`/`.dmg`, release workflow | |

## Layout

```
crates/
  ytdl-core/   UI-independent logic, fully unit tested
assets/        app icons
```

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## License

MIT. Release packages also bundle yt-dlp (Unlicense) and ffmpeg (GPL/LGPL builds),
which keep their own licenses.
