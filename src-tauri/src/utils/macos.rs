#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionaryRef;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    pub fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
pub fn check_accessibility_permissions(prompt: bool) -> bool {
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    unsafe {
        if prompt {
            let key = CFString::new("AXTrustedCheckOptionPrompt");
            let value = CFBoolean::true_value();

            let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

            AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
        } else {
            AXIsProcessTrusted()
        }
    }
}

/// App Nap mitigation: `NSAppSleepDisabled` in `src-tauri/Info.plist` is the primary fix.
/// A runtime `NSProcessInfo` activity token would be a partial complement; not wired here
/// to avoid extra objc/cocoa surface — plist alone matches most dictation apps' needs.
#[cfg(target_os = "macos")]
pub fn disable_app_nap() {
    // Intentionally empty — see Info.plist and comment above.
}
