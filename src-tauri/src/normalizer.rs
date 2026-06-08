//! Local dictation normalizer (Qwen3.5-0.8B or legacy Gemma GGUF via llama-cpp-4 / Metal).

pub use crate::normalizer_install::{gguf_path, model_downloaded};

use crate::normalizer_install;

use llama_cpp_4::context::params::LlamaContextParams;
use llama_cpp_4::llama_backend::LlamaBackend;
use llama_cpp_4::llama_batch::LlamaBatch;
use llama_cpp_4::model::params::LlamaModelParams;
use llama_cpp_4::model::{AddBos, LlamaModel, Special};
use llama_cpp_4::sampling::LlamaSampler;
use once_cell::sync::OnceCell;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::AppHandle;

const SYSTEM_PROMPT: &str = "Ты нормализуешь русскую голосовую диктовку разработчика. Английские \
технические термины, имена файлов, бренды и аббревиатуры, записанные русскими буквами по \
звучанию, замени на правильное написание (коммит -> commit, карго томл -> @Cargo.toml, зум -> \
Zoom). Имена файлов пиши через собачку @. Русские слова не переводи и не меняй. Меняй только \
термины, всё остальное оставляй как есть.";

const MAX_NEW_TOKENS: usize = 256;
const MIN_CTX: u32 = 1024;

/// Qwen3.5 ChatML markers (must match HF `apply_chat_template`, `enable_thinking=false`).
const QWEN_IM_START: &str = "<|im_start|>";
const QWEN_IM_END: &str = concat!("<|", "im_end", "|>");

static BACKEND: OnceCell<LlamaBackend> = OnceCell::new();
static MODEL: OnceCell<Mutex<Option<CachedModel>>> = OnceCell::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptFormat {
    /// ChatML: `<|im_start|>system` / `` (Qwen3.5)
    Qwen,
    /// Gemma turn markers (legacy Gemma 3 270M)
    Gemma,
}

struct CachedModel {
    path: PathBuf,
    format: PromptFormat,
    model: LlamaModel,
}

fn backend() -> Result<&'static LlamaBackend, String> {
    BACKEND
        .get_or_try_init(LlamaBackend::init)
        .map_err(|e| format!("Failed to init llama backend: {e}"))
}

fn model_lock() -> &'static Mutex<Option<CachedModel>> {
    MODEL.get_or_init(|| Mutex::new(None))
}

fn prompt_format_for(model_path: &Path) -> PromptFormat {
    let name = model_path.to_string_lossy().to_lowercase();
    if name.contains("qwen") {
        PromptFormat::Qwen
    } else {
        PromptFormat::Gemma
    }
}

