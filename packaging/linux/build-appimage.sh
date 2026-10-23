#!/usr/bin/env bash
# Builds the AppImage from a release binary and the fetched tools.
#
#   packaging/linux/build-appimage.sh <binary> <tools-dir> <output.AppImage>
#
# Needs appimagetool on PATH or in $APPIMAGETOOL (it is downloaded if missing).
set -euo pipefail

bin=${1:?binary}
tools=${2:?tools dir}
out=${3:?output file}
root=$(cd "$(dirname "$0")/../.." && pwd)

appdir=$(mktemp -d)/AppDir
trap 'rm -rf "$(dirname "$appdir")"' EXIT

install -Dm755 "$bin" "$appdir/usr/bin/youtube-downloader"
for t in yt-dlp ffmpeg ffprobe; do
  install -Dm755 "$tools/$t" "$appdir/usr/lib/youtube-downloader/bin/$t"
done
install -Dm644 "$root/packaging/linux/youtube-downloader.desktop" \
  "$appdir/usr/share/applications/youtube-downloader.desktop"
for n in 16 24 32 48 64 128 256 512; do
  install -Dm644 "$root/assets/icon_$n.png" \
    "$appdir/usr/share/icons/hicolor/${n}x${n}/apps/youtube-downloader.png"
done
install -Dm644 "$root/packaging/THIRD-PARTY-NOTICES.md" "$appdir/usr/share/doc/youtube-downloader/THIRD-PARTY-NOTICES.md"
install -Dm644 "$root/crates/app/fonts/OFL.txt" "$appdir/usr/share/doc/youtube-downloader/fonts-OFL.txt"
install -Dm644 "$root/LICENSE" "$appdir/usr/share/doc/youtube-downloader/LICENSE"

# AppImage expects the desktop file and icon at the top level, and an AppRun.
cp "$root/packaging/linux/youtube-downloader.desktop" "$appdir/"
cp "$root/assets/icon_256.png" "$appdir/youtube-downloader.png"
cat > "$appdir/AppRun" <<'RUN'
#!/bin/sh
here="$(dirname "$(readlink -f "$0")")"
exec "$here/usr/bin/youtube-downloader" "$@"
RUN
chmod 755 "$appdir/AppRun"

tool=${APPIMAGETOOL:-$(command -v appimagetool || true)}
if [ -z "$tool" ]; then
  tool=$(dirname "$appdir")/appimagetool
  curl -sSfL -o "$tool" \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod 755 "$tool"
fi
# Extract-and-run avoids needing FUSE on the build machine.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "$tool" --no-appstream "$appdir" "$out"
echo "built $out"
