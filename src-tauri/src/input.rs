use arboard::Clipboard;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::oneshot;

#[cfg(target_os = "macos")]
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
#[cfg(target_os = "macos")]
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

pub async fn type_text(
    app: &AppHandle,
    text: String,
    _app_name: Option<String>,
) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    let (tx, rx) = oneshot::channel();
    let text_clone = text.clone();

    app.run_on_main_thread(move || {
        let result = (move || {
            let mut clipboard =
                Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;

            clipboard
                .set_text(&text_clone)
                .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

            thread::sleep(Duration::from_millis(50));
            simulate_cmd_v()?;
            thread::sleep(Duration::from_millis(30));
            Ok::<(), String>(())
        })();

        let _ = tx.send(result);
    })
    .map_err(|e| format!("Failed to dispatch to main thread: {}", e))?;

    rx.await
        .map_err(|_| "Main thread typing task cancelled".to_string())?
}

#[cfg(target_os = "macos")]
fn simulate_cmd_v() -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Failed to create event source".to_string())?;

    const V_KEY: CGKeyCode = 9;

    let cmd_down = CGEvent::new_keyboard_event(source.clone(), 55, true)
        .map_err(|_| "Failed to create Cmd key down event".to_string())?;
    cmd_down.set_flags(CGEventFlags::CGEventFlagCommand);
    cmd_down.post(CGEventTapLocation::HID);

    let v_down = CGEvent::new_keyboard_event(source.clone(), V_KEY, true)
        .map_err(|_| "Failed to create V key down event".to_string())?;
    v_down.set_flags(CGEventFlags::CGEventFlagCommand);
    v_down.post(CGEventTapLocation::HID);

    let v_up = CGEvent::new_keyboard_event(source.clone(), V_KEY, false)
        .map_err(|_| "Failed to create V key up event".to_string())?;
    v_up.set_flags(CGEventFlags::CGEventFlagCommand);
    v_up.post(CGEventTapLocation::HID);

    let cmd_up = CGEvent::new_keyboard_event(source, 55, false)
        .map_err(|_| "Failed to create Cmd key up event".to_string())?;
    cmd_up.post(CGEventTapLocation::HID);

    Ok(())
}
