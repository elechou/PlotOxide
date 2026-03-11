use crate::state::{AxisDetectionResult, DataCurveMode, DataDetectionResult, DetectedColorGroup};
use std::collections::{HashMap, HashSet, VecDeque};

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
//  Color Distance
// ────────────────────────────────────────────────────────────────

fn color_distance_sq(a: [u8; 3], b: [u8; 3]) -> f32 {
    let dr = a[0] as f32 - b[0] as f32;
    let dg = a[1] as f32 - b[1] as f32;
    let db = a[2] as f32 - b[2] as f32;
    dr * dr + dg * dg + db * db
}

fn is_bg_color(pixel: [u8; 3], bg: [u8; 3]) -> bool {
    color_distance_sq(pixel, bg) < 30.0 * 30.0 * 3.0
}

fn _is_similar_with_tolerance(a: [u8; 3], b: [u8; 3], tolerance: f32) -> bool {
    color_distance_sq(a, b) < tolerance * tolerance * 3.0
}

// ────────────────────────────────────────────────────────────────
//  Axis Detection (Improved)
// ────────────────────────────────────────────────────────────────

pub fn analyze_mask_for_axes(
    rgba: &[u8],
    mask: &[bool],
    w: u32,
    h: u32,
    bg_color: [u8; 3],
) -> AxisDetectionResult {
    let w_us = w as usize;
    let h_us = h as usize;

    // Step 1: Collect ALL non-background pixels inside the mask.
    // We trust the user's mask entirely. Any non-bg color is a valid candidate for the axis/ticks.
    // Our BFS approach will later guarantee we only extract the structure actually attached to the main axis line.
    let mut pixel_set: HashSet<(u32, u32)> = HashSet::new();

    for y in 0..h_us {
        for x in 0..w_us {
            let idx = y * w_us + x;
            if mask[idx] {
                let off = idx * 4;
                if off + 2 < rgba.len() {
                    let pixel = [rgba[off], rgba[off + 1], rgba[off + 2]];
                    if !is_bg_color(pixel, bg_color) {
                        pixel_set.insert((x as u32, y as u32));
                    }
                }
            }
        }
    }

    if pixel_set.is_empty() {
        return AxisDetectionResult {
            x_axis: None,
            y_axis: None,
            x_axis_pixels: Vec::new(),
            y_axis_pixels: Vec::new(),
            x_ticks: Vec::new(),
            y_ticks: Vec::new(),
        };
    }

    // Step 2: Extract Connected Components (Mask Strokes)
    let mut unvisited: HashSet<(u32, u32)> = pixel_set.iter().copied().collect();
    let mut islands: Vec<HashSet<(u32, u32)>> = Vec::new();

    while let Some(&start) = unvisited.iter().next() {
        let mut island = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        unvisited.remove(&start);
        island.insert(start);

        while let Some(curr) = queue.pop_front() {
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = curr.0 as i32 + dx;
                    let ny = curr.1 as i32 + dy;
                    if nx < 0 || ny < 0 {
                        continue;
                    }
                    let np = (nx as u32, ny as u32);
                    if unvisited.remove(&np) {
                        island.insert(np);
                        queue.push_back(np);
                    }
                }
            }
        }
        islands.push(island);
    }

    // Step 3: Classify each island using 1D Projection Density
    let min_dim = (w.min(h)) as f32;
    // Dynamic thresholds: a valid axis stroke must have a 1D density
    // spanning at least 3% of the image, or a minimum of 20 pixels.
    let l_min = (min_dim * 0.03).max(20.0) as u32;
    // Overlap margin for splitting L-shapes
    let split_margin = (min_dim * 0.02).max(10.0) as u32;

    let mut x_pixel_set: HashSet<(u32, u32)> = HashSet::new();
    let mut y_pixel_set: HashSet<(u32, u32)> = HashSet::new();

    for island in islands {
        let mut row_counts: HashMap<u32, u32> = HashMap::new();
        let mut col_counts: HashMap<u32, u32> = HashMap::new();

        for &(px, py) in &island {
            *row_counts.entry(py).or_insert(0) += 1;
            *col_counts.entry(px).or_insert(0) += 1;
        }

        // 3-pixel window moving density (H_score)
        let mut max_h = 0;
        let mut ay = 0;
        for &y in row_counts.keys() {
            let density = row_counts.get(&(y.saturating_sub(1))).unwrap_or(&0)
                + row_counts.get(&y).unwrap_or(&0)
                + row_counts.get(&(y + 1)).unwrap_or(&0);
            if density > max_h {
                max_h = density;
                ay = y;
            }
        }

        // 3-pixel window moving density (V_score)
        let mut max_v = 0;
        let mut ax = 0;
        for &x in col_counts.keys() {
            let density = col_counts.get(&(x.saturating_sub(1))).unwrap_or(&0)
                + col_counts.get(&x).unwrap_or(&0)
                + col_counts.get(&(x + 1)).unwrap_or(&0);
            if density > max_v {
                max_v = density;
                ax = x;
            }
        }

        // Routing Logic
        if max_h < l_min && max_v < l_min {
            // NOISE (Arabic Numerals, dots, small wiggles) -> DISCARD!
            continue;
        } else if max_h >= l_min && max_v < l_min {
            // PURE X-AXIS
            x_pixel_set.extend(island);
        } else if max_v >= l_min && max_h < l_min {
            // PURE Y-AXIS
            y_pixel_set.extend(island);
        } else {
            // L-SHAPE / CROSS -> Split geometrically around the dense crosshairs
            for &(px, py) in &island {
                let dist_x = (py as i32 - ay as i32).abs() as u32;
                let dist_y = (px as i32 - ax as i32).abs() as u32;

                if dist_x <= dist_y + split_margin {
                    x_pixel_set.insert((px, py));
                }
                if dist_y <= dist_x + split_margin {
                    y_pixel_set.insert((px, py));
                }
            }
        }
    }

    // Determine absolute Densest Line separately for purified X and Y pools
    let mut x_row_counts: HashMap<u32, u32> = HashMap::new();
    for &(_, py) in &x_pixel_set {
        *x_row_counts.entry(py).or_insert(0) += 1;
    }
    let x_axis_row = x_row_counts
        .iter()
        .max_by_key(|&(_, &count)| count)
        .map(|(&y, _)| y);

    let mut y_col_counts: HashMap<u32, u32> = HashMap::new();
    for &(px, _) in &y_pixel_set {
        *y_col_counts.entry(px).or_insert(0) += 1;
    }
    let y_axis_col = y_col_counts
        .iter()
        .max_by_key(|&(_, &count)| count)
        .map(|(&x, _)| x);

    // Helper closure: 2D BFS Island + 1D Silhouette Profiling
    // Note: It now operates strictly on the active_set provided.
    let extract_axis_and_ticks = |axis_line: u32,
                                  is_horizontal: bool,
                                  active_set: &HashSet<(u32, u32)>|
     -> (Vec<(u32, u32)>, Vec<(f32, f32)>) {
        // 1. Break active_set into strictly connected components
        let mut unvisited: HashSet<(u32, u32)> = active_set.iter().copied().collect();
        let mut best_island: HashSet<(u32, u32)> = HashSet::new();
        let mut max_span = 0;

        while let Some(&start) = unvisited.iter().next() {
            let mut island = HashSet::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(start);
            unvisited.remove(&start);
            island.insert(start);

            let mut min_pos = u32::MAX;
            let mut max_pos = 0;
            let mut touches_axis = false;

            while let Some(curr) = queue.pop_front() {
                let (px, py) = curr;
                if is_horizontal {
                    if (py as i32 - axis_line as i32).abs() <= 1 {
                        touches_axis = true;
                    }
                    min_pos = min_pos.min(px);
                    max_pos = max_pos.max(px);
                } else {
                    if (px as i32 - axis_line as i32).abs() <= 1 {
                        touches_axis = true;
                    }
                    min_pos = min_pos.min(py);
                    max_pos = max_pos.max(py);
                }

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = curr.0 as i32 + dx;
                        let ny = curr.1 as i32 + dy;
                        if nx < 0 || ny < 0 {
                            continue;
                        }
                        let np = (nx as u32, ny as u32);
                        if unvisited.remove(&np) {
                            island.insert(np);
                            queue.push_back(np);
                        }
                    }
                }
            }

            if touches_axis {
                let span = max_pos.saturating_sub(min_pos);
                // Use span >= max_span so even a span of 0 (a single pixel on the axis) is captured
                if span >= max_span {
                    max_span = span;
                    best_island = island;
                }
            }
        }

        let island = best_island;
        if island.is_empty() {
            return (Vec::new(), Vec::new());
        }


        // 2. 1D Silhouette Profiling
        // Group by position along the axis
        let mut profile: HashMap<u32, u32> = HashMap::new(); // along -> max perp extension
        let mut extent_bounds: HashMap<u32, (u32, u32)> = HashMap::new(); // along -> (min_perp, max_perp)

        for &(px, py) in &island {
            let (along, perp) = if is_horizontal { (px, py) } else { (py, px) };
            let ext = (perp as i32 - axis_line as i32).abs() as u32;

            profile
                .entry(along)
                .and_modify(|e| *e = (*e).max(ext))
                .or_insert(ext);

            let entry = extent_bounds.entry(along).or_insert((perp, perp));
            entry.0 = entry.0.min(perp);
            entry.1 = entry.1.max(perp);
        }

        // 3. Measure axis length to auto-scale thresholds
        let min_along = profile.keys().min().copied().unwrap_or(0);
        let max_along = profile.keys().max().copied().unwrap_or(0);
        let _axis_length = max_along - min_along + 1;

        // Auto-scale limits
        // Instead of hard PX, we define logical dimensions for ticks vs text.
        // A tick must protrude a little bit, but not be insanely wide.
        // We REMOVE the maximum length limit so that grid lines and crossing axes ARE identified as ticks.
        let tick_min_ext = 2u32; // Must stick out a bit from the 1px line
        let tick_max_width = 12u32; // Not too wide (avoids arabic numerals "0" "1" etc)

        // 4. Scan the profile for "bumps" (runs of extension >= tick_min_ext)
        let mut ticks: Vec<(f32, f32)> = Vec::new();
        let mut current_run: Vec<u32> = Vec::new(); // stores the 'along' positions forming a bump

        let mut sorted_along: Vec<u32> = profile.keys().copied().collect();
        sorted_along.sort();

        let process_bump = |run: &[u32], ticks_out: &mut Vec<(f32, f32)>| {
            if run.is_empty() {
                return;
            }
            let r_start = *run.first().unwrap();
            let r_end = *run.last().unwrap();
            let r_width = r_end - r_start + 1;

            // Classification!
            // If it's narrow, it's a tick (or a grid line, or a crossing axis).
            if r_width <= tick_max_width {
                let center = (r_start + r_end) / 2;
                let tick_pos = if is_horizontal {
                    (center as f32, axis_line as f32)
                } else {
                    (axis_line as f32, center as f32)
                };
                ticks_out.push(tick_pos);
            }
        };

        for pos in sorted_along {
            let ext = *profile.get(&pos).unwrap();
            if ext >= tick_min_ext {
                // If there's a gap in `pos`, process the old run
                if !current_run.is_empty() && pos > *current_run.last().unwrap() + 1 {
                    process_bump(&current_run, &mut ticks);
                    current_run.clear();
                }
                current_run.push(pos);
            } else {
                if !current_run.is_empty() {
                    process_bump(&current_run, &mut ticks);
                    current_run.clear();
                }
            }
        }
        if !current_run.is_empty() {
            process_bump(&current_run, &mut ticks);
        }

        let body_pixels_vec: Vec<(u32, u32)> = island.into_iter().collect();
        (body_pixels_vec, ticks)
    };

    let mut x_ticks: Vec<(f32, f32)> = Vec::new();
    let mut y_ticks: Vec<(f32, f32)> = Vec::new();
    let mut x_axis_pixels: Vec<(u32, u32)> = Vec::new();
    let mut y_axis_pixels: Vec<(u32, u32)> = Vec::new();

    if let Some(axis_y) = x_axis_row {
        let (body, ticks) = extract_axis_and_ticks(axis_y, true, &x_pixel_set);
        x_axis_pixels = body;
        x_ticks = ticks;
    }

    if let Some(axis_x) = y_axis_col {
        let (body, ticks) = extract_axis_and_ticks(axis_x, false, &y_pixel_set);
        y_axis_pixels = body;
        y_ticks = ticks;
    }

    // Step 5: Endpoints = outermost ticks (or axis line ends if no ticks found)
    // We STRICTLY use the purified isolated body `x_axis_pixels` instead of the broad `x_pixel_set` pool.
    let x_axis = if !x_ticks.is_empty() {
        Some((*x_ticks.first().unwrap(), *x_ticks.last().unwrap()))
    } else if let Some(axis_y) = x_axis_row {
        if !x_axis_pixels.is_empty() {
            let min_x = x_axis_pixels.iter().map(|&(x, _)| x).min().unwrap();
            let max_x = x_axis_pixels.iter().map(|&(x, _)| x).max().unwrap();
            Some(((min_x as f32, axis_y as f32), (max_x as f32, axis_y as f32)))
        } else {
            None
        }
    } else {
        None
    };

    let y_axis = if !y_ticks.is_empty() {
        Some((*y_ticks.first().unwrap(), *y_ticks.last().unwrap()))
    } else if let Some(axis_x) = y_axis_col {
        if !y_axis_pixels.is_empty() {
            let min_y = y_axis_pixels.iter().map(|&(_, y)| y).min().unwrap();
            let max_y = y_axis_pixels.iter().map(|&(_, y)| y).max().unwrap();
            Some(((axis_x as f32, min_y as f32), (axis_x as f32, max_y as f32)))
        } else {
            None
        }
    } else {
        None
    };

    AxisDetectionResult {
        x_axis,
        y_axis,
        x_axis_pixels,
        y_axis_pixels,
        x_ticks,
        y_ticks,
    }
}

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
    // Use a greedy centroid-based clustering approach
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
                // Update running centroid
                centroids[idx][0] = (centroids[idx][0] * n + pf[0]) / (n + 1.0);
                centroids[idx][1] = (centroids[idx][1] * n + pf[1]) / (n + 1.0);
                centroids[idx][2] = (centroids[idx][2] * n + pf[2]) / (n + 1.0);
                cluster_pixels[idx].push((x, y));
            } else {
                // New cluster
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

        let sampled = sample_points_arc_length(&pixels, 10, w);

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

// ────────────────────────────────────────────────────────────────
//  Arc-Length Point Sampling (Multi-Segment)
// ────────────────────────────────────────────────────────────────

/// Sample N points along a pixel cluster using arc-length parameterization.
/// Handles non-function curves (circles, hyperbolas) correctly.
/// Supports multi-segment curves (occluded curves with gaps).
pub fn sample_points_from_cluster(pixels: &[(u32, u32)], n: usize, w: u32) -> Vec<(f32, f32)> {
    sample_points_arc_length(pixels, n, w)
}

fn sample_points_arc_length(pixels: &[(u32, u32)], n: usize, _w: u32) -> Vec<(f32, f32)> {
    if pixels.is_empty() || n == 0 {
        return Vec::new();
    }

    // Build all connected chains (handles gaps from occlusion)
    let chains = build_pixel_chains(pixels);

    if chains.is_empty() {
        return Vec::new();
    }

    // Compute arc-lengths for each chain
    let mut chain_lengths: Vec<f32> = Vec::new();
    let mut chain_arc_data: Vec<Vec<f32>> = Vec::new(); // cumulative arc-lengths per chain

    for chain in &chains {
        let mut arcs: Vec<f32> = vec![0.0];
        for i in 1..chain.len() {
            let dx = chain[i].0 as f32 - chain[i - 1].0 as f32;
            let dy = chain[i].1 as f32 - chain[i - 1].1 as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            arcs.push(arcs[i - 1] + dist);
        }
        let total = *arcs.last().unwrap_or(&0.0);
        chain_lengths.push(total);
        chain_arc_data.push(arcs);
    }

    let grand_total: f32 = chain_lengths.iter().sum();
    if grand_total < 1.0 {
        // All chains are trivially short
        return chains
            .iter()
            .flat_map(|c| c.iter().map(|&(x, y)| (x as f32, y as f32)))
            .take(n)
            .collect();
    }

    // Total points in all chains combined
    let total_points_available: usize = chains.iter().map(|c| c.len()).sum();
    if total_points_available <= n {
        return chains
            .iter()
            .flat_map(|c| c.iter().map(|&(x, y)| (x as f32, y as f32)))
            .collect();
    }

    // Distribute N sample points across chains proportionally to arc-length
    let mut points_per_chain: Vec<usize> = Vec::new();
    let mut allocated = 0usize;

    for (i, &len) in chain_lengths.iter().enumerate() {
        let share = if grand_total > 0.0 {
            (len / grand_total * n as f32).round() as usize
        } else {
            0
        };
        // Ensure at least 1 point per chain (if chain is non-trivial)
        let share = if chains[i].len() >= 2 {
            share.max(1)
        } else {
            share
        };
        points_per_chain.push(share);
        allocated += share;
    }

    // Adjust for rounding: add/remove from the longest chain
    if allocated != n {
        if let Some(longest_idx) = chain_lengths
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
        {
            if allocated > n {
                let excess = allocated - n;
                points_per_chain[longest_idx] =
                    points_per_chain[longest_idx].saturating_sub(excess);
            } else {
                points_per_chain[longest_idx] += n - allocated;
            }
        }
    }

    // Sample each chain independently
    let mut sampled = Vec::with_capacity(n);

    for (chain_idx, chain) in chains.iter().enumerate() {
        let cn = points_per_chain[chain_idx];
        if cn == 0 || chain.is_empty() {
            continue;
        }

        let arcs = &chain_arc_data[chain_idx];
        let total_length = chain_lengths[chain_idx];

        if chain.len() <= cn {
            sampled.extend(chain.iter().map(|&(x, y)| (x as f32, y as f32)));
            continue;
        }

        if total_length < 1.0 {
            sampled.push((chain[0].0 as f32, chain[0].1 as f32));
            continue;
        }

        for i in 0..cn {
            let target = if cn == 1 {
                total_length / 2.0
            } else {
                (i as f32) * total_length / ((cn - 1) as f32)
            };

            // Binary search for the segment containing this arc-length
            let seg = match arcs
                .binary_search_by(|v| v.partial_cmp(&target).unwrap_or(std::cmp::Ordering::Equal))
            {
                Ok(idx) => idx,
                Err(idx) => idx.saturating_sub(1),
            };
            let seg = seg.min(chain.len() - 1);

            // Interpolate within the segment
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
                sampled.push((x, y));
            } else {
                sampled.push((chain[seg].0 as f32, chain[seg].1 as f32));
            }
        }
    }

    sampled
}

