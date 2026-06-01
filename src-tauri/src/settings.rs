use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::parakeet::DEFAULT_MODEL_ID;

fn default_hotkey() -> String {
    "control+`".to_string()
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_parakeet_model() -> String {
    DEFAULT_MODEL_ID.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Transcription hint for UI only (Parakeet v3 auto-detects language).
    pub language: String,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_parakeet_model")]
    pub parakeet_model: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: default_language(),
            hotkey: default_hotkey(),
            parakeet_model: default_parakeet_model(),
        }
    }
}

fn get_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    Ok(data_dir.join("settings.json"))
}

pub fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = get_settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read settings: {e}"))?;
    let mut settings: AppSettings =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {e}"))?;

    // Migrate legacy MLX Hugging Face model id to local ONNX id.
    if settings.parakeet_model == "mlx-community/parakeet-tdt-0.6b-v3" {
        settings.parakeet_model = DEFAULT_MODEL_ID.to_string();
        let _ = save_settings(app, &settings);
    }

    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create settings dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write settings: {e}"))
}
