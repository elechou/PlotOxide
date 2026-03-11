pub mod axis;
pub mod data;
pub mod sampling;

use std::collections::HashMap;

// ────────────────────────────────────────────────────────────────
//  Background Color Detection
// ────────────────────────────────────────────────────────────────

/// Detect the most common color in the image (quantized to reduce palette).
pub fn detect_background_color(rgba: &[u8], w: u32, h: u32) -> [u8; 3] {
    let mut histogram: HashMap<(u8, u8, u8), u32> = HashMap::new();
    let total = (w as usize) * (h as usize);

    for i in 0..total {
        let off = i * 4;
        if off + 2 >= rgba.len() {
            break;
        }
        let r = rgba[off] >> 5;
        let g = rgba[off + 1] >> 5;
        let b = rgba[off + 2] >> 5;
        *histogram.entry((r, g, b)).or_insert(0) += 1;
    }

    let (best_q, _) = histogram
        .iter()
        .max_by_key(|&(_, &c)| c)
        .unwrap_or((&(7, 7, 7), &0));

    [
        (best_q.0 << 5) | 0x10,
        (best_q.1 << 5) | 0x10,
        (best_q.2 << 5) | 0x10,
    ]
}

// ────────────────────────────────────────────────────────────────
//  Color Distance Utilities
// ────────────────────────────────────────────────────────────────

pub fn color_distance_sq(a: [u8; 3], b: [u8; 3]) -> f32 {
    let dr = a[0] as f32 - b[0] as f32;
    let dg = a[1] as f32 - b[1] as f32;
    let db = a[2] as f32 - b[2] as f32;
    dr * dr + dg * dg + db * db
}

pub fn is_bg_color(pixel: [u8; 3], bg: [u8; 3]) -> bool {
    color_distance_sq(pixel, bg) < 30.0 * 30.0 * 3.0
}
