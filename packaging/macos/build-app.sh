#!/usr/bin/env bash
# Builds "YouTube Downloader.app" and a .dmg on macOS.
#
#   packaging/macos/build-app.sh <binary> <tools-dir> <version> <output.dmg>
set -euo pipefail

bin=${1:?binary}
tools=${2:?tools dir}
version=${3:?version}
out=${4:?output dmg}
root=$(cd "$(dirname "$0")/../.." && pwd)

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
app="$work/dmg/YouTube Downloader.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/bin"

install -m 755 "$bin" "$app/Contents/MacOS/youtube-downloader"
for t in yt-dlp ffmpeg ffprobe; do
  install -m 755 "$tools/$t" "$app/Contents/Resources/bin/$t"
done
sed "s/@VERSION@/$version/g" "$root/packaging/macos/Info.plist" > "$app/Contents/Info.plist"
cp "$root/packaging/THIRD-PARTY-NOTICES.md" "$root/LICENSE" "$app/Contents/Resources/"
cp "$root/crates/app/fonts/OFL.txt" "$app/Contents/Resources/fonts-OFL.txt"

# App icon from the PNGs (missing @2x sizes are scaled with sips).
iconset="$work/AppIcon.iconset"
mkdir -p "$iconset"
for n in 16 32 128 256 512; do
  sips -z $n $n "$root/assets/icon.png" --out "$iconset/icon_${n}x${n}.png" >/dev/null
  d=$((n * 2))
  sips -z $d $d "$root/assets/icon.png" --out "$iconset/icon_${n}x${n}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"

# Ad-hoc signature: required for Apple Silicon, not a notarized Developer ID.
codesign --force --deep --sign - "$app"

ln -s /Applications "$work/dmg/Applications"
hdiutil create -volname "YouTube Downloader" -srcfolder "$work/dmg" -ov -format UDZO "$out"
echo "built $out"
