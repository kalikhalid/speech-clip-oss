#!/usr/bin/env python3
"""Copy raster assets from Tauri icons into static/ and tray."""

from __future__ import annotations

import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "src-tauri" / "icons"
STATIC = ROOT / "static"


def main() -> None:
    favicon_src = ICONS / "32x32.png"
    if not favicon_src.is_file():
        raise SystemExit(f"Missing {favicon_src}; run `bunx @tauri-apps/cli icon` first.")

    shutil.copy2(favicon_src, STATIC / "favicon.png")

    icon_png = ICONS / "icon.png"
    if icon_png.is_file():
        shutil.copy2(icon_png, STATIC / "logo.png")

    tray_src = ICONS / "32x32.png"
    shutil.copy2(tray_src, ICONS / "tray-icon.png")

    print("Updated static/favicon.png, static/logo.png (if present), icons/tray-icon.png")


if __name__ == "__main__":
    main()
