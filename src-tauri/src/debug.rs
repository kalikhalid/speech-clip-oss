use std::fs;
use std::io::Write;

// Save debug audio files (dev only)
pub async fn save_debug_audio(
    _original_audio: Vec<u8>,
    _processed_audio: Vec<u8>,
    _timestamp: String,
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        let log_dir = dirs::document_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("SpeechClipLogs");

        fs::create_dir_all(&log_dir).map_err(|e| format!("Failed to create log dir: {}", e))?;

        // Save original audio
        let original_path = log_dir.join(format!("original_{}.webm", _timestamp));
        let mut original_file = fs::File::create(&original_path)
            .map_err(|e| format!("Failed to create original file: {}", e))?;
        original_file
            .write_all(&_original_audio)
            .map_err(|e| format!("Failed to write original audio: {}", e))?;

        // Save VAD-processed audio
        let processed_path = log_dir.join(format!("vad_processed_{}.webm", _timestamp));
        let mut processed_file = fs::File::create(&processed_path)
            .map_err(|e| format!("Failed to create processed file: {}", e))?;
        processed_file
            .write_all(&_processed_audio)
            .map_err(|e| format!("Failed to write processed audio: {}", e))?;

        #[cfg(debug_assertions)]
        println!(
            "✓ Saved debug audio: {:?} and {:?}",
            original_path, processed_path
        );
    }

    Ok(())
}

// Save raw WAV audio bytes (dev only) - no re-encoding needed
pub fn save_debug_wav_raw(wav_data: &[u8], filename: &str) -> Result<(), String> {
    let log_dir = dirs::document_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("SpeechClipLogs");

    fs::create_dir_all(&log_dir).map_err(|e| format!("Failed to create log dir: {}", e))?;

    let path = log_dir.join(format!("{}.wav", filename));
    fs::write(&path, wav_data).map_err(|e| format!("Failed to write WAV: {}", e))?;

    #[cfg(debug_assertions)]
    println!("💾 Saved debug audio: {:?}", path);
    Ok(())
}
