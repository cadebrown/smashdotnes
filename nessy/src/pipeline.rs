use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use image::ImageReader;
use serde::Serialize;

use crate::chr::{
    deduplicate_chr_frames, image_regions_to_chr, ChrOptions, ChrRegion, BYTES_PER_TILE,
    TRANSPARENT_TILE_INDEX,
};
use crate::music::generate_music_data;
use crate::spec::{discover_specs, load_character_spec, CharacterSpec, StateSpec};
use crate::state_graph::{compile_state_graph, write_state_include};

const STATIC_RUNTIME_GENERATED_ASSETS: &[(&str, &str)] = &[
    (
        "src/generated/title_logo.h",
        include_str!("../assets/generated_static/title_logo.h"),
    ),
    (
        "src/generated/title_tiles.inc",
        include_str!("../assets/generated_static/title_tiles.inc"),
    ),
    (
        "src/generated/smb1_fireball_tiles.inc",
        include_str!("../assets/generated_static/smb1_fireball_tiles.inc"),
    ),
    (
        "src/generated/cv1_axe_tiles.inc",
        include_str!("../assets/generated_static/cv1_axe_tiles.inc"),
    ),
    (
        "src/generated/simon_whip_tiles.inc",
        include_str!("../assets/generated_static/simon_whip_tiles.inc"),
    ),
    (
        "src/generated/rizer_bullet_tile.inc",
        include_str!("../assets/generated_static/rizer_bullet_tile.inc"),
    ),
];

const RUNTIME_FRAME_WIDTH_TILES: usize = 4;
const RUNTIME_FRAME_HEIGHT_TILES: usize = 5;
const RUNTIME_FRAME_TILE_COUNT: usize = RUNTIME_FRAME_WIDTH_TILES * RUNTIME_FRAME_HEIGHT_TILES;
const RUNTIME_EFFECT_TILE_COUNT: usize = 8;
const MAX_RUNTIME_FIGHTER_TILES: usize = 256 - RUNTIME_EFFECT_TILE_COUNT;

