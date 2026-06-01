# Speech Clip OSS

Open-source voice dictation built with **Tauri**, **Rust**, and **SvelteKit**. Transcription runs **on device** with [Parakeet TDT 0.6B v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) via [transcribe-rs](https://github.com/cjpais/transcribe-rs) ONNX.

The app is designed as a **cross-platform** desktop product (macOS and Windows). **v0.1 is macOS-only** — Apple Silicon recommended. Windows support is planned for a later release.

- **No account** — no API keys, no cloud auth
- **Privacy** — audio stays on your device
- **Zero manual setup** — the app downloads the Parakeet v3 ONNX bundle (~456 MB) on first run

## Platform support

| Platform | v0.1 | Notes |
|----------|------|--------|
| **macOS** (Apple Silicon) | Supported | Full feature set: global hotkey, overlay, paste into any app |
| **macOS** (Intel) | Best-effort | May build; not the primary test target |
| **Windows** | Planned | Same goals: local Parakeet, global hotkey, no cloud — not in this release |
| **Linux** | Out of scope for now | May be considered after Windows |

## Why transcribe-rs?

| Option | Reliability | v3 support | Notes |
|--------|-------------|------------|--------|
| **transcribe-rs ONNX** | High | Yes | Used here — native Rust, no Python sidecar |
| parakeet-mlx (Python) | Fragile | Yes | Apple Silicon / MLX only; not used in this repo |
| parakeet.cpp | Incomplete | v2 encoder only | Not full ASR yet |

## Prerequisites (macOS v0.1)

1. **macOS** on Apple Silicon (M1 or newer recommended)  
2. **Node.js** (npm) + **Rust** for building the Tauri app

You do **not** need Python, Homebrew, or ffmpeg — the app bundles what it needs on macOS.

## Quick start (macOS)

```bash
cd speech-clip-oss
npm install
npm run check          # regenerates .svelte-kit/ after a clean checkout
npm run tauri:dev      # same as: npm run tauri -- dev
```

Use the local Tauri CLI from `node_modules` (do not run bare `tauri` on your PATH). If you use Bun: `bun install` then `bun run tauri:dev`.

### Dock icon looks square during `tauri dev`?

`tauri dev` runs the raw binary — macOS often shows a **square** placeholder icon. The rounded icon from your `AppIcon.iconset.zip` is in the **`.app` bundle** only.

After changing icons, build and launch the bundle:

```bash
npm run tauri:build:debug
npm run tauri:open:debug
```

Or install icons from zip: `npm run icons:install` then rebuild as above.

On first launch, open the dashboard (Dock icon) or dictate once — the app will download and extract `parakeet-tdt-0.6b-v3-int8` into:

`~/Library/Application Support/dev.speechclip.oss/models/parakeet-tdt-0.6b-v3-int8/`

## Usage (macOS)

1. Grant **Accessibility** when prompted (required for paste into other apps).  
2. Hold the global hotkey (default **Ctrl + `**), speak, release.  
3. Text is transcribed locally and pasted into the frontmost app.  
4. Open the dashboard from the Dock icon for history, settings, and install progress.

## Project layout

```
speech-clip-oss/
├── src/                         # SvelteKit UI (overlay + dashboard)
├── src-tauri/
│   ├── src/audio.rs             # WAV decode + 16 kHz resample
│   ├── src/parakeet.rs          # transcribe-rs Parakeet engine
│   └── src/parakeet_install.rs  # ONNX model download/extract
└── branding/                    # AppIcon.iconset + install scripts
```

## Open source

- **License:** [MIT](./LICENSE) for application source in this directory.
- **Model:** [Parakeet TDT 0.6B v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) weights are [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) — attribute NVIDIA when redistributing the model.
- **Contributing:** see [CONTRIBUTING.md](./CONTRIBUTING.md).
- **Maintainers:** OpenAI [Codex for Open Source](https://developers.openai.com/community/codex-for-oss) — apply at [openai.com/form/codex-for-oss](https://openai.com/form/codex-for-oss/).

## License

Application code in `speech-clip-oss/`: **MIT** — see [LICENSE](./LICENSE). Parakeet model weights: **CC BY 4.0** — see link above.
