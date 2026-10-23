#!/usr/bin/env bash
# Downloads the tools that ship inside the packages and verifies them.
#
#   packaging/fetch-tools.sh <platform> <out-dir>
#   platform: linux-x64 | macos-arm64 | macos-x64
#
# yt-dlp: the latest official standalone build, checked against the
# SHA2-256SUMS of the same release (the app keeps it updated afterwards).
# ffmpeg/ffprobe: static builds from eugeneware/ffmpeg-static, pinned to a
# release and to the SHA-256 recorded below.
set -euo pipefail

platform=${1:?platform}
out=${2:?out dir}
mkdir -p "$out"

FFMPEG_RELEASE=b6.1.1
# Works with the bash 3.2 that macOS ships (no associative arrays).
ffmpeg_sha256() {
  case "$1" in
    ffmpeg-linux-x64)     echo bfe8a8fc511530457b528c48d77b5737527b504a3797a9bc4866aeca69c2dffa ;;
    ffprobe-linux-x64)    echo 25d9b6ccb05e3d9de9e04e31e2506d8dd7f9f0418981965ac6df12e8d3afd067 ;;
    ffmpeg-darwin-arm64)  echo 8923876afa8db5585022d7860ec7e589af192f441c56793971276d450ed3bbfa ;;
    ffprobe-darwin-arm64) echo d986a8ec7b030899fe66a8a288ed809a3543338705a3ce178cfb85869c5d80be ;;
    ffmpeg-darwin-x64)    echo 929b375c1182d956c51f7ac25e0b2b0411fb01f6f407aa15c9758efeb4242106 ;;
    ffprobe-darwin-x64)   echo d4da574d6e2e197bd259b47d69cf262df9e312af24ad960444f6d806d3d4c186 ;;
  esac
}

case "$platform" in
  linux-x64)   ytdlp_asset=yt-dlp_linux; ff_suffix=linux-x64 ;;
  macos-arm64) ytdlp_asset=yt-dlp_macos; ff_suffix=darwin-arm64 ;;
  macos-x64)   ytdlp_asset=yt-dlp_macos; ff_suffix=darwin-x64 ;;
  *) echo "unknown platform: $platform" >&2; exit 1 ;;
esac

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# yt-dlp: resolve the latest tag from the redirect, then fetch both files from that tag.
location=$(curl -sSI https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS \
  | tr -d '\r' | awk 'tolower($1)=="location:" {print $2}')
tag=$(echo "$location" | sed -n 's#.*/releases/download/\([0-9][0-9.]*\)/.*#\1#p')
[ -n "$tag" ] || { echo "could not find the latest yt-dlp release" >&2; exit 1; }
base="https://github.com/yt-dlp/yt-dlp/releases/download/$tag"
curl -sSfL -o "$tmp/SHA2-256SUMS" "$base/SHA2-256SUMS"
curl -sSfL -o "$tmp/$ytdlp_asset" "$base/$ytdlp_asset"
expected=$(awk -v f="$ytdlp_asset" '$2==f {print $1}' "$tmp/SHA2-256SUMS")
actual=$(sha256 "$tmp/$ytdlp_asset")
[ "$expected" = "$actual" ] || { echo "yt-dlp checksum mismatch" >&2; exit 1; }
install -m 755 "$tmp/$ytdlp_asset" "$out/yt-dlp"
echo "yt-dlp $tag"

for tool in ffmpeg ffprobe; do
  name="$tool-$ff_suffix"
  curl -sSfL -o "$tmp/$name.gz" \
    "https://github.com/eugeneware/ffmpeg-static/releases/download/$FFMPEG_RELEASE/$name.gz"
  actual=$(sha256 "$tmp/$name.gz")
  [ "$actual" = "$(ffmpeg_sha256 "$name")" ] || { echo "$name checksum mismatch" >&2; exit 1; }
  gunzip -c "$tmp/$name.gz" > "$out/$tool"
  chmod 755 "$out/$tool"
  echo "$tool from ffmpeg-static $FFMPEG_RELEASE"
done
