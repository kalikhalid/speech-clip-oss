#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionaryRef;
#[cfg(target_os = "macos")]
use once_cell::sync::OnceCell;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use tauri::AppHandle;

#[cfg(target_os = "macos")]
static ACTIVATION_APP: OnceCell<AppHandle> = OnceCell::new();
#[cfg(target_os = "macos")]
static ACTIVATION_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "macos")]
static ACTIVATION_HANDLER_READY: AtomicBool = AtomicBool::new(false);

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

/// Open the dashboard when the already-running app is selected via Cmd+Tab.
#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn install_activation_handler(app: AppHandle) {
    if ACTIVATION_HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let _ = ACTIVATION_APP.set(app);

    unsafe {
        use cocoa::base::{id, nil};
        use cocoa::foundation::NSString;
        use objc::{class, msg_send, sel, sel_impl};

        let observer: id = msg_send![activation_observer_class(), new];
        if observer == nil {
            return;
        }

        let center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let name = NSString::alloc(nil).init_str("NSApplicationDidBecomeActiveNotification");

        let _: () = msg_send![
            center,
            addObserver: observer
            selector: sel!(applicationDidBecomeActive:)
            name: name
            object: app
        ];
    }

    tauri::async_runtime::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        ACTIVATION_HANDLER_READY.store(true, Ordering::SeqCst);
    });
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn activation_observer_class() -> &'static objc::runtime::Class {
    use objc::declare::ClassDecl;
    use objc::runtime::{Class, Object, Sel};
    use objc::{class, sel, sel_impl};

    static CLASS: OnceCell<&'static Class> = OnceCell::new();
    CLASS.get_or_init(|| {
        if let Some(existing) = Class::get("SpeechClipActivationObserver") {
            return existing;
        }

        extern "C" fn application_did_become_active(
            _this: &Object,
            _cmd: Sel,
            _notification: cocoa::base::id,
        ) {
            if !ACTIVATION_HANDLER_READY.load(Ordering::SeqCst) {
                return;
            }
            if let Some(app) = ACTIVATION_APP.get() {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::window::open_dashboard(app).await;
                });
            }
        }

        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("SpeechClipActivationObserver", superclass)
            .expect("activation observer class must register once");
        unsafe {
            decl.add_method(
                sel!(applicationDidBecomeActive:),
                application_did_become_active as extern "C" fn(&Object, Sel, cocoa::base::id),
            );
        }
        decl.register()
    })
}
