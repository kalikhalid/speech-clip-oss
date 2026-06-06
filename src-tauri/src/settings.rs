use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use crate::dictionary::{
    effective_dictionary_entries, sanitize_entries, seed_dictionary_count,
    CompiledDictionaryRule, DictionaryEntry,
};
use crate::parakeet::DEFAULT_MODEL_ID;

const SETTINGS_DEBOUNCE_MS: u64 = 300;

fn default_hotkey() -> String {
    "control+`".to_string()
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

fn default_hide_idle_pill() -> bool {
    false
}

fn default_seed_dictionary_enabled() -> bool {
    true
}

fn default_show_asr_raw_in_history() -> bool {
    false
}

fn default_dictation_normalize() -> bool {
    true
}

pub const UI_LOCALE_EN: &str = "en";
pub const UI_LOCALE_RU: &str = "ru";

pub const RECORDING_MODE_PUSH_TO_TALK: &str = "push_to_talk";
pub const RECORDING_MODE_TOGGLE: &str = "toggle";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
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
    /// Hide the bottom dictation pill until the hotkey is held or recording is active.
    #[serde(default = "default_hide_idle_pill")]
    pub hide_idle_pill: bool,
    /// Apply bundled IT/ASR correction rules after transcription.
    #[serde(default = "default_seed_dictionary_enabled")]
    pub seed_dictionary_enabled: bool,
    /// Bundled rule count (recomputed on load/save, not authoritative on disk).
    #[serde(default)]
    pub seed_dictionary_count: usize,
    /// Show Parakeet raw output alongside final text in History.
    #[serde(default = "default_show_asr_raw_in_history")]
    pub show_asr_raw_in_history: bool,
    /// LLM post-normalization of Russian dev dictation (tech terms → Latin).
    #[serde(default = "default_dictation_normalize")]
    pub dictation_normalize: bool,
    /// Pre-lowercased dictionary phrases (rebuilt on load/save).
    #[serde(skip, default)]
    pub dictionary_rules: Vec<CompiledDictionaryRule>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
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
            hide_idle_pill: default_hide_idle_pill(),
            seed_dictionary_enabled: default_seed_dictionary_enabled(),
            seed_dictionary_count: seed_dictionary_count(),
            show_asr_raw_in_history: default_show_asr_raw_in_history(),
            dictation_normalize: default_dictation_normalize(),
            dictionary_rules: Vec::new(),
        }
    }
}

fn compile_dictionary_rules(settings: &mut AppSettings) {
    let effective = effective_dictionary_entries(
        &settings.dictionary,
        settings.seed_dictionary_enabled,
    );
    settings.dictionary_rules = CompiledDictionaryRule::compile_all(&effective);
    settings.seed_dictionary_count = seed_dictionary_count();
}

fn get_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    Ok(data_dir.join("settings.json"))
}

fn normalize_loaded(mut settings: AppSettings) -> (AppSettings, bool) {
    let mut migrated = false;
    if settings.parakeet_model == "mlx-community/parakeet-tdt-0.6b-v3" {
        settings.parakeet_model = DEFAULT_MODEL_ID.to_string();
        migrated = true;
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
    compile_dictionary_rules(&mut settings);
    (settings, migrated)
}

fn prepare_for_save(mut settings: AppSettings) -> AppSettings {
    settings.dictionary = sanitize_entries(std::mem::take(&mut settings.dictionary));
    compile_dictionary_rules(&mut settings);
    settings
}

pub fn load_settings_from_disk(app: &AppHandle) -> Result<AppSettings, String> {
    let path = get_settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read settings: {e}"))?;
    let settings: AppSettings =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {e}"))?;
    Ok(normalize_loaded(settings).0)
}

fn write_settings_to_disk(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create settings dir: {e}"))?;
    }
    let json = serde_json::to_vec(settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write settings: {e}"))
}

/// In-memory settings cache with serialized writes and debounced disk flush.
pub struct SettingsStore {
    cache: RwLock<AppSettings>,
    /// Serializes concurrent save_settings / update_hotkey paths.
    writer: Mutex<()>,
    flush_generation: AtomicU64,
}

impl SettingsStore {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let raw = load_settings_from_disk(app)?;
        let (settings, migrated) = normalize_loaded(raw);
        if migrated {
            let _ = write_settings_to_disk(app, &settings);
        }
        Ok(Self {
            cache: RwLock::new(settings),
            writer: Mutex::new(()),
            flush_generation: AtomicU64::new(0),
        })
    }

    pub fn get(&self) -> AppSettings {
        self.cache.read().expect("settings lock poisoned").clone()
    }

    pub fn save(&self, app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
        let _writer = self.writer.lock().map_err(|e| e.to_string())?;
        let prepared = prepare_for_save(settings.clone());
        *self.cache.write().expect("settings lock poisoned") = prepared.clone();
        self.schedule_debounced_flush(app.clone(), prepared);
        Ok(())
    }

    /// Immediate disk write (startup migration, tests).
    pub fn save_immediate(&self, app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
        let _writer = self.writer.lock().map_err(|e| e.to_string())?;
        let prepared = prepare_for_save(settings.clone());
        *self.cache.write().expect("settings lock poisoned") = prepared.clone();
        write_settings_to_disk(app, &prepared)
    }

    fn schedule_debounced_flush(&self, app: AppHandle, _settings: AppSettings) {
        let generation = self.flush_generation.fetch_add(1, Ordering::SeqCst) + 1;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(SETTINGS_DEBOUNCE_MS)).await;
            if let Some(store) = app.try_state::<std::sync::Arc<SettingsStore>>() {
                if store.flush_generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                let snapshot = store.get();
                let _ = tauri::async_runtime::spawn_blocking(move || write_settings_to_disk(&app, &snapshot)).await;
            }
        });
    }
}

fn settings_store(app: &AppHandle) -> Result<std::sync::Arc<SettingsStore>, String> {
    app.try_state::<std::sync::Arc<SettingsStore>>()
        .map(|s| s.inner().clone())
        .ok_or_else(|| "Settings store not initialized".to_string())
}

pub fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    if let Ok(store) = settings_store(app) {
        return Ok(store.get());
    }
    load_settings_from_disk(app).map(|s| normalize_loaded(s).0)
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    if let Ok(store) = settings_store(app) {
        return store.save(app, settings);
    }
    let prepared = prepare_for_save(settings.clone());
    write_settings_to_disk(app, &prepared)
}

/// Persist settings immediately (e.g. hotkey fallback at startup before debounce).
pub fn save_settings_immediate(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    if let Ok(store) = settings_store(app) {
        return store.save_immediate(app, settings);
    }
    let prepared = prepare_for_save(settings.clone());
    write_settings_to_disk(app, &prepared)
}
