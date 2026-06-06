//! Dictation normalizer GGUF model install (dev: copy from ~/Downloads or env).

#![allow(dead_code)]

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::parakeet_install::models_root;

/// Primary normalizer (Qwen3.5-0.8B fine-tuned for dictation).
pub const NORMALIZER_MODEL_ID: &str = "qwen35-08b-norm";
pub const NORMALIZER_MODEL_DIR: &str = "qwen35-08b-norm";
pub const NORMALIZER_GGUF_FILENAME: &str = "qwen35-08b-norm.gguf";

/// Legacy Gemma 3 270M — still detected if present (older installs).
pub const LEGACY_NORMALIZER_MODEL_DIR: &str = "gemma3-270m-norm";
pub const LEGACY_NORMALIZER_GGUF_FILENAME: &str = "gemma3-270m-norm.gguf";

/// Future: hosted tarball URL (same pattern as Parakeet).
pub const NORMALIZER_MODEL_URL: &str = "";
pub const NORMALIZER_MODEL_SHA256: &str = "";

static INSTALL_MUTEX: Mutex<()> = Mutex::const_new(());
static INSTALL_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
pub struct InstallProgress {
    pub stage: String,
    pub message: String,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct NormalizerSetupStatus {
    pub model_downloaded: bool,
    /// `qwen35-08b-norm` or legacy `gemma3-270m-norm`
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_path: Option<String>,
    pub legacy: bool,
}

pub fn setup_status(app: &AppHandle) -> Result<NormalizerSetupStatus, String> {
    let root = models_root(app)?;
    let qwen = root
        .join(NORMALIZER_MODEL_DIR)
        .join(NORMALIZER_GGUF_FILENAME);
    if qwen.is_file() {
        return Ok(NormalizerSetupStatus {
            model_downloaded: true,
            model_id: NORMALIZER_MODEL_ID.to_string(),
            model_path: Some(qwen.to_string_lossy().into_owned()),
            legacy: false,
        });
    }

    let gemma = root
        .join(LEGACY_NORMALIZER_MODEL_DIR)
        .join(LEGACY_NORMALIZER_GGUF_FILENAME);
    if gemma.is_file() {
        return Ok(NormalizerSetupStatus {
            model_downloaded: true,
            model_id: LEGACY_NORMALIZER_MODEL_DIR.to_string(),
            model_path: Some(gemma.to_string_lossy().into_owned()),
            legacy: true,
        });
    }

    Ok(NormalizerSetupStatus {
        model_downloaded: false,
        model_id: NORMALIZER_MODEL_ID.to_string(),
        model_path: None,
        legacy: false,
    })
}

pub fn install_in_progress() -> bool {
    INSTALL_IN_PROGRESS.load(Ordering::Relaxed)
}

pub fn model_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(models_root(app)?.join(NORMALIZER_MODEL_DIR))
}

pub fn legacy_model_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(models_root(app)?.join(LEGACY_NORMALIZER_MODEL_DIR))
}

/// Resolved GGUF path: prefers Qwen, falls back to legacy Gemma if installed.
pub fn gguf_path(app: &AppHandle) -> Result<PathBuf, String> {
    let root = models_root(app)?;
    let qwen = root
        .join(NORMALIZER_MODEL_DIR)
        .join(NORMALIZER_GGUF_FILENAME);
    if qwen.is_file() {
        return Ok(qwen);
    }
    let gemma = root
        .join(LEGACY_NORMALIZER_MODEL_DIR)
        .join(LEGACY_NORMALIZER_GGUF_FILENAME);
    if gemma.is_file() {
        return Ok(gemma);
    }
    Ok(qwen)
}

pub fn model_downloaded(app: &AppHandle) -> bool {
    gguf_path(app)
        .ok()
        .map(|path| path.is_file())
        .unwrap_or(false)
}

fn emit_progress(app: &AppHandle, stage: &str, message: &str, percent: u8) {
    let payload = InstallProgress {
        stage: stage.to_string(),
        message: message.to_string(),
        percent,
    };
    let _ = app.emit("normalizer-install-progress", payload);
}

fn copy_file(src: &Path, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    std::fs::copy(src, dest).map_err(|e| {
        format!(
            "Failed to copy {} to {}: {e}",
            src.display(),
            dest.display()
        )
    })?;
    Ok(())
}

