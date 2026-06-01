#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::boolean::CFBooleanRef;
#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionaryRef;
#[cfg(target_os = "macos")]
use core_foundation::string::CFStringRef;
#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::ptr;

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

#[cfg(target_os = "macos")]
pub fn open_accessibility_settings() {
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}
