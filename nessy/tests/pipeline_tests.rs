use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use image::RgbaImage;
use nessy::chr::{image_region_to_chr, ChrOptions, ChrRegion};
use nessy::pipeline::sync_runtime_static_includes;
use nessy::spec::load_character_spec;
use nessy::state_graph::compile_state_graph;

#[test]
fn converts_synthetic_8x8_tile() {
    let mut image = RgbaImage::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            let value = if (x + y) % 2 == 0 { 0 } else { 255 };
            image.put_pixel(x, y, image::Rgba([value, value, value, 255]));
        }
    }

    let tile_set = image_region_to_chr(
        &image,
        ChrRegion {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        },
        ChrOptions::default(),
    )
    .expect("tile conversion should succeed");

    assert_eq!(tile_set.tile_count, 1);
    assert_eq!(tile_set.bytes.len(), 16);
}

#[test]
fn compiles_state_graph_from_fixture_spec() {
    let spec_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/chars/mario/character.toml");
    let spec = load_character_spec(&spec_path).expect("fixture spec should parse");
    let compiled = compile_state_graph(&spec).expect("state graph should compile");

    assert!(compiled.bytes.starts_with(b"NSG1"));
    assert_eq!(compiled.states.len(), spec.state_graph.states.len());
}

#[test]
fn syncs_runtime_static_includes() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nessy-sync-static-{stamp}"));
    fs::create_dir_all(&root).expect("create temp root");

    sync_runtime_static_includes(&root).expect("sync should succeed");

    let expected = [
        "src/generated/title_logo.h",
        "src/generated/title_tiles.inc",
        "src/generated/mario_smb1_tiles.inc",
        "src/generated/simon_cv1_tiles.inc",
        "src/generated/tommy_kf_tiles.inc",
        "src/generated/rizer_contra_tiles.inc",
        "src/generated/tommy_kick_ext_tiles.inc",
        "src/generated/rizer_jump_spin_tiles.inc",
        "src/generated/smb1_fireball_tiles.inc",
        "src/generated/cv1_axe_tiles.inc",
        "src/generated/simon_whip_tiles.inc",
        "src/generated/rizer_bullet_tile.inc",
    ];

    for rel in expected {
        assert!(
            root.join(rel).exists(),
            "expected static include to exist: {rel}"
        );
    }

    fs::remove_dir_all(&root).expect("cleanup temp root");
}