fn dev_source_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = std::env::var("SPEECHCLIP_NORMALIZER_GGUF") {
        candidates.push(PathBuf::from(path));
    }

    if let Ok(home) = std::env::var("HOME") {
        let downloads = PathBuf::from(&home).join("Downloads");

        // Qwen (primary)
        candidates.push(
            downloads
                .join(NORMALIZER_MODEL_DIR)
                .join(NORMALIZER_GGUF_FILENAME),
        );
        candidates.push(downloads.join(NORMALIZER_GGUF_FILENAME));
        candidates.push(
            downloads
                .join("qwen35_08b_norm_merged")
                .join(NORMALIZER_GGUF_FILENAME),
        );
        candidates.push(downloads.join("qwen35-08b-norm-f16.gguf"));
        candidates.push(downloads.join("qwen35-08b-norm-q4_k_m.gguf"));
        candidates.push(downloads.join("qwen35-08b-norm-q4.gguf"));

        // Legacy Gemma
        candidates.push(
            downloads
                .join(LEGACY_NORMALIZER_MODEL_DIR)
                .join(LEGACY_NORMALIZER_GGUF_FILENAME),
        );
        candidates.push(downloads.join(LEGACY_NORMALIZER_GGUF_FILENAME));
        candidates.push(downloads.join("gemma3-270m-norm-f16.gguf"));
        candidates.push(downloads.join("gemma3-270m-norm-q4_k_m.gguf"));
    }

    candidates.push(PathBuf::from("/tmp/qwen35-08b-norm-f16.gguf"));
    candidates.push(PathBuf::from("/tmp/qwen35-08b-norm-q4_k_m.gguf"));
    candidates.push(PathBuf::from("/tmp/gemma3-270m-norm-f16.gguf"));

    candidates
}

fn find_dev_gguf() -> Option<PathBuf> {
    dev_source_candidates()
        .into_iter()
        .find(|path| path.is_file())
}

async fn install_from_dev_source(app: &AppHandle, dest: &Path) -> Result<(), String> {
    let src = find_dev_gguf().ok_or_else(|| {
        format!(
            "Normalizer GGUF not found. Set SPEECHCLIP_NORMALIZER_GGUF or place \
             {NORMALIZER_GGUF_FILENAME} in ~/Downloads/{NORMALIZER_MODEL_DIR}/"
        )
    })?;

    emit_progress(
        app,
        "install",
        &format!("Installing normalizer from {}…", src.display()),
        50,
    );

    if dest.exists() {
        std::fs::remove_file(dest)
            .map_err(|e| format!("Failed to replace existing normalizer model: {e}"))?;
    }

    copy_file(&src, dest)?;
    Ok(())
}

async fn ensure_model_inner(app: &AppHandle) -> Result<(), String> {
    if model_downloaded(app) {
        emit_progress(app, "ready", "Normalizer model is ready", 100);
        return Ok(());
    }

    let dest = gguf_path(app)?;
    let models_root = models_root(app)?;
    tokio::fs::create_dir_all(models_root.join(NORMALIZER_MODEL_DIR))
        .await
        .map_err(|e| format!("Failed to create normalizer models directory: {e}"))?;

    if !NORMALIZER_MODEL_URL.is_empty() {
        return Err(
            "Normalizer model download URL is not configured yet".to_string(),
        );
    }

    emit_progress(
        app,
        "install",
        "Copying normalizer model from local dev path…",
        20,
    );
    install_from_dev_source(app, &dest).await?;

    if !dest.is_file() {
        return Err("Normalizer install finished but GGUF file was not found".to_string());
    }

    emit_progress(app, "ready", "Normalizer model is ready", 100);
    Ok(())
}

pub async fn ensure_model(app: &AppHandle) -> Result<(), String> {
    let _guard = INSTALL_MUTEX.lock().await;
    INSTALL_IN_PROGRESS.store(true, Ordering::Relaxed);

    let result = ensure_model_inner(app).await;

    INSTALL_IN_PROGRESS.store(false, Ordering::Relaxed);
    if let Err(ref err) = result {
        emit_progress(app, "failed", err, 0);
    }
    result
}
