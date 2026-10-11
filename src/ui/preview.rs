//! Universal ANSI TrueColor half-block image renderer for terminal previews.
//!
//! Supports both direct vector SVG rendering (via `resvg`) and raster PNG/JPEG
//! thumbnails (via `image`). Renders logos on a clean, high-contrast light card canvas
//! (#f6f7f9) so dark logos, black typography, and colored marks are instantly recognizable
//! on any dark or light terminal.

use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const CARD_BG_R: u8 = 246;
const CARD_BG_G: u8 = 247;
const CARD_BG_B: u8 = 249;

/// Dark banner background for logos that are designed for dark surfaces
/// (e.g. `*-dark.svg` variants with white/light artwork on transparency).
const BANNER_BG_R: u8 = 17;
const BANNER_BG_G: u8 = 17;
const BANNER_BG_B: u8 = 19;

/// Convert image bytes (either SVG XML or raster PNG/JPEG) into ratatui Lines using Unicode half-blocks (▀ and ▄).
///
/// Each terminal character cell is 1 column wide and 1 row high.
/// With half-blocks (▀), 1 character cell renders 2 vertical pixels (top & bottom).
/// This provides crisp, high-density 24-bit TrueColor graphics on any terminal:
/// Antigravity IDE, VS Code, Ghostty, iTerm2, macOS Terminal, Windows Terminal, etc.
pub fn render_halfblocks(bytes: &[u8], max_cols: u16, max_rows: u16) -> Option<Vec<Line<'static>>> {
    // 1. Try rendering as SVG first (handles raw SVG from Wikimedia or disk)
    if let Some(lines) = render_svg(bytes, max_cols, max_rows, false) {
        return Some(lines);
    }
    // 2. Fall back to raster image (PNG, JPEG, WebP, etc.)
    render_raster(bytes, max_cols, max_rows)
}

/// Render a dark-surface logo (e.g. `full_logo_dark.svg`) with transparent terminal background
/// and automatic tight cropping so vector marks scale to their full crisp size on the search screen.
pub fn render_banner(bytes: &[u8], max_cols: u16, max_rows: u16) -> Option<Vec<Line<'static>>> {
    // 1. Try rendering as SVG first (handles raw SVG from disk or memory)
    if let Some(lines) = render_banner_svg(bytes, max_cols, max_rows) {
        return Some(lines);
    }
    // 2. Fall back to raster image (PNG, JPEG, WebP, etc.)
    render_banner_raster(bytes, max_cols, max_rows)
}

fn render_banner_svg(svg_bytes: &[u8], max_cols: u16, max_rows: u16) -> Option<Vec<Line<'static>>> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &opt).ok()?;
    let orig_size = tree.size();
    let (orig_w, orig_h) = (orig_size.width(), orig_size.height());
    if orig_w <= 0.0 || orig_h <= 0.0 {
        return None;
    }

    // Render at high supersampled resolution on a transparent canvas
    let render_w = 1200u32;
    let render_h = ((orig_h * (render_w as f32 / orig_w)).round() as u32).max(1);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(render_w, render_h)?;
    let transform =
        resvg::tiny_skia::Transform::from_scale(render_w as f32 / orig_w, render_h as f32 / orig_h);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Auto-crop: detect tight bounding box of visible artwork
    let mut min_x = render_w;
    let mut max_x = 0;
    let mut min_y = render_h;
    let mut max_y = 0;
    for y in 0..render_h {
        for x in 0..render_w {
            if let Some(px) = pixmap.pixel(x, y) {
                if px.alpha() > 10 {
                    if x < min_x {
                        min_x = x;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    if y < min_y {
                        min_y = y;
                    }
                    if y > max_y {
                        max_y = y;
                    }
                }
            }
        }
    }

    if min_x > max_x || min_y > max_y {
        return None;
    }

    let crop_w = (max_x - min_x + 1).max(1);
    let crop_h = (max_y - min_y + 1).max(1);

    let full_img = image::RgbaImage::from_raw(render_w, render_h, pixmap.data().to_vec())?;
    let cropped = image::imageops::crop_imm(&full_img, min_x, min_y, crop_w, crop_h).to_image();

    let max_cols = (max_cols.max(12) as u32).min(100);
    let max_pixel_rows = (max_rows.max(4) as u32) * 2;

    let scale_x = max_cols as f32 / crop_w as f32;
    let scale_y = max_pixel_rows as f32 / crop_h as f32;
    let scale = scale_x.min(scale_y);

    let target_w = ((crop_w as f32 * scale).round() as u32).clamp(4, max_cols);
    let target_pixel_h = ((crop_h as f32 * scale).round() as u32).clamp(4, max_pixel_rows);

    let downscaled = image::imageops::resize(
        &cropped,
        target_w,
        target_pixel_h,
        image::imageops::FilterType::Lanczos3,
    );

    Some(transparent_image_to_halfblocks(&downscaled, max_rows))
}

