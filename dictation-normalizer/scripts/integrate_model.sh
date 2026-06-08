#!/usr/bin/env bash
# integrate_model.sh — Convert and install a trained dictation-normalizer
# (Qwen3.5-0.8B by default) into the running SpeechClip Tauri app.
#
# Usage:
#   ./integrate_model.sh [INPUT]
#
# INPUT can be:
#   - Path to a merged HF checkpoint zip   (*.zip)  → unzip → convert → quantize → install
#   - Path to an extracted HF checkpoint dir        → convert → quantize → install
#   - Path to an f16 GGUF file             (*f16*)  → quantize → install
#   - Path to a Q4_K_M GGUF file                    → install only
#   - (omitted) Auto-detect newest ~/Downloads/*merged*.zip
#
# Environment overrides:
#   QUANT              Quantization type (default: Q4_K_M)
#   LLAMA_CPP_DIR      Path to llama.cpp checkout (default: /private/tmp/llama.cpp)
#   PYTHON             Python executable to use
#
set -euo pipefail

# ── Constants (must match normalizer_install.rs) ──────────────────────────────
NORMALIZER_MODEL_DIR="${NORMALIZER_MODEL_DIR:-qwen35-08b-norm}"
NORMALIZER_GGUF_FILENAME="${NORMALIZER_GGUF_FILENAME:-qwen35-08b-norm.gguf}"
APP_SUPPORT_MODELS="${HOME}/Library/Application Support/dev.speechclip.oss/models"
INSTALLED_DEST="${APP_SUPPORT_MODELS}/${NORMALIZER_MODEL_DIR}/${NORMALIZER_GGUF_FILENAME}"
DOWNLOADS_DEST="${HOME}/Downloads/${NORMALIZER_MODEL_DIR}/${NORMALIZER_GGUF_FILENAME}"

QUANT="${QUANT:-Q4_K_M}"
TMP_F16="/tmp/${NORMALIZER_MODEL_DIR}-f16.gguf"
TMP_QUANT="/tmp/${NORMALIZER_MODEL_DIR}-${QUANT,,}.gguf"

# ── Helpers ───────────────────────────────────────────────────────────────────
log()  { echo "[integrate] $*"; }
ok()   { echo "[integrate] ✓ $*"; }
die()  { echo "[integrate] ERROR: $*" >&2; exit 1; }
hr()   { echo "────────────────────────────────────────────────────────────────"; }

require_cmd() {
    command -v "$1" &>/dev/null || die "Required command '$1' not found. $2"
}

human_size() {
    local f="$1"
    if command -v gdu &>/dev/null; then gdu -sh "$f" | awk '{print $1}'
    elif command -v du &>/dev/null; then du -sh "$f" | awk '{print $1}'
    else echo "?"; fi
}

# ── Locate llama.cpp ──────────────────────────────────────────────────────────
find_llama_cpp() {
    local candidates=(
        "${LLAMA_CPP_DIR:-}"
        /private/tmp/llama.cpp
        /tmp/llama.cpp
        "${HOME}/llama.cpp"
        /opt/llama.cpp
    )
    for d in "${candidates[@]}"; do
        [[ -z "$d" ]] && continue
        if [[ -f "${d}/convert_hf_to_gguf.py" ]]; then
            echo "$d"
            return 0
        fi
    done
    return 1
}

find_llama_quantize() {
    local llama_dir="$1"
    local candidates=(
        "${llama_dir}/build/bin/llama-quantize"
        "${llama_dir}/build/bin/quantize"
        "${llama_dir}/llama-quantize"
    )
    for f in "${candidates[@]}"; do
        [[ -x "$f" ]] && echo "$f" && return 0
    done
    # Also try PATH
    if command -v llama-quantize &>/dev/null; then
        command -v llama-quantize
        return 0
    fi
    return 1
}

