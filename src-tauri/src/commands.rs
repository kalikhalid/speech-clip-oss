use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::app_context::get_frontmost_app_name;
use crate::audio;
use crate::dictionary::{self, apply_dictionary};
use crate::input::type_text;
use crate::normalizer;
use crate::normalizer_install::{self, NormalizerSetupStatus};
use crate::parakeet::{self, ParakeetSetupStatus};
use crate::postprocess;
use crate::shortcuts;
use crate::spoken_normalization;
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

enum RecordedAudio {
    Wav(Vec<u8>),
    PcmF32Le16k(Vec<u8>),
}

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

fn emit_history_updated(app: &AppHandle, entry: &storage::TranscriptionEntry) {
    if let Some(dashboard) = app.get_webview_window("dashboard") {
        let _ = dashboard.emit("history-updated", entry);
    } else {
        let _ = app.emit("history-updated", entry);
    }
}

async fn finalize_text(
    app: &AppHandle,
    raw: &str,
    normalize: bool,
    app_name: Option<&str>,
    app_settings: &settings::AppSettings,
) -> (String, Option<u64>) {
    let mut text = if app_settings.spoken_normalization_enabled {
        spoken_normalization::normalize_text(raw)
    } else {
        raw.to_string()
    };
    let mut normalizer_ms = None;

    if normalize && app_settings.dictation_normalize {
        let norm_start = Instant::now();
        let model_output = normalizer::normalize_text(app, &text).await;
        text = spoken_normalization::guard_model_output(&text, &model_output);
        normalizer_ms = Some(norm_start.elapsed().as_millis() as u64);
    }

    if app_settings.spoken_normalization_enabled {
        text = spoken_normalization::normalize_text(&text);
    }
    text = apply_dictionary(&text, &app_settings.dictionary_rules);
    if app_settings.strip_filler_words {
        text = postprocess::strip_filler_words(&text);
    }
    text = postprocess::strip_messenger_terminal_period(&text, app_name);
    (text.trim().to_string(), normalizer_ms)
}

