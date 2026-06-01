use mouse_position::mouse_position::Mouse;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};

static HOVER_TRACKING_ACTIVE: AtomicBool = AtomicBool::new(false);

// Make window visible on all desktops/spaces
#[cfg(target_os = "macos")]
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

        #[cfg(debug_assertions)]
        println!("✓ Window configured to appear on all Spaces (Overlay Mode)");
    }

    Ok(())
}

#[cfg(windows)]
pub fn set_window_on_all_spaces(window: &tauri::WebviewWindow) -> Result<(), String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
        },
    };

    unsafe {
        let handle = window.window_handle().map_err(|e| e.to_string())?;
        let raw_handle = handle.as_raw();
        let hwnd = match raw_handle {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as isize),
            _ => return Err("Not Windows".to_string()),
        };

        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn set_window_on_all_spaces(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(()) // No-op on unsupported platforms
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
    window
        .set_always_on_top(true)
        .map_err(|e| e.to_string())?;

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

// Resize overlay - No-op for fullscreen overlay
pub async fn resize_overlay(
    _app: AppHandle,
    _recording: bool,
    _width: Option<f64>,
    _height: Option<f64>,
) -> Result<(), String> {
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
    window
        .set_always_on_top(false)
        .map_err(|e| e.to_string())?;

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

    window
        .set_always_on_top(false)
        .map_err(|e| e.to_string())?;

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

// Hide recording overlay
pub async fn hide_overlay(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    window.hide().map_err(|e| e.to_string())?;
    HOVER_TRACKING_ACTIVE.store(false, Ordering::Relaxed);
    Ok(())
}

fn start_hover_tracking(app: AppHandle) {
    if HOVER_TRACKING_ACTIVE.swap(true, Ordering::Relaxed) {
        return;
    }

    tokio::spawn(async move {
        let mut was_interactive = false;

        // Dimensions of the "pill" (mini-bar)
        // Match these with CSS in +page.svelte
        const PILL_WIDTH: f64 = 140.0; // Slightly larger for easier hover
        const PILL_HEIGHT: f64 = 60.0; // Including padding
        const BOTTOM_OFFSET: f64 = 0.0; // CSS is bottom-8, roughly 32px

        while HOVER_TRACKING_ACTIVE.load(Ordering::Relaxed) {
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = window.primary_monitor() {
                    let size = monitor.size();
                    let scale_factor = monitor.scale_factor();

                    let screen_w = size.width as f64 / scale_factor;
                    let screen_h = size.height as f64 / scale_factor;

                    // Calculate pill zone (centered horizontally at bottom)
                    let pill_x_start = (screen_w - PILL_WIDTH) / 2.0;
                    let pill_x_end = pill_x_start + PILL_WIDTH;
                    let pill_y_start = screen_h - PILL_HEIGHT - BOTTOM_OFFSET;
                    let pill_y_end = screen_h;

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

// Sync version for tray menu (non-async context)
pub fn open_dashboard_sync(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("dashboard") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
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