/// Build ordered chains of points by nearest-neighbor walking.
/// Returns ALL segments (handles gaps from occlusion).
/// Each gap > threshold starts a new chain instead of stopping.
fn build_pixel_chains(pixels: &[(u32, u32)]) -> Vec<Vec<(u32, u32)>> {
    if pixels.is_empty() {
        return Vec::new();
    }
    if pixels.len() <= 2 {
        return vec![pixels.to_vec()];
    }

    // For large pixel sets, thin to medial axis first (take median Y per X column)
    let mut by_x: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(x, y) in pixels {
        by_x.entry(x).or_default().push(y);
    }

    let mut thin_points: Vec<(u32, u32)> = Vec::new();
    for (&x, ys) in &by_x {
        let mut ys_sorted = ys.clone();
        ys_sorted.sort();
        let median_y = ys_sorted[ys_sorted.len() / 2];
        thin_points.push((x, median_y));
    }

    if thin_points.len() <= 2 {
        thin_points.sort_by_key(|p| p.0);
        return vec![thin_points];
    }

    // Nearest-neighbor chain starting from the leftmost point
    thin_points.sort_by_key(|p| p.0);
    let mut used = vec![false; thin_points.len()];
    let mut chains: Vec<Vec<(u32, u32)>> = Vec::new();

    // Start from the point with the smallest x
    let mut current_chain: Vec<(u32, u32)> = Vec::new();
    let current = 0;
    current_chain.push(thin_points[current]);
    used[current] = true;

    let gap_threshold_sq = 100.0 * 100.0;

    for _ in 1..thin_points.len() {
        let mut best_idx = None;
        let mut best_dist = f64::MAX;

        for (j, &pt) in thin_points.iter().enumerate() {
            if used[j] {
                continue;
            }
            let dx = pt.0 as f64 - current_chain.last().unwrap().0 as f64;
            let dy = pt.1 as f64 - current_chain.last().unwrap().1 as f64;
            let dist = dx * dx + dy * dy;
            if dist < best_dist {
                best_dist = dist;
                best_idx = Some(j);
            }
        }

        if let Some(idx) = best_idx {
            if best_dist > gap_threshold_sq {
                // Gap detected: save current chain and start a new one
                if current_chain.len() >= 2 {
                    chains.push(std::mem::take(&mut current_chain));
                } else {
                    current_chain.clear();
                }
            }
            current_chain.push(thin_points[idx]);
            used[idx] = true;
        } else {
            break;
        }
    }

    // Don't forget the last chain
    if current_chain.len() >= 2 {
        chains.push(current_chain);
    }

    // If nothing was produced (e.g. all single-point chains), fall back
    if chains.is_empty() {
        thin_points.sort_by_key(|p| p.0);
        return vec![thin_points];
    }

    chains
}
