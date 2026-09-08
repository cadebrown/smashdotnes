use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{bail, Context, Result};
use image::ImageReader;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotInspection {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub unique_colors: usize,
    pub dominant_color_ratio: f64,
}

pub fn inspect_screenshot(path: &Path) -> Result<ScreenshotInspection> {
    let image = ImageReader::open(path)
        .with_context(|| format!("failed to open screenshot {}", path.display()))?
        .decode()
        .with_context(|| format!("failed to decode screenshot {}", path.display()))?
        .to_rgba8();
    let width = image.width();
    let height = image.height();
    if width < 256 || height < 240 {
        bail!(
            "screenshot {} is {}x{}, expected at least 256x240",
            path.display(),
            width,
            height
        );
    }

    let mut colors = HashSet::new();
    let mut counts = HashMap::new();
    for pixel in image.pixels() {
        let rgba = pixel.0;
        colors.insert(rgba);
        *counts.entry(rgba).or_insert(0usize) += 1;
    }
    let pixels = width as usize * height as usize;
    let dominant = counts.values().copied().max().unwrap_or(pixels);
    let dominant_color_ratio = dominant as f64 / pixels as f64;
    if colors.len() < 4 || dominant_color_ratio >= 0.99 {
        bail!(
            "screenshot {} is visually empty ({} colors, {:.2}% dominant)",
            path.display(),
            colors.len(),
            dominant_color_ratio * 100.0
        );
    }

    Ok(ScreenshotInspection {
        path: path.display().to_string(),
        width,
        height,
        unique_colors: colors.len(),
        dominant_color_ratio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn rejects_placeholder_and_accepts_real_frame() {
        let dir = tempfile::tempdir().unwrap();
        let placeholder = dir.path().join("placeholder.png");
        RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 255]))
            .save(&placeholder)
            .unwrap();
        assert!(inspect_screenshot(&placeholder).is_err());

        let frame = dir.path().join("frame.png");
        let mut image = RgbaImage::from_pixel(256, 240, Rgba([0, 0, 0, 255]));
        for y in 40..200 {
            for x in 40..216 {
                let color = match (x + y) % 4 {
                    0 => [255, 64, 32, 255],
                    1 => [32, 255, 64, 255],
                    2 => [255, 255, 255, 255],
                    _ => [32, 32, 32, 255],
                };
                image.put_pixel(x, y, Rgba(color));
            }
        }
        image.save(&frame).unwrap();
        assert_eq!(inspect_screenshot(&frame).unwrap().width, 256);
    }
}
