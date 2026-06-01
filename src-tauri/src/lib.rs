mod app_context;
mod audio;
mod commands;
mod debug;
mod input;
mod parakeet;
mod parakeet_install;
mod settings;
mod shortcuts;
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
                let _ = settings::save_settings(app, &settings);
            }
            shortcut
        }
    };

    shortcuts::register_hotkey(app, shortcut).map_err(|e| Box::<dyn std::error::Error>::from(e))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let _ = setup_global_shortcuts(&handle);

            if let Some(window) = handle.get_webview_window("main") {
                let _ = window::set_window_on_all_spaces(&window);
            }

            app.set_activation_policy(tauri::ActivationPolicy::Regular);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::show_overlay,
            commands::hide_overlay,
            commands::resize_overlay,
            commands::process_audio,
            commands::process_audio_with_history,
            commands::get_settings,
            commands::save_settings,
            commands::get_parakeet_status,
            commands::ensure_parakeet_runtime,
            commands::update_hotkey,
            commands::get_history,
            commands::delete_history_entry,
            commands::clear_all_history,
            commands::open_dashboard,
            commands::check_accessibility_permission,
            commands::open_accessibility_settings,
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
