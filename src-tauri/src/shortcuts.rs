use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use std::sync::atomic::{AtomicU64, Ordering};

static LAST_HOTKEY_RELEASE_MS: AtomicU64 = AtomicU64::new(0);
const HOTKEY_RELEASE_DEBOUNCE_MS: u64 = 400;

pub const DEFAULT_HOTKEY: &str = "control+`";

pub fn parse_shortcut(input: &str) -> Result<Shortcut, String> {
    input
        .parse::<Shortcut>()
        .map_err(|e| format!("Invalid hotkey: {}", e))
}

pub fn register_hotkey(app: &AppHandle, shortcut: Shortcut) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;

    register_with_handler(app, shortcut)?;

    Ok(())
}

pub fn replace_hotkey(
    app: &AppHandle,
    new_shortcut: Shortcut,
    old_shortcut: Option<Shortcut>,
) -> Result<(), String> {
    if let Some(old) = old_shortcut {
        if old == new_shortcut {
            return Ok(());
        }
        // Сначала удаляем старую горячую клавишу
        app.global_shortcut()
            .unregister(old)
            .map_err(|e| e.to_string())?;
    }
    // Затем регистрируем новую
    register_with_handler(app, new_shortcut)?;
    Ok(())
}

fn register_with_handler(app: &AppHandle, shortcut: Shortcut) -> Result<(), String> {
    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| match event.state {
            ShortcutState::Pressed => {
                let _ = app_handle.emit("hotkey-pressed", ());
            }
            ShortcutState::Released => {
                let release_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let last = LAST_HOTKEY_RELEASE_MS.load(Ordering::SeqCst);
                if release_time.saturating_sub(last) < HOTKEY_RELEASE_DEBOUNCE_MS {
                    return;
                }
                LAST_HOTKEY_RELEASE_MS.store(release_time, Ordering::SeqCst);
                let _ = app_handle.emit("hotkey-released", release_time);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
