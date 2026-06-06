// Get frontmost application name (macOS only).
// Does not require special permissions — app name only, not window title.
#[cfg(target_os = "macos")]
pub fn get_frontmost_app_name() -> Option<String> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil {
            return None;
        }

        let frontmost_app: id = msg_send![workspace, frontmostApplication];
        if frontmost_app == nil {
            return None;
        }

        let app_name: id = msg_send![frontmost_app, localizedName];
        if app_name == nil {
            return None;
        }

        let utf8_ptr: *const i8 = msg_send![app_name, UTF8String];
        if utf8_ptr.is_null() {
            return None;
        }

        Some(
            std::ffi::CStr::from_ptr(utf8_ptr)
                .to_string_lossy()
                .into_owned(),
        )
    }
}