fn render_banner_raster(bytes: &[u8], max_cols: u16, max_rows: u16) -> Option<Vec<Line<'static>>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }

    let mut min_x = w;
    let mut max_x = 0;
    let mut min_y = h;
    let mut max_y = 0;
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x, y)[3] > 10 {
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }
    }

    let cropped = if min_x <= max_x && min_y <= max_y {
        let crop_w = max_x - min_x + 1;
        let crop_h = max_y - min_y + 1;
        image::imageops::crop_imm(&img, min_x, min_y, crop_w, crop_h).to_image()
    } else {
        img
    };

    let crop_w = cropped.width();
    let crop_h = cropped.height();

    let max_cols = (max_cols.max(12) as u32).min(100);
    let max_pixel_rows = (max_rows.max(4) as u32) * 2;

    let scale_x = max_cols as f32 / crop_w as f32;
    let scale_y = max_pixel_rows as f32 / crop_h as f32;
    let scale = scale_x.min(scale_y);

    let target_w = ((crop_w as f32 * scale).round() as u32).clamp(4, max_cols);
    let target_pixel_h = ((crop_h as f32 * scale).round() as u32).clamp(4, max_pixel_rows);

    let downscaled = image::imageops::resize(
        &cropped,
        target_w,
        target_pixel_h,
        image::imageops::FilterType::Lanczos3,
    );

    Some(transparent_image_to_halfblocks(&downscaled, max_rows))
}

fn transparent_image_to_halfblocks(img: &image::RgbaImage, max_rows: u16) -> Vec<Line<'static>> {
    let width = img.width();
    let pixel_height = img.height();
    let char_rows = pixel_height.div_ceil(2);
    let vertical_pad = (max_rows as u32).saturating_sub(char_rows) / 2;

    let mut lines = Vec::with_capacity((char_rows + vertical_pad * 2) as usize);

    for _ in 0..vertical_pad {
        lines.push(Line::default());
    }

    for cy in 0..char_rows {
        let py_top = cy * 2;
        let py_bot = cy * 2 + 1;
        let mut spans = Vec::with_capacity(width as usize);

        for px in 0..width {
            let top = img.get_pixel(px, py_top);
            let top_a = top[3];

            let bot = if py_bot < pixel_height {
                img.get_pixel(px, py_bot)
            } else {
                &image::Rgba([0, 0, 0, 0])
            };
            let bot_a = bot[3];

            const ALPHA_THRESHOLD: u8 = 25;

            if top_a < ALPHA_THRESHOLD && bot_a < ALPHA_THRESHOLD {
                spans.push(Span::raw(" "));
            } else if top_a >= ALPHA_THRESHOLD && bot_a < ALPHA_THRESHOLD {
                let r = ((top[0] as u16 * top_a as u16) / 255) as u8;
                let g = ((top[1] as u16 * top_a as u16) / 255) as u8;
                let b = ((top[2] as u16 * top_a as u16) / 255) as u8;
                spans.push(Span::styled("▀", Style::default().fg(Color::Rgb(r, g, b))));
            } else if top_a < ALPHA_THRESHOLD && bot_a >= ALPHA_THRESHOLD {
                let r = ((bot[0] as u16 * bot_a as u16) / 255) as u8;
                let g = ((bot[1] as u16 * bot_a as u16) / 255) as u8;
                let b = ((bot[2] as u16 * bot_a as u16) / 255) as u8;
                spans.push(Span::styled("▄", Style::default().fg(Color::Rgb(r, g, b))));
            } else {
                let fr = ((top[0] as u16 * top_a as u16) / 255) as u8;
                let fg = ((top[1] as u16 * top_a as u16) / 255) as u8;
                let fb = ((top[2] as u16 * top_a as u16) / 255) as u8;
                let br = ((bot[0] as u16 * bot_a as u16) / 255) as u8;
                let bg = ((bot[1] as u16 * bot_a as u16) / 255) as u8;
                let bb = ((bot[2] as u16 * bot_a as u16) / 255) as u8;
                if (fr, fg, fb) == (br, bg, bb) {
                    spans.push(Span::styled(
                        "█",
                        Style::default().fg(Color::Rgb(fr, fg, fb)),
                    ));
                } else {
                    spans.push(Span::styled(
                        "▀",
                        Style::default()
                            .fg(Color::Rgb(fr, fg, fb))
                            .bg(Color::Rgb(br, bg, bb)),
                    ));
                }
            }
        }
        lines.push(Line::from(spans).alignment(Alignment::Center));
    }

    for _ in 0..vertical_pad {
        lines.push(Line::default());
    }

    lines
}

