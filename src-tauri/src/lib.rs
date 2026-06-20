mod app_context;
mod audio;
mod commands;
mod dictionary;
mod input;
mod normalizer;
mod normalizer_install;
mod parakeet;
mod parakeet_install;
mod postprocess;
mod settings;
mod shortcuts;
mod spoken_normalization;
mod storage;
mod timing;
mod utils;
mod window;

use tauri::{AppHandle, Manager};

#[cfg(not(target_os = "macos"))]
compile_error!("speech-clip-oss supports macOS (Apple Silicon) only");

fn setup_global_shortcuts(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = settings::load_settings(app).unwrap_or_default();
    let shortcut = match shortcuts::parse_shortcut(&settings.hotkey) {
        Ok(shortcut) => shortcut,
        Err(_) => {
            let fallback = shortcuts::DEFAULT_HOTKEY.to_string();
            let shortcut =
                shortcuts::parse_shortcut(&fallback).expect("Default hotkey must be valid");
            if settings.hotkey != fallback {
                settings.hotkey = fallback.clone();
                let _ = settings::save_settings_immediate(app, &settings);
            }
            shortcut
        }
    };

    shortcuts::register_hotkey(app, shortcut).map_err(Box::<dyn std::error::Error>::from)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(target_os = "macos")]
            crate::utils::macos::disable_app_nap();

            let settings_store = std::sync::Arc::new(
                settings::SettingsStore::load(&handle)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?,
            );
            handle.manage(settings_store);

            let history_store = std::sync::Arc::new(
                storage::HistoryStore::load(&handle)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?,
            );
            handle.manage(history_store);

            let _ = setup_global_shortcuts(&handle);

            if let Some(window) = handle.get_webview_window("main") {
                let _ = window::set_window_on_all_spaces(&window);
            }

            let warmup_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let settings = settings::load_settings(&warmup_handle).unwrap_or_default();
                if settings.warmup_on_start {
                    let _ = commands::warmup_parakeet(warmup_handle.clone()).await;
                    if settings.dictation_normalize {
                        let _ = normalizer::warmup(&warmup_handle).await;
                    }
                }
            });

            app.set_activation_policy(tauri::ActivationPolicy::Regular);

            #[cfg(target_os = "macos")]
            crate::utils::macos::install_activation_handler(handle.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::show_overlay,
            commands::resize_overlay,
            commands::set_overlay_pill_shown,
            commands::process_audio_with_history,
            commands::process_pcm16k_with_history,
            commands::paste_text_command,
            commands::export_dictionary_csv,
            commands::import_dictionary_csv,
            commands::warmup_parakeet,
            commands::get_settings,
            commands::save_settings,
            commands::get_parakeet_status,
            commands::ensure_parakeet_runtime,
            commands::get_normalizer_status,
            commands::ensure_normalizer_model,
            commands::update_hotkey,
            commands::get_history,
            commands::get_dictation_stats,
            commands::delete_history_entry,
            commands::clear_all_history,
            commands::check_accessibility_permission,
            commands::set_guide_mode,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Reopen { .. } = event {
            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let _ = window::open_dashboard(handle).await;
            });
        }
    });
}