/// Parakeet (ASR) inserts commas mid-phrase ("файл, Мэйн, точка Раст") which
/// fragment filenames/terms and block normalization. Drop commas before
/// prompting; the model assembles "@main.rs" reliably from the comma-free text.
fn sanitize_input(raw: &str) -> String {
    let no_commas: String = raw.chars().filter(|&c| c != ',').collect();
    no_commas.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_prompt(raw: &str, format: PromptFormat) -> String {
    let raw = &sanitize_input(raw);
    match format {
        PromptFormat::Qwen => format!(
            "{QWEN_IM_START}system\n{SYSTEM_PROMPT}{QWEN_IM_END}\n\
             {QWEN_IM_START}user\n{raw}{QWEN_IM_END}\n\
             {QWEN_IM_START}assistant\n"
        ),
        PromptFormat::Gemma => format!(
            "<start_of_turn>user\n{SYSTEM_PROMPT}\n\n{raw}<end_of_turn>\n<start_of_turn>model\n"
        ),
    }
}

fn strip_at_stop_token(mut text: String, format: PromptFormat) -> String {
    let stop = match format {
        PromptFormat::Qwen => QWEN_IM_END,
        PromptFormat::Gemma => "<end_of_turn>",
    };
    if let Some((head, _)) = text.split_once(stop) {
        text = head.to_string();
    }
    // Qwen3.5 (enable_thinking=false) prefixes the answer with an empty
    // `<think>\n\n</think>` block; keep only the text after it.
    if format == PromptFormat::Qwen {
        if let Some(idx) = text.rfind("</think>") {
            text = text[idx + "</think>".len()..].to_string();
        }
    }
    text.trim().to_string()
}

fn load_model(model_path: &Path) -> Result<LlamaModel, String> {
    let backend = backend()?;
    let params = LlamaModelParams::default().with_n_gpu_layers(99);
    let params = std::pin::pin!(params);
    LlamaModel::load_from_file(backend, model_path, &params)
        .map_err(|e| format!("Failed to load normalizer model: {e}"))
}

fn with_model<F, T>(model_path: &Path, f: F) -> Result<T, String>
where
    F: FnOnce(&LlamaModel, PromptFormat) -> Result<T, String>,
{
    let format = prompt_format_for(model_path);
    let mut guard = model_lock()
        .lock()
        .map_err(|_| "Normalizer model lock poisoned".to_string())?;

    let needs_reload = guard
        .as_ref()
        .map(|cached| cached.path != model_path || cached.format != format)
        .unwrap_or(true);

    if needs_reload {
        let model = load_model(model_path)?;
        *guard = Some(CachedModel {
            path: model_path.to_path_buf(),
            format,
            model,
        });
    }

    let cached = guard
        .as_ref()
        .ok_or_else(|| "Normalizer model failed to initialize".to_string())?;
    f(&cached.model, cached.format)
}

fn generate_normalized(
    model: &LlamaModel,
    raw: &str,
    format: PromptFormat,
) -> Result<String, String> {
    let prompt = build_prompt(raw, format);
    let stop_marker = match format {
        PromptFormat::Qwen => QWEN_IM_END,
        PromptFormat::Gemma => "<end_of_turn>",
    };
    let add_bos = match format {
        PromptFormat::Qwen => AddBos::Never,
        PromptFormat::Gemma => AddBos::Always,
    };

    let tokens = model
        .str_to_token(&prompt, add_bos)
        .map_err(|e| format!("Failed to tokenize prompt: {e}"))?;

    if tokens.is_empty() {
        return Err("Normalizer prompt produced no tokens".to_string());
    }

    let n_ctx = NonZeroU32::new((tokens.len() as u32 + MAX_NEW_TOKENS as u32 + 64).max(MIN_CTX))
        .ok_or_else(|| "Invalid context size".to_string())?;

    let backend = backend()?;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(Some(n_ctx))
        .with_n_batch(n_ctx.get());

    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Failed to create normalizer context: {e}"))?;

    let mut batch = LlamaBatch::new(n_ctx.get() as usize, 1);
    for (i, &tok) in tokens.iter().enumerate() {
        batch
            .add(tok, i as i32, &[0], i + 1 == tokens.len())
            .map_err(|e| format!("Failed to build prefill batch: {e}"))?;
    }
    ctx.decode(&mut batch)
        .map_err(|e| format!("Normalizer prefill failed: {e}"))?;

    let eos = model.token_eos();
    let eot = model.token_eot();
    let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
    let mut generated: Vec<llama_cpp_4::token::LlamaToken> = Vec::new();
    let mut output = String::new();
    let mut logit_slot = tokens.len() as i32 - 1;

    for _ in 0..MAX_NEW_TOKENS {
        let token = sampler.sample(&ctx, logit_slot);
        logit_slot = 0;

        if model.is_eog_token(token) || token == eos || token == eot {
            break;
        }

        let piece = model
            .token_to_str(token, Special::Plaintext)
            .unwrap_or_default();
        output.push_str(&piece);
        generated.push(token);
        sampler.accept(token);

        if output.contains(stop_marker) {
            break;
        }

        let next_pos = tokens.len() as i32 + generated.len() as i32 - 1;
        batch.clear();
        batch
            .add(token, next_pos, &[0], true)
            .map_err(|e| format!("Failed to build decode batch: {e}"))?;
        ctx.decode(&mut batch)
            .map_err(|e| format!("Normalizer decode failed: {e}"))?;
    }

    if generated.is_empty() {
        return Err("Normalizer produced no output tokens".to_string());
    }

    let trimmed = strip_at_stop_token(output, format);
    if trimmed.is_empty() {
        return Err("Normalizer output was empty".to_string());
    }

    Ok(trimmed)
}

fn normalize_with_model(model_path: &Path, raw: &str) -> Result<String, String> {
    with_model(model_path, |model, format| {
        generate_normalized(model, raw, format)
    })
}

/// Best-effort preload (warmup). Errors are non-fatal.
pub async fn warmup(app: &AppHandle) -> Result<(), String> {
    if !model_downloaded(app) {
        normalizer_install::ensure_model(app).await?;
    }
    let path = gguf_path(app)?;
    let path_for_task = path.clone();
    tokio::task::spawn_blocking(move || {
        let _ = with_model(&path_for_task, |_, _| Ok(()));
    })
    .await
    .map_err(|e| format!("Normalizer warmup task failed: {e}"))?;
    Ok(())
}

/// Normalize ASR text. On any failure returns the original `raw` unchanged.
pub async fn normalize_text(app: &AppHandle, raw: &str) -> String {
    let raw_owned = raw.to_string();
    if raw_owned.trim().is_empty() {
        return raw_owned;
    }

    if normalizer_install::ensure_model(app).await.is_err() {
        return raw_owned;
    }

    let model_path = match gguf_path(app) {
        Ok(path) => path,
        Err(_) => return raw_owned,
    };

    let raw_for_task = raw_owned.clone();
    let task =
        tokio::task::spawn_blocking(move || normalize_with_model(&model_path, &raw_for_task));
    match task.await {
        Ok(Ok(text)) if !text.trim().is_empty() => text,
        Ok(Ok(_)) => raw_owned,
        Ok(Err(_)) => raw_owned,
        Err(_) => raw_owned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn qwen_im_end_token_is_chatml() {
        assert_eq!(QWEN_IM_END, concat!("<|", "im_end", "|>"));
    }

    #[test]
    fn qwen_prompt_uses_chatml_markers() {
        let prompt = build_prompt("сделай коммит", PromptFormat::Qwen);
        assert!(prompt.contains(QWEN_IM_START));
        assert!(prompt.contains(QWEN_IM_END));
        assert!(prompt.contains("сделай коммит"));
        assert!(prompt.ends_with(&format!("{QWEN_IM_START}assistant\n")));
        assert!(!prompt.contains("think"));
    }

    #[test]
    fn sanitize_input_drops_commas_and_collapses_spaces() {
        assert_eq!(
            sanitize_input("Закамить изменения файл, Мэйн, точка Раст и создать тэг."),
            "Закамить изменения файл Мэйн точка Раст и создать тэг."
        );
        assert_eq!(
            sanitize_input("Глянь, пэкэдж, точка, Джейсон."),
            "Глянь пэкэдж точка Джейсон."
        );
    }

    #[test]
    fn qwen_prompt_has_no_commas() {
        let prompt = build_prompt("файл, Мэйн, точка Раст", PromptFormat::Qwen);
        assert!(!prompt.contains("Мэйн,"));
        assert!(prompt.contains("файл Мэйн точка Раст"));
    }

    #[test]
    fn qwen_strips_think_block_and_stop_token() {
        let raw = "<think>\n\n</think>\n\nсделай commit и push в master<|im_end|>".to_string();
        let out = strip_at_stop_token(raw, PromptFormat::Qwen);
        assert_eq!(out, "сделай commit и push в master");
    }

    #[test]
    fn gemma_prompt_uses_turn_markers() {
        let prompt = build_prompt("сделай коммит", PromptFormat::Gemma);
        assert!(prompt.contains("<start_of_turn>user"));
        assert!(prompt.contains("<start_of_turn>model"));
    }

    #[test]
    fn detects_qwen_from_path() {
        assert_eq!(
            prompt_format_for(Path::new("/models/qwen35-08b-norm/qwen35-08b-norm.gguf")),
            PromptFormat::Qwen
        );
        assert_eq!(
            prompt_format_for(Path::new("/models/gemma3-270m-norm/gemma3-270m-norm.gguf")),
            PromptFormat::Gemma
        );
    }

    #[test]
    fn normalizes_russian_dev_dictation_when_model_installed() {
        let home = std::env::var("HOME").expect("HOME");
        let models = [
            format!(
                "{home}/Library/Application Support/dev.speechclip.oss/models/qwen35-08b-norm/qwen35-08b-norm.gguf"
            ),
            format!(
                "{home}/Library/Application Support/dev.speechclip.oss/models/gemma3-270m-norm/gemma3-270m-norm.gguf"
            ),
        ];
        let model_path = models.iter().map(PathBuf::from).find(|p| p.is_file());
        let Some(model_path) = model_path else {
            eprintln!("Skipping: no normalizer GGUF installed");
            return;
        };

        let input = "сделай коммит и запуш в мастер";
        let output = normalize_with_model(&model_path, input).expect("normalize");
        let lower = output.to_lowercase();
        assert!(
            lower.contains("commit"),
            "expected at least one normalized tech term, got: {output}"
        );
        assert_ne!(
            output, input,
            "expected model to change input, got unchanged text"
        );
    }
}