fn render_svg(
    svg_bytes: &[u8],
    max_cols: u16,
    max_rows: u16,
    dark: bool,
) -> Option<Vec<Line<'static>>> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &opt).ok()?;
    let orig_size = tree.size();
    let (orig_w, orig_h) = (orig_size.width(), orig_size.height());
    if orig_w <= 0.0 || orig_h <= 0.0 {
        return None;
    }

    let max_cols = (max_cols.max(12) as u32).min(52);
    let max_pixel_rows = ((max_rows.max(6) as u32) * 2).min(26);

    let scale_x = max_cols as f32 / orig_w;
    let scale_y = max_pixel_rows as f32 / orig_h;
    let scale = scale_x.min(scale_y);

    let target_w = ((orig_w * scale).round() as u32).clamp(4, max_cols);
    let target_pixel_h = ((orig_h * scale).round() as u32).clamp(4, max_pixel_rows);

    // Render at 3x supersampled resolution for razor-sharp vector rasterization
    let super_scale = 3u32;
    let super_w = target_w * super_scale;
    let super_h = target_pixel_h * super_scale;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(super_w, super_h)?;
    let (bg_r, bg_g, bg_b) = if dark {
        (BANNER_BG_R, BANNER_BG_G, BANNER_BG_B)
    } else {
        (CARD_BG_R, CARD_BG_G, CARD_BG_B)
    };
    pixmap.fill(resvg::tiny_skia::Color::from_rgba8(bg_r, bg_g, bg_b, 255));

    let transform =
        resvg::tiny_skia::Transform::from_scale(super_w as f32 / orig_w, super_h as f32 / orig_h);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Downscale from 3x to target using high-quality Lanczos3 filter
    let super_img = image::RgbaImage::from_raw(super_w, super_h, pixmap.data().to_vec())?;
    let downscaled = image::imageops::resize(
        &super_img,
        target_w,
        target_pixel_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mut pixels = Vec::with_capacity((target_w * target_pixel_h) as usize);
    for y in 0..target_pixel_h {
        for x in 0..target_w {
            let px = downscaled.get_pixel(x, y);
            let mut r = px[0] as f32;
            let mut g = px[1] as f32;
            let mut b = px[2] as f32;
            // Contrast boost for typography & dark brand marks. Applied only on the
            // light preview card; dark banners keep their original artwork.
            if !dark {
                let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                if luma < 210.0 {
                    let curve = (luma / 210.0).powf(1.35);
                    r *= curve;
                    g *= curve;
                    b *= curve;
                }
            }
            pixels.push((r.round() as u8, g.round() as u8, b.round() as u8, 255));
        }
    }

    Some(pixels_to_halfblocks(
        &pixels,
        target_w,
        target_pixel_h,
        max_rows,
    ))
}

fn render_raster(bytes: &[u8], max_cols: u16, max_rows: u16) -> Option<Vec<Line<'static>>> {
    render_raster_with_bg(bytes, max_cols, max_rows, CARD_BG_R, CARD_BG_G, CARD_BG_B)
}

