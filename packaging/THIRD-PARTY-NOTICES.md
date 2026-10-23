# Third-party software in YouTube Downloader packages

The release packages (.deb, AppImage, macOS .dmg) contain these programs next to the
app. They are separate programs that the app runs; each keeps its own license.

## yt-dlp

- Project: https://github.com/yt-dlp/yt-dlp
- License: The Unlicense (public domain)
- The official standalone build of the latest release at packaging time. The app may
  replace it with newer official builds, checked against their published SHA-256.

## FFmpeg (ffmpeg and ffprobe)

- Project: https://ffmpeg.org
- License: GNU General Public License version 3 (these builds include GPL components)
- Builds: static binaries from https://github.com/eugeneware/ffmpeg-static, release
  `b6.1.1`. The Linux build comes from https://johnvansickle.com/ffmpeg/ (FFmpeg 7.0.2).
- Source code: https://ffmpeg.org/download.html#get-sources and the build scripts of
  the projects above. You may also request the corresponding source by opening an
  issue at https://github.com/genetzhs/Youtube-downloader/issues.

## Geist fonts

- Project: https://github.com/vercel/geist-font
- License: SIL Open Font License 1.1 (included as `fonts-OFL.txt`)

## Rust libraries

The app itself is built from Rust crates (egui, eframe, ureq, serde and others) under
the MIT and/or Apache-2.0 licenses. See `Cargo.lock` in the source repository for the
full list.
