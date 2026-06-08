use mouse_position::mouse_position::Mouse;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

static HOVER_TRACKING_ACTIVE: AtomicBool = AtomicBool::new(false);
static OVERLAY_PILL_SHOWN: AtomicBool = AtomicBool::new(true);

pub fn set_overlay_pill_shown(shown: bool) {
    OVERLAY_PILL_SHOWN.store(shown, Ordering::Relaxed);
}

/// Matches overlay pill sizes from `src/lib/config.ts` and `+page.svelte` CSS.
const IDLE_PILL_WIDTH: f64 = 40.0;
const IDLE_PILL_HEIGHT: f64 = 10.0;
/// CSS `.liquid-bar.hovered:not(.dictating)` — idle hover animation size.
const IDLE_HOVER_WIDTH: f64 = 50.0;
const IDLE_HOVER_HEIGHT: f64 = 14.0;
/// Tailwind `bottom-8` on the pill anchor (32px).
const PILL_BOTTOM_OFFSET: f64 = 32.0;
const HIT_PAD_X: f64 = 4.0;
const HIT_PAD_Y: f64 = 4.0;

#[derive(Clone, Copy)]
struct PillHitbox {
    width: f64,
    height: f64,
    recording: bool,
}

impl Default for PillHitbox {
    fn default() -> Self {
        Self {
            width: IDLE_PILL_WIDTH,
            height: IDLE_PILL_HEIGHT,
            recording: false,
        }
    }
}

static PILL_HITBOX: Mutex<PillHitbox> = Mutex::new(PillHitbox {
    width: IDLE_PILL_WIDTH,
    height: IDLE_PILL_HEIGHT,
    recording: false,
});

fn pill_hit_zone(hitbox: PillHitbox) -> (f64, f64) {
    if hitbox.recording {
        return (
            hitbox.width + HIT_PAD_X * 2.0,
            hitbox.height + HIT_PAD_Y * 2.0,
        );
    }
    // Idle: only cover the hovered pill size, not the old oversized zone.
    (
        IDLE_HOVER_WIDTH + HIT_PAD_X * 2.0,
        IDLE_HOVER_HEIGHT + HIT_PAD_Y * 2.0,
    )
}

// Make window visible on all desktops/spaces
#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn set_window_on_all_spaces(window: &tauri::WebviewWindow) -> Result<(), String> {
    use cocoa::base::id;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let ns_window: id = window.ns_window().map_err(|e| e.to_string())? as id;
        if ns_window.is_null() {
            return Err("Failed to get NSWindow".to_string());
        }

        // NSWindowCollectionBehaviorCanJoinAllSpaces = 1 << 0
        // NSWindowCollectionBehaviorStationary = 1 << 4
        // NSWindowCollectionBehaviorIgnoresCycle = 1 << 6
        // NSWindowCollectionBehaviorFullScreenAuxiliary = 1 << 8
        // This makes the window appear on all Spaces, stationary, ignores cmd+tab cycle, and allows it over fullscreen apps
        let behavior: u64 = (1 << 0) | (1 << 4) | (1 << 6) | (1 << 8);
        let _: () = msg_send![ns_window, setCollectionBehavior: behavior];
    }

    Ok(())
}

// Show recording overlay
pub async fn show_overlay(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;

    // Configure window to appear on all Spaces/desktops (macOS)
    set_window_on_all_spaces(&window)?;

    // Ensure window is transparent and ignores mouse events initially
    window
        .set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    window.set_always_on_top(true).map_err(|e| e.to_string())?;

    // Manually set size to full screen (Fake Fullscreen)
    if let Some(monitor) = window.primary_monitor().map_err(|e| e.to_string())? {
        let size = monitor.size();
        let scale_factor = monitor.scale_factor();

        // Convert to logical size
        let logical_width = size.width as f64 / scale_factor;
        let logical_height = size.height as f64 / scale_factor;

        window
            .set_size(tauri::Size::Logical(tauri::LogicalSize {
                width: logical_width,
                height: logical_height,
            }))
            .map_err(|e| e.to_string())?;
        window
            .set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: 0.0,
                y: 0.0,
            }))
            .map_err(|e| e.to_string())?;
    }

    window.show().map_err(|e| e.to_string())?;

    start_hover_tracking(app.clone());

    Ok(())
}