# ── Locate Python with torch + transformers ───────────────────────────────────
find_python() {
    local candidates=(
        "${PYTHON:-}"
        python3
        /opt/homebrew/bin/python3
        /usr/local/bin/python3
    )
    for py in "${candidates[@]}"; do
        [[ -z "$py" ]] && continue
        if "$py" -c "import torch, transformers" &>/dev/null 2>&1; then
            echo "$py"
            return 0
        fi
    done
    return 1
}

ensure_gguf_python_pkg() {
    local py="$1"
    if ! "$py" -c "import gguf" &>/dev/null 2>&1; then
        log "gguf package not found — installing via pip…"
        "$py" -m pip install --quiet gguf || die "Failed to install gguf Python package."
        ok "gguf installed"
    fi
}

# ── Stage: detect input type ──────────────────────────────────────────────────
detect_input() {
    local input="$1"
    if [[ "$input" == *.zip ]]; then
        echo "zip"
    elif [[ -d "$input" ]]; then
        # HF checkpoint dir must contain config.json
        [[ -f "${input}/config.json" ]] || die "Directory '$input' does not look like an HF checkpoint (no config.json)."
        echo "hf_dir"
    elif [[ -f "$input" && "$input" == *.gguf ]]; then
        # Guess quantization level from filename
        if [[ "$input" == *f16* ]]; then
            echo "gguf_f16"
        else
            echo "gguf_final"
        fi
    else
        die "Cannot determine input type for: $input"
    fi
}

autodetect_zip() {
    # Find the newest *merged*.zip in ~/Downloads
    local newest
    newest=$(find "${HOME}/Downloads" -maxdepth 1 -name '*merged*.zip' -print0 2>/dev/null \
        | xargs -0 ls -t 2>/dev/null | head -1)
    [[ -n "$newest" ]] || die "No *merged*.zip found in ~/Downloads. Pass an explicit INPUT path."
    echo "$newest"
}

# ── Pipeline stages ───────────────────────────────────────────────────────────
stage_unzip() {
    local zip="$1"
    local out_dir="/tmp/${NORMALIZER_MODEL_DIR}-hf"
    log "Unzipping ${zip} → ${out_dir}…"
    rm -rf "$out_dir"
    mkdir -p "$out_dir"
    require_cmd unzip "Install Xcode Command Line Tools or brew install unzip."
    unzip -q "$zip" -d "$out_dir"
    # If the zip extracted a single subdirectory, return that
    local contents
    contents=$(find "$out_dir" -maxdepth 1 -mindepth 1 -type d)
    local count
    count=$(echo "$contents" | grep -c . || true)
    if [[ "$count" -eq 1 ]]; then
        echo "$contents"
    else
        echo "$out_dir"
    fi
}

stage_convert() {
    local hf_dir="$1"
    local llama_dir="$2"
    local py="$3"
    log "Converting HF checkpoint → GGUF f16…"
    log "  source : ${hf_dir}"
    log "  output : ${TMP_F16}"
    ensure_gguf_python_pkg "$py"
    # Qwen3.5 carries a multi-token-prediction (MTP) head the normalizer does not
    # need; --no-mtp drops it (flag only valid for Qwen3.5/3.6 text variants).
    local mtp_flag=()
    if [[ "$NORMALIZER_MODEL_DIR" == *qwen* ]]; then
        mtp_flag=(--no-mtp)
    fi
    "$py" "${llama_dir}/convert_hf_to_gguf.py" \
        "$hf_dir" \
        --outfile "$TMP_F16" \
        --outtype f16 \
        "${mtp_flag[@]}"
    [[ -f "$TMP_F16" ]] || die "convert_hf_to_gguf.py finished but ${TMP_F16} not found."
    ok "Conversion done: $(human_size "$TMP_F16")"
    echo "$TMP_F16"
}

stage_quantize() {
    local f16="$1"
    local quantize_bin="$2"
    log "Quantizing ${f16} → ${TMP_QUANT} (${QUANT})…"
    "$quantize_bin" "$f16" "$TMP_QUANT" "$QUANT"
    [[ -f "$TMP_QUANT" ]] || die "llama-quantize finished but ${TMP_QUANT} not found."
    ok "Quantization done: $(human_size "$TMP_QUANT")"
    echo "$TMP_QUANT"
}

