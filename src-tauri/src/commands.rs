use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::app_context::get_frontmost_app_name;
use crate::audio;
use crate::dictionary::{self, apply_dictionary};
use crate::input::type_text;
use crate::last_transcription;
use crate::parakeet::{self, ParakeetSetupStatus};
use crate::postprocess;
use crate::shortcuts;
use crate::timing::TimingLogger;
use crate::{settings, storage, window};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Serialize)]
pub struct ProcessingResult {
    pub raw: String,
    pub final_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<storage::TranscriptionTiming>,
}

#[derive(Debug, Serialize)]
pub struct DictionaryImportResult {
    pub entries_added: usize,
    pub dictionary: Vec<dictionary::DictionaryEntry>,
}

static AUDIO_PROCESSING: AtomicBool = AtomicBool::new(false);

fn emit_settings_updated(app: &AppHandle, settings: &settings::AppSettings) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit("settings-updated", settings);
    } else {
        let _ = app.emit("settings-updated", settings);
    }
}

fn emit_transcription_empty(app: &AppHandle, reason: &str) {
    let payload = serde_json::json!({ "reason": reason });
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit("transcription-empty", payload);
    } else {
        let _ = app.emit("transcription-empty", payload);
    }
}

fn finalize_text(raw: &str, app_settings: &settings::AppSettings) -> String {
    let mut text = apply_dictionary(raw, &app_settings.dictionary);
    if app_settings.strip_filler_words {
        text = postprocess::strip_filler_words(&text);
    }
    text.trim().to_string()
}

async fn paste_text(
    app: &AppHandle,
    text: String,
    app_name: Option<String>,
) -> Result<(), String> {
    if text.is_empty() {
        return Err("Nothing to paste".to_string());
    }
    type_text(app, text, app_name).await
}

#[tauri::command]
pub async fn show_overlay(app: AppHandle) -> Result<(), String> {
    window::show_overlay(app).await
}

#[tauri::command]
pub async fn resize_overlay(
    app: AppHandle,
    recording: bool,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<(), String> {
    window::resize_overlay(app, recording, width, height).await
}

#[tauri::command]
pub async fn set_guide_mode(app: AppHandle, enable: bool) -> Result<(), String> {
    if enable {
        window::set_window_size(&app, 340.0, 500.0).await
    } else {
        window::show_overlay(app).await
    }
}

#[tauri::command]
pub async fn hide_overlay(app: AppHandle) -> Result<(), String> {
    window::hide_overlay(app).await
}

#[tauri::command]
pub async fn process_audio(
    app: AppHandle,
    audio_data: Vec<u8>,
    _normalize: bool,
) -> Result<ProcessingResult, String> {
    if audio::audio_too_short(&audio_data) {
        emit_transcription_empty(&app, "audio_too_short");
        return Err("Recording too short".to_string());
    }

    let pipeline_start = Instant::now();
    let mut timer = TimingLogger::new();
    timer.mark("audio_received");

    let app_name = get_frontmost_app_name();
    let app_settings = settings::load_settings(&app).unwrap_or_default();
    let model_id = app_settings.parakeet_model.clone();

    timer.mark_start("parakeet_asr");
    let asr_start = Instant::now();
    let raw_text = parakeet::transcribe_audio(&app, audio_data, &model_id).await?;
    let asr_ms = asr_start.elapsed().as_millis() as u64;
    timer.mark_end("parakeet_asr");

    if raw_text.trim().is_empty() {
        emit_transcription_empty(&app, "empty_transcript");
        return Err("No speech detected".to_string());
    }

    let post_start = Instant::now();
    let final_text = finalize_text(&raw_text, &app_settings);
    let postprocess_ms = post_start.elapsed().as_millis() as u64;

    if final_text.is_empty() {
        emit_transcription_empty(&app, "empty_after_postprocess");
        return Err("No text to paste".to_string());
    }

    last_transcription::set_last_final_text(&final_text);

    timer.mark_start("typing");
    let typing_start = Instant::now();
    paste_text(&app, final_text.clone(), app_name).await?;
    let typing_ms = typing_start.elapsed().as_millis() as u64;
    timer.mark_end("typing");

    timer.finish();

    let timing = storage::TranscriptionTiming {
        total_ms: pipeline_start.elapsed().as_millis() as u64,
        asr_ms: Some(asr_ms),
        postprocess_ms: Some(postprocess_ms),
        typing_ms: Some(typing_ms),
    };

    Ok(ProcessingResult {
        raw: raw_text,
        final_text,
        timing: Some(timing),
    })
}

#[tauri::command]
pub async fn process_audio_with_history(
    app: AppHandle,
    audio_data: Vec<u8>,
    normalize: bool,
    release_timestamp: Option<u64>,
) -> Result<String, String> {
    if AUDIO_PROCESSING.swap(true, Ordering::SeqCst) {
        return Err("Already processing audio".to_string());
    }

    struct ProcessingGuard;
    impl Drop for ProcessingGuard {
        fn drop(&mut self) {
            AUDIO_PROCESSING.store(false, Ordering::SeqCst);
        }
    }
    let _processing_guard = ProcessingGuard;

    let app_name = get_frontmost_app_name();

    let result = process_audio(app.clone(), audio_data, normalize).await;

    if let Some(release_ts) = release_timestamp {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        #[cfg(debug_assertions)]
        println!(
            "⏱️  TOTAL LATENCY (key release → done): {}ms",
            now - release_ts
        );
    }

    if let Ok(ref proc_result) = result {
        let settings = settings::load_settings(&app).unwrap_or_default();
        let timing = proc_result.timing.clone();
        let total_ms = timing
            .as_ref()
            .map(|t| t.total_ms)
            .unwrap_or(0);
        let entry = storage::TranscriptionEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            raw_text: proc_result.raw.clone(),
            normalized_text: proc_result.final_text.clone(),
            app_name,
            duration_ms: total_ms,
            engine: "transcribe-rs".to_string(),
            local_model: Some(settings.parakeet_model),
            comment: None,
            timing,
        };
        let app_clone = app.clone();
        tokio::spawn(async move {
            let _ = storage::save_transcription(&app_clone, entry);
        });
    }

    result.map(|r| r.final_text)
}

