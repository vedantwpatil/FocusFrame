use image::GenericImageView;
use std::error::Error;

pub struct CursorSprite {
    pub data: Vec<u8>, // Raw RGBA8 bytes
    pub width: u32,
    pub height: u32,
}

pub fn load_cursor_sprite(path: &str) -> Result<CursorSprite, Box<dyn Error>> {
    let img = image::open(path)?;
    let (width, height) = img.dimensions();
    // Pre-convert to raw RGBA bytes for O(1) access
    let data = img.to_rgba8().into_raw();

    Ok(CursorSprite {
        data,
        width,
        height,
    })
}

/// Composite cursor onto RGBA frame buffer with sub-pixel accuracy
pub fn composite_cursor_subpixel(
    frame: &mut [u8],
    frame_width: u32,
    _frame_height: u32, // Unused in optimized math
    cursor: &CursorSprite,
    x: f32,
    y: f32,
) {
    // 1. Determine the integer bounding box on the FRAME
    let start_x = x.floor() as i32;
    let start_y = y.floor() as i32;
    let end_x = start_x + cursor.width as i32 + 1; // +1 for bilinear spill
    let end_y = start_y + cursor.height as i32 + 1;

    // 2. Clamp to frame boundaries
    let draw_start_x = start_x.max(0);
    let draw_start_y = start_y.max(0);
    let draw_end_x = end_x.min(frame_width as i32);
    let draw_end_y = end_y.min((frame.len() / (frame_width as usize * 4)) as i32);

    // 3. Iterate DESTINATION pixels (Gather)
    for dy in draw_start_y..draw_end_y {
        for dx in draw_start_x..draw_end_x {
            // Map pixel center back to cursor space
            let src_x = (dx as f32) - x;
            let src_y = (dy as f32) - y;

            // Bilinear Sample
            if let Some((r, g, b, a)) = sample_bilinear_fast(cursor, src_x, src_y) {
                let alpha = a as f32 / 255.0;
                if alpha > 0.0 {
                    let idx = ((dy as u32 * frame_width + dx as u32) * 4) as usize;
                    // Standard Over Operator
                    frame[idx] = blend(frame[idx], r, alpha);
                    frame[idx + 1] = blend(frame[idx + 1], g, alpha);
                    frame[idx + 2] = blend(frame[idx + 2], b, alpha);
                    // Assume frame alpha stays 255 (opaque video)
                }
            }
        }
    }
}

#[inline(always)]
fn blend(bg: u8, fg: u8, alpha: f32) -> u8 {
    ((bg as f32 * (1.0 - alpha)) + (fg as f32 * alpha)) as u8
}

#[inline]
#[allow(dead_code)]
fn _blend_channel(bg: u8, fg: u8, alpha: f32) -> u8 {
    ((bg as f32 * (1.0 - alpha)) + (fg as f32 * alpha)).clamp(0.0, 255.0) as u8
}

#[inline(always)]
fn sample_bilinear_fast(cursor: &CursorSprite, x: f32, y: f32) -> Option<(u8, u8, u8, u8)> {
    // 1. Bounds Check (Strict)
    // We allow -0.5 to allow sampling the center of the edge pixels,
    // but we must not access data outside valid indices.
    if x < -0.5 || y < -0.5 || x >= (cursor.width as f32 - 0.5) || y >= (cursor.height as f32 - 0.5)
    {
        return None;
    }

    // 2. Identify neighbors (Top-Left coordinate)
    let x_floor = x.floor();
    let y_floor = y.floor();

    // Indices for Top-Left (tl)
    let tl_x = x_floor as i32;
    let tl_y = y_floor as i32;

    // Weights (fractional part)
    let u = x - x_floor; // Weight for Right
    let v = y - y_floor; // Weight for Bottom
    let inv_u = 1.0 - u; // Weight for Left
    let inv_v = 1.0 - v; // Weight for Top

    // Helper to safely get a pixel (handling edges where x+1 might be OOB)
    // In valid range due to strict bounds check above, but safe clamping is good practice
    let get_pixel = |cx: i32, cy: i32| -> (f32, f32, f32, f32) {
        let cx = cx.clamp(0, cursor.width as i32 - 1) as usize;
        let cy = cy.clamp(0, cursor.height as i32 - 1) as usize;
        let idx = (cy * cursor.width as usize + cx) * 4;
        (
            cursor.data[idx] as f32,     // R
            cursor.data[idx + 1] as f32, // G
            cursor.data[idx + 2] as f32, // B
            cursor.data[idx + 3] as f32, // A
        )
    };

    // 3. Fetch 4 Neighbors
    let tl = get_pixel(tl_x, tl_y);
    let tr = get_pixel(tl_x + 1, tl_y);
    let bl = get_pixel(tl_x, tl_y + 1);
    let br = get_pixel(tl_x + 1, tl_y + 1);

    // 4. Interpolate Each Channel
    // Formula: P = TL*(1-u)(1-v) + TR*u(1-v) + BL*(1-u)v + BR*uv
    let interp = |c_tl, c_tr, c_bl, c_br| -> u8 {
        let top = c_tl * inv_u + c_tr * u;
        let bot = c_bl * inv_u + c_br * u;
        let val = top * inv_v + bot * v;
        val as u8
    };

    Some((
        interp(tl.0, tr.0, bl.0, br.0),
        interp(tl.1, tr.1, bl.1, br.1),
        interp(tl.2, tr.2, bl.2, br.2),
        interp(tl.3, tr.3, bl.3, br.3),
    ))
}