async fn paste_text(
    app: &AppHandle,
    text: String,
    app_name: Option<String>,
    app_settings: &settings::AppSettings,
) -> Result<(), String> {
    if text.is_empty() {
        return Err("Nothing to paste".to_string());
    }
    type_text(
        app,
        text,
        app_name,
        Duration::from_millis(app_settings.paste_delay_before_ms),
        Duration::from_millis(app_settings.paste_delay_after_ms),
        app_settings.restore_clipboard_after_paste,
    )
    .await
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
pub async fn set_overlay_pill_shown(shown: bool) -> Result<(), String> {
    window::set_overlay_pill_shown(shown);
    Ok(())
}

pub async fn process_audio(
    app: AppHandle,
    audio_data: Vec<u8>,
    normalize: bool,
    app_name: Option<String>,
    app_settings: &settings::AppSettings,
) -> Result<ProcessingResult, String> {
    let (samples, sample_rate) = audio::decode_wav_bytes(&audio_data)?;
    process_decoded_audio(app, samples, sample_rate, normalize, app_name, app_settings).await
}

async fn process_decoded_audio(
    app: AppHandle,
    samples: Vec<f32>,
    sample_rate: u32,
    normalize: bool,
    app_name: Option<String>,
    app_settings: &settings::AppSettings,
) -> Result<ProcessingResult, String> {
    if audio::samples_too_short(samples.len()) {
        emit_transcription_empty(&app, "audio_too_short");
        return Err("Recording too short".to_string());
    }

    let pipeline_start = Instant::now();
    let mut timer = TimingLogger::new();
    timer.mark("audio_received");

    let model_id = app_settings.parakeet_model.clone();

    timer.mark_start("parakeet_asr");
    let asr_start = Instant::now();
    let raw_text = parakeet::transcribe_decoded(&app, samples, sample_rate, &model_id).await?;
    let asr_ms = asr_start.elapsed().as_millis() as u64;
    timer.mark_end("parakeet_asr");

    if raw_text.trim().is_empty() {
        emit_transcription_empty(&app, "empty_transcript");
        return Err("No speech detected".to_string());
    }

    let post_start = Instant::now();
    let (final_text, normalizer_ms) = finalize_text(
        &app,
        &raw_text,
        normalize,
        app_name.as_deref(),
        app_settings,
    )
    .await;
    let postprocess_ms = post_start.elapsed().as_millis() as u64;

    if final_text.is_empty() {
        emit_transcription_empty(&app, "empty_after_postprocess");
        return Err("No text to paste".to_string());
    }

    timer.mark_start("typing");
    let typing_start = Instant::now();
    paste_text(&app, final_text.clone(), app_name, app_settings).await?;
    let typing_ms = typing_start.elapsed().as_millis() as u64;
    timer.mark_end("typing");

    timer.finish();

    let timing = storage::TranscriptionTiming {
        total_ms: pipeline_start.elapsed().as_millis() as u64,
        asr_ms: Some(asr_ms),
        normalizer_ms,
        postprocess_ms: Some(postprocess_ms),
        typing_ms: Some(typing_ms),
    };

    Ok(ProcessingResult {
        raw: raw_text,
        final_text,
        timing: Some(timing),
    })
}

async fn process_recorded_audio_with_history(
    app: AppHandle,
    audio: RecordedAudio,
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
    let app_settings = settings::load_settings(&app)?;

    let result = match audio {
        RecordedAudio::Wav(audio_data) => {
            process_audio(
                app.clone(),
                audio_data,
                normalize,
                app_name.clone(),
                &app_settings,
            )
            .await
        }
        RecordedAudio::PcmF32Le16k(audio_data) => {
            let samples = audio::decode_pcm_f32le_16k(&audio_data)?;
            process_decoded_audio(
                app.clone(),
                samples,
                audio::PARAKEET_SAMPLE_RATE,
                normalize,
                app_name.clone(),
                &app_settings,
            )
            .await
        }
    };

    let _ = release_timestamp;

    match result {
        Ok(proc_result) => {
            let timing = proc_result.timing.clone();
            let total_ms = timing.as_ref().map(|t| t.total_ms).unwrap_or(0);
            let entry = storage::TranscriptionEntry {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now().timestamp_millis(),
                raw_text: proc_result.raw,
                normalized_text: proc_result.final_text.clone(),
                app_name,
                duration_ms: total_ms,
                engine: "transcribe-rs".to_string(),
                local_model: Some(app_settings.parakeet_model),
                timing,
            };
            if storage::save_transcription(&app, entry.clone()).is_ok() {
                emit_history_updated(&app, &entry);
            }
            Ok(proc_result.final_text)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn process_audio_with_history(
    app: AppHandle,
    audio_data: Vec<u8>,
    normalize: bool,
    release_timestamp: Option<u64>,
) -> Result<String, String> {
    process_recorded_audio_with_history(
        app,
        RecordedAudio::Wav(audio_data),
        normalize,
        release_timestamp,
    )
    .await
}

#[tauri::command]
pub async fn process_pcm16k_with_history(
    app: AppHandle,
    audio_data: Vec<u8>,
    normalize: bool,
    release_timestamp: Option<u64>,
) -> Result<String, String> {
    process_recorded_audio_with_history(
        app,
        RecordedAudio::PcmF32Le16k(audio_data),
        normalize,
        release_timestamp,
    )
    .await
}

#[tauri::command]
pub async fn paste_text_command(app: AppHandle, text: String) -> Result<(), String> {
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return Err("Nothing to paste".to_string());
    }
    let app_name = get_frontmost_app_name();
    let app_settings = settings::load_settings(&app)?;
    paste_text(&app, trimmed, app_name, &app_settings).await
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
    let settings = settings::load_settings(&app)?;
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
    Ok(status)
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
    let previous = settings::load_settings(&app).unwrap_or_default();
    if previous.hotkey != new_settings.hotkey {
        let shortcut = shortcuts::parse_shortcut(&new_settings.hotkey)?;
        let old_shortcut = shortcuts::parse_shortcut(&previous.hotkey).ok();
        shortcuts::replace_hotkey(&app, shortcut, old_shortcut)?;
    }
    settings::save_settings(&app, &new_settings)?;
    let settings = settings::load_settings(&app)?;
    emit_settings_updated(&app, &settings);
    Ok(())
}

#[tauri::command]
pub async fn get_normalizer_status(app: AppHandle) -> Result<NormalizerSetupStatus, String> {
    normalizer_install::setup_status(&app)
}

#[tauri::command]
pub async fn ensure_normalizer_model(app: AppHandle) -> Result<NormalizerSetupStatus, String> {
    normalizer_install::ensure_model(&app).await?;
    normalizer_install::setup_status(&app)
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
pub async fn get_history(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<storage::TranscriptionHistory, String> {
    storage::load_history(&app, limit)
}

#[tauri::command]
pub fn get_dictation_stats(app: AppHandle) -> Result<storage::DictationStats, String> {
    storage::dictation_stats(&app)
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
pub async fn check_accessibility_permission(app: AppHandle, prompt: bool) -> bool {
    if prompt {
        let _ = window::step_aside_for_system_dialog(&app).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    crate::utils::macos::check_accessibility_permissions(prompt)
}
