# Integrating a New Normalizer Model

Take a fine-tuned **Qwen3.5-0.8B** checkpoint from Colab and install it into SpeechClip.

## Pipeline

```
Colab export (qwen35_08b_norm_merged.zip)
  └─▶ HF checkpoint dir   (unzip)
        └─▶ GGUF f16      (convert_hf_to_gguf.py)
              └─▶ GGUF Q4_K_M  (llama-quantize)
                    └─▶ install to App Support + Downloads
```

The app loads the model via `llama-cpp-4` (Metal). Installed path:

```
~/Library/Application Support/dev.speechclip.oss/models/qwen35-08b-norm/qwen35-08b-norm.gguf
```

Legacy Gemma installs are still detected as fallback:

```
~/Library/Application Support/dev.speechclip.oss/models/gemma3-270m-norm/gemma3-270m-norm.gguf
```

## Prerequisites

**llama.cpp** — clone and build once:

```sh
git clone https://github.com/ggerganov/llama.cpp /private/tmp/llama.cpp
cd /private/tmp/llama.cpp
cmake -B build && cmake --build build -j
```

**Python deps** — `torch` and `transformers>=5.2` for Qwen3.5 conversion; `gguf` is installed by the script if missing:

```sh
pip3 install torch "transformers>=5.2.0"
```

## Usage

```sh
# Auto-detect newest *merged*.zip in ~/Downloads:
./dictation-normalizer/scripts/integrate_model.sh

# Explicit zip from Colab:
./dictation-normalizer/scripts/integrate_model.sh ~/Downloads/qwen35_08b_norm_merged.zip

# Already-extracted HF dir:
./dictation-normalizer/scripts/integrate_model.sh /tmp/qwen35-08b-norm-hf

# Skip to quantize (already have f16 GGUF):
./dictation-normalizer/scripts/integrate_model.sh /tmp/qwen35-08b-norm-f16.gguf

# Skip to install (already have final GGUF):
./dictation-normalizer/scripts/integrate_model.sh /tmp/qwen35-08b-norm-q4_k_m.gguf
```

### Environment overrides

| Variable | Default | Purpose |
|---|---|---|
| `QUANT` | `Q4_K_M` | llama.cpp quantization type |
| `LLAMA_CPP_DIR` | `/private/tmp/llama.cpp` | Path to llama.cpp checkout |
| `PYTHON` | auto-detected | Python with torch+transformers |
| `SPEECHCLIP_NORMALIZER_GGUF` | — | Direct path to GGUF (dev override) |

## App integration (already wired)

| Component | Role |
|---|---|
| `normalizer.rs` | ChatML prompt for Qwen, Gemma fallback for legacy |
| `normalizer_install.rs` | Prefers `qwen35-08b-norm.gguf`, copies from `~/Downloads` |
| `commands.rs` | `normalize_text` in ASR pipeline when `dictation_normalize` is on |
| `integrate_model.sh` | HF → GGUF → install |

Tauri commands for the dashboard:

- `get_normalizer_status` — is the GGUF installed? which model?
- `ensure_normalizer_model` — copy from dev paths if missing

## Restart required

After install, **fully quit and reopen the app (Cmd+Q)**.

The normalizer caches the loaded model in memory. A hot-reload is not possible — restart is required.

## Quick smoke test (terminal)

```sh
cd src-tauri
cargo test normalizer::tests::qwen_prompt_uses_chatml_markers -- --nocapture
cargo test normalizer::tests::normalizes_russian_dev_dictation_when_model_installed -- --nocapture
```
