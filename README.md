# YouTube Downloader

[![CI](https://github.com/genetzhs/Youtube-downloader/actions/workflows/ci.yml/badge.svg)](https://github.com/genetzhs/Youtube-downloader/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-2024_edition-orange)
![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS-blue)
![License](https://img.shields.io/badge/license-MIT-green)

A fast, native desktop app for downloading audio and video with
[yt-dlp](https://github.com/yt-dlp/yt-dlp). Written in Rust with
[egui](https://github.com/emilk/egui). It replaces the earlier Python app,
[genetzhs/youtubeDL](https://github.com/genetzhs/youtubeDL).

| Dark | Light |
|---|---|
| ![Main window, dark theme, audio mode](docs/screenshots/main-dark.png) | ![Main window, light theme, video mode](docs/screenshots/main-light.png) |

> **Note:** The app is still being built. These are screenshots of the current build with
> sample queue items (`--demo`); downloading from the window arrives in M4. See [Status](#status).

## Features

- **Audio or video, one click.** Audio as MP3, M4A, Opus, FLAC, WAV or Vorbis (64–320 kbps or
  best). Video up to 2160p as MP4, MKV or WebM.
- **Paste many links at once.** Every link gets its own card with title, thumbnail, progress,
  speed and time left. Two downloads run at the same time and the rest wait their turn.
- **Playlists**, or just the single video with *skip playlist*.
- **Cover art and tags.** Embeds the thumbnail as album art and writes title, artist and other
  metadata. Subtitles and an info JSON are optional.
- **Handles "Sign in to confirm you're not a bot".** One click retries with the cookies from your
  browser (Chrome, Chromium, Brave, Firefox, Edge, Opera, Vivaldi or Safari).
- **Nothing else to install.** yt-dlp, ffmpeg and ffprobe ship inside the package, and yt-dlp
  updates itself (checksum-verified) because YouTube changes often.
- **Shows the exact command** it runs, with a copy button, and a log for every download.
- **History** of finished downloads, and a desktop notification when the queue is done.
- **Dark and light themes**, or follow the system setting. Switch with the sun/moon button
  in the top right.

## Install

Packages will be published on the [Releases](https://github.com/genetzhs/Youtube-downloader/releases)
page with the first release. Until then, see [Build from source](#build-from-source).

### Linux: .deb (Debian, Ubuntu, Linux Mint, Pop!_OS)

```bash
sudo apt install ./youtube-downloader_<version>_amd64.deb
```

Then open **YouTube Downloader** from your applications menu, or run `youtube-downloader`.
To remove it: `sudo apt remove youtube-downloader`.

### Linux: AppImage (any distribution)

```bash
chmod +x YouTube_Downloader-<version>-x86_64.AppImage
./YouTube_Downloader-<version>-x86_64.AppImage
```

If it does not start, your system may be missing FUSE 2: install `libfuse2`
(on Ubuntu 24.04 and newer: `sudo apt install libfuse2t64`).

### macOS (Apple Silicon and Intel)

1. Open the `.dmg` and drag **YouTube Downloader** into **Applications**.
2. The first time, macOS may say the app "cannot be opened" because it is not notarized by
   Apple. Open **System Settings → Privacy & Security**, scroll down and click **Open Anyway**.
   Or run:

   ```bash
   xattr -dr com.apple.quarantine "/Applications/YouTube Downloader.app"
   ```

## Using it

1. Copy a link from YouTube (or any [site yt-dlp supports](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md)).
2. Choose **Audio** or **Video** at the top, then the format and quality.
3. Paste the link (<kbd>Ctrl</kbd>/<kbd>⌘</kbd>+<kbd>V</kbd>) and press **Download**. Paste
   several links at once to queue them all.
4. When a card shows a check mark, click **Show in folder**.

Files go to `~/Music/YouTube` for audio and `~/Videos/YouTube` (Linux) or `~/Movies/YouTube`
(macOS) for video. Change this in **Settings**.

## Troubleshooting

**"YouTube wants you signed in" / "confirm you're not a bot"**
: Sign in to YouTube in one of your browsers, then click **Use … cookies** on the card (or set
  *Cookies from browser* in Settings) and retry.

**Safari cookies on macOS**
: macOS protects Safari's cookies. Give the app *Full Disk Access* in
  **System Settings → Privacy & Security**, or use another browser's cookies.

**Chrome/Brave/Edge cookies on Linux**
: These browsers encrypt cookies with your desktop keyring (GNOME Keyring or KWallet). Make sure
  you are logged in to a desktop session with the keyring unlocked.

**A site stopped working**
: Sites change and yt-dlp follows quickly. The app updates yt-dlp on its own; you can also click
  **Check for update** in Settings.

**Something else**
: Click **Show log** on the download and include the log when you
  [open an issue](https://github.com/genetzhs/Youtube-downloader/issues).

## Where settings are stored

| | Linux | macOS |
|---|---|---|
| Settings | `~/.config/youtube-downloader/config.toml` | `~/Library/Application Support/youtube-downloader/config.toml` |
| History | `~/.local/share/youtube-downloader/history.json` | `~/Library/Application Support/youtube-downloader/history.json` |
| Updated yt-dlp | `~/.local/share/youtube-downloader/bin/yt-dlp` | `~/Library/Application Support/youtube-downloader/bin/yt-dlp` |

Deleting these folders resets the app.

## Status

| | Milestone | State |
|---|---|---|
| M1 | `ytdl-core`: command builder, progress parser, queue, settings, history, tool lookup | ✅ done |
| M2 | `ytdl-runner`: run yt-dlp, live progress, cancel, two at a time, yt-dlp self-update | ✅ done |
| M3 | Window, dark and light themes, custom widgets | ✅ done |
| M4 | Connect the window to the queue: real downloads, thumbnails, log | next |
| M5 | Settings, history, notifications | |
| M6 | Packages: `.deb`, AppImage, macOS `.dmg`, release workflow | |

## Build from source

You need [Rust](https://rustup.rs) 1.88 or newer. On Linux the window also needs
`libxkbcommon-x11-0`, which desktop systems normally have already.

```bash
git clone https://github.com/genetzhs/Youtube-downloader.git
cd Youtube-downloader
cargo test
```

Run the app (add `--demo` to fill the queue with sample items, `--theme dark|light` to
force a theme):

```bash
cargo run -p youtube-downloader -- --demo
```

Until the window starts downloads (M4), you can try the download engine from the terminal.
It uses `yt-dlp` and `ffmpeg` from your `PATH`:

```bash
cargo run -p ytdl-runner --example cli -- "https://www.youtube.com/watch?v=<id>" --out ~/Downloads
# add --update to download the latest yt-dlp into a temporary folder first
```

### Project layout

```
crates/
  app/           The egui window: theme, widgets, layout. Builds the
                 `youtube-downloader` binary.
  ytdl-core/     Logic with no UI: yt-dlp arguments, output parsing, queue,
                 settings, history, finding tools. Fully unit tested.
  ytdl-runner/   Runs yt-dlp processes for the queue and updates yt-dlp.
assets/          App icons
docs/            Screenshots
```

### Checks

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test -p ytdl-runner -- --ignored   # also downloads the real yt-dlp from GitHub
```

## License

MIT, see [LICENSE](LICENSE). The Geist fonts in `crates/app/fonts` are under the
SIL Open Font License (`crates/app/fonts/OFL.txt`).

The packages also contain third-party programs under their own licenses:
[yt-dlp](https://github.com/yt-dlp/yt-dlp) (Unlicense) and [FFmpeg](https://ffmpeg.org)
(GPL). Each release lists the exact builds and links to their source code.

This app is not affiliated with YouTube or Google. Download only content you have the
right to download.