stage_install() {
    local gguf="$1"

    hr
    log "Installing GGUF to both locations…"

    # 1. App Support (overwrite even if exists — cache won't reload until restart)
    mkdir -p "$(dirname "$INSTALLED_DEST")"
    cp -f "$gguf" "$INSTALLED_DEST"
    ok "App Support : ${INSTALLED_DEST}"
    log "             size: $(human_size "$INSTALLED_DEST")"

    # 2. Downloads fallback (for normalizer_install.rs dev-source detection)
    mkdir -p "$(dirname "$DOWNLOADS_DEST")"
    cp -f "$gguf" "$DOWNLOADS_DEST"
    ok "Downloads   : ${DOWNLOADS_DEST}"
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
    hr
    log "SpeechClip — Dictation Normalizer Integration"
    hr

    # Resolve INPUT
    local input
    if [[ $# -ge 1 ]]; then
        input="$1"
        [[ -e "$input" ]] || die "Input path not found: $input"
    else
        log "No input given — auto-detecting newest *merged*.zip in ~/Downloads…"
        input=$(autodetect_zip)
        log "Found: ${input}"
    fi

    local kind
    kind=$(detect_input "$input")
    log "Input type detected: ${kind}"
    hr

    # Locate tools (only when needed)
    local llama_dir=""
    local quantize_bin=""
    local py=""

    if [[ "$kind" != "gguf_final" ]]; then
        # We'll need llama-quantize for quantization
        llama_dir=$(find_llama_cpp) || die \
            "llama.cpp directory not found. Clone it to /private/tmp/llama.cpp and build:\n" \
            "  git clone https://github.com/ggerganov/llama.cpp /private/tmp/llama.cpp\n" \
            "  cd /private/tmp/llama.cpp && cmake -B build && cmake --build build -j\n" \
            "Or set LLAMA_CPP_DIR=/path/to/llama.cpp"
        log "llama.cpp dir : ${llama_dir}"

        quantize_bin=$(find_llama_quantize "$llama_dir") || die \
            "llama-quantize binary not found in ${llama_dir}/build/bin/. " \
            "Build llama.cpp first: cd ${llama_dir} && cmake -B build && cmake --build build -j"
        log "llama-quantize: ${quantize_bin}"
    fi

    if [[ "$kind" == "zip" || "$kind" == "hf_dir" ]]; then
        py=$(find_python) || die \
            "No Python with torch+transformers found. " \
            "Try: pip3 install torch transformers\n" \
            "Or set PYTHON=/path/to/python3"
        log "python        : ${py} ($(${py} --version 2>&1))"
    fi

    hr

    # Run pipeline stages
    local gguf_f16=""
    local gguf_final=""

    case "$kind" in
        zip)
            local hf_dir
            hf_dir=$(stage_unzip "$input")
            ok "Extracted to: ${hf_dir}"
            gguf_f16=$(stage_convert "$hf_dir" "$llama_dir" "$py")
            gguf_final=$(stage_quantize "$gguf_f16" "$quantize_bin")
            ;;
        hf_dir)
            gguf_f16=$(stage_convert "$input" "$llama_dir" "$py")
            gguf_final=$(stage_quantize "$gguf_f16" "$quantize_bin")
            ;;
        gguf_f16)
            gguf_final=$(stage_quantize "$input" "$quantize_bin")
            ;;
        gguf_final)
            gguf_final="$input"
            log "Input is already a final GGUF — skipping conversion and quantization."
            ;;
    esac

    stage_install "$gguf_final"

    hr
    ok "Integration complete."
    echo ""
    echo "  Installed : ${INSTALLED_DEST}"
    echo "  Size      : $(human_size "$INSTALLED_DEST")"
    echo ""
    echo "  *** Restart the app (Cmd+Q then reopen) for the new model to take effect. ***"
    echo "  The normalizer caches the model in memory keyed by file path;"
    echo "  a full restart is required even though the path has not changed."
    hr
}

main "$@"
