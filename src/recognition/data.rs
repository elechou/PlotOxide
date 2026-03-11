use crate::state::{DataCurveMode, DataDetectionResult, DetectedColorGroup};

use super::is_bg_color;
use super::sampling::sample_points_from_cluster;

// ────────────────────────────────────────────────────────────────
//  Data Recognition: Color Clustering with Tolerance
// ────────────────────────────────────────────────────────────────

pub fn analyze_mask_for_data(
    rgba: &[u8],
    mask: &[bool],
    w: u32,
    h: u32,
    bg_color: [u8; 3],
    tolerance: f32,
) -> DataDetectionResult {
    let w_us = w as usize;
    let h_us = h as usize;

    // Step 1: Collect all non-background pixels in masked region
    let mut pixel_colors: Vec<([u8; 3], u32, u32)> = Vec::new();

    for y in 0..h_us {
        for x in 0..w_us {
            let idx = y * w_us + x;
            if !mask[idx] {
                continue;
            }
            let off = idx * 4;
            if off + 2 >= rgba.len() {
                continue;
            }
            let pixel = [rgba[off], rgba[off + 1], rgba[off + 2]];
            if is_bg_color(pixel, bg_color) {
                continue;
            }
            pixel_colors.push((pixel, x as u32, y as u32));
        }
    }

    if pixel_colors.is_empty() {
        return DataDetectionResult { groups: Vec::new() };
    }

    // Step 2: Cluster using user-adjustable tolerance
    let mut centroids: Vec<[f32; 3]> = Vec::new();
    let mut cluster_pixels: Vec<Vec<(u32, u32)>> = Vec::new();

    for &(pixel, x, y) in &pixel_colors {
        let pf = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];

        // Find nearest centroid within tolerance
        let mut best_idx: Option<usize> = None;
        let mut best_dist = f32::MAX;
        for (i, centroid) in centroids.iter().enumerate() {
            let dr = pf[0] - centroid[0];
            let dg = pf[1] - centroid[1];
            let db = pf[2] - centroid[2];
            let dist = dr * dr + dg * dg + db * db;
            if dist < best_dist {
                best_dist = dist;
                best_idx = Some(i);
            }
        }

        let tol_sq = tolerance * tolerance * 3.0;
        if let Some(idx) = best_idx {
            if best_dist < tol_sq {
                // Add to existing cluster
                let n = cluster_pixels[idx].len() as f32;
                centroids[idx][0] = (centroids[idx][0] * n + pf[0]) / (n + 1.0);
                centroids[idx][1] = (centroids[idx][1] * n + pf[1]) / (n + 1.0);
                centroids[idx][2] = (centroids[idx][2] * n + pf[2]) / (n + 1.0);
                cluster_pixels[idx].push((x, y));
            } else {
                centroids.push(pf);
                cluster_pixels.push(vec![(x, y)]);
            }
        } else {
            centroids.push(pf);
            cluster_pixels.push(vec![(x, y)]);
        }
    }

    // Step 3: Build groups from clusters (skip noise clusters < 5 pixels)
    let mut groups: Vec<DetectedColorGroup> = Vec::new();

    for (i, pixels) in cluster_pixels.into_iter().enumerate() {
        if pixels.len() < 5 {
            continue;
        }

        let avg_color = [
            centroids[i][0] as u8,
            centroids[i][1] as u8,
            centroids[i][2] as u8,
        ];

        let sampled = sample_points_from_cluster(&pixels, 10, w);

        groups.push(DetectedColorGroup {
            color: avg_color,
            pixel_coords: pixels,
            curve_mode: DataCurveMode::Continuous,
            point_count: 10,
            sampled_points: sampled,
        });
    }

    groups.sort_by(|a, b| b.pixel_coords.len().cmp(&a.pixel_coords.len()));

    DataDetectionResult { groups }
}
