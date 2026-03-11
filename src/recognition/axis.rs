use crate::state::AxisDetectionResult;
use std::collections::{HashMap, HashSet};

use super::is_bg_color;

// ────────────────────────────────────────────────────────────────
//  Axis Detection
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
    let l_min = (min_dim * 0.03).max(20.0) as u32;
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
            // NOISE -> DISCARD
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
//  Helper: 2D BFS Island + 1D Silhouette Profiling
// ────────────────────────────────────────────────────────────────

fn extract_axis_and_ticks(
    axis_line: u32,
    is_horizontal: bool,
    active_set: &HashSet<(u32, u32)>,
) -> (Vec<(u32, u32)>, Vec<(f32, f32)>) {
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
    let mut profile: HashMap<u32, u32> = HashMap::new();

    for &(px, py) in &island {
        let (along, perp) = if is_horizontal { (px, py) } else { (py, px) };
        let ext = (perp as i32 - axis_line as i32).abs() as u32;

        profile
            .entry(along)
            .and_modify(|e| *e = (*e).max(ext))
            .or_insert(ext);
    }

    // 3. Auto-scale thresholds
    let tick_min_ext = 2u32;
    let tick_max_width = 12u32;

    // 4. Scan the profile for "bumps" (runs of extension >= tick_min_ext)
    let mut ticks: Vec<(f32, f32)> = Vec::new();
    let mut current_run: Vec<u32> = Vec::new();

    let mut sorted_along: Vec<u32> = profile.keys().copied().collect();
    sorted_along.sort();

    let process_bump = |run: &[u32], ticks_out: &mut Vec<(f32, f32)>| {
        if run.is_empty() {
            return;
        }
        let r_start = *run.first().unwrap();
        let r_end = *run.last().unwrap();
        let r_width = r_end - r_start + 1;

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
}
