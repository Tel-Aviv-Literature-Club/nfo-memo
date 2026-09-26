use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use anyhow::{Context, Result};
use fontdb::{Database, Family, Query};
use image::{ImageFormat, Rgba, RgbaImage};
use std::path::Path;

static FONT_BYTES: &[u8] = include_bytes!("../assets/PxPlus_IBM_VGA8.ttf");

pub fn render(text: &str, path: &Path, format: &str, font_family: &str) -> Result<()> {
    let font = load_font(font_family)?;
    let scale = PxScale::from(24.0);
    let scaled = font.as_scaled(scale);
    let cell_width = scaled.h_advance(font.glyph_id('M')).ceil() as u32;
    let line_height = 30u32;
    let padding = 32u32;
    let lines: Vec<&str> = if text.is_empty() {
        vec![""]
    } else {
        text.split('\n').collect()
    };
    let columns = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1)
        .max(1) as u32;
    let width = (padding * 2 + columns * cell_width).min(16_384);
    let height = (padding * 2 + lines.len() as u32 * line_height).min(16_384);
    let mut image = RgbaImage::from_pixel(width, height, Rgba([7, 11, 16, 255]));
    let foreground = [214u8, 255u8, 93u8, 255u8];

    for (row, line) in lines.iter().enumerate() {
        let baseline = padding as f32 + row as f32 * line_height as f32 + scaled.ascent();
        for (column, ch) in line.chars().enumerate() {
            let glyph = font.glyph_id(ch).with_scale_and_position(
                scale,
                point(padding as f32 + column as f32 * cell_width as f32, baseline),
            );
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|x, y, coverage| {
                    let px = bounds.min.x.max(0.0) as u32 + x;
                    let py = bounds.min.y.max(0.0) as u32 + y;
                    if px < width && py < height {
                        let alpha = (coverage * 255.0) as u8;
                        image.put_pixel(
                            px,
                            py,
                            Rgba([foreground[0], foreground[1], foreground[2], alpha]),
                        );
                    }
                });
            }
        }
    }

    let image_format = if format.eq_ignore_ascii_case("webp") {
        ImageFormat::WebP
    } else {
        ImageFormat::Png
    };
    image
        .save_with_format(path, image_format)
        .with_context(|| format!("could not export {}", path.display()))?;
    Ok(())
}

fn load_font(font_family: &str) -> Result<FontArc> {
    if font_family == "PxPlus IBM VGA8" {
        return FontArc::try_from_slice(FONT_BYTES).context("embedded font is invalid");
    }

    let mut database = Database::new();
    database.load_system_fonts();
    let families = [Family::Name(font_family), Family::Monospace];
    let selected = database
        .query(&Query {
            families: &families,
            ..Query::default()
        })
        .and_then(|id| {
            database.with_face_data(id, |bytes, _| FontArc::try_from_vec(bytes.to_vec()).ok())
        })
        .flatten();

    selected
        .or_else(|| FontArc::try_from_slice(FONT_BYTES).ok())
        .context("no usable export font found")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_box_drawing_to_png() {
        let file = tempfile::NamedTempFile::new().unwrap();
        render("╔══╗\n║░▓║\n╚══╝", file.path(), "png", "PxPlus IBM VGA8").unwrap();
        let decoded = image::load_from_memory(&std::fs::read(file.path()).unwrap()).unwrap();
        assert!(decoded.width() > 64);
        assert!(decoded.height() > 64);
    }
}
