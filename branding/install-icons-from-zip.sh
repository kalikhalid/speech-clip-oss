#!/usr/bin/env bash
# Install macOS icons from AppIcon.iconset.zip (ChatGPT / designer export)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ZIP="${1:-$HOME/Downloads/ChatGPT Image 1 июн. 2026 г., 11_07_40-macos-iconset.zip}"

if [[ ! -f "$ZIP" ]]; then
  echo "Zip not found: $ZIP" >&2
  echo "Usage: $0 [path-to-AppIcon.iconset.zip]" >&2
  exit 1
fi

cd "$ROOT/branding"
rm -rf AppIcon.iconset
unzip -o "$ZIP"
iconutil -c icns AppIcon.iconset -o ../src-tauri/icons/icon.icns

ICONS="$ROOT/src-tauri/icons"
cp AppIcon.iconset/icon_32x32.png "$ICONS/32x32.png"
cp AppIcon.iconset/icon_128x128.png "$ICONS/128x128.png"
cp AppIcon.iconset/icon_128x128@2x.png "$ICONS/128x128@2x.png"
cp AppIcon.iconset/icon_512x512.png "$ICONS/icon.png"
cp AppIcon.iconset/icon_512x512@2x.png "$ICONS/../branding/app-icon-1024.png"
cp "$ROOT/branding/app-icon-1024.png" "$ROOT/branding/app-icon-source.png"

python3 "$ROOT/branding/export-assets.py"
rm -f "$ICONS/icon.ico"

echo "Icons installed. Rebuild the app bundle:"
echo "  cd $ROOT && npm run tauri:build:debug && npm run tauri:open:debug"
