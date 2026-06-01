# Contributing to Speech Clip OSS

Thank you for your interest in this project.

## Scope

Speech Clip OSS is an **open-source**, privacy-first voice dictation app (local Parakeet v3 via transcribe-rs). The long-term goal is **macOS and Windows** desktop support under one codebase (Tauri + Rust).

**Current release focus:** macOS only (v0.1). Pull requests that only target Windows are welcome as **draft groundwork** (behind `cfg`, docs, CI stubs) but may wait until a Windows milestone is announced.

This directory may live inside a larger monorepo (`speech-clip`, `speech-clip-api`, `domain`) that is **not** part of the public OSS repo — contribute against the `speech-clip-oss` GitHub repository.

## How to contribute

1. Fork the repository and create a branch from `main`.
2. Make focused changes with clear commit messages.
3. **macOS:** run `npm install && npm run tauri:dev` (Apple Silicon recommended).
4. Open a pull request describing what changed and why, including platform (macOS / Windows / both).

## Platform notes

- **macOS:** primary platform for reviews and merges in v0.1.
- **Windows:** discuss in an issue before large UI or input-simulation changes; we want parity with macOS hotkey + paste behavior later.

## Code of conduct

Be respectful in issues and reviews. We welcome bug reports, docs improvements, and small features that fit the on-device, no-account mission.

## Questions

Open a GitHub issue with the `question` label if you are unsure whether a change belongs in the OSS app vs. other packages in the monorepo.
