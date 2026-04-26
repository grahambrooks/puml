//! SVG → PNG rasterisation via resvg.
//!
//! Used by the CLI when `--output` ends in `.png`. Pure Rust path: parse
//! the SVG with `usvg`, render to a `tiny_skia::Pixmap`, encode as PNG.
//! No system fonts, no Cairo, no librsvg.

use anyhow::{anyhow, Context, Result};

/// Rasterise an SVG document string to PNG bytes.
///
/// `scale` multiplies the SVG's intrinsic pixel size — pass 1.0 for a 1:1
/// pixel-for-pixel render, 2.0 for a retina-sharp render. Values <0.1 are
/// rejected to avoid degenerate zero-pixel images on accidental misuse.
pub fn svg_to_png(svg: &str, scale: f32) -> Result<Vec<u8>> {
    if !(scale.is_finite() && scale >= 0.1) {
        return Err(anyhow!("scale must be a finite number ≥ 0.1, got {scale}"));
    }

    // Load system fonts so the text inside class boxes, sequence labels,
    // etc. actually rasterises. Without a populated fontdb, usvg silently
    // drops every <text> element — the visible "no labels in any PNG"
    // bug. `load_system_fonts` walks platform font directories once and
    // takes ~50–200 ms; the cost is acceptable for a one-shot CLI render.
    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    let opts = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..resvg::usvg::Options::default()
    };
    let tree = resvg::usvg::Tree::from_str(svg, &opts).context("parsing SVG with usvg")?;

    let size = tree.size().to_int_size();
    let pixel_w = ((size.width() as f32) * scale).round() as u32;
    let pixel_h = ((size.height() as f32) * scale).round() as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixel_w, pixel_h).ok_or_else(|| {
        anyhow!(
            "PNG canvas dimensions {pixel_w}x{pixel_h} are out of range — \
                 try a smaller --scale"
        )
    })?;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap.encode_png().context("encoding pixmap as PNG")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><rect width="40" height="20" fill="#abcdef"/></svg>"##;

    #[test]
    fn renders_basic_svg_to_png_bytes() {
        let bytes = svg_to_png(TINY_SVG, 1.0).expect("render");
        // PNG magic header: 89 50 4E 47 0D 0A 1A 0A
        assert!(
            bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            "output should start with the PNG magic bytes"
        );
    }

    #[test]
    fn scale_factor_changes_pixel_size() {
        // Same SVG at different scales should produce different PNG byte
        // lengths (a 2x render has ~4x the pixels).
        let one = svg_to_png(TINY_SVG, 1.0).expect("render 1x");
        let two = svg_to_png(TINY_SVG, 2.0).expect("render 2x");
        assert!(
            two.len() > one.len(),
            "2x render ({} bytes) should be larger than 1x ({} bytes)",
            two.len(),
            one.len()
        );
    }

    #[test]
    fn rejects_invalid_scale() {
        assert!(svg_to_png(TINY_SVG, 0.0).is_err());
        assert!(svg_to_png(TINY_SVG, -1.0).is_err());
        assert!(svg_to_png(TINY_SVG, f32::NAN).is_err());
    }
}
