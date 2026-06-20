//! Decode browser WAV payloads and resample to 16 kHz mono f32 (Parakeet input).

use hound::{SampleFormat, WavReader, WavSpec};
use rubato::{FftFixedIn, Resampler};
use std::io::Cursor;

pub const PARAKEET_SAMPLE_RATE: u32 = 16000;
/// ~0.15s at 16 kHz — shorter clips are treated as accidental taps.
pub const MIN_AUDIO_SAMPLES: usize = 2400;

pub fn samples_too_short(sample_count: usize) -> bool {
    sample_count < MIN_AUDIO_SAMPLES
}

pub fn decode_wav_bytes(data: &[u8]) -> Result<(Vec<f32>, u32), String> {
    let reader =
        WavReader::new(Cursor::new(data)).map_err(|e| format!("Invalid WAV audio: {e}"))?;
    let spec = reader.spec();
    let samples = read_samples(reader, &spec)?;
    Ok((samples, spec.sample_rate))
}

pub fn decode_pcm_f32le_16k(data: &[u8]) -> Result<Vec<f32>, String> {
    if !data.len().is_multiple_of(std::mem::size_of::<f32>()) {
        return Err("Invalid PCM audio: byte length is not divisible by 4".to_string());
    }

    let mut samples = Vec::with_capacity(data.len() / std::mem::size_of::<f32>());
    for chunk in data.chunks_exact(std::mem::size_of::<f32>()) {
        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if !sample.is_finite() {
            return Err("Invalid PCM audio: non-finite sample".to_string());
        }
        samples.push(sample.clamp(-1.0, 1.0));
    }
    Ok(samples)
}

pub fn resample_to_16k(samples: &[f32], sample_rate: u32) -> Result<Vec<f32>, String> {
    if sample_rate == PARAKEET_SAMPLE_RATE {
        return Ok(samples.to_vec());
    }

    let in_hz = sample_rate as usize;
    let out_hz = PARAKEET_SAMPLE_RATE as usize;
    let chunk_size = 1024;

    let mut resampler = FftFixedIn::<f32>::new(in_hz, out_hz, chunk_size, 1, 1)
        .map_err(|e| format!("Audio resampler init failed: {e}"))?;

    let mut output = Vec::with_capacity(samples.len() * out_hz / in_hz + chunk_size);
    let mut offset = 0;

    while offset < samples.len() {
        let end = (offset + chunk_size).min(samples.len());
        let mut chunk = samples[offset..end].to_vec();
        if chunk.len() < chunk_size {
            chunk.resize(chunk_size, 0.0);
        }

        let out = resampler
            .process(&[&chunk], None)
            .map_err(|e| format!("Audio resample failed: {e}"))?;

        let produced = out[0].len();
        let valid = if end == samples.len() {
            // Last chunk: trim zero-padded tail proportionally.
            let input_used = end - offset;
            ((input_used as f64 / chunk_size as f64) * produced as f64).round() as usize
        } else {
            produced
        };

        output.extend_from_slice(&out[0][..valid.min(produced)]);
        offset = end;
    }

    Ok(output)
}

fn read_samples(mut reader: WavReader<Cursor<&[u8]>>, spec: &WavSpec) -> Result<Vec<f32>, String> {
    match (spec.channels, spec.sample_format) {
        (1, SampleFormat::Int) => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read WAV samples: {e}")),
        (2, SampleFormat::Int) => {
            let interleaved = reader
                .samples::<i16>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to read WAV samples: {e}"))?;
            let mut mono = Vec::with_capacity(interleaved.len() / 2);
            for pair in interleaved.chunks(2) {
                let left = pair.first().copied().unwrap_or(0) as f32 / i16::MAX as f32;
                let right = pair.get(1).copied().unwrap_or(0) as f32 / i16::MAX as f32;
                mono.push((left + right) * 0.5);
            }
            Ok(mono)
        }
        (1, SampleFormat::Float) => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read WAV samples: {e}")),
        (channels, format) => Err(format!(
            "Unsupported WAV format ({channels} channels, {format:?}). Use mono 16-bit PCM."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_pcm_f32le_16k_reads_little_endian_samples() {
        let input = [0.0_f32, 0.5, -0.25, 1.25, -1.25];
        let bytes = input
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect::<Vec<_>>();

        let samples = decode_pcm_f32le_16k(&bytes).expect("decode pcm");

        assert_eq!(samples, vec![0.0, 0.5, -0.25, 1.0, -1.0]);
    }

    #[test]
    fn decode_pcm_f32le_16k_rejects_partial_sample() {
        let err = decode_pcm_f32le_16k(&[0, 1, 2]).expect_err("partial sample");

        assert!(err.contains("divisible by 4"));
    }

    #[test]
    fn decode_pcm_f32le_16k_rejects_non_finite_samples() {
        let bytes = f32::NAN.to_le_bytes();
        let err = decode_pcm_f32le_16k(&bytes).expect_err("nan sample");

        assert!(err.contains("non-finite"));
    }
}
