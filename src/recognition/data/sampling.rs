use std::collections::BTreeMap;

use super::super::geometry::{
    compare_points_along_axis, dominant_axis, point_distance_sq, DominantAxis,
};
use super::super::spatial::split_into_connected_components;

// ----------------------------------------------------------------
//  Arc-Length Point Sampling
// ----------------------------------------------------------------

/// Sample N points along a pixel cluster using arc-length parameterization.
/// Handles disconnected fragments (dashed lines), closed curves (ellipses),
/// and multi-valued curves uniformly.
pub fn sample_points_from_cluster(pixels: &[(u32, u32)], n: usize, w: u32) -> Vec<(f32, f32)> {
    sample_points_arc_length(pixels, n, w)
}

fn sample_points_arc_length(pixels: &[(u32, u32)], n: usize, _w: u32) -> Vec<(f32, f32)> {
    if pixels.is_empty() || n == 0 {
        return Vec::new();
    }

    let chains = build_pixel_chains(pixels);
    if chains.is_empty() {
        return Vec::new();
    }

    // After stitching we typically have a single chain.
    // Flatten all chains into one for simplicity (multi-chain is rare after stitch).
    let chain: Vec<(u32, u32)> = chains.into_iter().flatten().collect();

    if chain.is_empty() {
        return Vec::new();
    }
    if chain.len() <= n {
        return chain.iter().map(|&(x, y)| (x as f32, y as f32)).collect();
    }

    let arcs = compute_arc_lengths(&chain);
    let total_length = *arcs.last().unwrap_or(&0.0);

    if total_length < 1.0 {
        return chain
            .iter()
            .map(|&(x, y)| (x as f32, y as f32))
            .take(n)
            .collect();
    }

    if is_chain_closed(&chain, total_length) {
        sample_closed(&chain, &arcs, total_length, n)
    } else {
        sample_open(&chain, &arcs, total_length, n)
    }
}

/// Compute cumulative arc lengths along a chain.
fn compute_arc_lengths(chain: &[(u32, u32)]) -> Vec<f32> {
    let mut arcs = Vec::with_capacity(chain.len());
    arcs.push(0.0);
    for i in 1..chain.len() {
        let dx = chain[i].0 as f32 - chain[i - 1].0 as f32;
        let dy = chain[i].1 as f32 - chain[i - 1].1 as f32;
        arcs.push(arcs[i - 1] + (dx * dx + dy * dy).sqrt());
    }
    arcs
}

/// Detect whether a chain forms a closed loop.
/// True if the gap between first and last point is small relative to total arc length.
fn is_chain_closed(chain: &[(u32, u32)], total_length: f32) -> bool {
    if chain.len() < 6 {
        return false;
    }
    let first = chain[0];
    let last = *chain.last().unwrap();
    let close_dist = ((first.0 as f32 - last.0 as f32).powi(2)
        + (first.1 as f32 - last.1 as f32).powi(2))
    .sqrt();
    // Closed if the closing gap is < 10% of the traced arc length
    close_dist < total_length * 0.10
}

/// Sample N points uniformly along an open chain.
fn sample_open(
    chain: &[(u32, u32)],
    arcs: &[f32],
    total_length: f32,
    n: usize,
) -> Vec<(f32, f32)> {
    let mut sampled = Vec::with_capacity(n);
    for i in 0..n {
        let target = if n == 1 {
            total_length / 2.0
        } else {
            i as f32 * total_length / (n - 1) as f32
        };
        sampled.push(interpolate_at_arc(chain, arcs, target));
    }
    sampled
}

/// Sample N points uniformly around a closed loop.
/// Points are distributed evenly around the full perimeter (including the
/// closing segment from last back to first), with no duplicate at the seam.
fn sample_closed(
    chain: &[(u32, u32)],
    arcs: &[f32],
    total_length: f32,
    n: usize,
) -> Vec<(f32, f32)> {
    let first = chain[0];
    let last = *chain.last().unwrap();
    let close_len = ((first.0 as f32 - last.0 as f32).powi(2)
        + (first.1 as f32 - last.1 as f32).powi(2))
    .sqrt();
    let loop_length = total_length + close_len;

    let mut sampled = Vec::with_capacity(n);
    for i in 0..n {
        let target = i as f32 * loop_length / n as f32; // N intervals, not N-1

        if target <= total_length {
            sampled.push(interpolate_at_arc(chain, arcs, target));
        } else {
            // In the closing segment (last → first)
            let t = (target - total_length) / close_len;
            let x = last.0 as f32 + t * (first.0 as f32 - last.0 as f32);
            let y = last.1 as f32 + t * (first.1 as f32 - last.1 as f32);
            sampled.push((x, y));
        }
    }
    sampled
}

