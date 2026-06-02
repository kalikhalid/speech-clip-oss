use std::sync::Mutex;

static LAST_FINAL_TEXT: Mutex<Option<String>> = Mutex::new(None);

pub fn set_last_final_text(text: &str) {
    if let Ok(mut guard) = LAST_FINAL_TEXT.lock() {
        *guard = Some(text.to_string());
    }
}

pub fn get_last_final_text() -> Option<String> {
    LAST_FINAL_TEXT
        .lock()
        .ok()
        .and_then(|g| g.clone())
}
