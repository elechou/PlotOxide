use std::collections::HashMap;

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
    let mut chain_arc_data: Vec<Vec<f32>> = Vec::new();

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
        return chains
            .iter()
            .flat_map(|c| c.iter().map(|&(x, y)| (x as f32, y as f32)))
            .take(n)
            .collect();
    }

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

    // If nothing was produced, fall back
    if chains.is_empty() {
        thin_points.sort_by_key(|p| p.0);
        return vec![thin_points];
    }

    chains
}