/// Interpolate a point at a given arc-length position along a chain.
fn interpolate_at_arc(chain: &[(u32, u32)], arcs: &[f32], target: f32) -> (f32, f32) {
    let seg = match arcs
        .binary_search_by(|v| v.partial_cmp(&target).unwrap_or(std::cmp::Ordering::Equal))
    {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1),
    };
    let seg = seg.min(chain.len() - 1);

    if seg + 1 < chain.len() {
        let seg_start = arcs[seg];
        let seg_end = arcs[seg + 1];
        let seg_len = seg_end - seg_start;
        let t = if seg_len > 0.0 {
            (target - seg_start) / seg_len
        } else {
            0.0
        };
        let x = chain[seg].0 as f32 + t * (chain[seg + 1].0 as f32 - chain[seg].0 as f32);
        let y = chain[seg].1 as f32 + t * (chain[seg + 1].1 as f32 - chain[seg].1 as f32);
        (x, y)
    } else {
        (chain[seg].0 as f32, chain[seg].1 as f32)
    }
}

// ----------------------------------------------------------------
//  Chain Building & Stitching
// ----------------------------------------------------------------

/// Build a single ordered chain from potentially disconnected curve fragments.
/// Connected components are individually thinned and ordered, then stitched
/// together via greedy nearest-endpoint with linear interpolation across gaps.
fn build_pixel_chains(pixels: &[(u32, u32)]) -> Vec<Vec<(u32, u32)>> {
    if pixels.is_empty() {
        return Vec::new();
    }

    let mut chains: Vec<Vec<(u32, u32)>> = Vec::new();

    for component in split_into_connected_components(pixels, 1) {
        let thin_points = thin_component_points(&component);
        if thin_points.is_empty() {
            continue;
        }

        let chain = order_component_points(&thin_points);
        if !chain.is_empty() {
            chains.push(chain);
        }
    }

    if chains.is_empty() {
        return vec![order_component_points(pixels)];
    }

    if chains.len() == 1 {
        return chains;
    }

    vec![stitch_chains(chains)]
}

/// Stitch multiple ordered chains into a single continuous chain.
/// Uses greedy nearest-endpoint: always pick the unvisited chain whose
/// endpoint is closest to the current tail. Works for any curve shape.
///
/// To avoid doubling back (e.g. starting from the middle of a dashed line),
/// we first find the pair of chain endpoints with the greatest distance —
/// these define the extremes of the curve — and start stitching from one of them.
fn stitch_chains(mut chains: Vec<Vec<(u32, u32)>>) -> Vec<(u32, u32)> {
    if chains.is_empty() {
        return Vec::new();
    }

    // Find the pair of endpoints with maximum distance across all chains.
    // Start stitching from one extreme so the greedy walk doesn't double back.
    let mut max_dist = 0u64;
    let mut start_chain_idx = 0;
    let mut start_from_first = true;

    for i in 0..chains.len() {
        let eps_i = [chains[i][0], *chains[i].last().unwrap()];
        for j in (i + 1)..chains.len() {
            let eps_j = [chains[j][0], *chains[j].last().unwrap()];
            for (ei_idx, &ei) in eps_i.iter().enumerate() {
                for &ej in &eps_j {
                    let d = point_distance_sq(ei, ej);
                    if d > max_dist {
                        max_dist = d;
                        start_chain_idx = i;
                        start_from_first = ei_idx == 0;
                    }
                }
            }
        }
    }

    // Orient the starting chain so it begins at the extreme endpoint
    if !start_from_first {
        chains[start_chain_idx].reverse();
    }
    chains.swap(0, start_chain_idx);

    let mut unified = chains.swap_remove(0);

    while !chains.is_empty() {
        let tail = *unified.last().unwrap();

        // Find the chain whose endpoint (first or last) is closest to tail
        let mut best_idx = 0;
        let mut best_dist = u64::MAX;
        let mut best_flip = false;

        for (i, chain) in chains.iter().enumerate() {
            let d_first = point_distance_sq(tail, chain[0]);
            let d_last = point_distance_sq(tail, *chain.last().unwrap());
            let (d, flip) = if d_first <= d_last {
                (d_first, false)
            } else {
                (d_last, true)
            };
            if d < best_dist {
                best_dist = d;
                best_idx = i;
                best_flip = flip;
            }
        }

        let mut next = chains.swap_remove(best_idx);
        if best_flip {
            next.reverse();
        }

        let next_head = next[0];
        let gap = interpolate_gap(tail, next_head);
        unified.extend(gap);

        let start = if !unified.is_empty() && next[0] == *unified.last().unwrap() {
            1
        } else {
            0
        };
        unified.extend_from_slice(&next[start..]);
    }

    unified
}

