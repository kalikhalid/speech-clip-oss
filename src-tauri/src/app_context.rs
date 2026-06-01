#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppCategory {
    Developer, // IDEs, terminals, AI tools
    Messenger, // Chat apps
    Browser,   // Web browsers (likely AI prompting)
    General,   // Everything else
}

impl std::fmt::Display for AppCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppCategory::Developer => write!(f, "Developer"),
            AppCategory::Messenger => write!(f, "Messenger"),
            AppCategory::Browser => write!(f, "Browser"),
            AppCategory::General => write!(f, "General"),
        }
    }
}

pub fn detect_app_category(app_name: &str) -> AppCategory {
    let lower = app_name.to_lowercase();

    // Developer tools: IDEs, Terminals, API Tools, Editors
    if lower.contains("visual studio code")
        || lower.contains("cursor")
        || lower.contains("antigravity") // Modern AI IDE
        || lower.contains("zed")         // Next-gen high-performance editor
        || lower.contains("nova")        // Native macOS editor
        || lower.contains("xcode")
        || lower.contains("intellij")
        || lower.contains("jetbrains")
        || lower.contains("pycharm")
        || lower.contains("webstorm")
        || lower.contains("rustrover")
        || lower.contains("android")
        || lower.contains("terminal")
        || lower.contains("iterm")
        || lower.contains("warp")
        || lower.contains("ghostty")      // Modern GPU terminal
        || lower.contains("kitty")       // GPU-accelerated terminal
        || lower.contains("wezterm")
        || lower.contains("alacritty")
        || lower.contains("tabby")
        || lower.contains("hyper")
        || lower.contains("shell")
        || lower.contains("code")
        || lower.contains("sublime")
        || lower.contains("bbedit")
        || lower.contains("vim")
        || lower.contains("neovim")
        || lower.contains("postman")     // API testing
        || lower.contains("insomnia")
    // API testing
    {
        return AppCategory::Developer;
    }

    // Messengers & Collaboration Tools
    if lower.contains("telegram")
        || lower.contains("slack")
        || lower.contains("discord")
        || lower.contains("whatsapp")
        || lower.contains("messages")     // iMessage
        || lower.contains("signal")
        || lower.contains("messenger")    // Meta Messenger
        || lower.contains("viber")
        || lower.contains("skype")
        || lower.contains("teams")        // Microsoft Teams
        || lower.contains("zoom")         // Zoom Workplace
        || lower.contains("google chat")
        || lower.contains("line")
        || lower.contains("element")
        || lower.contains("session")
        || lower.contains("threema")
        || lower.contains("wickr")
    {
        return AppCategory::Messenger;
    }

    // Browsers (often used for AI prompting like Claude, ChatGPT)
    if lower.contains("chrome")
        || lower.contains("safari")
        || lower.contains("firefox")
        || lower.contains("arc")          // Browser for work
        || lower.contains("brave")
        || lower.contains("edge")
        || lower.contains("orion")        // Privacy-focused native browser
        || lower.contains("vivaldi")
        || lower.contains("tor browser")
        || lower.contains("chromium")
        || lower.contains("opera")
        || lower.contains("claude")
        || lower.contains("chatgpt")
        || lower.contains("gemini")
        || lower.contains("perplexity")
        || lower.contains("comet")
        || lower.contains("atlas")
    {
        return AppCategory::Browser;
    }

    AppCategory::General
}

// Get frontmost application name (macOS only)
// This doesn't require special permissions - just the app name, not window title
#[cfg(target_os = "macos")]
pub fn get_frontmost_app_name() -> Option<String> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        // Get shared workspace
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil {
            return None;
        }

        // Get frontmost application
        let frontmost_app: id = msg_send![workspace, frontmostApplication];
        if frontmost_app == nil {
            return None;
        }

        // Get localized name (e.g., "Visual Studio Code", "Terminal", "Notes")
        let app_name: id = msg_send![frontmost_app, localizedName];
        if app_name == nil {
            return None;
        }

        // Convert NSString to Rust String
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

#[cfg(windows)]
pub fn get_frontmost_app_name() -> Option<String> {
    use windows::Win32::{
        Foundation::HWND,
        System::{
            ProcessStatus::{EnumProcessModules, GetModuleBaseNameW},
            Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == 0 {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut module = std::mem::zeroed();
        let mut needed = 0u32;
        EnumProcessModules(handle, std::slice::from_mut(&mut module), &mut needed).ok()?;

        let mut buffer = [0u16; 260];
        let len = GetModuleBaseNameW(handle, module, &mut buffer);

        windows::Win32::Foundation::CloseHandle(handle).ok()?;

        if len > 0 {
            Some(String::from_utf16_lossy(&buffer[..len as usize]))
        } else {
            None
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn get_frontmost_app_name() -> Option<String> {
    None
}
