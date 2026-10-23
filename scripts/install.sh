#!/bin/sh
# Takt installer for macOS. No Apple Developer account, no Gatekeeper dialog:
# curl does not set the quarantine flag that Safari/Finder add to downloads,
# so an app fetched this way launches like any other.
#
#   curl -fsSL https://raw.githubusercontent.com/xanymanywka/takt/main/scripts/install.sh | sh
#
# Options via environment:
#   TAKT_VERSION=v0.2.1   install a specific tag instead of the latest release
#   TAKT_DIR=~/Applications  install somewhere else than /Applications
set -eu

REPO="xanymanywka/takt"
ASSET="Takt_universal.app.tar.gz"
VERSION="${TAKT_VERSION:-latest}"
DEST="${TAKT_DIR:-/Applications}"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "Takt installer: this script is for macOS. Windows/Linux builds are on the roadmap." >&2
  exit 1
fi

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/$REPO/releases/latest/download/$ASSET"
else
  URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET"
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/takt-install.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

echo "Takt: downloading $VERSION ..."
curl -fsSL --retry 3 -o "$TMP/$ASSET" "$URL"
tar -xzf "$TMP/$ASSET" -C "$TMP"

if [ ! -d "$TMP/Takt.app" ]; then
  echo "Takt installer: archive does not contain Takt.app" >&2
  exit 1
fi

if [ ! -d "$DEST" ] || [ ! -w "$DEST" ]; then
  DEST="$HOME/Applications"
  mkdir -p "$DEST"
fi

# Close a running copy so the bundle can be replaced.
if pgrep -xq "orchestra" 2>/dev/null || pgrep -xq "Takt" 2>/dev/null; then
  osascript -e 'tell application "Takt" to quit' >/dev/null 2>&1 || true
  sleep 1
fi

rm -rf "$DEST/Takt.app"
mv "$TMP/Takt.app" "$DEST/Takt.app"
# Belt and braces: strip the quarantine flag in case the file was proxied through a browser.
xattr -dr com.apple.quarantine "$DEST/Takt.app" 2>/dev/null || true

VER="$(defaults read "$DEST/Takt.app/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo "?")"
echo "Takt $VER installed to $DEST/Takt.app"
open -a "$DEST/Takt.app"
