use eframe::egui;
use egui::epaint::PathStroke;
use egui::{Color32, ColorImage, Pos2, Rect, Stroke, TextureOptions};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::state::{AppState, AxisHighlight, CachedHighlight, MaskState, MaskTool, PixelBounds};

// ────────────────────────────────────────────────────────────────
//  Mask Overlay Rendering
// ────────────────────────────────────────────────────────────────

pub fn draw_mask_overlay(
    state: &AppState,
    painter: &egui::Painter,
    to_screen: &dyn Fn(f32, f32) -> Pos2,
    _zoom: f32,
) {
    let mask = if state.axis_mask.active {
        &state.axis_mask
    } else if state.data_mask.active {
        &state.data_mask
    } else {
        return;
    };
    if !mask.visible || mask.buffer.is_empty() {
        return;
    }

    let is_erasing = mask.painting && mask.tool != crate::state::MaskTool::Pen;

    // Draw cached texture (skip during eraser strokes — texture is stale)
    if !is_erasing {
        if let Some(tex) = &mask.mask_texture {
            let p0 = to_screen(0.0, 0.0);
            let p1 = to_screen(mask.width as f32, mask.height as f32);
            painter.image(
                tex.id(),
                egui::Rect::from_min_max(p0, p1),
                egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    if mask.painting && !mask.stroke_snapshot.is_empty() {
        if !is_erasing {
            // Pen: incremental — only render NEW pixels added in this stroke
            draw_mask_rects_diff(
                &mask.buffer,
                &mask.stroke_snapshot,
                mask.width as usize,
                mask.height as usize,
                painter,
                to_screen,
            );
        } else {
            // Eraser: texture is stale, full rect render of current buffer
            draw_mask_rects_full(
                &mask.buffer,
                mask.width as usize,
                mask.height as usize,
                painter,
                to_screen,
            );
        }
    } else if mask.mask_texture.is_none() {
        // No texture yet — full rect fallback
        draw_mask_rects_full(
            &mask.buffer,
            mask.width as usize,
            mask.height as usize,
            painter,
            to_screen,
        );
    }
}

/// Draw rects for ALL masked pixels in the buffer.
fn draw_mask_rects_full(
    buffer: &[bool],
    w: usize,
    h: usize,
    painter: &egui::Painter,
    to_screen: &dyn Fn(f32, f32) -> Pos2,
) {
    let color = Color32::from_rgba_unmultiplied(220, 80, 40, 90);
    for y in 0..h {
        let row = y * w;
        let mut x = 0;
        while x < w {
            if buffer[row + x] {
                let run_start = x;
                while x < w && buffer[row + x] {
                    x += 1;
                }
                let p0 = to_screen(run_start as f32, y as f32);
                let p1 = to_screen(x as f32, (y + 1) as f32);
                painter.rect_filled(egui::Rect::from_min_max(p0, p1), 0.0, color);
            } else {
                x += 1;
            }
        }
    }
}

/// Draw rects only for pixels that are ON in `buffer` but OFF in `snapshot` (new this stroke).
fn draw_mask_rects_diff(
    buffer: &[bool],
    snapshot: &[bool],
    w: usize,
    h: usize,
    painter: &egui::Painter,
    to_screen: &dyn Fn(f32, f32) -> Pos2,
) {
    let color = Color32::from_rgba_unmultiplied(220, 80, 40, 90);
    for y in 0..h {
        let row = y * w;
        let mut x = 0;
        while x < w {
            let i = row + x;
            if buffer[i] && !snapshot[i] {
                let run_start = x;
                x += 1;
                while x < w {
                    let j = row + x;
                    if buffer[j] && !snapshot[j] {
                        x += 1;
                    } else {
                        break;
                    }
                }
                let p0 = to_screen(run_start as f32, y as f32);
                let p1 = to_screen(x as f32, (y + 1) as f32);
                painter.rect_filled(egui::Rect::from_min_max(p0, p1), 0.0, color);
            } else {
                x += 1;
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────
//  Mask Highlight Rendering (for axis/data hover)
// ────────────────────────────────────────────────────────────────

pub fn draw_mask_highlights(
    state: &mut AppState,
    ctx: &egui::Context,
    painter: &egui::Painter,
    to_screen: &dyn Fn(f32, f32) -> Pos2,
    _zoom: f32,
) {
    let mask = if state.axis_mask.active {
        &mut state.axis_mask
    } else if state.data_mask.active {
        &mut state.data_mask
    } else {
        return;
    };

    // Axis highlight
    if let Some(axis_hl) = mask.highlight_axis {
        ensure_axis_highlight_cache(mask, ctx, axis_hl);
        if let Some(ref result) = mask.axis_result {
            let cached = match axis_hl {
                AxisHighlight::X => mask.highlight_cache.axis_x.as_ref(),
                AxisHighlight::Y => mask.highlight_cache.axis_y.as_ref(),
            };
            if let Some(cached) = cached {
                draw_cached_highlight(painter, cached, to_screen, axis_stroke_color(axis_hl));
            }

            // Draw ❌ crosses at tick positions
            let ticks = match axis_hl {
                AxisHighlight::X => &result.x_ticks,
                AxisHighlight::Y => &result.y_ticks,
            };
            let endpoints = match axis_hl {
                AxisHighlight::X => &result.x_axis,
                AxisHighlight::Y => &result.y_axis,
            };

            for tick in ticks {
                let pos = to_screen(tick.0, tick.1);
                let is_endpoint = endpoints.map_or(false, |(start, end)| {
                    ((tick.0 - start.0).abs() < 1.0 && (tick.1 - start.1).abs() < 1.0)
                        || ((tick.0 - end.0).abs() < 1.0 && (tick.1 - end.1).abs() < 1.0)
                });
                let cross_size = if is_endpoint { 8.0 } else { 5.0 };
                let cross_width = if is_endpoint { 2.0 } else { 1.5 };
                let cross_color = match axis_hl {
                    AxisHighlight::X => Color32::from_rgb(0x42, 0x85, 0xF4),
                    AxisHighlight::Y => Color32::from_rgb(0x34, 0xA8, 0x53),
                };

                // White outline
                painter.line_segment(
                    [
                        Pos2::new(pos.x - cross_size, pos.y - cross_size),
                        Pos2::new(pos.x + cross_size, pos.y + cross_size),
                    ],
                    Stroke::new(cross_width + 1.5, Color32::WHITE),
                );
                painter.line_segment(
                    [
                        Pos2::new(pos.x + cross_size, pos.y - cross_size),
                        Pos2::new(pos.x - cross_size, pos.y + cross_size),
                    ],
                    Stroke::new(cross_width + 1.5, Color32::WHITE),
                );
                // Colored cross
                painter.line_segment(
                    [
                        Pos2::new(pos.x - cross_size, pos.y - cross_size),
                        Pos2::new(pos.x + cross_size, pos.y + cross_size),
                    ],
                    Stroke::new(cross_width, cross_color),
                );
                painter.line_segment(
                    [
                        Pos2::new(pos.x + cross_size, pos.y - cross_size),
                        Pos2::new(pos.x - cross_size, pos.y + cross_size),
                    ],
                    Stroke::new(cross_width, cross_color),
                );
            }
        }
    }

    // Data highlight
    if let Some(idx) = mask.highlight_data_idx {
        ensure_data_highlight_cache(mask, ctx, idx);
        if let Some(ref result) = mask.data_result {
            if let Some(group) = result.groups.get(idx) {
                if let Some(cached) = mask
                    .highlight_cache
                    .data_groups
                    .get(idx)
                    .and_then(|entry| entry.as_ref())
                {
                    draw_cached_highlight(painter, cached, to_screen, brighten_rgb(group.color));
                }

                // Draw sampled points
                for &(px, py) in &group.sampled_points {
                    let sp = to_screen(px, py);
                    painter.circle_filled(sp, 4.0, Color32::WHITE);
                    painter.circle_filled(
                        sp,
                        3.0,
                        Color32::from_rgb(group.color[0], group.color[1], group.color[2]),
                    );
                }
            }
        }
    }
}

fn ensure_axis_highlight_cache(mask: &mut MaskState, ctx: &egui::Context, axis: AxisHighlight) {
    let already_cached = match axis {
        AxisHighlight::X => mask.highlight_cache.axis_x.is_some(),
        AxisHighlight::Y => mask.highlight_cache.axis_y.is_some(),
    };
    if already_cached {
        return;
    }

    let pixels = match mask.axis_result.as_ref() {
        Some(result) => match axis {
            AxisHighlight::X => result.x_axis_pixels.clone(),
            AxisHighlight::Y => result.y_axis_pixels.clone(),
        },
        None => return,
    };

    let texture_name = match axis {
        AxisHighlight::X => "mask_axis_highlight_x",
        AxisHighlight::Y => "mask_axis_highlight_y",
    };
    let cache = build_cached_highlight(ctx, texture_name, &pixels, axis_fill_color(axis));

    match axis {
        AxisHighlight::X => mask.highlight_cache.axis_x = cache,
        AxisHighlight::Y => mask.highlight_cache.axis_y = cache,
    }
}

fn ensure_data_highlight_cache(mask: &mut MaskState, ctx: &egui::Context, idx: usize) {
    if idx >= mask.highlight_cache.data_groups.len() {
        mask.highlight_cache.data_groups.resize(idx + 1, None);
    }
    if mask.highlight_cache.data_groups[idx].is_some() {
        return;
    }

    let Some(result) = mask.data_result.as_ref() else {
        return;
    };
    let Some(group) = result.groups.get(idx) else {
        return;
    };

    let texture_name = format!("mask_data_highlight_{idx}");
    mask.highlight_cache.data_groups[idx] = build_cached_highlight(
        ctx,
        &texture_name,
        &group.pixel_coords,
        data_fill_color(group.color),
    );
}

fn build_cached_highlight(
    ctx: &egui::Context,
    texture_name: &str,
    pixels: &[(u32, u32)],
    tint: Color32,
) -> Option<CachedHighlight> {
    let bounds = pixel_bounds(pixels)?;
    let contours = extract_contour_loops(pixels);
    let image = build_highlight_image(bounds, pixels, tint);
    let texture = ctx.load_texture(texture_name, image, TextureOptions::LINEAR);

    Some(CachedHighlight {
        bounds,
        contours,
        texture,
    })
}

fn pixel_bounds(pixels: &[(u32, u32)]) -> Option<PixelBounds> {
    let &(first_x, first_y) = pixels.first()?;
    let mut bounds = PixelBounds {
        min_x: first_x,
        min_y: first_y,
        max_x: first_x,
        max_y: first_y,
    };

    for &(x, y) in pixels.iter().skip(1) {
        bounds.min_x = bounds.min_x.min(x);
        bounds.min_y = bounds.min_y.min(y);
        bounds.max_x = bounds.max_x.max(x);
        bounds.max_y = bounds.max_y.max(y);
    }

    Some(bounds)
}

fn build_highlight_image(bounds: PixelBounds, pixels: &[(u32, u32)], tint: Color32) -> ColorImage {
    let width = bounds.width();
    let height = bounds.height();
    let mut rgba = vec![0_u8; width * height * 4];

    for &(x, y) in pixels {
        let local_x = (x - bounds.min_x) as usize;
        let local_y = (y - bounds.min_y) as usize;
        let off = (local_y * width + local_x) * 4;
        rgba[off] = tint.r();
        rgba[off + 1] = tint.g();
        rgba[off + 2] = tint.b();
        rgba[off + 3] = tint.a();
    }

    ColorImage::from_rgba_unmultiplied([width, height], &rgba)
}

fn draw_cached_highlight(
    painter: &egui::Painter,
    cached: &CachedHighlight,
    to_screen: &dyn Fn(f32, f32) -> Pos2,
    stroke_color: Color32,
) {
    let p0 = to_screen(cached.bounds.min_x as f32, cached.bounds.min_y as f32);
    let p1 = to_screen(
        (cached.bounds.max_x + 1) as f32,
        (cached.bounds.max_y + 1) as f32,
    );
    painter.image(
        cached.texture.id(),
        Rect::from_min_max(p0, p1),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );

    let halo = Color32::from_rgba_unmultiplied(255, 255, 255, 48);
    let outer =
        Color32::from_rgba_unmultiplied(stroke_color.r(), stroke_color.g(), stroke_color.b(), 120);

    for contour in &cached.contours {
        if contour.len() < 3 {
            continue;
        }

        let points: Vec<Pos2> = contour
            .iter()
            .map(|&(x, y)| to_screen(x as f32, y as f32))
            .collect();
        painter.add(egui::Shape::closed_line(
            points.clone(),
            PathStroke::new(6.0, halo).outside(),
        ));
        painter.add(egui::Shape::closed_line(
            points.clone(),
            PathStroke::new(3.0, outer).outside(),
        ));
        painter.add(egui::Shape::closed_line(
            points,
            PathStroke::new(1.5, stroke_color).outside(),
        ));
    }
}

fn extract_contour_loops(pixels: &[(u32, u32)]) -> Vec<Vec<(u32, u32)>> {
    split_four_connected_components(pixels)
        .into_iter()
        .flat_map(|component| extract_component_contours(&component))
        .collect()
}

fn split_four_connected_components(pixels: &[(u32, u32)]) -> Vec<Vec<(u32, u32)>> {
    if pixels.is_empty() {
        return Vec::new();
    }

    let pixel_set: HashSet<(u32, u32)> = pixels.iter().copied().collect();
    let mut visited: HashSet<(u32, u32)> = HashSet::with_capacity(pixel_set.len());
    let mut components = Vec::new();

    for &start in pixels {
        if visited.contains(&start) {
            continue;
        }

        let mut component = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some((x, y)) = queue.pop_front() {
            component.push((x, y));

            let neighbors = [
                (x.wrapping_sub(1), y, x > 0),
                (x + 1, y, true),
                (x, y.wrapping_sub(1), y > 0),
                (x, y + 1, true),
            ];

            for (nx, ny, valid) in neighbors {
                if !valid {
                    continue;
                }
                let next = (nx, ny);
                if !pixel_set.contains(&next) || visited.contains(&next) {
                    continue;
                }
                visited.insert(next);
                queue.push_back(next);
            }
        }

        components.push(component);
    }

    components.sort_by(|a, b| b.len().cmp(&a.len()));
    components
}

fn extract_component_contours(component: &[(u32, u32)]) -> Vec<Vec<(u32, u32)>> {
    let pixel_set: HashSet<(u32, u32)> = component.iter().copied().collect();
    let mut edges: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();

    for &(x, y) in component {
        if y == 0 || !pixel_set.contains(&(x, y - 1)) {
            add_boundary_edge(&mut edges, (x, y), (x + 1, y));
        }
        if !pixel_set.contains(&(x + 1, y)) {
            add_boundary_edge(&mut edges, (x + 1, y), (x + 1, y + 1));
        }
        if !pixel_set.contains(&(x, y + 1)) {
            add_boundary_edge(&mut edges, (x + 1, y + 1), (x, y + 1));
        }
        if x == 0 || !pixel_set.contains(&(x - 1, y)) {
            add_boundary_edge(&mut edges, (x, y + 1), (x, y));
        }
    }

    let mut loops = Vec::new();
    while let Some(start) = edges
        .iter()
        .find_map(|(vertex, outgoing)| (!outgoing.is_empty()).then_some(*vertex))
    {
        let mut loop_points = Vec::new();
        let mut current = start;

        loop {
            loop_points.push(current);
            let Some(next) = pop_boundary_edge(&mut edges, current) else {
                break;
            };
            current = next;
            if current == start {
                break;
            }
        }

        if let Some(simplified) = simplify_closed_loop(&loop_points) {
            loops.push(simplified);
        }
    }

    loops
}

fn add_boundary_edge(
    edges: &mut HashMap<(u32, u32), Vec<(u32, u32)>>,
    start: (u32, u32),
    end: (u32, u32),
) {
    edges.entry(start).or_default().push(end);
}

fn pop_boundary_edge(
    edges: &mut HashMap<(u32, u32), Vec<(u32, u32)>>,
    start: (u32, u32),
) -> Option<(u32, u32)> {
    let outgoing = edges.get_mut(&start)?;
    let next = outgoing.pop();
    let remove_entry = outgoing.is_empty();
    let _ = outgoing;

    if remove_entry {
        edges.remove(&start);
    }

    next
}

fn simplify_closed_loop(points: &[(u32, u32)]) -> Option<Vec<(u32, u32)>> {
    if points.len() < 3 {
        return None;
    }

    let len = points.len();
    let mut simplified = Vec::new();

    for i in 0..len {
        let prev = points[(i + len - 1) % len];
        let curr = points[i];
        let next = points[(i + 1) % len];

        if curr == prev || curr == next {
            continue;
        }

        let dir_in = (
            (curr.0 as i64 - prev.0 as i64).signum(),
            (curr.1 as i64 - prev.1 as i64).signum(),
        );
        let dir_out = (
            (next.0 as i64 - curr.0 as i64).signum(),
            (next.1 as i64 - curr.1 as i64).signum(),
        );

        if dir_in != dir_out {
            simplified.push(curr);
        }
    }

    (simplified.len() >= 3).then_some(simplified)
}

fn axis_fill_color(axis: AxisHighlight) -> Color32 {
    match axis {
        AxisHighlight::X => Color32::from_rgba_unmultiplied(66, 133, 244, 72),
        AxisHighlight::Y => Color32::from_rgba_unmultiplied(52, 168, 83, 72),
    }
}

fn axis_stroke_color(axis: AxisHighlight) -> Color32 {
    match axis {
        AxisHighlight::X => Color32::from_rgb(0x42, 0x85, 0xF4),
        AxisHighlight::Y => Color32::from_rgb(0x34, 0xA8, 0x53),
    }
}

fn data_fill_color(color: [u8; 3]) -> Color32 {
    Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 72)
}

fn brighten_rgb(color: [u8; 3]) -> Color32 {
    fn lift(channel: u8) -> u8 {
        let boosted = channel as u16 + ((255 - channel as u16) * 2 / 5);
        boosted.min(255) as u8
    }

    Color32::from_rgb(lift(color[0]), lift(color[1]), lift(color[2]))
}

// ────────────────────────────────────────────────────────────────
//  Mask Brush Cursor
// ────────────────────────────────────────────────────────────────

pub fn draw_mask_cursor(
    state: &AppState,
    painter: &egui::Painter,
    ctx: &egui::Context,
    response: &egui::Response,
    zoom: f32,
) {
    let mask = if state.axis_mask.active {
        &state.axis_mask
    } else if state.data_mask.active {
        &state.data_mask
    } else {
        return;
    };

    if let Some(mouse_pos) = ctx.input(|i| i.pointer.hover_pos()) {
        if response.rect.contains(mouse_pos) {
            let screen_radius = mask.brush_size * zoom;
            let cursor_color = match mask.tool {
                MaskTool::Pen => Color32::from_rgba_unmultiplied(220, 80, 40, 160),
                MaskTool::Eraser => Color32::from_rgba_unmultiplied(100, 180, 255, 160),
            };
            painter.circle_stroke(mouse_pos, screen_radius, Stroke::new(1.5, cursor_color));
            // Small center dot
            painter.circle_filled(mouse_pos, 2.0, cursor_color);
        }
    }
}
