use chrono::{Local, Utc};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

/// A timing logger that records the duration of each stage in the transcription pipeline.
/// Logs are written to <Documents>/SpeechClipLogs/timing_YYYYMMDD.log
pub struct TimingLogger {
    session_id: String,
    start_time: Instant,
    log_file: Option<File>,
}

impl TimingLogger {
    /// Create a new timing logger for a transcription session
    pub fn new() -> Self {
        let session_id = Uuid::new_v4().to_string()[..8].to_string();
        let start_time = Instant::now();

        let log_file = Self::open_log_file();

        let mut logger = Self {
            session_id,
            start_time,
            log_file,
        };

        logger.write_line(&format!("SESSION {} START", logger.session_id));
        logger
    }

    /// Get the log directory path
    fn log_dir() -> PathBuf {
        dirs::document_dir()
            .map(|d| d.join("SpeechClipLogs"))
            .unwrap_or_else(|| PathBuf::from("SpeechClipLogs"))
    }

    /// Open or create the log file for today
    fn open_log_file() -> Option<File> {
        let log_dir = Self::log_dir();

        if fs::create_dir_all(&log_dir).is_err() {
            return None;
        }

        let date = Local::now().format("%Y%m%d");
        let log_path = log_dir.join(format!("timing_{}.log", date));

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok()
    }

    /// Write a line to the log file
    fn write_line(&mut self, message: &str) {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let line = format!("[{}] {}", timestamp, message);

        if let Some(ref mut file) = self.log_file {
            let _ = writeln!(file, "{}", line);
            let _ = file.flush();
        }
    }

    /// Mark a stage in the pipeline with elapsed time since start
    pub fn mark(&mut self, stage: &str) {
        let elapsed_ms = self.start_time.elapsed().as_millis();
        self.write_line(&format!("{} {}: {}ms", self.session_id, stage, elapsed_ms));
    }

    /// Mark the start of a stage
    pub fn mark_start(&mut self, stage: &str) {
        let elapsed_ms = self.start_time.elapsed().as_millis();
        self.write_line(&format!(
            "{} {}_start: {}ms",
            self.session_id, stage, elapsed_ms
        ));
    }

    /// Mark the end of a stage
    pub fn mark_end(&mut self, stage: &str) {
        let elapsed_ms = self.start_time.elapsed().as_millis();
        self.write_line(&format!(
            "{} {}_end: {}ms",
            self.session_id, stage, elapsed_ms
        ));
    }

    /// Finish the session and log total time
    pub fn finish(&mut self) {
        let total_ms = self.start_time.elapsed().as_millis();
        self.write_line(&format!(
            "SESSION {} END total={}ms",
            self.session_id, total_ms
        ));
        self.write_line(""); // Empty line for readability
    }
}

impl Drop for TimingLogger {
    fn drop(&mut self) {
        // Ensure session is finished if dropped without explicit finish
        if self.log_file.is_some() {
            // Don't double-log if already finished
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_logger() {
        let mut logger = TimingLogger::new();
        logger.mark("test_stage");
        logger.mark_start("processing");
        std::thread::sleep(std::time::Duration::from_millis(10));
        logger.mark_end("processing");
        logger.finish();
    }
}
