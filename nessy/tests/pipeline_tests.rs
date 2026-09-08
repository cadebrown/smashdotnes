use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use image::RgbaImage;
use nessy::chr::{
    chr_frames_to_image, deduplicate_chr_frames, image_region_to_chr, image_regions_to_chr,
    ChrOptions, ChrRegion, TRANSPARENT_TILE_INDEX,
};
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
fn reserves_index_zero_for_real_transparency() {
    let mut image = RgbaImage::new(8, 8);
    image.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
    image.put_pixel(1, 0, image::Rgba([32, 32, 32, 255]));
    image.put_pixel(2, 0, image::Rgba([128, 128, 128, 255]));
    image.put_pixel(3, 0, image::Rgba([255, 255, 255, 255]));

    let tile = image_region_to_chr(
        &image,
        ChrRegion {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        },
        ChrOptions {
            quantize_if_needed: false,
            transparent_index_zero: true,
        },
    )
    .unwrap();

    assert_eq!(tile.bytes[0] & 0x70, 0x50);
    assert_eq!(tile.bytes[8] & 0x70, 0x30);
}

#[test]
fn shares_palette_indexes_across_frames() {
    let mut image = RgbaImage::new(16, 8);
    image.put_pixel(0, 0, image::Rgba([255, 255, 255, 255]));
    image.put_pixel(8, 0, image::Rgba([128, 128, 128, 255]));
    image.put_pixel(9, 0, image::Rgba([255, 255, 255, 255]));
    let regions = [
        ChrRegion {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        },
        ChrRegion {
            x: 8,
            y: 0,
            w: 8,
            h: 8,
        },
    ];

    let tiles = image_regions_to_chr(&image, &regions, ChrOptions::default()).unwrap();
    assert_eq!(tiles[0].bytes[0] & 0x80, 0);
    assert_eq!(tiles[0].bytes[8] & 0x80, 0x80);
}

#[test]
fn decodes_chr_frames_for_lossless_reimport() {
    let mut bytes = vec![0u8; 2 * 16];
    bytes[0] = 0x80;
    bytes[8] = 0x40;
    bytes[16] = 0x20;
    bytes[24] = 0x10;
    let image = chr_frames_to_image(&bytes, 2, 1, 1).unwrap();
    let regions = [
        ChrRegion {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        },
        ChrRegion {
            x: 8,
            y: 0,
            w: 8,
            h: 8,
        },
    ];
    let encoded = image_regions_to_chr(&image, &regions, ChrOptions::default()).unwrap();
    assert_eq!(encoded[0].bytes, bytes[0..16]);
    assert_eq!(encoded[1].bytes, bytes[16..32]);
}

#[test]
fn deduplicates_frame_tiles_and_skips_transparency() {
    let transparent = vec![0u8; 16];
    let tile_a = vec![0xAA; 16];
    let tile_b = vec![0x33; 16];
    let frames = vec![
        [transparent.clone(), tile_a.clone(), tile_b.clone()].concat(),
        [tile_a, transparent, tile_b].concat(),
    ];

    let atlas = deduplicate_chr_frames(&frames, 3, 4).unwrap();

    assert_eq!(atlas.tile_count(), 2);
    assert_eq!(
        atlas.frame_maps,
        vec![
            vec![TRANSPARENT_TILE_INDEX, 0, 1],
            vec![0, TRANSPARENT_TILE_INDEX, 1],
        ]
    );
    assert_eq!(atlas.frame_attributes.len(), 2);
}

#[test]
fn shares_chr_tiles_that_only_differ_by_hardware_flip() {
    let mut tile = vec![0u8; 16];
    tile[0] = 0b1000_0000;
    tile[8] = 0b0100_0000;
    let mut mirrored = tile.clone();
    mirrored[0] = tile[0].reverse_bits();
    mirrored[8] = tile[8].reverse_bits();

    let atlas = deduplicate_chr_frames(&[tile, mirrored], 1, 2).unwrap();

    assert_eq!(atlas.tile_count(), 1);
    assert_eq!(atlas.frame_maps[0], atlas.frame_maps[1]);
    assert_eq!(
        atlas.frame_attributes[0][0] ^ atlas.frame_attributes[1][0],
        0x40
    );
}

#[test]
fn rejects_fighter_atlas_over_budget() {
    let frames = vec![[vec![0x11; 16], vec![0x22; 16]].concat()];
    let error = deduplicate_chr_frames(&frames, 2, 1).unwrap_err();
    assert!(error.to_string().contains("budget is 1"));
}

#[test]
fn compiles_state_graph_from_fixture_spec() {
    let spec_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/chars/mario/character.toml");
    let spec = load_character_spec(&spec_path).expect("fixture spec should parse");
    let compiled = compile_state_graph(&spec).expect("state graph should compile");

    assert!(compiled.bytes.starts_with(b"NSG2"));
    let jab = compiled
        .states
        .iter()
        .find(|state| state.name == "jab")
        .expect("jab state should compile");
    assert_eq!(jab.active_start, 3);
    assert_eq!(jab.active_end, 5);
    assert_eq!(compiled.states.len(), spec.state_graph.states.len());
}

#[test]
fn simons_original_single_pose_jump_does_not_turn_midair() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");

    let spec_path = root.join("src/chars/simon/character.toml");
    let spec = load_character_spec(&spec_path).expect("Simon spec should parse");
    let jump = spec
        .animations
        .iter()
        .find(|animation| animation.name == "jump")
        .expect("jump animation should exist");
    assert_eq!(jump.minimum_distinct_frames, Some(1));

    let sheet = image::open(spec_path.parent().unwrap().join(&spec.spritesheet.path))
        .expect("fighter sheet should load")
        .to_rgba8();
    let mut poses = jump.frames.iter().map(|name| {
        let frame = spec
            .spritesheet
            .frames
            .iter()
            .find(|frame| frame.name == *name)
            .expect("jump frame should exist");
        image::imageops::crop_imm(
            &sheet,
            frame.x,
            frame.y,
            frame.w.unwrap_or(spec.spritesheet.frame_width),
            frame.h.unwrap_or(spec.spritesheet.frame_height),
        )
        .to_image()
    });
    let canonical = poses.next().expect("jump needs a pose");
    assert!(poses.all(|pose| pose == canonical));
}

#[test]
fn authored_walk_cycles_return_through_the_contact_pose() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");

    for fighter in ["mario", "simon", "tommy", "rizer"] {
        let spec_path = root.join(format!("src/chars/{fighter}/character.toml"));
        let spec = load_character_spec(&spec_path).expect("fighter spec should parse");
        let walk = spec
            .animations
            .iter()
            .find(|animation| animation.name == "walk")
            .expect("walk animation should exist");
        assert_eq!(walk.frames.len(), 4);
        assert_eq!(walk.frames[1], walk.frames[3]);
    }
}

#[test]
fn rizers_airborne_animation_is_a_four_pose_loop() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let spec_path = root.join("src/chars/rizer/character.toml");
    let spec = load_character_spec(&spec_path).expect("Rizer spec should parse");
    let jump = spec
        .animations
        .iter()
        .find(|animation| animation.name == "jump")
        .expect("jump animation should exist");

    assert!(jump.looping);
    assert_eq!(jump.frames.len(), 4);
    assert_eq!(jump.minimum_distinct_frames, Some(4));
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
