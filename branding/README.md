# Speech Clip OSS branding

macOS app icons for **v0.1**. Windows icon assets will be added when Windows support ships.

| File | Use |
|------|-----|
| `AppIcon.iconset/` | macOS icon set (from designer / ChatGPT zip) |
| `app-icon-1024.png` | Master 1024×1024 PNG |
| `app-icon-source.png` | Original export archive |
| `install-icons-from-zip.sh` | Install icons from `AppIcon.iconset.zip` |
| `export-assets.py` | Copy favicon + `static/logo.png` from Tauri icons |

## Install icons from zip

```bash
npm run icons:install
# or: bash branding/install-icons-from-zip.sh ~/Downloads/your-macos-iconset.zip
```

Then rebuild the app bundle (`npm run tauri:build:debug`) so the Dock icon updates.
