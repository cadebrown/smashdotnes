use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use anyhow::{bail, Context, Result};
use image::{ImageReader, Rgba, RgbaImage};

pub const TILE_WIDTH: u32 = 8;
pub const TILE_HEIGHT: u32 = 8;
pub const BYTES_PER_TILE: usize = 16;

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
    ensure_region(image, region)?;
    ensure_tile_alignment(region.w, region.h)?;

    let palette_map = build_palette_map(image, region, options)?;

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
                    let index = map_pixel_to_index(*pixel, &palette_map, options)?;
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
    if width % TILE_WIDTH != 0 || height % TILE_HEIGHT != 0 {
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
    region: ChrRegion,
    options: ChrOptions,
) -> Result<HashMap<[u8; 4], u8>> {
    let mut unique = BTreeSet::new();
    for y in region.y..(region.y + region.h) {
        for x in region.x..(region.x + region.w) {
            let pixel = image.get_pixel(x, y).0;
            if options.transparent_index_zero && pixel[3] == 0 {
                continue;
            }
            unique.insert(pixel);
        }
    }

    if unique.len() > 4 && !options.quantize_if_needed {
        bail!(
            "image region uses {} colors, expected <= 4 NES indexes",
            unique.len()
        );
    }

    if unique.is_empty() {
        return Ok(HashMap::new());
    }

    if unique.len() > 4 {
        return Ok(HashMap::new());
    }

    let mut ordered: Vec<[u8; 4]> = unique.into_iter().collect();
    ordered.sort_by_key(|color| luminance(*color));

    let mut mapping = HashMap::new();
    for (idx, color) in ordered.into_iter().enumerate() {
        mapping.insert(color, idx as u8);
    }

    Ok(mapping)
}

fn map_pixel_to_index(
    pixel: Rgba<u8>,
    palette_map: &HashMap<[u8; 4], u8>,
    options: ChrOptions,
) -> Result<u8> {
    let rgba = pixel.0;
    if options.transparent_index_zero && rgba[3] == 0 {
        return Ok(0);
    }

    if let Some(index) = palette_map.get(&rgba) {
        return Ok(*index & 0x03);
    }

    if options.quantize_if_needed {
        return Ok(((luminance(rgba) as u16 * 3) / 255) as u8);
    }

    bail!("found pixel outside 4-color palette without quantization enabled")
}

fn luminance(pixel: [u8; 4]) -> u8 {
    let r = pixel[0] as u32;
    let g = pixel[1] as u32;
    let b = pixel[2] as u32;
    ((r * 299 + g * 587 + b * 114) / 1000) as u8
}
