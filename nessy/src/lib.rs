//! Core tooling library for SmashDotNES.
//!
//! This crate exposes reusable generation and validation APIs used by:
//! - the `nessy` CLI
//! - tests
//! - external automation wrappers

pub mod chr;
pub mod emulator;
pub mod ines;
pub mod pipeline;
pub mod spec;
pub mod state_graph;

pub use pipeline::{
    generate_character, generate_roster, sync_runtime_static_includes, CharacterGenerationResult,
    RosterGenerationResult,
};
pub use spec::{discover_specs, load_character_spec, CharacterSpec};