// Window stays fullscreen; we only update the native hover/click hit zone.
pub async fn resize_overlay(
    _app: AppHandle,
    recording: bool,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<(), String> {
    if let (Some(width), Some(height)) = (width, height) {
        if let Ok(mut hitbox) = PILL_HITBOX.lock() {
            hitbox.width = width;
            hitbox.height = height;
            hitbox.recording = recording;
        }
    } else if let Ok(mut hitbox) = PILL_HITBOX.lock() {
        hitbox.recording = recording;
    }
    Ok(())
}

// Set explicit window size for guide/onboarding mode
pub async fn set_window_size(app: &AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;

    HOVER_TRACKING_ACTIVE.store(false, Ordering::Relaxed);
    window
        .set_ignore_cursor_events(false)
        .map_err(|e| e.to_string())?;
    // Guide must not float above the macOS accessibility prompt.
    window.set_always_on_top(false).map_err(|e| e.to_string())?;

    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
        .map_err(|e| e.to_string())?;

    center_on_primary_monitor(&window, width, height)?;

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

/// Move the guide window out of the way and drop always-on-top before a system dialog.
pub async fn step_aside_for_system_dialog(app: &AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;

    window.set_always_on_top(false).map_err(|e| e.to_string())?;

    let monitor = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("Primary monitor not found")?;

    let scale = monitor.scale_factor();
    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();

    let outer_size = window.outer_size().map_err(|e| e.to_string())?;
    let width = outer_size.width as f64 / scale;
    let height = outer_size.height as f64 / scale;

    let screen_w = monitor_size.width as f64 / scale;
    let screen_h = monitor_size.height as f64 / scale;
    let origin_x = monitor_pos.x as f64 / scale;
    let origin_y = monitor_pos.y as f64 / scale;

    const MARGIN: f64 = 24.0;
    let x = origin_x + screen_w - width - MARGIN;
    let y = origin_y + screen_h - height - MARGIN;

    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
        .map_err(|e| e.to_string())
}

fn center_on_primary_monitor(
    window: &tauri::WebviewWindow,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("Primary monitor not found")?;

    let scale = monitor.scale_factor();
    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();

    let screen_w = monitor_size.width as f64 / scale;
    let screen_h = monitor_size.height as f64 / scale;
    let origin_x = monitor_pos.x as f64 / scale;
    let origin_y = monitor_pos.y as f64 / scale;

    let x = origin_x + (screen_w - width) / 2.0;
    let y = origin_y + (screen_h - height) / 2.0;

    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
        .map_err(|e| e.to_string())
}

fn start_hover_tracking(app: AppHandle) {
    if HOVER_TRACKING_ACTIVE.swap(true, Ordering::Relaxed) {
        return;
    }

    tokio::spawn(async move {
        let mut was_interactive = false;

        while HOVER_TRACKING_ACTIVE.load(Ordering::Relaxed) {
            if let Some(window) = app.get_webview_window("main") {
                if !OVERLAY_PILL_SHOWN.load(Ordering::Relaxed) {
                    if was_interactive {
                        let _ = window.set_ignore_cursor_events(true);
                        was_interactive = false;
                        let _ = app.emit("hover-changed", false);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    continue;
                }

                if let Ok(Some(monitor)) = window.primary_monitor() {
                    let size = monitor.size();
                    let scale_factor = monitor.scale_factor();

                    let screen_w = size.width as f64 / scale_factor;
                    let screen_h = size.height as f64 / scale_factor;

                    let hitbox = PILL_HITBOX.lock().map(|guard| *guard).unwrap_or_default();
                    let (pill_width, pill_height) = pill_hit_zone(hitbox);

                    // Pill zone: centered horizontally, anchored above bottom-8.
                    let pill_x_start = (screen_w - pill_width) / 2.0;
                    let pill_x_end = pill_x_start + pill_width;
                    let pill_y_start = screen_h - PILL_BOTTOM_OFFSET - pill_height;
                    let pill_y_end = screen_h - PILL_BOTTOM_OFFSET;

                    if let Mouse::Position {
                        x: mouse_x,
                        y: mouse_y,
                    } = Mouse::get_mouse_position()
                    {
                        // Mouse position is usually in physical pixels or logical depending on OS/crate
                        // mouse_position crate returns Logical pixels on macOS usually, but need to verify.
                        // Actually, mouse_position often returns physical pixels on some OS.
                        // Let's assume logical for now, or check coordinate system.
                        // Update: mouse_position usually returns global screen coordinates.

                        // NOTE: If this doesn't work, we might need to adjust for scale_factor on mouse_x/y

                        let is_over_pill = mouse_x as f64 >= pill_x_start
                            && mouse_x as f64 <= pill_x_end
                            && mouse_y as f64 >= pill_y_start
                            && mouse_y as f64 <= pill_y_end;

                        if is_over_pill != was_interactive {
                            // Toggle click-through
                            // If over pill -> ignore_cursor_events(false) [Interactive]
                            // If NOT over pill -> ignore_cursor_events(true) [Click-through]
                            let _ = window.set_ignore_cursor_events(!is_over_pill);
                            was_interactive = is_over_pill;
                            let _ = app.emit("hover-changed", is_over_pill);
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    });
}

// Dashboard window management
pub async fn open_dashboard(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("dashboard") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
        // Create window if it doesn't exist
        let _window = tauri::WebviewWindowBuilder::new(
            &app,
            "dashboard",
            tauri::WebviewUrl::App("/dashboard".into()),
        )
        .title("Speech Clip OSS")
        .inner_size(800.0, 600.0)
        .min_inner_size(910.0, 625.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