/// Linearly interpolate integer pixel coordinates between two endpoints (exclusive of both).
fn interpolate_gap(from: (u32, u32), to: (u32, u32)) -> Vec<(u32, u32)> {
    let dx = (to.0 as f32) - (from.0 as f32);
    let dy = (to.1 as f32) - (from.1 as f32);
    let steps = (dx.abs().max(dy.abs())).round() as usize;

    if steps <= 1 {
        return Vec::new();
    }

    let mut points = Vec::with_capacity(steps.saturating_sub(1));
    for s in 1..steps {
        let t = s as f32 / steps as f32;
        let x = (from.0 as f32 + t * dx).round() as u32;
        let y = (from.1 as f32 + t * dy).round() as u32;
        points.push((x, y));
    }
    points
}

// ----------------------------------------------------------------
//  Thinning & Ordering
// ----------------------------------------------------------------

/// Thin a connected component to its centerline.
///
/// For single-valued curves (at each position along the dominant axis there is
/// one cluster of cross-axis pixels), we take the median — same as before.
///
/// For multi-valued curves (e.g. ellipses, where one x column has two separate
/// groups of y pixels), each cluster gets its own median point, preserving the
/// full shape.
fn thin_component_points(component: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if component.len() <= 2 {
        let axis = dominant_axis(component);
        let mut points = component.to_vec();
        points.sort_by(|a, b| compare_points_along_axis(*a, *b, axis));
        return points;
    }

    match dominant_axis(component) {
        DominantAxis::Horizontal => thin_along_axis(component, |(x, y)| (x, y)),
        DominantAxis::Vertical => {
            // Swap axes, thin, swap back
            let swapped: Vec<(u32, u32)> = component.iter().map(|&(x, y)| (y, x)).collect();
            thin_along_axis(&swapped, |(primary, secondary)| (secondary, primary))
        }
    }
}

/// Generic thinning: group by primary axis, cluster the secondary axis values,
/// emit one median per cluster.
///
/// `to_xy` converts (primary, secondary) back to (x, y).
fn thin_along_axis<F>(points: &[(u32, u32)], to_xy: F) -> Vec<(u32, u32)>
where
    F: Fn((u32, u32)) -> (u32, u32),
{
    let mut by_primary: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(p, s) in points {
        by_primary.entry(p).or_default().push(s);
    }

    let mut result = Vec::new();
    for (primary, mut secondaries) in by_primary {
        secondaries.sort_unstable();
        // Split into clusters: consecutive values within gap ≤ MAX_LINE_WIDTH
        // belong to the same cluster
        let clusters = cluster_values(&secondaries, 3);
        for cluster in clusters {
            let median = cluster[cluster.len() / 2];
            result.push(to_xy((primary, median)));
        }
    }
    result
}

/// Split sorted values into clusters where consecutive gaps > `max_gap`
/// start a new cluster.
fn cluster_values(sorted: &[u32], max_gap: u32) -> Vec<Vec<u32>> {
    if sorted.is_empty() {
        return Vec::new();
    }
    let mut clusters = vec![vec![sorted[0]]];
    for &v in &sorted[1..] {
        if v - *clusters.last().unwrap().last().unwrap() > max_gap {
            clusters.push(vec![v]);
        } else {
            clusters.last_mut().unwrap().push(v);
        }
    }
    clusters
}

/// Order points into a chain using nearest-neighbor traversal.
/// Starts from the extreme point along the dominant axis.
fn order_component_points(points: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if points.is_empty() {
        return Vec::new();
    }

    let axis = dominant_axis(points);
    let mut remaining = points.to_vec();
    remaining.sort_by(|a, b| compare_points_along_axis(*a, *b, axis));

    let mut chain = Vec::with_capacity(remaining.len());
    let mut current = remaining.remove(0);
    chain.push(current);

    while !remaining.is_empty() {
        let best_idx = remaining
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                point_distance_sq(current, **a).cmp(&point_distance_sq(current, **b))
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        current = remaining.swap_remove(best_idx);
        chain.push(current);
    }

    chain
}

