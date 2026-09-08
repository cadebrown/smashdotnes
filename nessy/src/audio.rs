use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WavInspection {
    pub path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub samples: usize,
    pub peak: i16,
    pub mean_absolute_amplitude: f64,
}

pub fn inspect_pcm_wav(path: &Path) -> Result<WavInspection> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("{} is not a RIFF/WAVE file", path.display());
    }

    let mut offset = 12usize;
    let mut format = None;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start.saturating_add(size);
        if end > bytes.len() {
            bail!("{} contains a truncated WAV chunk", path.display());
        }
        if id == b"fmt " {
            format = Some(&bytes[start..end]);
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = end + (size & 1);
    }

    let format = format.context("WAV is missing fmt chunk")?;
    let data = data.context("WAV is missing data chunk")?;
    if format.len() < 16 {
        bail!("WAV fmt chunk is too small");
    }
    let encoding = u16::from_le_bytes([format[0], format[1]]);
    let channels = u16::from_le_bytes([format[2], format[3]]);
    let sample_rate = u32::from_le_bytes(format[4..8].try_into().unwrap());
    let bits_per_sample = u16::from_le_bytes([format[14], format[15]]);
    if encoding != 1 || bits_per_sample != 16 || channels == 0 {
        bail!("WAV must be 16-bit PCM with at least one channel");
    }

    let mut peak = 0i16;
    let mut absolute_sum = 0u64;
    let mut samples = 0usize;
    for chunk in data.chunks_exact(2) {
        let value = i16::from_le_bytes([chunk[0], chunk[1]]);
        let absolute = value.unsigned_abs();
        peak = peak.max(absolute.min(i16::MAX as u16) as i16);
        absolute_sum += absolute as u64;
        samples += 1;
    }
    let mean_absolute_amplitude = absolute_sum as f64 / samples.max(1) as f64;
    if samples < sample_rate as usize || peak < 256 || mean_absolute_amplitude < 20.0 {
        bail!(
            "WAV is silent or too short (samples={}, peak={}, mean={:.1})",
            samples,
            peak,
            mean_absolute_amplitude
        );
    }
    if peak == i16::MAX {
        bail!("WAV reaches full-scale clipping");
    }

    Ok(WavInspection {
        path: path.display().to_string(),
        sample_rate,
        channels,
        bits_per_sample,
        samples,
        peak,
        mean_absolute_amplitude,
    })
}
