use std::path::PathBuf;

use image::RgbaImage;
use nessy::chr::{image_region_to_chr, ChrOptions, ChrRegion};
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
