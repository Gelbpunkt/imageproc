use crate::definitions::{Clamp, Image};
use crate::drawing::Canvas;
use conv::ValueInto;
use image::{GenericImage, ImageBuffer, Pixel};
use std::f32;

use crate::pixelops::weighted_sum;
use ab_glyph::{point, Font, Glyph, Point, PxScale, ScaleFont};

/// Simple paragraph layout for glyphs into `target`.
/// Taken from https://github.com/alexheretic/ab-glyph/blob/master/dev/src/layout.rs
pub fn layout_paragraph<F, SF>(font: SF, position: Point, text: &str, target: &mut Vec<Glyph>)
where
    F: Font,
    SF: ScaleFont<F>,
{
    let v_advance = font.height() + font.line_gap();
    let mut caret = position + point(0.0, font.ascent());
    let mut last_glyph: Option<Glyph> = None;
    for c in text.chars() {
        if c.is_control() {
            if c == '\n' {
                caret = point(position.x, caret.y + v_advance);
                last_glyph = None;
            }
            continue;
        }
        let mut glyph = font.scaled_glyph(c);
        if let Some(previous) = last_glyph.take() {
            caret.x += font.kern(previous.id, glyph.id);
        }
        glyph.position = caret;

        last_glyph = Some(glyph.clone());
        caret.x += font.h_advance(glyph.id);

        target.push(glyph);
    }
}

/// Draws colored text on an image in place. `scale` is augmented font scaling on both the x and y axis (in pixels). Note that this function *does not* support newlines, you must do this manually
pub fn draw_text_mut<'a, C, F>(
    canvas: &'a mut C,
    color: C::Pixel,
    x: u32,
    y: u32,
    scale: PxScale,
    max_width: u32,
    font: F,
    text: &'a str,
) where
    C: Canvas,
    <C::Pixel as Pixel>::Subpixel: ValueInto<f32> + Clamp<f32>,
    F: Font,
{
    let scaled_font = font.as_scaled(scale);
    let mut glyphs = Vec::new();
    let position = point(x as f32, y as f32);
    layout_paragraph(scaled_font, position, text, &mut glyphs);

    let last_glyph = &glyphs[glyphs.len() - 1];
    let actual_width = last_glyph.position.x + last_glyph.scale.x;
    if actual_width > max_width as f32 {
        let shrink_factor = actual_width / (max_width as f32);
        let new_scale = PxScale {
            x: scale.x / shrink_factor,
            y: scale.y,
        };
        glyphs.clear();
        let rescaled_font = font.as_scaled(new_scale);
        layout_paragraph(rescaled_font, position, text, &mut glyphs);
    }

    // Loop through the glyphs in the text, positing each one on a line
    for glyph in glyphs {
        if let Some(outlined) = scaled_font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            // Draw the glyph into the image per-pixel by using the draw closure
            outlined.draw(|x, y, v| {
                // Offset the position by the glyph bounding box
                let pixel_x = x + bounds.min.x as u32;
                let pixel_y = y + bounds.min.y as u32;
                let px = canvas.get_pixel(pixel_x, pixel_y);
                let weighted_color = weighted_sum(px, color, 1.0 - v, v);
                // Turn the coverage into an alpha value (blended with any previous)
                canvas.draw_pixel(pixel_x, pixel_y, weighted_color);
            });
        }
    }
}

/// Draws colored text on an image in place. `scale` is augmented font scaling on both the x and y axis (in pixels). Note that this function *does not* support newlines, you must do this manually
pub fn draw_text<'a, I, F>(
    image: &'a mut I,
    color: I::Pixel,
    x: u32,
    y: u32,
    scale: PxScale,
    max_width: u32,
    font: F,
    text: &'a str,
) -> Image<I::Pixel>
where
    I: GenericImage,
    <I::Pixel as Pixel>::Subpixel: ValueInto<f32> + Clamp<f32>,
    I::Pixel: 'static,
    F: Font,
{
    let mut out = ImageBuffer::new(image.width(), image.height());
    out.copy_from(image, 0, 0).unwrap();
    draw_text_mut(&mut out, color, x, y, scale, max_width, font, text);
    out
}
