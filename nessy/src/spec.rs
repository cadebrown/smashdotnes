use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

/// Full TOML schema for a fighter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSpec {
    pub character: CharacterInfo,
    pub spritesheet: SpritesheetSpec,
    #[serde(default)]
    pub output: OutputSpec,
    pub animations: Vec<AnimationSpec>,
    pub state_graph: StateGraphSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub id: String,
    pub display_name: String,
    pub runtime_slot: u8,
    #[serde(default)]
    pub franchise: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpritesheetSpec {
    pub path: String,
    pub frame_width: u32,
    pub frame_height: u32,
    #[serde(default)]
    pub frames: Vec<FrameSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSpec {
    pub name: String,
    pub x: u32,
    pub y: u32,
    #[serde(default)]
    pub w: Option<u32>,
    #[serde(default)]
    pub h: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationSpec {
    pub name: String,
    pub frames: Vec<String>,
    pub frame_duration: u8,
    #[serde(default)]
    pub minimum_distinct_frames: Option<u8>,
    #[serde(default = "default_looping")]
    pub looping: bool,
}

const fn default_looping() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateGraphSpec {
    pub initial: String,
    pub states: Vec<StateSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSpec {
    pub name: String,
    pub animation: String,
    #[serde(default)]
    pub cancel_start: Option<u8>,
    #[serde(default)]
    pub cancel_end: Option<u8>,
    #[serde(default)]
    pub active_start: Option<u8>,
    #[serde(default)]
    pub active_end: Option<u8>,
    #[serde(default)]
    pub transitions: Vec<TransitionSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSpec {
    pub on: String,
    pub to: String,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpec {
    #[serde(default = "default_chr_include")]
    pub chr_include: String,
    #[serde(default = "default_state_include")]
    pub state_include: String,
    #[serde(default = "default_meta_json")]
    pub meta_json: String,
}

fn default_chr_include() -> String {
    "src/chars/{id}/generated/chr.inc".to_string()
}

fn default_state_include() -> String {
    "src/chars/{id}/generated/state.inc".to_string()
}

fn default_meta_json() -> String {
    "src/chars/{id}/generated/meta.json".to_string()
}

impl Default for OutputSpec {
    fn default() -> Self {
        Self {
            chr_include: default_chr_include(),
            state_include: default_state_include(),
            meta_json: default_meta_json(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharacterOutputPaths {
    pub chr_include: PathBuf,
    pub state_include: PathBuf,
    pub meta_json: PathBuf,
}

/// Loads and validates one character spec from disk.
pub fn load_character_spec(path: &Path) -> Result<CharacterSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read character spec {}", path.display()))?;
    let spec: CharacterSpec =
        toml::from_str(&raw).with_context(|| format!("failed to parse TOML {}", path.display()))?;
    spec.validate(path)?;
    Ok(spec)
}

/// Recursively discovers character specs in a directory tree.
pub fn discover_specs(spec_dir: &Path) -> Result<Vec<PathBuf>> {
    if !spec_dir.exists() {
        bail!("spec directory does not exist: {}", spec_dir.display());
    }

    let mut found = Vec::new();
    for entry in WalkDir::new(spec_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if name.starts_with('_') {
            continue;
        }
        if name != "character.toml" && !name.ends_with(".character.toml") {
            continue;
        }

        found.push(path.to_path_buf());
    }

    found.sort();
    if found.is_empty() {
        bail!("no character specs found under {}", spec_dir.display());
    }

    Ok(found)
}

impl CharacterSpec {
    pub fn validate(&self, spec_path: &Path) -> Result<()> {
        if self.character.id.trim().is_empty() {
            bail!("{}: character.id cannot be empty", spec_path.display());
        }
        if self.character.runtime_slot >= 64 {
            bail!(
                "{}: character.runtime_slot must be less than 64",
                spec_path.display()
            );
        }
        if self.spritesheet.path.trim().is_empty() {
            bail!("{}: spritesheet.path cannot be empty", spec_path.display());
        }
        if self.spritesheet.frame_width == 0 || self.spritesheet.frame_height == 0 {
            bail!(
                "{}: spritesheet frame dimensions must be > 0",
                spec_path.display()
            );
        }
        if self.spritesheet.frames.is_empty() {
            bail!(
                "{}: spritesheet.frames must include at least one frame",
                spec_path.display()
            );
        }

        let mut frame_names = HashSet::new();
        for frame in &self.spritesheet.frames {
            if frame.name.trim().is_empty() {
                bail!("{}: frame names cannot be empty", spec_path.display());
            }
            if !frame_names.insert(frame.name.as_str()) {
                bail!(
                    "{}: duplicate frame name '{}'",
                    spec_path.display(),
                    frame.name
                );
            }
        }

        if self.animations.is_empty() {
            bail!(
                "{}: at least one animation is required",
                spec_path.display()
            );
        }

        let mut animation_names = HashSet::new();
        for animation in &self.animations {
            if animation.name.trim().is_empty() {
                bail!("{}: animation names cannot be empty", spec_path.display());
            }
            if animation.frame_duration == 0 {
                bail!(
                    "{}: animation '{}' has frame_duration=0",
                    spec_path.display(),
                    animation.name
                );
            }
            if animation.frames.is_empty() {
                bail!(
                    "{}: animation '{}' has no frames",
                    spec_path.display(),
                    animation.name
                );
            }
            if let Some(minimum) = animation.minimum_distinct_frames {
                let maximum = animation.frames.len();
                if minimum == 0 || minimum as usize > maximum {
                    bail!(
                        "{}: animation '{}' minimum_distinct_frames must be between 1 and {}",
                        spec_path.display(),
                        animation.name,
                        maximum
                    );
                }
            }
            if !animation_names.insert(animation.name.as_str()) {
                bail!(
                    "{}: duplicate animation name '{}'",
                    spec_path.display(),
                    animation.name
                );
            }
            for frame_ref in &animation.frames {
                if !frame_names.contains(frame_ref.as_str()) {
                    bail!(
                        "{}: animation '{}' references unknown frame '{}'",
                        spec_path.display(),
                        animation.name,
                        frame_ref
                    );
                }
            }
        }

        if self.state_graph.states.is_empty() {
            bail!(
                "{}: state_graph.states cannot be empty",
                spec_path.display()
            );
        }

        let mut state_names = HashSet::new();
        for state in &self.state_graph.states {
            if state.name.trim().is_empty() {
                bail!("{}: state names cannot be empty", spec_path.display());
            }
            if !animation_names.contains(state.animation.as_str()) {
                bail!(
                    "{}: state '{}' references unknown animation '{}'",
                    spec_path.display(),
                    state.name,
                    state.animation
                );
            }
            if !state_names.insert(state.name.as_str()) {
                bail!(
                    "{}: duplicate state name '{}'",
                    spec_path.display(),
                    state.name
                );
            }
            if state.cancel_start.is_some() != state.cancel_end.is_some() {
                bail!(
                    "{}: state '{}' must set both cancel_start and cancel_end",
                    spec_path.display(),
                    state.name
                );
            }
            if state.active_start.is_some() != state.active_end.is_some() {
                bail!(
                    "{}: state '{}' must set both active_start and active_end",
                    spec_path.display(),
                    state.name
                );
            }
            if state.cancel_start > state.cancel_end {
                bail!(
                    "{}: state '{}' cancel window is reversed",
                    spec_path.display(),
                    state.name
                );
            }
            if state.active_start > state.active_end {
                bail!(
                    "{}: state '{}' active window is reversed",
                    spec_path.display(),
                    state.name
                );
            }
        }

        if !state_names.contains(self.state_graph.initial.as_str()) {
            bail!(
                "{}: state_graph.initial '{}' not found in state list",
                spec_path.display(),
                self.state_graph.initial
            );
        }

        for state in &self.state_graph.states {
            for transition in &state.transitions {
                if transition.on.trim().is_empty() {
                    bail!(
                        "{}: state '{}' has transition with empty on",
                        spec_path.display(),
                        state.name
                    );
                }
                if !state_names.contains(transition.to.as_str()) {
                    bail!(
                        "{}: state '{}' transition '{}' targets unknown state '{}'",
                        spec_path.display(),
                        state.name,
                        transition.on,
                        transition.to
                    );
                }
            }
        }

        Ok(())
    }

    pub fn resolve_output_paths(&self, workspace_root: &Path) -> CharacterOutputPaths {
        CharacterOutputPaths {
            chr_include: workspace_root.join(render_template(
                &self.output.chr_include,
                &self.character.id,
            )),
            state_include: workspace_root.join(render_template(
                &self.output.state_include,
                &self.character.id,
            )),
            meta_json: workspace_root
                .join(render_template(&self.output.meta_json, &self.character.id)),
        }
    }
}

fn render_template(raw: &str, character_id: &str) -> PathBuf {
    PathBuf::from(raw.replace("{id}", character_id))
}
