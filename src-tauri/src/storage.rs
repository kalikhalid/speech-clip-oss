// Storage module for transcription history
// In-memory cache with async disk persistence

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use tauri::{AppHandle, Manager};

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
    /// Pipeline stage timings in milliseconds (when available).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<TranscriptionTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranscriptionTiming {
    pub total_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asr_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalizer_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postprocess_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typing_ms: Option<u64>,
}

/// Container for all history entries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranscriptionHistory {
    pub entries: Vec<TranscriptionEntry>,
}

/// Word counts from transcription history (`normalized_text`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DictationStats {
    pub words_24h: u64,
    pub words_7d: u64,
    pub words_all_time: u64,
}

const MAX_HISTORY_ENTRIES: usize = 500;

static DISK_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// In-memory history loaded at startup; managed via `app.manage`.
pub struct HistoryStore {
    inner: RwLock<TranscriptionHistory>,
}

impl HistoryStore {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let history = read_history_from_disk(app)?;
        Ok(Self {
            inner: RwLock::new(history),
        })
    }

    pub fn get(&self, limit: Option<usize>) -> TranscriptionHistory {
        let guard = self.inner.read().expect("history lock poisoned");
        match limit {
            Some(n) => TranscriptionHistory {
                entries: guard.entries.iter().take(n).cloned().collect(),
            },
            None => guard.clone(),
        }
    }

    fn with_write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TranscriptionHistory) -> R,
    {
        let mut guard = self.inner.write().expect("history lock poisoned");
        f(&mut guard)
    }

    fn snapshot(&self) -> TranscriptionHistory {
        self.inner.read().expect("history lock poisoned").clone()
    }

    pub fn dictation_stats(&self) -> DictationStats {
        let guard = self.inner.read().expect("history lock poisoned");
        let now_ms = chrono::Utc::now().timestamp_millis();
        let window_24h_ms = 24_i64 * 60 * 60 * 1000;
        let window_7d_ms = 7 * window_24h_ms;

        let mut stats = DictationStats::default();
        for entry in &guard.entries {
            let words = word_count(&entry.normalized_text);
            stats.words_all_time += words;

            let age_ms = now_ms.saturating_sub(entry.timestamp);
            if age_ms <= window_24h_ms {
                stats.words_24h += words;
            }
            if age_ms <= window_7d_ms {
                stats.words_7d += words;
            }
        }
        stats
    }

    fn schedule_persist(app: &AppHandle, history: TranscriptionHistory) {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _ = write_history_to_disk(&app, &history);
        });
    }
}

fn with_history_store<R>(app: &AppHandle, f: impl FnOnce(&HistoryStore) -> R) -> Result<R, String> {
    let store = app
        .try_state::<std::sync::Arc<HistoryStore>>()
        .ok_or_else(|| "History store not initialized".to_string())?;
    Ok(f(store.inner()))
}

fn get_history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    Ok(data_dir.join("history.json"))
}

fn read_history_from_disk(app: &AppHandle) -> Result<TranscriptionHistory, String> {
    let _guard = DISK_LOCK.lock().map_err(|e| e.to_string())?;
    let path = get_history_path(app)?;
    if !path.exists() {
        return Ok(TranscriptionHistory::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read history: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse history: {}", e))
}

fn write_history_to_disk(app: &AppHandle, history: &TranscriptionHistory) -> Result<(), String> {
    let _guard = DISK_LOCK.lock().map_err(|e| e.to_string())?;
    let path = get_history_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create history dir: {}", e))?;
    }
    let json =
        serde_json::to_vec(history).map_err(|e| format!("Failed to serialize history: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write history: {}", e))?;
    Ok(())
}

/// Load history from in-memory cache (optional limit for IPC).
pub fn load_history(app: &AppHandle, limit: Option<usize>) -> Result<TranscriptionHistory, String> {
    with_history_store(app, |store| store.get(limit))
}

/// Aggregate word counts for overview stats (from in-memory history).
pub fn dictation_stats(app: &AppHandle) -> Result<DictationStats, String> {
    with_history_store(app, |store| store.dictation_stats())
}

fn word_count(text: &str) -> u64 {
    if text.trim().is_empty() {
        return 0;
    }
    text.split_whitespace().count() as u64
}

/// Update RAM immediately, persist to disk on a blocking thread.
pub fn save_transcription(app: &AppHandle, entry: TranscriptionEntry) -> Result<(), String> {
    let snapshot = with_history_store(app, |store| {
        store.with_write(|history| {
            history.entries.insert(0, entry);
            if history.entries.len() > MAX_HISTORY_ENTRIES {
                history.entries.truncate(MAX_HISTORY_ENTRIES);
            }
        });
        store.snapshot()
    })?;
    HistoryStore::schedule_persist(app, snapshot);
    Ok(())
}

/// Delete a specific history entry by ID
pub fn delete_entry(app: &AppHandle, entry_id: &str) -> Result<(), String> {
    let snapshot = with_history_store(app, |store| {
        store.with_write(|history| {
            history.entries.retain(|e| e.id != entry_id);
        });
        store.snapshot()
    })?;
    HistoryStore::schedule_persist(app, snapshot);
    Ok(())
}

/// Clear all history
pub fn clear_history(app: &AppHandle) -> Result<(), String> {
    with_history_store(app, |store| {
        store.with_write(|history| {
            history.entries.clear();
        });
    })?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = DISK_LOCK.lock();
        if let Ok(path) = get_history_path(&app_clone) {
            if path.exists() {
                let _ = fs::remove_file(&path);
            }
        }
    });
    Ok(())
}