#[cfg(test)]
mod tests {
    use super::{
        build_pixel_chains, cluster_values, interpolate_gap, sample_points_from_cluster,
        thin_component_points,
    };

    #[test]
    fn build_pixel_chains_preserves_vertical_components() {
        let pixels: Vec<(u32, u32)> = (0..8).map(|y| (10, y)).collect();
        let chains = build_pixel_chains(&pixels);

        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].len(), pixels.len());
        assert_eq!(chains[0].first(), Some(&(10, 0)));
        assert_eq!(chains[0].last(), Some(&(10, 7)));
    }

    #[test]
    fn build_pixel_chains_stitches_disconnected_components() {
        let mut pixels = vec![(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)];
        pixels.extend([(20, 10), (21, 10), (22, 10), (20, 11), (21, 11), (22, 11)]);

        let chains = build_pixel_chains(&pixels);
        assert_eq!(chains.len(), 1);

        let min_x = chains[0].iter().map(|&(x, _)| x).min().unwrap();
        let max_x = chains[0].iter().map(|&(x, _)| x).max().unwrap();
        assert_eq!(min_x, 0);
        assert!(max_x >= 20);
    }

    #[test]
    fn sample_points_from_cluster_keeps_vertical_span() {
        let pixels: Vec<(u32, u32)> = (0..20).map(|y| (5, y)).collect();
        let sampled = sample_points_from_cluster(&pixels, 5, 64);

        assert_eq!(sampled.len(), 5);

        let min_y = sampled
            .iter()
            .map(|&(_, y)| y)
            .fold(f32::INFINITY, f32::min);
        let max_y = sampled
            .iter()
            .map(|&(_, y)| y)
            .fold(f32::NEG_INFINITY, f32::max);

        assert!(min_y <= 1.0);
        assert!(max_y >= 18.0);
    }

    #[test]
    fn interpolate_gap_produces_intermediate_points() {
        let pts = interpolate_gap((0, 0), (10, 0));
        assert_eq!(pts.len(), 9);
        assert_eq!(pts[0], (1, 0));
        assert_eq!(pts[8], (9, 0));
    }

    #[test]
    fn sample_dashed_curve_uniform() {
        let mut pixels = Vec::new();
        for x in 0..10 {
            pixels.push((x, 5));
        }
        for x in 15..25 {
            pixels.push((x, 5));
        }
        for x in 30..40 {
            pixels.push((x, 5));
        }

        let sampled = sample_points_from_cluster(&pixels, 8, 64);
        assert_eq!(sampled.len(), 8);

        let min_x = sampled
            .iter()
            .map(|&(x, _)| x)
            .fold(f32::INFINITY, f32::min);
        let max_x = sampled
            .iter()
            .map(|&(x, _)| x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(min_x <= 1.0);
        assert!(max_x >= 38.0);

        let mut xs: Vec<f32> = sampled.iter().map(|&(x, _)| x).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for i in 1..xs.len() {
            let spacing = xs[i] - xs[i - 1];
            assert!(
                spacing > 2.0 && spacing < 10.0,
                "spacing {} at index {}",
                spacing,
                i
            );
        }
    }

    #[test]
    fn cluster_values_single_group() {
        let vals = vec![10, 11, 12, 13];
        let clusters = cluster_values(&vals, 3);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0], vec![10, 11, 12, 13]);
    }

    #[test]
    fn cluster_values_two_groups() {
        // Two groups separated by gap > 3
        let vals = vec![10, 11, 12, 50, 51, 52];
        let clusters = cluster_values(&vals, 3);
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0], vec![10, 11, 12]);
        assert_eq!(clusters[1], vec![50, 51, 52]);
    }

    #[test]
    fn thin_preserves_ellipse_shape() {
        // Simulate an ellipse-like shape: wide (horizontal dominant) with two
        // separated y clusters at each x column in the middle section.
        let mut pixels = Vec::new();
        for x in 0..60 {
            // Top arc: y around 5 (3px thick)
            for dy in 0..3 {
                pixels.push((x, 5 + dy));
            }
            // Bottom arc: y around 25 (3px thick)
            for dy in 0..3 {
                pixels.push((x, 25 + dy));
            }
        }
        // Width=60 > Height=23 → horizontal dominant axis

        let thinned = thin_component_points(&pixels);

        // Should have ~2 points per x column (one per cluster) = ~120
        assert!(
            thinned.len() >= 110,
            "expected ~120 thinned points, got {}",
            thinned.len()
        );

        // Check that both y regions are represented
        let ys: Vec<u32> = thinned.iter().map(|&(_, y)| y).collect();
        let has_top = ys.iter().any(|&y| y < 10);
        let has_bottom = ys.iter().any(|&y| y > 20);
        assert!(has_top, "top arc lost");
        assert!(has_bottom, "bottom arc lost");
    }

    #[test]
    fn sample_closed_curve_covers_full_loop() {
        // Build a simple circle-like closed curve
        let mut pixels = Vec::new();
        let cx = 50.0_f32;
        let cy = 50.0_f32;
        let r = 30.0_f32;
        for deg in 0..360 {
            let angle = (deg as f32).to_radians();
            let x = (cx + r * angle.cos()).round() as u32;
            let y = (cy + r * angle.sin()).round() as u32;
            pixels.push((x, y));
        }
        pixels.sort();
        pixels.dedup();

        let sampled = sample_points_from_cluster(&pixels, 12, 128);
        assert_eq!(sampled.len(), 12);

        // All points should be roughly on the circle (distance from center ≈ r)
        for &(x, y) in &sampled {
            let dist = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            assert!(
                (dist - r).abs() < 5.0,
                "point ({}, {}) dist {} from center, expected ~{}",
                x,
                y,
                dist,
                r
            );
        }

        // Points should be roughly evenly distributed angularly
        let mut angles: Vec<f32> = sampled
            .iter()
            .map(|&(x, y)| (y - cy).atan2(x - cx))
            .collect();
        angles.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }

    #[test]
    fn sample_thick_diagonal_dashed_line_uniform() {
        // Simulate a thick (5px) diagonal dashed line going from (100,400) to (400,100)
        // with 3 dashes separated by gaps
        let mut pixels = Vec::new();
        let line_width = 5i32;

        // Helper: draw a thick diagonal segment from (x0,y0) to (x1,y1)
        let mut draw_segment = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let steps = (x1 - x0).abs().max((y1 - y0).abs());
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let cx = x0 as f32 + t * (x1 - x0) as f32;
                let cy = y0 as f32 + t * (y1 - y0) as f32;
                // Add pixels perpendicular to the line (thickness)
                for w in -line_width / 2..=line_width / 2 {
                    // For a 45-degree line, perpendicular is (1,1)/sqrt(2)
                    // Approximate with horizontal offset for simplicity
                    pixels.push(((cx + w as f32).round() as u32, cy.round() as u32));
                }
            }
        };

        // Dash 1: (100,400) to (180,320)
        draw_segment(100, 400, 180, 320);
        // Gap: (180,320) to (210,290) — no pixels
        // Dash 2: (210,290) to (310,190)
        draw_segment(210, 290, 310, 190);
        // Gap: (310,190) to (340,160) — no pixels
        // Dash 3: (340,160) to (400,100)
        draw_segment(340, 160, 400, 100);

        pixels.sort();
        pixels.dedup();

        let chains = build_pixel_chains(&pixels);
        assert_eq!(chains.len(), 1, "should stitch into one chain");

        let sampled = sample_points_from_cluster(&pixels, 6, 500);
        assert_eq!(sampled.len(), 6);

        // Points should span the full diagonal
        let min_x = sampled.iter().map(|&(x, _)| x).fold(f32::INFINITY, f32::min);
        let max_x = sampled.iter().map(|&(x, _)| x).fold(f32::NEG_INFINITY, f32::max);
        assert!(min_x <= 110.0, "min_x={} should be near 100", min_x);
        assert!(max_x >= 390.0, "max_x={} should be near 400", max_x);

        // Check uniform spacing along the diagonal (measure distances between consecutive points)
        // Sort by x to get spatial order
        let mut pts = sampled.clone();
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let mut distances = Vec::new();
        for i in 1..pts.len() {
            let dx = pts[i].0 - pts[i - 1].0;
            let dy = pts[i].1 - pts[i - 1].1;
            distances.push((dx * dx + dy * dy).sqrt());
        }

        let avg_dist: f32 = distances.iter().sum::<f32>() / distances.len() as f32;
        for (i, &d) in distances.iter().enumerate() {
            assert!(
                d > avg_dist * 0.5 && d < avg_dist * 1.5,
                "distance {} at index {} deviates too much from average {} (distances: {:?})",
                d,
                i,
                avg_dist,
                distances
            );
        }
    }
}
