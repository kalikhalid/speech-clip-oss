use crate::app_context::get_frontmost_app_name;
use crate::input::type_text;
use crate::parakeet::{self, ParakeetSetupStatus};
use crate::shortcuts;
use crate::timing::TimingLogger;
use crate::{settings, storage, window};
use serde::Serialize;
use tauri::AppHandle;

#[derive(Debug, Serialize)]
pub struct ProcessingResult {
    pub raw: String,
    pub final_text: String,
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
        window::set_window_size(&app, 320.0, 400.0).await
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
    let mut timer = TimingLogger::new();
    timer.mark("audio_received");

    let app_name = get_frontmost_app_name();
    let app_settings = settings::load_settings(&app).unwrap_or_default();
    let model_id = app_settings.parakeet_model;

    timer.mark_start("parakeet_asr");
    let raw_text = parakeet::transcribe_audio(&app, audio_data, &model_id).await?;
    timer.mark_end("parakeet_asr");

    let final_text = raw_text.clone();

    timer.mark_start("typing");
    type_text(&app, final_text.clone(), app_name).await?;
    timer.mark_end("typing");

    timer.finish();
    Ok(ProcessingResult {
        raw: raw_text,
        final_text,
    })
}

#[tauri::command]
pub async fn process_audio_with_history(
    app: AppHandle,
    audio_data: Vec<u8>,
    normalize: bool,
    release_timestamp: Option<u64>,
) -> Result<String, String> {
    let start_time = std::time::Instant::now();
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
        let entry = storage::TranscriptionEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            raw_text: proc_result.raw.clone(),
            normalized_text: proc_result.final_text.clone(),
            app_name,
            duration_ms: start_time.elapsed().as_millis() as u64,
            engine: "transcribe-rs".to_string(),
            local_model: Some(settings.parakeet_model),
            comment: None,
        };
        let app_clone = app.clone();
        tokio::spawn(async move {
            let _ = storage::save_transcription(&app_clone, entry);
        });
    }

    result.map(|r| r.final_text)
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
    settings::save_settings(&app, &new_settings)
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
pub async fn check_accessibility_permission(prompt: bool) -> bool {
    crate::utils::macos::check_accessibility_permissions(prompt)
}

#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    crate::utils::macos::open_accessibility_settings();
    Ok(())
}
