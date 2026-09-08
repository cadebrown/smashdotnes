use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

const NTSC_CPU_HZ: f64 = 1_789_773.0;
const PERIOD_REST: u16 = 0xffff;
const PERIOD_HOLD: u16 = 0xfffe;

#[derive(Debug, Deserialize)]
struct MusicSpec {
    timing: Timing,
    song: Vec<Song>,
}

#[derive(Debug, Deserialize)]
struct Timing {
    ticks_per_beat: u32,
    ntsc_frame_rate_num: u64,
    ntsc_frame_rate_den: u64,
}

#[derive(Debug, Deserialize)]
struct Song {
    id: String,
    loop_order: usize,
    tempo_millibpm: u32,
    pulse1_duty: Option<u8>,
    pulse1_volume: Option<u8>,
    pulse2_duty: Option<u8>,
    pulse2_volume: Option<u8>,
    order: Vec<String>,
    pattern: Vec<Pattern>,
}

#[derive(Debug, Deserialize)]
struct Pattern {
    id: String,
    ticks: Vec<u16>,
    pulse1: Vec<String>,
    pulse2: Vec<String>,
    triangle: Vec<String>,
    noise: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicGenerationResult {
    pub source: PathBuf,
    pub output: PathBuf,
    pub songs: usize,
    pub steps: usize,
}

pub fn generate_music_data(workspace_root: &Path) -> Result<MusicGenerationResult> {
    let source = workspace_root.join("nessy/assets/music.toml");
    let output = workspace_root.join("src/generated/music_data.h");
    let raw = if source.exists() {
        fs::read_to_string(&source)
            .with_context(|| format!("failed to read {}", source.display()))?
    } else {
        include_str!("../assets/music.toml").to_string()
    };
    let spec: MusicSpec =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", source.display()))?;
    let (generated, steps) = compile_music(&spec)?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let should_write = fs::read_to_string(&output).ok().as_deref() != Some(generated.as_str());
    if should_write {
        fs::write(&output, generated)
            .with_context(|| format!("failed to write {}", output.display()))?;
    }

    Ok(MusicGenerationResult {
        source,
        output,
        songs: spec.song.len(),
        steps,
    })
}

fn compile_music(spec: &MusicSpec) -> Result<(String, usize)> {
    if spec.timing.ticks_per_beat == 0 {
        bail!("timing.ticks_per_beat must be greater than zero");
    }
    if spec.timing.ntsc_frame_rate_num == 0 || spec.timing.ntsc_frame_rate_den == 0 {
        bail!("timing NTSC frame-rate terms must be greater than zero");
    }
    if spec.song.is_empty() {
        bail!("music spec must define at least one song");
    }

    let mut output = String::from("#ifndef SMASH_MUSIC_DATA_H\n#define SMASH_MUSIC_DATA_H\n\n");
    output.push_str("#define MUSIC_PERIOD_REST 0xFFFFu\n");
    output.push_str("#define MUSIC_PERIOD_HOLD 0xFFFEu\n\n");
    output.push_str("typedef struct {\n");
    output.push_str("    unsigned int pulse1_period;\n");
    output.push_str("    unsigned int pulse2_period;\n");
    output.push_str("    unsigned int triangle_period;\n");
    output.push_str("    unsigned char noise;\n");
    output.push_str("    unsigned char frames;\n");
    output.push_str("} MusicStep;\n\n");

    let mut total_steps = 0;
    for song in &spec.song {
        validate_identifier(&song.id)?;
        if song.order.is_empty() {
            bail!("song '{}' has an empty order", song.id);
        }
        if song.loop_order >= song.order.len() {
            bail!(
                "song '{}' loop_order {} is outside {} order entries",
                song.id,
                song.loop_order,
                song.order.len()
            );
        }

        if song.tempo_millibpm == 0 {
            bail!("song '{}' has zero tempo", song.id);
        }
        let pulse1_ctrl = pulse_control(
            &song.id,
            "pulse1",
            song.pulse1_duty.unwrap_or(2),
            song.pulse1_volume.unwrap_or(10),
        )?;
        let pulse2_ctrl = pulse_control(
            &song.id,
            "pulse2",
            song.pulse2_duty.unwrap_or(1),
            song.pulse2_volume.unwrap_or(6),
        )?;

        for (index, pattern) in song.pattern.iter().enumerate() {
            validate_identifier(&pattern.id)?;
            if song.pattern[..index]
                .iter()
                .any(|other| other.id == pattern.id)
            {
                bail!("song '{}' repeats pattern id '{}'", song.id, pattern.id);
            }
            validate_pattern(&song.id, pattern)?;
            if !song.order.iter().any(|id| id == &pattern.id) {
                bail!(
                    "song '{}' pattern '{}' is never used in its order",
                    song.id,
                    pattern.id
                );
            }
        }

        let mut expanded = Vec::new();
        let mut loop_step = 0;
        for (order_index, pattern_id) in song.order.iter().enumerate() {
            if order_index == song.loop_order {
                loop_step = expanded.len();
            }
            let pattern = song
                .pattern
                .iter()
                .find(|pattern| pattern.id == *pattern_id)
                .with_context(|| {
                    format!(
                        "song '{}' order references unknown pattern '{}'",
                        song.id, pattern_id
                    )
                })?;
            for index in 0..pattern.pulse1.len() {
                expanded.push((
                    note_period(&pattern.pulse1[index], 16.0)?,
                    note_period(&pattern.pulse2[index], 16.0)?,
                    note_period(&pattern.triangle[index], 32.0)?,
                    noise_code(&pattern.noise[index])?,
                    pattern.ticks[index],
                ));
            }
        }
        if expanded.len() > u16::MAX as usize {
            bail!(
                "song '{}' expands to {} steps; the runtime limit is 65535",
                song.id,
                expanded.len()
            );
        }

        let frame_numerator = 60u128 * 1000 * u128::from(spec.timing.ntsc_frame_rate_num);
        let frame_denominator = u128::from(spec.timing.ticks_per_beat)
            * u128::from(song.tempo_millibpm)
            * u128::from(spec.timing.ntsc_frame_rate_den);
        let mut cumulative_ticks = 0u128;
        let mut cumulative_frames = 0u128;
        let mut compiled = Vec::with_capacity(expanded.len());
        for (index, &(pulse1, pulse2, triangle, noise, ticks)) in expanded.iter().enumerate() {
            cumulative_ticks += u128::from(ticks);
            let next_frames = rounded_div(cumulative_ticks * frame_numerator, frame_denominator);
            let frames = next_frames.saturating_sub(cumulative_frames);
            if frames == 0 || frames > u8::MAX.into() {
                bail!(
                    "song '{}' step {} resolves to {} frames",
                    song.id,
                    index,
                    frames
                );
            }
            compiled.push((pulse1, pulse2, triangle, noise, frames as u8));
            cumulative_frames = next_frames;
        }
        let loop_frame = compiled[..loop_step]
            .iter()
            .map(|step| u32::from(step.4))
            .sum::<u32>();
        let total_frames = compiled.iter().map(|step| u32::from(step.4)).sum::<u32>();

        let upper = song.id.to_ascii_uppercase();
        output.push_str(&format!("static const MusicStep MUSIC_{upper}[] = {{\n"));
        for (pulse1, pulse2, triangle, noise, frames) in &compiled {
            output.push_str(&format!(
                "    {{ 0x{pulse1:04X}u, 0x{pulse2:04X}u, 0x{triangle:04X}u, {noise}, {frames} }},\n"
            ));
        }
        output.push_str("};\n");
        output.push_str(&format!("#define MUSIC_{upper}_LEN {}\n", expanded.len()));
        output.push_str(&format!("#define MUSIC_{upper}_LOOP {loop_step}\n"));
        output.push_str(&format!(
            "#define MUSIC_{upper}_TOTAL_FRAMES {total_frames}\n"
        ));
        output.push_str(&format!(
            "#define MUSIC_{upper}_LOOP_FRAMES {}\n",
            total_frames - loop_frame
        ));
        output.push_str(&format!(
            "#define MUSIC_{upper}_PULSE1_CTRL 0x{pulse1_ctrl:02X}\n"
        ));
        output.push_str(&format!(
            "#define MUSIC_{upper}_PULSE2_CTRL 0x{pulse2_ctrl:02X}\n\n"
        ));
        total_steps += expanded.len();
    }

    output.push_str("#endif\n");
    Ok((output, total_steps))
}

fn validate_pattern(song_id: &str, pattern: &Pattern) -> Result<()> {
    let length = pattern.pulse1.len();
    if length == 0 {
        bail!("song '{song_id}' pattern '{}' is empty", pattern.id);
    }
    for (voice, voice_length) in [
        ("ticks", pattern.ticks.len()),
        ("pulse2", pattern.pulse2.len()),
        ("triangle", pattern.triangle.len()),
        ("noise", pattern.noise.len()),
    ] {
        if voice_length != length {
            bail!(
                "song '{song_id}' pattern '{}' has {length} pulse1 steps but {voice_length} {voice} steps",
                pattern.id
            );
        }
    }
    if pattern.ticks.contains(&0) {
        bail!(
            "song '{song_id}' pattern '{}' has a zero-tick step",
            pattern.id
        );
    }
    Ok(())
}

fn rounded_div(numerator: u128, denominator: u128) -> u128 {
    (numerator + denominator / 2) / denominator
}

fn pulse_control(song_id: &str, voice: &str, duty: u8, volume: u8) -> Result<u8> {
    if duty > 3 {
        bail!("song '{song_id}' {voice}_duty must be between 0 and 3");
    }
    if volume > 15 {
        bail!("song '{song_id}' {voice}_volume must be between 0 and 15");
    }
    Ok((duty << 6) | 0x30 | volume)
}

fn validate_identifier(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        || id.as_bytes()[0].is_ascii_digit()
    {
        bail!("song id '{id}' must be a lowercase C identifier");
    }
    Ok(())
}

fn noise_code(value: &str) -> Result<u8> {
    match value {
        "rest" => Ok(0),
        "kick" => Ok(1),
        "snare" => Ok(2),
        "hat" => Ok(3),
        "hold" => Ok(4),
        _ => bail!("unknown noise voice '{value}'"),
    }
}

fn note_period(note: &str, divisor: f64) -> Result<u16> {
    match note {
        "-" => return Ok(PERIOD_REST),
        "=" => return Ok(PERIOD_HOLD),
        _ => {}
    }

    let bytes = note.as_bytes();
    if !(bytes.len() == 2 || bytes.len() == 3) {
        bail!("invalid note '{note}'");
    }
    let octave = bytes[bytes.len() - 1];
    if !octave.is_ascii_digit() {
        bail!("invalid octave in note '{note}'");
    }
    let accidental = if bytes.len() == 3 { bytes[1] } else { 0 };
    let base = match bytes[0].to_ascii_uppercase() {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => bail!("invalid pitch in note '{note}'"),
    };
    let offset = match accidental {
        0 => 0,
        b'#' => 1,
        b'b' => -1,
        _ => bail!("invalid accidental in note '{note}'"),
    };
    let midi = 12 * ((octave - b'0') as i32 + 1) + base + offset;
    let frequency = 440.0 * 2.0f64.powf((midi as f64 - 69.0) / 12.0);
    let period = (NTSC_CPU_HZ / (divisor * frequency) - 1.0).round() as i32;
    if !(2..=2047).contains(&period) {
        bail!("note '{note}' is outside the NES timer range");
    }
    Ok(period as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_periods_match_known_ntsc_values() {
        assert_eq!(note_period("A4", 16.0).unwrap(), 253);
        assert_eq!(note_period("A4", 32.0).unwrap(), 126);
        assert_eq!(note_period("-", 16.0).unwrap(), PERIOD_REST);
        assert_eq!(note_period("=", 16.0).unwrap(), PERIOD_HOLD);
    }

    #[test]
    fn bundled_score_keeps_the_complete_menu_loop() {
        let spec: MusicSpec = toml::from_str(include_str!("../assets/music.toml")).unwrap();
        let (output, steps) = compile_music(&spec).unwrap();
        let song = &spec.song[0];
        let flattened = song
            .order
            .iter()
            .flat_map(|pattern_id| {
                let pattern = song
                    .pattern
                    .iter()
                    .find(|pattern| pattern.id == *pattern_id)
                    .unwrap();
                pattern
                    .pulse1
                    .iter()
                    .zip(&pattern.pulse2)
                    .map(|(pulse1, pulse2)| (pulse1.as_str(), pulse2.as_str()))
            })
            .collect::<Vec<_>>();
        assert_eq!(steps, 926);
        assert!(output.contains("MUSIC_MENU_LEN 926"));
        assert!(output.contains("MUSIC_MENU_LOOP 124"));
        assert!(output.contains("MUSIC_MENU_TOTAL_FRAMES 3428"));
        assert!(output.contains("MUSIC_MENU_LOOP_FRAMES 2995"));
        for index in [580, 586, 589] {
            assert_eq!(flattened[index], ("=", "="));
        }
        assert_eq!(flattened[585].1, "G3");
    }

    #[test]
    fn expands_patterns_and_emits_scene_timbre() {
        let valid: MusicSpec = toml::from_str(
            r#"
                [timing]
                ticks_per_beat = 480
                ntsc_frame_rate_num = 60
                ntsc_frame_rate_den = 1
                [[song]]
                id = "title"
                loop_order = 1
                tempo_millibpm = 120000
                pulse1_duty = 2
                pulse1_volume = 12
                pulse2_duty = 1
                pulse2_volume = 7
                order = ["intro", "loop", "loop"]
                [[song.pattern]]
                id = "intro"
                ticks = [480]
                pulse1 = ["A4"]
                pulse2 = ["-"]
                triangle = ["A3"]
                noise = ["kick"]
                [[song.pattern]]
                id = "loop"
                ticks = [240, 240]
                pulse1 = ["D5", "="]
                pulse2 = ["A4", "="]
                triangle = ["D3", "="]
                noise = ["snare", "hat"]
            "#,
        )
        .unwrap();
        let (output, steps) = compile_music(&valid).unwrap();
        assert_eq!(steps, 5);
        assert!(output.contains("MUSIC_TITLE_LEN 5"));
        assert!(output.contains("MUSIC_TITLE_LOOP 1"));
        assert!(output.contains("MUSIC_TITLE_PULSE1_CTRL 0xBC"));
        assert!(output.contains("MUSIC_TITLE_PULSE2_CTRL 0x77"));
        assert!(output.contains("0x00FDu"));
        assert!(output.contains(", 30 }"));
        assert!(output.contains("MUSIC_TITLE_TOTAL_FRAMES 90"));
        assert!(output.contains("MUSIC_TITLE_LOOP_FRAMES 60"));
    }

    #[test]
    fn rejects_malformed_patterns_and_controls() {
        let invalid_pattern: MusicSpec = toml::from_str(
            r#"
                [timing]
                ticks_per_beat = 480
                ntsc_frame_rate_num = 60
                ntsc_frame_rate_den = 1
                [[song]]
                id = "battle"
                loop_order = 0
                tempo_millibpm = 120000
                order = ["riff"]
                [[song.pattern]]
                id = "riff"
                ticks = [240, 240]
                pulse1 = ["A4", "D5"]
                pulse2 = ["A3"]
                triangle = ["A2", "D3"]
                noise = ["kick", "snare"]
            "#,
        )
        .unwrap();
        assert!(compile_music(&invalid_pattern)
            .unwrap_err()
            .to_string()
            .contains("pulse2"));

        let invalid_control: MusicSpec = toml::from_str(
            r#"
                [timing]
                ticks_per_beat = 480
                ntsc_frame_rate_num = 60
                ntsc_frame_rate_den = 1
                [[song]]
                id = "battle"
                loop_order = 0
                tempo_millibpm = 120000
                pulse1_duty = 4
                order = ["riff"]
                [[song.pattern]]
                id = "riff"
                ticks = [480]
                pulse1 = ["A4"]
                pulse2 = ["A3"]
                triangle = ["A2"]
                noise = ["kick"]
            "#,
        )
        .unwrap();
        assert!(compile_music(&invalid_control)
            .unwrap_err()
            .to_string()
            .contains("pulse1_duty"));
    }
}
