// Storage module for transcription history
// Handles saving and loading history entries from app data directory

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

fn default_transcription_engine() -> String {
    "server".to_string()
}

/// A single transcription history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionEntry {
    /// Unique identifier (UUID)
    pub id: String,
    /// Unix timestamp (milliseconds)
    pub timestamp: i64,
    /// Raw transcription from Whisper
    pub raw_text: String,
    /// Normalized text after LLM processing
    pub normalized_text: String,
    /// Active application name (optional)
    pub app_name: Option<String>,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
    /// Transcription engine used: "local" or "server"
    #[serde(default = "default_transcription_engine")]
    pub engine: String,
    /// Local model used for transcription (for engine=local)
    #[serde(default)]
    pub local_model: Option<String>,
    /// User comment for debug/feedback
    #[serde(default)]
    pub comment: Option<String>,
}

/// Container for all history entries
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TranscriptionHistory {
    pub entries: Vec<TranscriptionEntry>,
}

const MAX_HISTORY_ENTRIES: usize = 500;

// Global lock to prevent race conditions during file I/O
static HISTORY_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn get_history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    Ok(data_dir.join("history.json"))
}

/// Load history from disk
pub fn load_history(app: &AppHandle) -> Result<TranscriptionHistory, String> {
    let _guard = HISTORY_LOCK.lock().map_err(|e| e.to_string())?;

    let path = get_history_path(app)?;

    if !path.exists() {
        return Ok(TranscriptionHistory::default());
    }

    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read history: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse history: {}", e))
}

/// Save a new transcription entry (prepends to history, async-friendly)
pub fn save_transcription(app: &AppHandle, entry: TranscriptionEntry) -> Result<(), String> {
    let _guard = HISTORY_LOCK.lock().map_err(|e| e.to_string())?;

    let path = get_history_path(app)?;

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create history dir: {}", e))?;
    }

    // Load existing history (manual read to stay within lock)
    let mut history = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        TranscriptionHistory::default()
    };

    // Prepend new entry
    history.entries.insert(0, entry.clone());

    // Truncate to max size
    if history.entries.len() > MAX_HISTORY_ENTRIES {
        history.entries.truncate(MAX_HISTORY_ENTRIES);
    }

    // Write back
    let json = serde_json::to_string_pretty(&history)
        .map_err(|e| format!("Failed to serialize history: {}", e))?;

    fs::write(&path, json).map_err(|e| format!("Failed to write history: {}", e))?;

    #[cfg(debug_assertions)]
    println!("📜 History entry saved (total: {})", history.entries.len());

    // Emit event for frontend update
    if let Err(e) = app.emit("history-updated", entry) {
        #[cfg(debug_assertions)]
        eprintln!("Failed to emit history update event: {}", e);
    }

    Ok(())
}

/// Delete a specific history entry by ID
pub fn delete_entry(app: &AppHandle, entry_id: &str) -> Result<(), String> {
    let _guard = HISTORY_LOCK.lock().map_err(|e| e.to_string())?;

    let path = get_history_path(app)?;

    if !path.exists() {
        return Ok(());
    }

    // Manual load/save within lock
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut history: TranscriptionHistory = serde_json::from_str(&content).unwrap_or_default();

    history.entries.retain(|e| e.id != entry_id);

    let json = serde_json::to_string_pretty(&history)
        .map_err(|e| format!("Failed to serialize history: {}", e))?;

    fs::write(&path, json).map_err(|e| format!("Failed to write history: {}", e))?;

    Ok(())
}

/// Update comment for a specific history entry
pub fn update_comment(app: &AppHandle, entry_id: &str, comment: String) -> Result<(), String> {
    let _guard = HISTORY_LOCK.lock().map_err(|e| e.to_string())?;

    let path = get_history_path(app)?;

    if !path.exists() {
        return Err("History not found".to_string());
    }

    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut history: TranscriptionHistory = serde_json::from_str(&content).unwrap_or_default();

    if let Some(entry) = history.entries.iter_mut().find(|e| e.id == entry_id) {
        entry.comment = if comment.trim().is_empty() {
            None
        } else {
            Some(comment)
        };
    } else {
        return Err("Entry not found".to_string());
    }

    let json = serde_json::to_string_pretty(&history)
        .map_err(|e| format!("Failed to serialize history: {}", e))?;

    fs::write(&path, json).map_err(|e| format!("Failed to write history: {}", e))?;

    Ok(())
}

/// Get a specific history entry by ID
pub fn get_entry(app: &AppHandle, entry_id: &str) -> Result<Option<TranscriptionEntry>, String> {
    let history = load_history(app)?;
    Ok(history.entries.into_iter().find(|e| e.id == entry_id))
}

/// Clear all history
pub fn clear_history(app: &AppHandle) -> Result<(), String> {
    let _guard = HISTORY_LOCK.lock().map_err(|e| e.to_string())?;

    let path = get_history_path(app)?;

    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete history: {}", e))?;
    }

    Ok(())
}
