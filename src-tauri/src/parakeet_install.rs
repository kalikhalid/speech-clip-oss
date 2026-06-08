//! In-app Parakeet v3 ONNX model download and extract.

use flate2::read::GzDecoder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

pub const DEFAULT_MODEL_ID: &str = "parakeet-tdt-0.6b-v3";
pub const DEFAULT_MODEL_DIR: &str = "parakeet-tdt-0.6b-v3-int8";
pub const DEFAULT_MODEL_URL: &str = "https://blob.handy.computer/parakeet-v3-int8.tar.gz";
pub const DEFAULT_MODEL_SHA256: &str =
    "43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77";

static INSTALL_MUTEX: Mutex<()> = Mutex::const_new(());
static INSTALL_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
pub struct InstallProgress {
    pub stage: String,
    pub message: String,
    pub percent: u8,
}

pub fn install_in_progress() -> bool {
    INSTALL_IN_PROGRESS.load(Ordering::Relaxed)
}

pub fn models_root(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {e}"))
        .map(|p| p.join("models"))
}

pub fn model_dir_name(model_id: &str) -> Result<&'static str, String> {
    match model_id {
        DEFAULT_MODEL_ID | "mlx-community/parakeet-tdt-0.6b-v3" => Ok(DEFAULT_MODEL_DIR),
        _ => Err(format!("Unsupported Parakeet model: {model_id}")),
    }
}

pub fn model_path(app: &AppHandle, model_id: &str) -> Result<PathBuf, String> {
    let dir = model_dir_name(model_id)?;
    Ok(models_root(app)?.join(dir))
}

pub fn model_downloaded(app: &AppHandle, model_id: &str) -> bool {
    model_path(app, model_id)
        .ok()
        .map(|path| model_dir_valid(&path))
        .unwrap_or(false)
}

fn model_dir_valid(path: &Path) -> bool {
    path.is_dir()
        && path.join("encoder-model.int8.onnx").exists()
        && path.join("decoder_joint-model.int8.onnx").exists()
        && path.join("vocab.txt").exists()
}

fn emit_progress(app: &AppHandle, stage: &str, message: &str, percent: u8) {
    let payload = InstallProgress {
        stage: stage.to_string(),
        message: message.to_string(),
        percent,
    };
    let _ = app.emit("parakeet-install-progress", payload);
}

async fn download_file(
    app: &AppHandle,
    url: &str,
    dest: &Path,
    stage: &str,
    message: &str,
    percent: u8,
) -> Result<(), String> {
    emit_progress(app, stage, message, percent);

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download model: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Model download failed with HTTP {}",
            response.status()
        ));
    }

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read model download: {e}"))?;

    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| format!("Failed to write {}: {e}", dest.display()))?;

    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    let actual = format!("{digest:x}");
    if actual != expected {
        return Err(format!(
            "Model checksum mismatch (expected {expected}, got {actual})"
        ));
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, extract_root: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| format!("Failed to open archive {}: {e}", archive_path.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    archive
        .unpack(extract_root)
        .map_err(|e| format!("Failed to extract model archive: {e}"))?;

    Ok(())
}

fn finalize_extracted_model(extract_root: &Path, final_dir: &Path) -> Result<(), String> {
    if final_dir.exists() {
        std::fs::remove_dir_all(final_dir)
            .map_err(|e| format!("Failed to replace existing model: {e}"))?;
    }

    let mut entries = std::fs::read_dir(extract_root)
        .map_err(|e| format!("Failed to inspect extracted model: {e}"))?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect::<Vec<_>>();

    if entries.len() == 1 {
        std::fs::rename(entries.remove(0).path(), final_dir)
            .map_err(|e| format!("Failed to install model directory: {e}"))?;
    } else if extract_root.join(DEFAULT_MODEL_DIR).is_dir() {
        std::fs::rename(extract_root.join(DEFAULT_MODEL_DIR), final_dir)
            .map_err(|e| format!("Failed to install model directory: {e}"))?;
    } else {
        std::fs::rename(extract_root, final_dir)
            .map_err(|e| format!("Failed to install model directory: {e}"))?;
        return Ok(());
    }

    if extract_root.exists() {
        let _ = std::fs::remove_dir_all(extract_root);
    }
    Ok(())
}

async fn ensure_model_inner(app: &AppHandle, model_id: &str) -> Result<(), String> {
    let final_dir = model_path(app, model_id)?;
    if model_dir_valid(&final_dir) {
        emit_progress(app, "ready", "Parakeet model is ready", 100);
        return Ok(());
    }

    let models_root = models_root(app)?;
    tokio::fs::create_dir_all(&models_root)
        .await
        .map_err(|e| format!("Failed to create models directory: {e}"))?;

    let archive_path = models_root.join(format!("{model_id}.tar.gz"));
    let extract_root = models_root.join(format!("{model_id}.extracting"));

    emit_progress(
        app,
        "download",
        "Downloading Parakeet v3 model (~456 MB)…",
        10,
    );
    download_file(
        app,
        DEFAULT_MODEL_URL,
        &archive_path,
        "download",
        "Downloading Parakeet v3 model (~456 MB)…",
        40,
    )
    .await?;

    emit_progress(app, "verify", "Verifying model download…", 55);
    verify_sha256(&archive_path, DEFAULT_MODEL_SHA256)?;

    if extract_root.exists() {
        let _ = tokio::fs::remove_dir_all(&extract_root).await;
    }
    tokio::fs::create_dir_all(&extract_root)
        .await
        .map_err(|e| format!("Failed to create extraction directory: {e}"))?;

    emit_progress(app, "extract", "Extracting Parakeet model…", 75);
    extract_tar_gz(&archive_path, &extract_root)?;

    emit_progress(app, "install", "Installing Parakeet model…", 90);
    finalize_extracted_model(&extract_root, &final_dir)?;

    if !model_dir_valid(&final_dir) {
        return Err(
            "Parakeet model install finished but required ONNX files were not found".to_string(),
        );
    }

    let _ = tokio::fs::remove_file(&archive_path).await;
    emit_progress(app, "ready", "Parakeet model is ready", 100);
    Ok(())
}

pub async fn ensure_runtime(app: &AppHandle) -> Result<(), String> {
    ensure_model(app, DEFAULT_MODEL_ID).await
}

pub async fn ensure_model(app: &AppHandle, model_id: &str) -> Result<(), String> {
    let _guard = INSTALL_MUTEX.lock().await;
    INSTALL_IN_PROGRESS.store(true, Ordering::Relaxed);

    let result = ensure_model_inner(app, model_id).await;

    INSTALL_IN_PROGRESS.store(false, Ordering::Relaxed);
    if let Err(ref err) = result {
        emit_progress(app, "failed", err, 0);
    }
    result
}
