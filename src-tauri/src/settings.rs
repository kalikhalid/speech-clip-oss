use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::dictionary::{sanitize_entries, DictionaryEntry};
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

fn default_sound_effects_enabled() -> bool {
    true
}

fn default_dictionary() -> Vec<DictionaryEntry> {
    Vec::new()
}

fn default_paste_delay_before_ms() -> u64 {
    50
}

fn default_paste_delay_after_ms() -> u64 {
    30
}

fn default_restore_clipboard_after_paste() -> bool {
    true
}

fn default_recording_mode() -> String {
    "push_to_talk".to_string()
}

fn default_strip_filler_words() -> bool {
    false
}

fn default_warmup_on_start() -> bool {
    true
}

fn default_ui_locale() -> String {
    "en".to_string()
}

pub const UI_LOCALE_EN: &str = "en";
pub const UI_LOCALE_RU: &str = "ru";

pub const RECORDING_MODE_PUSH_TO_TALK: &str = "push_to_talk";
pub const RECORDING_MODE_TOGGLE: &str = "toggle";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Transcription hint for UI only (Parakeet v3 auto-detects language).
    pub language: String,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_parakeet_model")]
    pub parakeet_model: String,
    #[serde(default = "default_sound_effects_enabled")]
    pub sound_effects_enabled: bool,
    /// Spoken phrase → replacement, applied after local transcription.
    #[serde(default = "default_dictionary")]
    pub dictionary: Vec<DictionaryEntry>,
    #[serde(default = "default_paste_delay_before_ms")]
    pub paste_delay_before_ms: u64,
    #[serde(default = "default_paste_delay_after_ms")]
    pub paste_delay_after_ms: u64,
    #[serde(default = "default_restore_clipboard_after_paste")]
    pub restore_clipboard_after_paste: bool,
    /// `push_to_talk` (hold) or `toggle` (press to start/stop).
    #[serde(default = "default_recording_mode")]
    pub recording_mode: String,
    #[serde(default = "default_strip_filler_words")]
    pub strip_filler_words: bool,
    /// Preload Parakeet on app/dashboard start when the model is installed.
    #[serde(default = "default_warmup_on_start")]
    pub warmup_on_start: bool,
    /// UI language: `en` or `ru`.
    #[serde(default = "default_ui_locale")]
    pub ui_locale: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: default_language(),
            hotkey: default_hotkey(),
            parakeet_model: default_parakeet_model(),
            sound_effects_enabled: default_sound_effects_enabled(),
            dictionary: default_dictionary(),
            paste_delay_before_ms: default_paste_delay_before_ms(),
            paste_delay_after_ms: default_paste_delay_after_ms(),
            restore_clipboard_after_paste: default_restore_clipboard_after_paste(),
            recording_mode: default_recording_mode(),
            strip_filler_words: default_strip_filler_words(),
            warmup_on_start: default_warmup_on_start(),
            ui_locale: default_ui_locale(),
        }
    }
}

impl AppSettings {
    pub fn is_toggle_recording(&self) -> bool {
        self.recording_mode == RECORDING_MODE_TOGGLE
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

    if settings.recording_mode != RECORDING_MODE_PUSH_TO_TALK
        && settings.recording_mode != RECORDING_MODE_TOGGLE
    {
        settings.recording_mode = RECORDING_MODE_PUSH_TO_TALK.to_string();
    }

    if settings.ui_locale != UI_LOCALE_EN && settings.ui_locale != UI_LOCALE_RU {
        settings.ui_locale = UI_LOCALE_EN.to_string();
    }

    settings.dictionary = sanitize_entries(std::mem::take(&mut settings.dictionary));
    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let mut settings = settings.clone();
    settings.dictionary = sanitize_entries(std::mem::take(&mut settings.dictionary));
    let path = get_settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create settings dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write settings: {e}"))
}
