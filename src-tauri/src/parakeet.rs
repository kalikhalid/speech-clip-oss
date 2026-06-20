//! Local Parakeet TDT 0.6B v3 via [transcribe-rs](https://github.com/cjpais/transcribe-rs) (ONNX).

use crate::parakeet_install::install_in_progress;
pub use crate::parakeet_install::{
    ensure_model, ensure_runtime, model_downloaded, model_path, DEFAULT_MODEL_ID,
};

use crate::audio::resample_to_16k;
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::AppHandle;
use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity};
use transcribe_rs::onnx::Quantization;

static ENGINE: OnceCell<Mutex<Option<CachedEngine>>> = OnceCell::new();

struct CachedEngine {
    model_id: String,
    model: ParakeetModel,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParakeetSetupStatus {
    pub model_id: String,
    pub model_dir: String,
    pub model_downloaded: bool,
    pub ready: bool,
    pub message: String,
    pub install_stage: String,
    pub install_in_progress: bool,
}

fn engine_lock() -> &'static Mutex<Option<CachedEngine>> {
    ENGINE.get_or_init(|| Mutex::new(None))
}

fn display_path(path: &str) -> String {
    if let Some(idx) = path.find("models/") {
        return format!("…/{}", &path[idx..]);
    }
    if path.len() > 44 {
        format!("…{}", &path[path.len().saturating_sub(40)..])
    } else {
        path.to_string()
    }
}

fn install_stage_label(ready: bool, in_progress: bool) -> String {
    if ready {
        return "ready".to_string();
    }
    if in_progress {
        return "installing".to_string();
    }
    "not_started".to_string()
}

pub fn normalize_model_id(model_id: &str) -> String {
    match model_id {
        "mlx-community/parakeet-tdt-0.6b-v3" => DEFAULT_MODEL_ID.to_string(),
        other => other.to_string(),
    }
}

pub async fn check_setup(app: &AppHandle, model_id: &str) -> Result<ParakeetSetupStatus, String> {
    let model_id = normalize_model_id(model_id);
    let downloaded = model_downloaded(app, &model_id);
    let in_progress = install_in_progress();
    let model_dir_path = model_path(app, &model_id)?;
    let model_dir = display_path(&model_dir_path.display().to_string());
    let ready = downloaded;
    let install_stage = install_stage_label(ready, in_progress);

    let message = if ready {
        format!("Ready. Parakeet v3 ONNX model is installed at {model_dir}.")
    } else if in_progress {
        "Downloading Parakeet v3 model…".to_string()
    } else {
        "Parakeet v3 model will download on first dictation or from the dashboard.".to_string()
    };

    Ok(ParakeetSetupStatus {
        model_id,
        model_dir,
        model_downloaded: downloaded,
        ready,
        message,
        install_stage,
        install_in_progress: in_progress,
    })
}

fn load_engine(model_id: &str, model_dir: PathBuf) -> Result<ParakeetModel, String> {
    ParakeetModel::load(&model_dir, &Quantization::Int8)
        .map_err(|e| format!("Failed to load Parakeet model \"{model_id}\": {e}"))
}

fn transcribe_samples(
    model_id: &str,
    model_dir: PathBuf,
    samples: Vec<f32>,
) -> Result<String, String> {
    let mut guard = engine_lock()
        .lock()
        .map_err(|_| "Speech engine lock poisoned".to_string())?;

    let needs_reload = guard
        .as_ref()
        .map(|cached| cached.model_id != model_id)
        .unwrap_or(true);

    if needs_reload {
        let model = load_engine(model_id, model_dir)?;
        *guard = Some(CachedEngine {
            model_id: model_id.to_string(),
            model,
        });
    }

    let cached = guard
        .as_mut()
        .ok_or_else(|| "Parakeet engine failed to initialize".to_string())?;

    if samples.is_empty() {
        return Err("No speech detected. Try speaking a bit longer.".to_string());
    }

    let params = ParakeetParams {
        timestamp_granularity: Some(TimestampGranularity::Segment),
        ..Default::default()
    };

    let result = cached
        .model
        .transcribe_with(&samples, &params)
        .map_err(|e| format!("Parakeet transcription failed: {e}"))?;

    let text = strip_unk_tokens(&result.text);
    if text.is_empty() {
        return Err("No speech detected. Try speaking a bit longer.".to_string());
    }

    Ok(text)
}

/// Parakeet sometimes emits the literal `<unk>` token (standalone or inside a word).
fn strip_unk_tokens(text: &str) -> String {
    text.split_whitespace()
        .filter(|token| *token != "<unk>")
        .map(|token| token.replace("<unk>", ""))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub async fn ensure_model_ready(app: &AppHandle, model_id: &str) -> Result<(), String> {
    let model_id = normalize_model_id(model_id);
    let status = check_setup(app, &model_id).await?;
    if status.ready {
        return Ok(());
    }
    ensure_model(app, &model_id).await?;
    let status = check_setup(app, &model_id).await?;
    if status.ready {
        Ok(())
    } else {
        Err(status.message)
    }
}

pub async fn transcribe_decoded(
    app: &AppHandle,
    samples: Vec<f32>,
    sample_rate: u32,
    model_id: &str,
) -> Result<String, String> {
    let model_id = normalize_model_id(model_id);
    ensure_model_ready(app, &model_id).await?;

    let samples = resample_to_16k(&samples, sample_rate)?;
    let model_dir = model_path(app, &model_id)?;

    let model_id_for_task = model_id.clone();
    tokio::task::spawn_blocking(move || transcribe_samples(&model_id_for_task, model_dir, samples))
        .await
        .map_err(|e| format!("Transcription task failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn strip_unk_tokens_removes_standalone_and_embedded() {
        assert_eq!(
            strip_unk_tokens("Учитывая вс<unk>, что ты знаешь"),
            "Учитывая вс, что ты знаешь"
        );
        assert_eq!(strip_unk_tokens("hello <unk> world"), "hello world");
        assert_eq!(strip_unk_tokens("no tokens here"), "no tokens here");
    }

    #[test]
    fn transcribes_16k_wav_with_installed_model() {
        let home = std::env::var("HOME").expect("HOME");
        let model_dir = PathBuf::from(format!(
            "{home}/Library/Application Support/dev.speechclip.oss/models/parakeet-tdt-0.6b-v3-int8"
        ));
        if !model_dir.join("encoder-model.int8.onnx").exists() {
            eprintln!("Skipping: model not installed at {}", model_dir.display());
            return;
        }

        let wav = std::fs::read("/tmp/test-tone.wav").expect("read wav bytes");
        let (samples, sample_rate) = crate::audio::decode_wav_bytes(&wav).expect("decode");
        let samples = crate::audio::resample_to_16k(&samples, sample_rate).expect("resample");
        let text =
            transcribe_samples("parakeet-tdt-0.6b-v3", model_dir, samples).expect("transcribe");
        assert!(text.is_empty() || !text.contains("AlignedResult"));
    }
}