#[derive(Debug, Clone, Serialize)]
pub struct CharacterGenerationResult {
    pub id: String,
    pub display_name: String,
    pub runtime_slot: u8,
    pub spec_path: String,
    pub source_image: String,
    pub chr_include: String,
    pub state_include: String,
    pub meta_json: String,
    pub frame_count: usize,
    pub total_tiles: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RosterGenerationResult {
    pub characters: Vec<CharacterGenerationResult>,
    pub roster_chr_include: String,
    pub roster_table_include: String,
    pub roster_runtime_header: String,
    pub roster_meta_json: String,
    pub total_tiles: usize,
}

#[derive(Debug, Clone, Serialize)]
struct FrameMeta {
    name: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    tile_offset: u16,
    tile_count: u16,
}

#[derive(Debug, Clone, Serialize)]
struct AnimationMeta {
    name: String,
    frame_indices: Vec<u16>,
    frame_duration: u8,
    minimum_distinct_frames: u8,
    looping: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CharacterMetaFile {
    id: String,
    display_name: String,
    source_image: String,
    chr_include: String,
    state_include: String,
    total_tiles: usize,
    frames: Vec<FrameMeta>,
    animations: Vec<AnimationMeta>,
    compiled_state_graph_bytes: usize,
}

struct CharacterArtifacts {
    result: CharacterGenerationResult,
    frame_chr_bytes: Vec<Vec<u8>>,
    runtime: RuntimeAnimationData,
}

struct RuntimeAnimationData {
    frame_count: usize,
    frame_tile_count: usize,
    idle_a: u16,
    idle_b: u16,
    idle_ticks: u8,
    walk: [u16; 4],
    walk_ticks: u8,
    jump: [u16; 4],
    jump_ticks: u8,
    jump_looping: bool,
    jab: RuntimeActionData,
    special: RuntimeActionData,
    hurt: u16,
}

struct RuntimeActionData {
    frames: [u16; 3],
    total_ticks: u8,
    phase_1_start: u8,
    phase_2_start: u8,
    active_start: u8,
    active_end: u8,
}

/// Syncs static generated includes required by the runtime into `src/generated/`.
///
/// These assets are versioned with the tooling crate so clean CI checkouts can
/// build the ROM without relying on local untracked generated files.
pub fn sync_runtime_static_includes(workspace_root: &Path) -> Result<()> {
    for (relative_path, contents) in STATIC_RUNTIME_GENERATED_ASSETS {
        write_text_if_changed(&workspace_root.join(relative_path), contents)?;
    }
    generate_music_data(workspace_root)?;
    Ok(())
}

/// Generates all outputs for a single fighter spec.
pub fn generate_character(
    spec_path: &Path,
    workspace_root: &Path,
) -> Result<CharacterGenerationResult> {
    sync_runtime_static_includes(workspace_root)?;
    let artifacts = generate_character_internal(spec_path, workspace_root)?;
    Ok(artifacts.result)
}

/// Generates all fighters in `spec_dir` and writes roster-level outputs.
pub fn generate_roster(spec_dir: &Path, workspace_root: &Path) -> Result<RosterGenerationResult> {
    sync_runtime_static_includes(workspace_root)?;
    let spec_paths = discover_specs(spec_dir)?;
    let mut artifacts = Vec::with_capacity(spec_paths.len());
    for spec_path in spec_paths {
        artifacts.push(generate_character_internal(&spec_path, workspace_root)?);
    }
    artifacts.sort_by_key(|artifact| artifact.result.runtime_slot);
    for (expected, artifact) in artifacts.iter().enumerate() {
        if artifact.result.runtime_slot as usize != expected {
            bail!(
                "runtime slots must be contiguous from zero; expected slot {}, found {} ({})",
                expected,
                artifact.result.runtime_slot,
                artifact.result.id
            );
        }
    }

    let all_frames = artifacts
        .iter()
        .flat_map(|artifact| artifact.frame_chr_bytes.iter().cloned())
        .collect::<Vec<_>>();
    let atlas = deduplicate_chr_frames(
        &all_frames,
        RUNTIME_FRAME_TILE_COUNT,
        TRANSPARENT_TILE_INDEX as usize,
    )?;
    let total_tiles = atlas.tile_count();
    if total_tiles > MAX_RUNTIME_FIGHTER_TILES {
        bail!(
            "fighter CHR atlas needs {} tiles, budget is {} ({} reserved for effects)",
            total_tiles,
            MAX_RUNTIME_FIGHTER_TILES,
            RUNTIME_EFFECT_TILE_COUNT
        );
    }
    let mut frame_cursor = 0usize;
    let mut roster_table = String::new();

    roster_table.push_str("; Auto-generated by nessy.\n");
    roster_table.push_str("; Entry format: .byte first_frame, frame_count\n");

    for artifact in &artifacts {
        roster_table.push_str(&format!(
            "; {}\n.byte ${:02X}, ${:02X}\n",
            artifact.result.id, frame_cursor, artifact.runtime.frame_count
        ));
        frame_cursor += artifact.runtime.frame_count;
    }

    let generated_dir = workspace_root.join("src/generated");
    fs::create_dir_all(&generated_dir)
        .with_context(|| format!("failed to create {}", generated_dir.display()))?;

    let roster_chr_include = generated_dir.join("roster_chr.inc");
    write_chr_include(
        &roster_chr_include,
        "Auto-generated deduplicated fighter CHR atlas.",
        &atlas.bytes,
    )?;

    let roster_table_include = generated_dir.join("roster_table.inc");
    fs::write(&roster_table_include, roster_table)
        .with_context(|| format!("failed to write {}", roster_table_include.display()))?;

    let roster_runtime_header = generated_dir.join("roster_runtime.h");
    write_roster_runtime_header(
        &roster_runtime_header,
        &artifacts,
        &atlas.frame_maps,
        &atlas.frame_attributes,
    )?;

    let results = artifacts
        .into_iter()
        .map(|item| item.result)
        .collect::<Vec<_>>();
    let roster_meta = RosterGenerationResult {
        characters: results.clone(),
        roster_chr_include: to_relative_string(workspace_root, &roster_chr_include),
        roster_table_include: to_relative_string(workspace_root, &roster_table_include),
        roster_runtime_header: to_relative_string(workspace_root, &roster_runtime_header),
        roster_meta_json: "src/generated/roster_meta.json".to_string(),
        total_tiles,
    };

    let roster_meta_json = workspace_root.join(&roster_meta.roster_meta_json);
    write_json(&roster_meta_json, &roster_meta)?;

    Ok(roster_meta)
}

fn generate_character_internal(
    spec_path: &Path,
    workspace_root: &Path,
) -> Result<CharacterArtifacts> {
    let spec = load_character_spec(spec_path)?;
    let output_paths = spec.resolve_output_paths(workspace_root);
    let source_image = resolve_source_image(spec_path, workspace_root, &spec)?;

    let image = ImageReader::open(&source_image)
        .with_context(|| format!("failed to open {}", source_image.display()))?
        .decode()
        .with_context(|| format!("failed to decode {}", source_image.display()))?
        .to_rgba8();

    let frame_regions = spec
        .spritesheet
        .frames
        .iter()
        .map(|frame| ChrRegion {
            x: frame.x,
            y: frame.y,
            w: frame.w.unwrap_or(spec.spritesheet.frame_width),
            h: frame.h.unwrap_or(spec.spritesheet.frame_height),
        })
        .collect::<Vec<_>>();
    let tile_sets = image_regions_to_chr(&image, &frame_regions, ChrOptions::default())
        .with_context(|| format!("failed to convert frames from {}", source_image.display()))?;

    let mut frame_to_index = HashMap::new();
    let mut frames = Vec::with_capacity(spec.spritesheet.frames.len());
    let mut chr_bytes = Vec::new();
    let mut frame_chr_bytes = Vec::with_capacity(spec.spritesheet.frames.len());

    for ((frame, region), tile_set) in spec
        .spritesheet
        .frames
        .iter()
        .zip(frame_regions)
        .zip(tile_sets)
    {
        let tile_offset = (chr_bytes.len() / BYTES_PER_TILE) as u16;
        let tile_count = tile_set.tile_count as u16;
        chr_bytes.extend_from_slice(&tile_set.bytes);
        frame_chr_bytes.push(tile_set.bytes);

        let meta = FrameMeta {
            name: frame.name.clone(),
            x: frame.x,
            y: frame.y,
            w: region.w,
            h: region.h,
            tile_offset,
            tile_count,
        };

        let frame_idx = frames.len() as u16;
        frame_to_index.insert(frame.name.as_str(), frame_idx);
        frames.push(meta);
    }

    let mut animations = Vec::with_capacity(spec.animations.len());
    for animation in &spec.animations {
        let mut frame_indices = Vec::with_capacity(animation.frames.len());
        for frame_name in &animation.frames {
            let Some(frame_index) = frame_to_index.get(frame_name.as_str()) else {
                bail!(
                    "animation '{}' references missing frame '{}'",
                    animation.name,
                    frame_name
                );
            };
            frame_indices.push(*frame_index);
        }

        animations.push(AnimationMeta {
            name: animation.name.clone(),
            frame_indices,
            frame_duration: animation.frame_duration,
            minimum_distinct_frames: animation
                .minimum_distinct_frames
                .unwrap_or_else(|| default_minimum_distinct_frames(&animation.name)),
            looping: animation.looping,
        });
    }
    let runtime = resolve_runtime_animation_data(
        &spec.character.id,
        &spec.state_graph.states,
        &frames,
        &animations,
    )?;
    validate_runtime_animation_art(&spec.character.id, &animations, &frame_chr_bytes)?;

    write_chr_include(
        &output_paths.chr_include,
        "Auto-generated CHR tile bytes for character sprites.",
        &chr_bytes,
    )?;

    let compiled_state_graph = compile_state_graph(&spec)?;
    write_state_include(&output_paths.state_include, &compiled_state_graph)?;

    let meta = CharacterMetaFile {
        id: spec.character.id.clone(),
        display_name: spec.character.display_name.clone(),
        source_image: to_relative_string(workspace_root, &source_image),
        chr_include: to_relative_string(workspace_root, &output_paths.chr_include),
        state_include: to_relative_string(workspace_root, &output_paths.state_include),
        total_tiles: chr_bytes.len() / BYTES_PER_TILE,
        frames,
        animations,
        compiled_state_graph_bytes: compiled_state_graph.bytes.len(),
    };

    write_json(&output_paths.meta_json, &meta)?;

    let result = CharacterGenerationResult {
        id: spec.character.id,
        display_name: spec.character.display_name,
        runtime_slot: spec.character.runtime_slot,
        spec_path: to_relative_string(workspace_root, spec_path),
        source_image: to_relative_string(workspace_root, &source_image),
        chr_include: to_relative_string(workspace_root, &output_paths.chr_include),
        state_include: to_relative_string(workspace_root, &output_paths.state_include),
        meta_json: to_relative_string(workspace_root, &output_paths.meta_json),
        frame_count: meta.frames.len(),
        total_tiles: meta.total_tiles,
    };

    Ok(CharacterArtifacts {
        result,
        frame_chr_bytes,
        runtime,
    })
}

fn resolve_runtime_animation_data(
    character_id: &str,
    states: &[StateSpec],
    frames: &[FrameMeta],
    animations: &[AnimationMeta],
) -> Result<RuntimeAnimationData> {
    let animation = |name: &str, minimum_frames: usize| -> Result<&AnimationMeta> {
        let animation = animations
            .iter()
            .find(|animation| animation.name == name)
            .with_context(|| format!("character '{character_id}' needs a '{name}' animation"))?;
        if animation.frame_indices.len() < minimum_frames {
            bail!(
                "character '{}' animation '{}' needs at least {} frame(s)",
                character_id,
                name,
                minimum_frames
            );
        }
        Ok(animation)
    };

    let first_tile_count = frames
        .first()
        .with_context(|| format!("character '{character_id}' has no runtime frames"))?
        .tile_count as usize;
    if frames
        .iter()
        .any(|frame| frame.tile_count as usize != first_tile_count)
    {
        bail!("character '{character_id}' runtime frames must have equal tile counts");
    }
    if first_tile_count == 0 || first_tile_count > u8::MAX as usize {
        bail!("character '{character_id}' has an invalid runtime frame tile count");
    }

    let idle = animation("idle", 1)?;
    let walk = animation("walk", 4)?;
    let jump = animation("jump", 4)?;
    let jab = animation("jab", 3)?;
    let special = animation("special", 3)?;
    let hurt = animation("hurt", 1)?;
    let action = |name: &str, animation: &AnimationMeta| -> Result<RuntimeActionData> {
        let state = states
            .iter()
            .find(|state| state.name == name)
            .with_context(|| format!("character '{character_id}' needs a '{name}' state"))?;
        if state.animation != animation.name {
            bail!(
                "character '{}' state '{}' must drive animation '{}', found '{}'",
                character_id,
                name,
                name,
                state.animation
            );
        }
        let total_ticks = animation.frame_duration.checked_mul(3).with_context(|| {
            format!("character '{character_id}' animation '{name}' is too long")
        })?;
        let phase_1_start = animation.frame_duration;
        let phase_2_start = animation.frame_duration * 2;
        let active_start = state.active_start.unwrap_or(phase_1_start);
        let active_end = state
            .active_end
            .unwrap_or_else(|| phase_2_start.saturating_sub(1));
        if active_start > active_end || active_end >= total_ticks {
            bail!(
                "character '{}' state '{}' active window {}..{} exceeds its {}-tick timeline",
                character_id,
                name,
                active_start,
                active_end,
                total_ticks
            );
        }
        Ok(RuntimeActionData {
            frames: [
                animation.frame_indices[0],
                animation.frame_indices[1],
                animation.frame_indices[2],
            ],
            total_ticks,
            phase_1_start,
            phase_2_start,
            active_start,
            active_end,
        })
    };
    Ok(RuntimeAnimationData {
        frame_count: frames.len(),
        frame_tile_count: first_tile_count,
        idle_a: idle.frame_indices[0],
        idle_b: *idle.frame_indices.get(1).unwrap_or(&idle.frame_indices[0]),
        idle_ticks: idle.frame_duration,
        walk: [
            walk.frame_indices[0],
            walk.frame_indices[1],
            walk.frame_indices[2],
            walk.frame_indices[3],
        ],
        walk_ticks: walk.frame_duration,
        jump: [
            jump.frame_indices[0],
            jump.frame_indices[1],
            jump.frame_indices[2],
            jump.frame_indices[3],
        ],
        jump_ticks: jump.frame_duration,
        jump_looping: jump.looping,
        jab: action("jab", jab)?,
        special: action("special", special)?,
        hurt: hurt.frame_indices[0],
    })
}

fn validate_runtime_animation_art(
    character_id: &str,
    animations: &[AnimationMeta],
    frame_chr_bytes: &[Vec<u8>],
) -> Result<()> {
    for name in ["walk", "jump", "jab", "special"] {
        let animation = animations
            .iter()
            .find(|animation| animation.name == name)
            .with_context(|| format!("character '{character_id}' needs a '{name}' animation"))?;
        let minimum_distinct = animation.minimum_distinct_frames as usize;
        let mut distinct = HashSet::new();
        let runtime_phases = if name == "walk" || name == "jump" {
            4
        } else {
            3
        };
        for frame_index in animation.frame_indices.iter().take(runtime_phases) {
            let frame = frame_chr_bytes
                .get(*frame_index as usize)
                .with_context(|| {
                    format!("character '{character_id}' has an invalid '{name}' frame")
                })?;
            distinct.insert(frame.as_slice());
        }
        if distinct.len() < minimum_distinct {
            bail!(
                "character '{}' animation '{}' needs at least {} visually distinct phase(s), found {}",
                character_id,
                name,
                minimum_distinct,
                distinct.len()
            );
        }
    }
    Ok(())
}

fn default_minimum_distinct_frames(name: &str) -> u8 {
    match name {
        "walk" | "jump" => 3,
        "jab" | "special" => 2,
        _ => 1,
    }
}

fn write_roster_runtime_header(
    path: &Path,
    artifacts: &[CharacterArtifacts],
    flat_frame_maps: &[Vec<u8>],
    flat_frame_attributes: &[Vec<u8>],
) -> Result<()> {
    let first = artifacts
        .first()
        .context("cannot write an empty runtime roster")?;
    let frame_tile_count = first.runtime.frame_tile_count;
    if artifacts
        .iter()
        .any(|artifact| artifact.runtime.frame_tile_count != frame_tile_count)
    {
        bail!("all runtime fighters must use the same frame dimensions");
    }
    if frame_tile_count != RUNTIME_FRAME_TILE_COUNT {
        bail!(
            "runtime frames must be {}x{} tiles, got {} tiles",
            RUNTIME_FRAME_WIDTH_TILES,
            RUNTIME_FRAME_HEIGHT_TILES,
            frame_tile_count
        );
    }

    let expected_frame_maps = artifacts
        .iter()
        .map(|artifact| artifact.runtime.frame_count)
        .sum::<usize>();
    if flat_frame_maps.len() != expected_frame_maps
        || flat_frame_attributes.len() != expected_frame_maps
    {
        bail!(
            "runtime frame map counts are {}/{}, expected {}",
            flat_frame_maps.len(),
            flat_frame_attributes.len(),
            expected_frame_maps
        );
    }

    let values = |select: fn(&RuntimeAnimationData) -> u16| -> String {
        artifacts
            .iter()
            .map(|artifact| select(&artifact.runtime).to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let rows_3 = |select: fn(&RuntimeAnimationData) -> [u16; 3]| -> String {
        artifacts
            .iter()
            .map(|artifact| {
                let row = select(&artifact.runtime);
                format!("    {{{}, {}, {}}}", row[0], row[1], row[2])
            })
            .collect::<Vec<_>>()
            .join(",\n")
    };
    let rows_4 = |select: fn(&RuntimeAnimationData) -> [u16; 4]| -> String {
        artifacts
            .iter()
            .map(|artifact| {
                let row = select(&artifact.runtime);
                format!("    {{{}, {}, {}, {}}}", row[0], row[1], row[2], row[3])
            })
            .collect::<Vec<_>>()
            .join(",\n")
    };
    let max_frames = artifacts
        .iter()
        .map(|artifact| artifact.runtime.frame_count)
        .max()
        .unwrap_or(0);

    let runtime_tile_count = flat_frame_maps
        .iter()
        .flatten()
        .copied()
        .filter(|index| *index != TRANSPARENT_TILE_INDEX)
        .max()
        .map_or(0usize, |index| index as usize + 1);
    let mut render_offsets = Vec::with_capacity(artifacts.len());
    let mut render_counts = Vec::with_capacity(artifacts.len());
    let mut render_bytes = Vec::new();
    let mut map_cursor = 0usize;
    for artifact in artifacts {
        let mut fighter_offsets = Vec::with_capacity(max_frames);
        let mut fighter_counts = Vec::with_capacity(max_frames);
        for frame_index in 0..max_frames {
            fighter_offsets.push(render_bytes.len());
            if frame_index >= artifact.runtime.frame_count {
                fighter_counts.push(0);
                continue;
            }

            let frame_map = &flat_frame_maps[map_cursor + frame_index];
            let frame_attributes = &flat_frame_attributes[map_cursor + frame_index];
            if frame_map.len() != frame_tile_count || frame_attributes.len() != frame_tile_count {
                bail!(
                    "runtime frame data has {}/{} tiles, expected {}",
                    frame_map.len(),
                    frame_attributes.len(),
                    frame_tile_count
                );
            }

            let before = render_bytes.len();
            for (position, (&tile, &attributes)) in
                frame_map.iter().zip(frame_attributes).enumerate()
            {
                if tile == TRANSPARENT_TILE_INDEX {
                    continue;
                }
                render_bytes.push(((position % RUNTIME_FRAME_WIDTH_TILES) * 8) as u8);
                render_bytes.push(((position / RUNTIME_FRAME_WIDTH_TILES) * 8) as u8);
                render_bytes.push(tile);
                render_bytes.push(attributes);
            }
            let count = (render_bytes.len() - before) / 4;
            if count > u8::MAX as usize {
                bail!("runtime frame has {count} visible tiles, maximum is 255");
            }
            fighter_counts.push(count);
        }
        map_cursor += artifact.runtime.frame_count;
        render_offsets.push(fighter_offsets);
        render_counts.push(fighter_counts);
    }
    if render_bytes.len() > u16::MAX as usize {
        bail!(
            "runtime render table has {} bytes, maximum is {}",
            render_bytes.len(),
            u16::MAX
        );
    }

    let usize_rows = |rows: &[Vec<usize>]| -> String {
        rows.iter()
            .map(|row| {
                format!(
                    "    {{{}}}",
                    row.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(",\n")
    };
    let flat_u8 = |values: &[u8]| -> String {
        values
            .chunks(24)
            .map(|chunk| {
                format!(
                    "    {}",
                    chunk
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(",\n")
    };

    let mut output = format!(
        "#ifndef ROSTER_RUNTIME_H\n#define ROSTER_RUNTIME_H\n\n\
         #define ROSTER_CHARACTER_COUNT {}\n\
         #define ROSTER_FRAME_COUNT {}\n\
         #define ROSTER_FRAME_WIDTH_TILES {}\n\
         #define ROSTER_FRAME_HEIGHT_TILES {}\n\
         #define ROSTER_FRAME_TILE_COUNT {}\n\
         #define ROSTER_RENDER_ENTRY_COUNT {}\n\
         #define ROSTER_RENDER_BYTE_COUNT {}\n\
         #define ROSTER_RUNTIME_TILE_COUNT {}\n\
         #define ROSTER_TILE_NONE {}\n\n",
        artifacts.len(),
        max_frames,
        RUNTIME_FRAME_WIDTH_TILES,
        RUNTIME_FRAME_HEIGHT_TILES,
        frame_tile_count,
        render_bytes.len() / 4,
        render_bytes.len(),
        runtime_tile_count,
        TRANSPARENT_TILE_INDEX,
    );

    output.push_str(&format!(
        "static const unsigned char roster_frame_idle_a[] = {{{}}};\n\
         static const unsigned char roster_frame_idle_b[] = {{{}}};\n\
         static const unsigned char roster_idle_frame_ticks[] = {{{}}};\n\
         static const unsigned char roster_walk_frame_ticks[] = {{{}}};\n\
         static const unsigned char roster_jump_frame_ticks[] = {{{}}};\n\
         static const unsigned char roster_jump_looping[] = {{{}}};\n\
         static const unsigned char roster_frame_hurt[] = {{{}}};\n\
         static const unsigned char roster_jab_total_ticks[] = {{{}}};\n\
         static const unsigned char roster_jab_phase_1_start[] = {{{}}};\n\
         static const unsigned char roster_jab_phase_2_start[] = {{{}}};\n\
         static const unsigned char roster_jab_active_start[] = {{{}}};\n\
         static const unsigned char roster_jab_active_end[] = {{{}}};\n\
         static const unsigned char roster_special_total_ticks[] = {{{}}};\n\
         static const unsigned char roster_special_phase_1_start[] = {{{}}};\n\
         static const unsigned char roster_special_phase_2_start[] = {{{}}};\n\
         static const unsigned char roster_special_active_start[] = {{{}}};\n\
         static const unsigned char roster_special_active_end[] = {{{}}};\n\n",
        values(|runtime| runtime.idle_a),
        values(|runtime| runtime.idle_b),
        values(|runtime| runtime.idle_ticks as u16),
        values(|runtime| runtime.walk_ticks as u16),
        values(|runtime| runtime.jump_ticks as u16),
        values(|runtime| u16::from(runtime.jump_looping)),
        values(|runtime| runtime.hurt),
        values(|runtime| runtime.jab.total_ticks as u16),
        values(|runtime| runtime.jab.phase_1_start as u16),
        values(|runtime| runtime.jab.phase_2_start as u16),
        values(|runtime| runtime.jab.active_start as u16),
        values(|runtime| runtime.jab.active_end as u16),
        values(|runtime| runtime.special.total_ticks as u16),
        values(|runtime| runtime.special.phase_1_start as u16),
        values(|runtime| runtime.special.phase_2_start as u16),
        values(|runtime| runtime.special.active_start as u16),
        values(|runtime| runtime.special.active_end as u16),
    ));
    output.push_str(&format!(
        "static const unsigned char roster_frame_walk[][4] = {{\n{}\n}};\n\
         static const unsigned char roster_frame_jump[][4] = {{\n{}\n}};\n\
         static const unsigned char roster_frame_jab[][3] = {{\n{}\n}};\n\
         static const unsigned char roster_frame_special[][3] = {{\n{}\n}};\n\n",
        rows_4(|runtime| runtime.walk),
        rows_4(|runtime| runtime.jump),
        rows_3(|runtime| runtime.jab.frames),
        rows_3(|runtime| runtime.special.frames),
    ));

    output.push_str(&format!(
        "static const unsigned int roster_render_offsets[][ROSTER_FRAME_COUNT] = {{\n{}\n}};\n\
         static const unsigned char roster_render_counts[][ROSTER_FRAME_COUNT] = {{\n{}\n}};\n\n\
         static const unsigned char roster_render_bytes[ROSTER_RENDER_BYTE_COUNT] = {{\n{}\n}};\n\n#endif\n",
        usize_rows(&render_offsets),
        usize_rows(&render_counts),
        flat_u8(&render_bytes),
    ));
    write_text_if_changed(path, &output)
}

fn resolve_source_image(
    spec_path: &Path,
    workspace_root: &Path,
    spec: &CharacterSpec,
) -> Result<PathBuf> {
    let relative_to_spec = spec_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&spec.spritesheet.path);
    if relative_to_spec.exists() {
        return Ok(relative_to_spec);
    }

    let relative_to_root = workspace_root.join(&spec.spritesheet.path);
    if relative_to_root.exists() {
        return Ok(relative_to_root);
    }

    bail!(
        "source spritesheet '{}' not found relative to {} or workspace root {}",
        spec.spritesheet.path,
        spec_path.display(),
        workspace_root.display()
    )
}

fn write_chr_include(path: &Path, comment: &str, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut out = String::new();
    out.push_str("; Auto-generated by nessy. Do not edit manually.\n");
    out.push_str(&format!("; {}\n", comment));

    if bytes.is_empty() {
        out.push_str(".byte $00\n");
    } else {
        for chunk in bytes.chunks(16) {
            out.push_str(".byte ");
            for (idx, byte) in chunk.iter().enumerate() {
                if idx != 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("${:02X}", byte));
            }
            out.push('\n');
        }
    }

    fs::write(path, out).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn to_relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn write_text_if_changed(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let should_write = match fs::read_to_string(path) {
        Ok(existing) => existing != contents,
        Err(_) => true,
    };

    if should_write {
        fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_animation_preserves_three_phase_actions_and_authored_timing() {
        let frames = (0..7)
            .map(|index| FrameMeta {
                name: format!("frame_{index}"),
                x: 0,
                y: 0,
                w: 32,
                h: 40,
                tile_offset: index * 20,
                tile_count: 20,
            })
            .collect::<Vec<_>>();
        let animations = vec![
            AnimationMeta {
                name: "idle".to_string(),
                frame_indices: vec![0, 1],
                frame_duration: 8,
                minimum_distinct_frames: 1,
                looping: true,
            },
            AnimationMeta {
                name: "walk".to_string(),
                frame_indices: vec![1, 2, 3, 2],
                frame_duration: 6,
                minimum_distinct_frames: 3,
                looping: true,
            },
            AnimationMeta {
                name: "jump".to_string(),
                frame_indices: vec![3, 4, 5, 6],
                frame_duration: 8,
                minimum_distinct_frames: 3,
                looping: false,
            },
            AnimationMeta {
                name: "jab".to_string(),
                frame_indices: vec![1, 4, 2],
                frame_duration: 6,
                minimum_distinct_frames: 2,
                looping: false,
            },
            AnimationMeta {
                name: "special".to_string(),
                frame_indices: vec![2, 5, 0],
                frame_duration: 14,
                minimum_distinct_frames: 2,
                looping: false,
            },
            AnimationMeta {
                name: "hurt".to_string(),
                frame_indices: vec![5],
                frame_duration: 10,
                minimum_distinct_frames: 1,
                looping: false,
            },
        ];

        let states = vec![
            StateSpec {
                name: "jab".to_string(),
                animation: "jab".to_string(),
                cancel_start: None,
                cancel_end: None,
                active_start: Some(7),
                active_end: Some(11),
                transitions: Vec::new(),
            },
            StateSpec {
                name: "special".to_string(),
                animation: "special".to_string(),
                cancel_start: None,
                cancel_end: None,
                active_start: None,
                active_end: None,
                transitions: Vec::new(),
            },
        ];

        let runtime =
            resolve_runtime_animation_data("simon", &states, &frames, &animations).unwrap();
        assert_eq!(runtime.idle_b, 1);
        assert_eq!(runtime.walk, [1, 2, 3, 2]);
        assert_eq!(runtime.walk_ticks, 6);
        assert_eq!(runtime.jump, [3, 4, 5, 6]);
        assert_eq!(runtime.jump_ticks, 8);
        assert!(!runtime.jump_looping);
        assert_eq!(runtime.jab.frames, [1, 4, 2]);
        assert_eq!(runtime.jab.total_ticks, 18);
        assert_eq!(runtime.jab.phase_1_start, 6);
        assert_eq!(runtime.jab.phase_2_start, 12);
        assert_eq!(runtime.jab.active_start, 7);
        assert_eq!(runtime.jab.active_end, 11);
        assert_eq!(runtime.hurt, 5);
    }

    #[test]
    fn runtime_animation_rejects_unapproved_static_jump_phases() {
        let animation = |name: &str, frame_indices: Vec<u16>| AnimationMeta {
            name: name.to_string(),
            frame_indices,
            frame_duration: 4,
            minimum_distinct_frames: default_minimum_distinct_frames(name),
            looping: false,
        };
        let animations = vec![
            animation("walk", vec![0, 1, 2, 1]),
            animation("jump", vec![3, 3, 4, 4]),
            animation("jab", vec![0, 0, 1]),
            animation("special", vec![1, 1, 2]),
        ];
        let frame_chr_bytes = (0..5).map(|value| vec![value; 16]).collect::<Vec<_>>();

        let error =
            validate_runtime_animation_art("mario", &animations, &frame_chr_bytes).unwrap_err();

        assert!(error
            .to_string()
            .contains("animation 'jump' needs at least 3 visually distinct phase(s), found 2"));
    }

    #[test]
    fn runtime_animation_allows_authored_single_pose_jump() {
        let animation = |name: &str, frame_indices: Vec<u16>| AnimationMeta {
            name: name.to_string(),
            frame_indices,
            frame_duration: 4,
            minimum_distinct_frames: if name == "jump" {
                1
            } else {
                default_minimum_distinct_frames(name)
            },
            looping: false,
        };
        let animations = vec![
            animation("walk", vec![0, 1, 2, 1]),
            animation("jump", vec![3, 3, 3, 3]),
            animation("jab", vec![0, 0, 1]),
            animation("special", vec![1, 1, 2]),
        ];
        let frame_chr_bytes = (0..4).map(|value| vec![value; 16]).collect::<Vec<_>>();

        validate_runtime_animation_art("mario", &animations, &frame_chr_bytes).unwrap();
    }
}
