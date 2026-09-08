use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use anyhow::{bail, Context, Result};
use image::{ImageReader, Rgba, RgbaImage};

pub const TILE_WIDTH: u32 = 8;
pub const TILE_HEIGHT: u32 = 8;
pub const BYTES_PER_TILE: usize = 16;
pub const TRANSPARENT_TILE_INDEX: u8 = 0xFF;

#[derive(Debug, Clone, Copy)]
pub struct ChrOptions {
    pub quantize_if_needed: bool,
    pub transparent_index_zero: bool,
}

impl Default for ChrOptions {
    fn default() -> Self {
        Self {
            quantize_if_needed: true,
            transparent_index_zero: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChrRegion {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone)]
pub struct ChrTileSet {
    pub bytes: Vec<u8>,
    pub tile_count: usize,
    pub width_tiles: u32,
    pub height_tiles: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeduplicatedChrAtlas {
    pub bytes: Vec<u8>,
    pub frame_maps: Vec<Vec<u8>>,
    pub frame_attributes: Vec<Vec<u8>>,
}

impl DeduplicatedChrAtlas {
    pub fn tile_count(&self) -> usize {
        self.bytes.len() / BYTES_PER_TILE
    }
}

/// Packs frame-local CHR payloads into one atlas while preserving each frame's
/// tile layout. Fully transparent tiles become a sentinel so the renderer can
/// skip their OAM entries as well as their CHR storage.
pub fn deduplicate_chr_frames(
    frames: &[Vec<u8>],
    expected_tiles_per_frame: usize,
    max_atlas_tiles: usize,
) -> Result<DeduplicatedChrAtlas> {
    if expected_tiles_per_frame == 0 {
        bail!("expected_tiles_per_frame must be greater than zero");
    }
    if max_atlas_tiles > TRANSPARENT_TILE_INDEX as usize {
        bail!("max_atlas_tiles must leave 0xFF available as the transparent sentinel");
    }

    let expected_bytes = expected_tiles_per_frame * BYTES_PER_TILE;
    let transparent = [0u8; BYTES_PER_TILE];
    let mut required_tiles = HashSet::new();
    for frame in frames {
        for tile in frame.chunks_exact(BYTES_PER_TILE) {
            let tile: [u8; BYTES_PER_TILE] = tile.try_into().unwrap();
            if tile == transparent {
                continue;
            }
            let canonical = [
                tile,
                flip_chr_tile(tile, true, false),
                flip_chr_tile(tile, false, true),
                flip_chr_tile(tile, true, true),
            ]
            .into_iter()
            .min()
            .unwrap();
            required_tiles.insert(canonical);
        }
    }
    if required_tiles.len() > max_atlas_tiles {
        bail!(
            "fighter CHR atlas needs {} tiles, budget is {}",
            required_tiles.len(),
            max_atlas_tiles
        );
    }

    let mut tile_indexes: HashMap<[u8; BYTES_PER_TILE], u8> = HashMap::new();
    let mut bytes = Vec::new();
    let mut frame_maps = Vec::with_capacity(frames.len());
    let mut frame_attributes = Vec::with_capacity(frames.len());

    for (frame_index, frame) in frames.iter().enumerate() {
        if frame.len() != expected_bytes {
            bail!(
                "frame {} contains {} CHR bytes, expected {}",
                frame_index,
                frame.len(),
                expected_bytes
            );
        }

        let mut frame_map = Vec::with_capacity(expected_tiles_per_frame);
        let mut frame_attrs = Vec::with_capacity(expected_tiles_per_frame);
        for tile in frame.chunks_exact(BYTES_PER_TILE) {
            let tile: [u8; BYTES_PER_TILE] = tile.try_into().unwrap();
            if tile == transparent {
                frame_map.push(TRANSPARENT_TILE_INDEX);
                frame_attrs.push(0);
                continue;
            }

            let variants = [
                (tile, 0),
                (flip_chr_tile(tile, true, false), 0x40),
                (flip_chr_tile(tile, false, true), 0x80),
                (flip_chr_tile(tile, true, true), 0xC0),
            ];
            let (tile, attributes) = variants
                .into_iter()
                .min_by_key(|(bytes, _)| *bytes)
                .unwrap();

            if let Some(index) = tile_indexes.get(&tile) {
                frame_map.push(*index);
                frame_attrs.push(attributes);
                continue;
            }

            let index = tile_indexes.len();
            if index >= max_atlas_tiles {
                bail!(
                    "fighter CHR atlas needs at least {} tiles, budget is {}",
                    index + 1,
                    max_atlas_tiles
                );
            }
            let index = index as u8;
            tile_indexes.insert(tile, index);
            bytes.extend_from_slice(&tile);
            frame_map.push(index);
            frame_attrs.push(attributes);
        }
        frame_maps.push(frame_map);
        frame_attributes.push(frame_attrs);
    }

    Ok(DeduplicatedChrAtlas {
        bytes,
        frame_maps,
        frame_attributes,
    })
}

fn flip_chr_tile(tile: [u8; BYTES_PER_TILE], horizontal: bool, vertical: bool) -> [u8; 16] {
    let mut output = [0u8; BYTES_PER_TILE];
    for plane in 0..2 {
        for row in 0..8 {
            let source_row = if vertical { 7 - row } else { row };
            let value = tile[plane * 8 + source_row];
            output[plane * 8 + row] = if horizontal {
                value.reverse_bits()
            } else {
                value
            };
        }
    }
    output
}

pub fn chr_frames_to_image(
    bytes: &[u8],
    frame_count: usize,
    frame_width_tiles: usize,
    frame_height_tiles: usize,
) -> Result<RgbaImage> {
    let tiles_per_frame = frame_width_tiles * frame_height_tiles;
    let expected = frame_count * tiles_per_frame * BYTES_PER_TILE;
    if bytes.len() != expected {
        bail!(
            "CHR frame payload is {} bytes, expected {}",
            bytes.len(),
            expected
        );
    }

    let mut image = RgbaImage::new(
        (frame_count * frame_width_tiles * TILE_WIDTH as usize) as u32,
        (frame_height_tiles * TILE_HEIGHT as usize) as u32,
    );
    let levels = [0u8, 85, 170, 255];

    for frame in 0..frame_count {
        for tile_y in 0..frame_height_tiles {
            for tile_x in 0..frame_width_tiles {
                let tile = frame * tiles_per_frame + tile_y * frame_width_tiles + tile_x;
                let tile_offset = tile * BYTES_PER_TILE;
                for row in 0..8 {
                    let low = bytes[tile_offset + row];
                    let high = bytes[tile_offset + 8 + row];
                    for col in 0..8 {
                        let shift = 7 - col;
                        let index = ((low >> shift) & 1) | (((high >> shift) & 1) << 1);
                        let level = levels[index as usize];
                        image.put_pixel(
                            (frame * frame_width_tiles * 8 + tile_x * 8 + col) as u32,
                            (tile_y * 8 + row) as u32,
                            Rgba([level, level, level, 255]),
                        );
                    }
                }
            }
        }
    }

    Ok(image)
}

struct PaletteMap {
    indexes: HashMap<[u8; 4], u8>,
    has_transparency: bool,
}

/// Converts a PNG region into NES 2bpp CHR bytes.
pub fn png_region_to_chr(
    path: &Path,
    region: ChrRegion,
    options: ChrOptions,
) -> Result<ChrTileSet> {
    let image = ImageReader::open(path)
        .with_context(|| format!("failed to open PNG {}", path.display()))?
        .decode()
        .with_context(|| format!("failed to decode PNG {}", path.display()))?
        .to_rgba8();

    image_region_to_chr(&image, region, options)
}

/// Converts an in-memory RGBA region into NES 2bpp CHR bytes.
pub fn image_region_to_chr(
    image: &RgbaImage,
    region: ChrRegion,
    options: ChrOptions,
) -> Result<ChrTileSet> {
    let mut sets = image_regions_to_chr(image, &[region], options)?;
    Ok(sets.remove(0))
}

pub fn image_regions_to_chr(
    image: &RgbaImage,
    regions: &[ChrRegion],
    options: ChrOptions,
) -> Result<Vec<ChrTileSet>> {
    if regions.is_empty() {
        bail!("at least one CHR region is required");
    }
    for region in regions {
        ensure_region(image, *region)?;
        ensure_tile_alignment(region.w, region.h)?;
    }

    let palette_map = build_palette_map(image, regions, options)?;
    regions
        .iter()
        .map(|region| encode_region(image, *region, &palette_map, options))
        .collect()
}

fn encode_region(
    image: &RgbaImage,
    region: ChrRegion,
    palette_map: &PaletteMap,
    options: ChrOptions,
) -> Result<ChrTileSet> {
    let width_tiles = region.w / TILE_WIDTH;
    let height_tiles = region.h / TILE_HEIGHT;
    let mut out = Vec::with_capacity((width_tiles * height_tiles) as usize * BYTES_PER_TILE);

    for tile_y in 0..height_tiles {
        for tile_x in 0..width_tiles {
            let mut low = [0u8; 8];
            let mut high = [0u8; 8];

            for row in 0..8 {
                for col in 0..8 {
                    let px = region.x + (tile_x * TILE_WIDTH) + col;
                    let py = region.y + (tile_y * TILE_HEIGHT) + row;
                    let pixel = image.get_pixel(px, py);
                    let index = map_pixel_to_index(*pixel, palette_map, options)?;
                    let shift = 7 - col;

                    low[row as usize] |= (index & 0x01) << shift;
                    high[row as usize] |= ((index >> 1) & 0x01) << shift;
                }
            }

            out.extend_from_slice(&low);
            out.extend_from_slice(&high);
        }
    }

    Ok(ChrTileSet {
        tile_count: out.len() / BYTES_PER_TILE,
        bytes: out,
        width_tiles,
        height_tiles,
    })
}

fn ensure_region(image: &RgbaImage, region: ChrRegion) -> Result<()> {
    if region.w == 0 || region.h == 0 {
        bail!("CHR region dimensions must be non-zero");
    }

    let x2 = region.x.saturating_add(region.w);
    let y2 = region.y.saturating_add(region.h);
    if x2 > image.width() || y2 > image.height() {
        bail!(
            "CHR region {}x{} at ({},{}) exceeds image bounds {}x{}",
            region.w,
            region.h,
            region.x,
            region.y,
            image.width(),
            image.height()
        );
    }

    Ok(())
}

fn ensure_tile_alignment(width: u32, height: u32) -> Result<()> {
    if !width.is_multiple_of(TILE_WIDTH) || !height.is_multiple_of(TILE_HEIGHT) {
        bail!(
            "CHR region must be divisible by 8x8 tiles, got {}x{}",
            width,
            height
        );
    }
    Ok(())
}

fn build_palette_map(
    image: &RgbaImage,
    regions: &[ChrRegion],
    options: ChrOptions,
) -> Result<PaletteMap> {
    let mut unique = BTreeSet::new();
    let mut has_transparency = false;
    for region in regions {
        for y in region.y..(region.y + region.h) {
            for x in region.x..(region.x + region.w) {
                let pixel = image.get_pixel(x, y).0;
                if options.transparent_index_zero && pixel[3] == 0 {
                    has_transparency = true;
                    continue;
                }
                unique.insert(pixel);
            }
        }
    }

    let color_limit = if has_transparency { 3 } else { 4 };
    if unique.len() > color_limit && !options.quantize_if_needed {
        bail!(
            "image regions use {} visible colors, expected <= {} NES indexes",
            unique.len(),
            color_limit
        );
    }

    if unique.is_empty() {
        return Ok(PaletteMap {
            indexes: HashMap::new(),
            has_transparency,
        });
    }

    if unique.len() > color_limit {
        return Ok(PaletteMap {
            indexes: HashMap::new(),
            has_transparency,
        });
    }

    let mut ordered: Vec<[u8; 4]> = unique.into_iter().collect();
    ordered.sort_by_key(|color| luminance(*color));

    let mut mapping = HashMap::new();
    for (idx, color) in ordered.into_iter().enumerate() {
        mapping.insert(color, idx as u8 + u8::from(has_transparency));
    }

    Ok(PaletteMap {
        indexes: mapping,
        has_transparency,
    })
}

fn map_pixel_to_index(
    pixel: Rgba<u8>,
    palette_map: &PaletteMap,
    options: ChrOptions,
) -> Result<u8> {
    let rgba = pixel.0;
    if palette_map.has_transparency && rgba[3] == 0 {
        return Ok(0);
    }

    if let Some(index) = palette_map.indexes.get(&rgba) {
        return Ok(*index & 0x03);
    }

    if options.quantize_if_needed {
        let level = if palette_map.has_transparency {
            1 + ((luminance(rgba) as u16 * 2) / 255) as u8
        } else {
            ((luminance(rgba) as u16 * 3) / 255) as u8
        };
        return Ok(level);
    }

    bail!("found pixel outside 4-color palette without quantization enabled")
}

fn luminance(pixel: [u8; 4]) -> u8 {
    let r = pixel[0] as u32;
    let g = pixel[1] as u32;
    let b = pixel[2] as u32;
    ((r * 299 + g * 587 + b * 114) / 1000) as u8
}
