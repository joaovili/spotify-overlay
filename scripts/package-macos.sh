#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE="${1:-native}"
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"

if [ "$MODE" = "universal" ]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release --target aarch64-apple-darwin
  cargo build --release --target x86_64-apple-darwin
  BIN="target/spotify-overlay-universal"
  lipo -create -output "$BIN" \
    target/aarch64-apple-darwin/release/spotify-overlay \
    target/x86_64-apple-darwin/release/spotify-overlay
else
  cargo build --release
  BIN="target/release/spotify-overlay"
fi

APP="dist/Spotify Overlay.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/spotify-overlay"
chmod +x "$APP/Contents/MacOS/spotify-overlay"
cp assets/icons/icon.icns "$APP/Contents/Resources/AppIcon.icns"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@BUILD@/$VERSION/g" assets/Info.plist \
  > "$APP/Contents/Info.plist"

echo "app: $APP"
