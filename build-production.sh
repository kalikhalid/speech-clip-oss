#!/bin/bash
# Production build for Speech Clip OSS (macOS v0.1 — Windows planned)
set -euo pipefail

cd "$(dirname "$0")"

echo "🔨 Building Speech Clip OSS…"
npm run build
npm run tauri -- build

echo "✅ Done."
echo "📦 App: src-tauri/target/release/bundle/macos/Speech Clip OSS.app"
echo "💡 For Dock icon during development, use: npm run tauri:open:debug"
