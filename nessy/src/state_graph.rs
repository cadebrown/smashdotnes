use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::spec::CharacterSpec;

#[derive(Debug, Clone, Serialize)]
pub struct CompiledStateGraph {
    pub bytes: Vec<u8>,
    pub initial_state: u8,
    pub states: Vec<CompiledState>,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompiledState {
    pub id: u8,
    pub name: String,
    pub animation_index: u8,
    pub cancel_start: u8,
    pub cancel_end: u8,
    pub transitions: Vec<CompiledTransition>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompiledTransition {
    pub on: String,
    pub event_id: u8,
    pub to_state: u8,
    pub condition: Option<String>,
}

/// Compiles TOML state graph definitions into a compact binary payload.
pub fn compile_state_graph(spec: &CharacterSpec) -> Result<CompiledStateGraph> {
    let mut animation_to_id = HashMap::new();
    for (idx, animation) in spec.animations.iter().enumerate() {
        animation_to_id.insert(animation.name.as_str(), idx as u8);
    }

    let mut state_to_id = HashMap::new();
    for (idx, state) in spec.state_graph.states.iter().enumerate() {
        state_to_id.insert(state.name.as_str(), idx as u8);
    }

    let initial_state = *state_to_id
        .get(spec.state_graph.initial.as_str())
        .with_context(|| {
            format!(
                "state graph initial state '{}' not found",
                spec.state_graph.initial
            )
        })?;

    let mut unique_events = BTreeSet::new();
    for state in &spec.state_graph.states {
        for transition in &state.transitions {
            unique_events.insert(transition.on.as_str());
        }
    }

    let events: Vec<String> = unique_events.into_iter().map(str::to_owned).collect();
    let mut event_to_id = HashMap::new();
    for (idx, event) in events.iter().enumerate() {
        event_to_id.insert(event.as_str(), idx as u8);
    }

    let mut states = Vec::with_capacity(spec.state_graph.states.len());
    for (idx, state) in spec.state_graph.states.iter().enumerate() {
        let animation_index =
            *animation_to_id
                .get(state.animation.as_str())
                .with_context(|| {
                    format!(
                        "state '{}' references missing animation '{}'",
                        state.name, state.animation
                    )
                })?;

        if state.transitions.len() > u8::MAX as usize {
            bail!(
                "state '{}' has too many transitions ({})",
                state.name,
                state.transitions.len()
            );
        }

        let mut compiled_transitions = Vec::with_capacity(state.transitions.len());
        for transition in &state.transitions {
            let event_id = *event_to_id
                .get(transition.on.as_str())
                .with_context(|| format!("event '{}' was not interned", transition.on))?;
            let to_state = *state_to_id
                .get(transition.to.as_str())
                .with_context(|| format!("unknown transition target '{}'", transition.to))?;

            compiled_transitions.push(CompiledTransition {
                on: transition.on.clone(),
                event_id,
                to_state,
                condition: transition.condition.clone(),
            });
        }

        states.push(CompiledState {
            id: idx as u8,
            name: state.name.clone(),
            animation_index,
            cancel_start: state.cancel_start.unwrap_or(0xFF),
            cancel_end: state.cancel_end.unwrap_or(0xFF),
            transitions: compiled_transitions,
        });
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"NSG1");
    bytes.push(states.len() as u8);
    bytes.push(events.len() as u8);
    bytes.push(initial_state);
    bytes.push(0);

    for state in &states {
        bytes.push(state.animation_index);
        bytes.push(state.cancel_start);
        bytes.push(state.cancel_end);
        bytes.push(state.transitions.len() as u8);

        for transition in &state.transitions {
            bytes.push(transition.event_id);
            bytes.push(transition.to_state);
            bytes.push(condition_tag(transition.condition.as_deref()));
        }
    }

    for event in &events {
        if event.len() > u8::MAX as usize {
            bail!("event '{}' exceeds 255 bytes", event);
        }
        bytes.push(event.len() as u8);
        bytes.extend_from_slice(event.as_bytes());
    }

    Ok(CompiledStateGraph {
        bytes,
        initial_state,
        states,
        events,
    })
}

fn condition_tag(condition: Option<&str>) -> u8 {
    match condition {
        Some(raw) => raw
            .as_bytes()
            .iter()
            .fold(0u8, |acc, item| acc.wrapping_mul(33) ^ item),
        None => 0,
    }
}

/// Writes a ca65 include with the compiled state-graph payload bytes.
pub fn write_state_include(path: &Path, compiled: &CompiledStateGraph) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut out = String::new();
    out.push_str("; Auto-generated by nessy. Do not edit manually.\n");
    out.push_str("; Binary payload format: NSG1 header + states + event strings\n");

    for chunk in compiled.bytes.chunks(16) {
        out.push_str(".byte ");
        for (idx, byte) in chunk.iter().enumerate() {
            if idx != 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("${:02X}", byte));
        }
        out.push('\n');
    }

    fs::write(path, out).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