#[tauri::command]
pub async fn repaste_last(app: AppHandle) -> Result<(), String> {
    let text = last_transcription::get_last_final_text()
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| "No previous transcription to paste".to_string())?;
    let app_name = get_frontmost_app_name();
    paste_text(&app, text, app_name).await
}

#[tauri::command]
pub async fn paste_text_command(app: AppHandle, text: String) -> Result<(), String> {
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return Err("Nothing to paste".to_string());
    }
    let app_name = get_frontmost_app_name();
    paste_text(&app, trimmed, app_name).await
}

#[tauri::command]
pub async fn export_dictionary_csv(app: AppHandle) -> Result<String, String> {
    let settings = settings::load_settings(&app)?;
    Ok(dictionary::export_csv(&settings.dictionary))
}

#[tauri::command]
pub async fn import_dictionary_csv(
    app: AppHandle,
    csv: String,
    merge: bool,
) -> Result<DictionaryImportResult, String> {
    let mut settings = settings::load_settings(&app)?;
    let before = settings.dictionary.len();
    settings.dictionary = dictionary::import_csv(&csv, &settings.dictionary, merge)?;
    let after = settings.dictionary.len();
    settings::save_settings(&app, &settings)?;
    emit_settings_updated(&app, &settings);
    Ok(DictionaryImportResult {
        entries_added: after.saturating_sub(if merge { before } else { 0 }),
        dictionary: settings.dictionary,
    })
}

#[tauri::command]
pub async fn warmup_parakeet(app: AppHandle) -> Result<ParakeetSetupStatus, String> {
    let settings = settings::load_settings(&app).unwrap_or_default();
    if !settings.warmup_on_start {
        return parakeet::check_setup(&app, &settings.parakeet_model).await;
    }
    let status = parakeet::check_setup(&app, &settings.parakeet_model).await?;
    if status.model_downloaded {
        let _ = parakeet::ensure_runtime(&app).await;
    }
    parakeet::check_setup(&app, &settings.parakeet_model).await
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<settings::AppSettings, String> {
    settings::load_settings(&app)
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    new_settings: settings::AppSettings,
) -> Result<(), String> {
    settings::save_settings(&app, &new_settings)?;
    emit_settings_updated(&app, &new_settings);
    Ok(())
}

#[tauri::command]
pub async fn get_parakeet_status(app: AppHandle) -> Result<ParakeetSetupStatus, String> {
    let model = settings::load_settings(&app)
        .unwrap_or_default()
        .parakeet_model;
    parakeet::check_setup(&app, &model).await
}

#[tauri::command]
pub async fn ensure_parakeet_runtime(app: AppHandle) -> Result<ParakeetSetupStatus, String> {
    parakeet::ensure_runtime(&app).await?;
    let model = settings::load_settings(&app)
        .unwrap_or_default()
        .parakeet_model;
    parakeet::check_setup(&app, &model).await
}

#[tauri::command]
pub async fn update_hotkey(app: AppHandle, hotkey: String) -> Result<String, String> {
    let shortcut = shortcuts::parse_shortcut(&hotkey)?;
    let mut settings = settings::load_settings(&app).unwrap_or_default();
    let old_shortcut = shortcuts::parse_shortcut(&settings.hotkey).ok();
    shortcuts::replace_hotkey(&app, shortcut, old_shortcut)?;
    settings.hotkey = hotkey.clone();
    settings::save_settings(&app, &settings)?;
    Ok(hotkey)
}

#[tauri::command]
pub async fn get_history(app: AppHandle) -> Result<storage::TranscriptionHistory, String> {
    storage::load_history(&app)
}

#[tauri::command]
pub async fn delete_history_entry(app: AppHandle, entry_id: String) -> Result<(), String> {
    storage::delete_entry(&app, &entry_id)
}

#[tauri::command]
pub async fn clear_all_history(app: AppHandle) -> Result<(), String> {
    storage::clear_history(&app)
}

#[tauri::command]
pub async fn open_dashboard(app: AppHandle) -> Result<(), String> {
    window::open_dashboard(app).await
}

#[tauri::command]
pub async fn check_accessibility_permission(app: AppHandle, prompt: bool) -> bool {
    if prompt {
        let _ = window::step_aside_for_system_dialog(&app).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    crate::utils::macos::check_accessibility_permissions(prompt)
}

#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    crate::utils::macos::open_accessibility_settings();
    Ok(())
}