fn render_raster_with_bg(
    bytes: &[u8],
    max_cols: u16,
    max_rows: u16,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
) -> Option<Vec<Line<'static>>> {
    let img = image::load_from_memory(bytes).ok()?;
    let (orig_w, orig_h) = (img.width(), img.height());
    if orig_w == 0 || orig_h == 0 {
        return None;
    }

    let max_cols = (max_cols.max(12) as u32).min(52);
    let max_pixel_rows = ((max_rows.max(6) as u32) * 2).min(26);

    let scale_x = max_cols as f32 / orig_w as f32;
    let scale_y = max_pixel_rows as f32 / orig_h as f32;
    let scale = scale_x.min(scale_y);

    let target_w = ((orig_w as f32 * scale).round() as u32).clamp(4, max_cols);
    let target_pixel_h = ((orig_h as f32 * scale).round() as u32).clamp(4, max_pixel_rows);

    let resized = image::imageops::resize(
        &img.to_rgba8(),
        target_w,
        target_pixel_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mut pixels = Vec::with_capacity((target_w * target_pixel_h) as usize);
    for y in 0..target_pixel_h {
        for x in 0..target_w {
            let px = resized.get_pixel(x, y);
            let a = px[3] as u16;
            let r = ((px[0] as u16 * a + bg_r as u16 * (255 - a)) / 255) as u8;
            let g = ((px[1] as u16 * a + bg_g as u16 * (255 - a)) / 255) as u8;
            let b = ((px[2] as u16 * a + bg_b as u16 * (255 - a)) / 255) as u8;
            pixels.push((r, g, b, 255));
        }
    }

    Some(pixels_to_halfblocks(
        &pixels,
        target_w,
        target_pixel_h,
        max_rows,
    ))
}

fn pixels_to_halfblocks(
    pixels: &[(u8, u8, u8, u8)],
    width: u32,
    pixel_height: u32,
    max_rows: u16,
) -> Vec<Line<'static>> {
    let char_rows = pixel_height.div_ceil(2);
    let vertical_pad = (max_rows as u32).saturating_sub(char_rows) / 2;

    let mut lines = Vec::with_capacity((char_rows + vertical_pad * 2) as usize);

    for _ in 0..vertical_pad {
        lines.push(Line::default());
    }

    for cy in 0..char_rows {
        let py_top = cy * 2;
        let py_bot = cy * 2 + 1;
        let mut spans = Vec::with_capacity(width as usize);

        for px in 0..width {
            let top_idx = (py_top * width + px) as usize;
            let top = pixels
                .get(top_idx)
                .copied()
                .unwrap_or((CARD_BG_R, CARD_BG_G, CARD_BG_B, 255));

            let bot = if py_bot < pixel_height {
                let bot_idx = (py_bot * width + px) as usize;
                pixels
                    .get(bot_idx)
                    .copied()
                    .unwrap_or((CARD_BG_R, CARD_BG_G, CARD_BG_B, 255))
            } else {
                (CARD_BG_R, CARD_BG_G, CARD_BG_B, 255)
            };

            let fg = Color::Rgb(top.0, top.1, top.2);
            let bg = Color::Rgb(bot.0, bot.1, bot.2);
            spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
        }
        lines.push(Line::from(spans).alignment(Alignment::Center));
    }

    for _ in 0..vertical_pad {
        lines.push(Line::default());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_halfblocks_generates_lines_for_valid_png() {
        let mut img = image::RgbaImage::new(10, 10);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([255, 0, 0, 255]);
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let bytes = buf.into_inner();

        let lines = render_halfblocks(&bytes, 20, 10);
        assert!(lines.is_some());
        let lines = lines.unwrap();
        assert!(!lines.is_empty());
    }

    #[test]
    fn render_halfblocks_generates_lines_for_valid_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect width="100" height="100" fill="red" />
        </svg>"#;
        let lines = render_halfblocks(svg.as_bytes(), 20, 10);
        assert!(lines.is_some());
        let lines = lines.unwrap();
        assert!(!lines.is_empty());
    }

    #[test]
    fn render_halfblocks_returns_none_on_corrupt_data() {
        let corrupt = b"not a real image or svg";
        assert!(render_halfblocks(corrupt, 20, 10).is_none());
    }

    #[test]
    fn render_banner_renders_dark_logo_without_light_card_boost() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="40">
            <rect width="100" height="40" fill="#ffffff" />
        </svg>"##;
        let lines = render_banner(svg.as_bytes(), 30, 7);
        assert!(lines.is_some());
        let lines = lines.unwrap();
        assert!(!lines.is_empty());
        // White artwork must stay near-white on the dark banner (no luma boost).
        let match_white = lines.iter().flatten().any(|span| {
            matches!(span.style.fg, Some(Color::Rgb(r, g, b)) if r > 200 && g > 200 && b > 200)
        });
        assert!(match_white);
    }

    #[test]
    fn render_banner_returns_none_on_missing_data() {
        assert!(render_banner(b"garbage", 30, 7).is_none());
    }
}
