use crate::state::{DataCurveMode, DataDetectionResult, DetectedColorGroup};

use super::super::pixels::{collect_masked_non_bg_pixels, color_distance_sq};
use super::super::spatial::split_into_connected_components;
use super::curve::sample_curve_points;

pub(super) fn analyze_mask_for_data(
    rgba: &[u8],
    mask: &[bool],
    w: u32,
    h: u32,
    bg_color: [u8; 3],
    tolerance: f32,
) -> DataDetectionResult {
    let pixel_colors = collect_masked_non_bg_pixels(rgba, mask, w, h, bg_color);

    if pixel_colors.is_empty() {
        return DataDetectionResult { groups: Vec::new() };
    }

    let mut centroids: Vec<[f32; 3]> = Vec::new();
    let mut cluster_pixels: Vec<Vec<(u32, u32)>> = Vec::new();

    for &(pixel, x, y) in &pixel_colors {
        let pf = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];

        let mut best_idx: Option<usize> = None;
        let mut best_dist = f32::MAX;
        for (i, centroid) in centroids.iter().enumerate() {
            let dist = color_distance_sq(
                pixel,
                [centroid[0] as u8, centroid[1] as u8, centroid[2] as u8],
            );
            if dist < best_dist {
                best_dist = dist;
                best_idx = Some(i);
            }
        }

        let tol_sq = tolerance * tolerance * 3.0;
        if let Some(idx) = best_idx {
            if best_dist < tol_sq {
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

    let mut groups = Vec::new();

    for (i, pixels) in cluster_pixels.into_iter().enumerate() {
        if pixels.len() < 5 {
            continue;
        }

        let filtered_pixels: Vec<(u32, u32)> = split_into_connected_components(&pixels, 1)
            .into_iter()
            .filter(|component| component.len() >= 3)
            .flatten()
            .collect();
        if filtered_pixels.len() < 5 {
            continue;
        }

        let avg_color = [
            centroids[i][0] as u8,
            centroids[i][1] as u8,
            centroids[i][2] as u8,
        ];

        let sampled = sample_curve_points(&filtered_pixels, 10, w);

        groups.push(DetectedColorGroup {
            color: avg_color,
            pixel_coords: filtered_pixels,
            curve_mode: DataCurveMode::Continuous,
            point_count: 10,
            sampled_points: sampled,
        });
    }

    groups.sort_by(|a, b| b.pixel_coords.len().cmp(&a.pixel_coords.len()));

    DataDetectionResult { groups }
}

#[cfg(test)]
mod tests {
    use super::super::super::super::state::{DataCurveMode, ProjectData, SerializableMask};
    use super::analyze_mask_for_data;
    use std::io::Read;

    // -----------------------------------------------------------------------
    //  Helpers
    // -----------------------------------------------------------------------

    struct PrdxData {
        rgba: Vec<u8>,
        mask: Vec<bool>,
        w: u32,
        h: u32,
        bg_color: [u8; 3],
        tolerance: f32,
    }

    fn load_prdx(path: &str) -> PrdxData {
        let file = std::fs::File::open(path).expect("open prdx");
        let mut archive = zip::ZipArchive::new(file).expect("open zip");

        // Read both entries into memory before processing (avoid borrow conflict)
        let img_bytes = {
            let mut entry = archive.by_name("image.png").expect("image.png");
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("read image");
            buf
        };
        let manifest_str = {
            let mut entry = archive.by_name("manifest.json").expect("manifest");
            let mut buf = String::new();
            entry.read_to_string(&mut buf).expect("read manifest");
            buf
        };

        let img = image::load_from_memory(&img_bytes)
            .expect("decode png")
            .to_rgba8();
        let w = img.width();
        let h = img.height();
        let rgba: Vec<u8> = img.into_raw();

        let project: ProjectData =
            serde_json::from_str(&manifest_str).expect("parse manifest");

        let data_mask: SerializableMask = project.data_mask.expect("data_mask present");
        assert_eq!(data_mask.width, w);
        assert_eq!(data_mask.height, h);
        let mask = data_mask.to_buffer();
        let bg_color = crate::recognition::detect_background_color(&rgba, w, h);
        let tolerance = data_mask.color_tolerance;

        PrdxData {
            rgba,
            mask,
            w,
            h,
            bg_color,
            tolerance,
        }
    }

    fn arc_length(pts: &[(f32, f32)]) -> f32 {
        pts.windows(2)
            .map(|w| {
                let dx = w[1].0 - w[0].0;
                let dy = w[1].1 - w[0].1;
                (dx * dx + dy * dy).sqrt()
            })
            .sum()
    }

    fn is_open_curve(pts: &[(f32, f32)]) -> bool {
        if pts.len() < 2 {
            return true;
        }
        let total = arc_length(pts);
        if total < 1.0 {
            return true;
        }
        let dx = pts.last().unwrap().0 - pts[0].0;
        let dy = pts.last().unwrap().1 - pts[0].1;
        let endpoint_dist = (dx * dx + dy * dy).sqrt();
        endpoint_dist > total * 0.2
    }

    // -----------------------------------------------------------------------
    //  Real-world test: quadratic curve from test1.prdx
    // -----------------------------------------------------------------------

    #[test]
    fn real_quadratic_curve_detected_as_open() {
        let data = load_prdx("recognition_test/test1.prdx");
        let result = analyze_mask_for_data(
            &data.rgba,
            &data.mask,
            data.w,
            data.h,
            data.bg_color,
            data.tolerance,
        );

        assert!(!result.groups.is_empty(), "expected at least 1 group");
        let group = &result.groups[0];

        assert!(matches!(group.curve_mode, DataCurveMode::Continuous));
        assert!(group.sampled_points.len() >= 8, "too few points");

        // The quadratic curve must NOT be detected as a closed loop
        assert!(
            is_open_curve(&group.sampled_points),
            "quadratic curve must be open. first: {:?}, last: {:?}, arc_length: {}",
            group.sampled_points.first(),
            group.sampled_points.last(),
            arc_length(&group.sampled_points)
        );
    }

    #[test]
    fn real_quadratic_curve_spans_image() {
        let data = load_prdx("recognition_test/test1.prdx");
        let result = analyze_mask_for_data(
            &data.rgba,
            &data.mask,
            data.w,
            data.h,
            data.bg_color,
            data.tolerance,
        );

        let pts = &result.groups[0].sampled_points;
        let min_x = pts.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_x = pts.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let x_span = max_x - min_x;

        assert!(
            x_span > 500.0,
            "x_span {} too small (min={}, max={})",
            x_span,
            min_x,
            max_x
        );
    }

    #[test]
    fn real_quadratic_curve_endpoint_ratio() {
        let data = load_prdx("recognition_test/test1.prdx");
        let result = analyze_mask_for_data(
            &data.rgba,
            &data.mask,
            data.w,
            data.h,
            data.bg_color,
            data.tolerance,
        );

        let group = &result.groups[0];
        let pts = &group.sampled_points;

        eprintln!("=== Sampled points ({}) ===", pts.len());
        for (i, p) in pts.iter().enumerate() {
            eprintln!("  [{}] ({:.1}, {:.1})", i, p.0, p.1);
        }

        let total = arc_length(pts);
        let dx = pts.last().unwrap().0 - pts[0].0;
        let dy = pts.last().unwrap().1 - pts[0].1;
        let endpoint_dist = (dx * dx + dy * dy).sqrt();
        let ratio = endpoint_dist / total;

        eprintln!(
            "\nendpoint_dist: {:.1}, arc_length: {:.1}, ratio: {:.3}",
            endpoint_dist, total, ratio
        );
        eprintln!(
            "pixel_coords count: {}",
            group.pixel_coords.len()
        );

        // Check if points form a loop by looking at all consecutive distances
        let mut max_gap = 0.0f32;
        for i in 1..pts.len() {
            let d = ((pts[i].0 - pts[i - 1].0).powi(2) + (pts[i].1 - pts[i - 1].1).powi(2)).sqrt();
            if d > max_gap {
                max_gap = d;
            }
        }
        // Also check the closing gap (last → first)
        let closing_gap = ((pts[0].0 - pts.last().unwrap().0).powi(2)
            + (pts[0].1 - pts.last().unwrap().1).powi(2))
        .sqrt();
        eprintln!("max consecutive gap: {:.1}, closing gap: {:.1}", max_gap, closing_gap);

        // If closing gap is similar to other gaps, it's likely treated as a loop
        if closing_gap < max_gap * 1.5 {
            eprintln!("WARNING: closing gap is small — curve likely sampled as closed loop");
        }

        assert!(
            ratio > 0.15,
            "ratio {:.3} too low — curve likely misidentified as closed",
            ratio
        );
    }
}
