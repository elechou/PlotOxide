pub mod results_panel;

use eframe::egui;
use egui::{Color32, Pos2, Stroke};

use crate::action::Action;
use crate::state::{AppState, MaskTool};

// ────────────────────────────────────────────────────────────────
//  Mask Sub-Toolbar (anchored at canvas top-left)
// ────────────────────────────────────────────────────────────────

pub fn draw_mask_toolbar(state: &AppState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let mask = if state.axis_mask.active {
        &state.axis_mask
    } else if state.data_mask.active {
        &state.data_mask
    } else {
        return;
    };

    let window = egui::Window::new("Mask Sub-Toolbar")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, [325.0, 25.0]);

    window.show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(mask.tool == MaskTool::Pen, "\u{270F} Pen")
                .on_hover_text("Paint mask")
                .clicked()
            {
                actions.push(Action::MaskSetTool(MaskTool::Pen));
            }
            if ui
                .selectable_label(mask.tool == MaskTool::Eraser, "\u{1F4D7} Eraser")
                .on_hover_text("Erase mask")
                .clicked()
            {
                actions.push(Action::MaskSetTool(MaskTool::Eraser));
            }

            // Compact brush size slider (no label, narrow)
            let mut size = mask.brush_size;
            let slider = egui::Slider::new(&mut size, 2.0..=80.0)
                .step_by(1.0)
                .show_value(false);
            if ui
                .add_sized([40.0, 18.0], slider)
                .on_hover_text(format!("Brush size: {:.0} px", mask.brush_size))
                .changed()
            {
                actions.push(Action::MaskSetBrushSize(size));
            }

            // Icon-only visibility toggle
            let vis_icon = if mask.visible {
                "\u{1F441}"
            } else {
                "\u{1F6AB}"
            };
            if ui
                .selectable_label(!mask.visible, vis_icon)
                .on_hover_text(if mask.visible {
                    "Hide mask overlay"
                } else {
                    "Show mask overlay"
                })
                .clicked()
            {
                actions.push(Action::MaskToggleVisibility);
            }

            // Icon-only clear button
            if ui.button("🗑").on_hover_text("Clear all mask").clicked() {
                actions.push(Action::MaskClear);
            }
        });
    });
}

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
    if !mask.visible {
        return;
    }
    if mask.buffer.is_empty() {
        return;
    }

    let w = mask.width as usize;
    let h = mask.height as usize;

    // Semi-transparent red-orange overlay color (multiply-like effect)
    let overlay_color = Color32::from_rgba_unmultiplied(220, 80, 40, 90);

    // Scan the mask and draw filled rectangles for contiguous runs of masked pixels.
    // This is much more efficient than drawing one rect per pixel.
    for y in 0..h {
        let row_start = y * w;
        let mut x = 0;
        while x < w {
            if mask.buffer[row_start + x] {
                // Find the end of this run
                let run_start = x;
                while x < w && mask.buffer[row_start + x] {
                    x += 1;
                }
                let run_end = x;

                // Draw a single rectangle for the entire run
                let p0 = to_screen(run_start as f32, y as f32);
                let p1 = to_screen(run_end as f32, (y + 1) as f32);
                let rect = egui::Rect::from_min_max(p0, p1);
                painter.rect_filled(rect, 0.0, overlay_color);
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
    state: &AppState,
    painter: &egui::Painter,
    to_screen: &dyn Fn(f32, f32) -> Pos2,
    _zoom: f32,
) {
    let rgba = state.decoded_rgba.as_deref();
    let mask = if state.axis_mask.active {
        &state.axis_mask
    } else if state.data_mask.active {
        &state.data_mask
    } else {
        return;
    };

    // Axis highlight
    if let Some(ref axis_hl) = mask.highlight_axis {
        if let Some(ref result) = mask.axis_result {
            let pixels = match axis_hl {
                crate::state::AxisHighlight::X => &result.x_axis_pixels,
                crate::state::AxisHighlight::Y => &result.y_axis_pixels,
            };

            draw_pixel_set_real_color(painter, pixels, to_screen, rgba, mask.width);

            // Draw ❌ crosses at tick positions
            let ticks = match axis_hl {
                crate::state::AxisHighlight::X => &result.x_ticks,
                crate::state::AxisHighlight::Y => &result.y_ticks,
            };
            let endpoints = match axis_hl {
                crate::state::AxisHighlight::X => &result.x_axis,
                crate::state::AxisHighlight::Y => &result.y_axis,
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
                    crate::state::AxisHighlight::X => Color32::from_rgb(0x42, 0x85, 0xF4),
                    crate::state::AxisHighlight::Y => Color32::from_rgb(0x34, 0xA8, 0x53),
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
        if let Some(ref result) = mask.data_result {
            if let Some(group) = result.groups.get(idx) {
                draw_pixel_set_real_color(
                    painter,
                    &group.pixel_coords,
                    to_screen,
                    rgba,
                    mask.width,
                );

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

/// Draw highlighted pixels using their actual color from the image,
/// with white boundary strokes for visibility.
fn draw_pixel_set_real_color(
    painter: &egui::Painter,
    pixels: &[(u32, u32)],
    to_screen: &dyn Fn(f32, f32) -> Pos2,
    rgba: Option<&[u8]>,
    img_width: u32,
) {
    if pixels.is_empty() {
        return;
    }

    let pixel_set: std::collections::HashSet<(u32, u32)> = pixels.iter().copied().collect();
    let w_us = img_width as usize;

    for &(px, py) in pixels {
        let color = if let Some(rgba_data) = rgba {
            let off = ((py as usize) * w_us + (px as usize)) * 4;
            if off + 2 < rgba_data.len() {
                Color32::from_rgb(rgba_data[off], rgba_data[off + 1], rgba_data[off + 2])
            } else {
                Color32::from_rgb(200, 100, 50)
            }
        } else {
            Color32::from_rgb(200, 100, 50)
        };

        let p0 = to_screen(px as f32, py as f32);
        let p1 = to_screen((px + 1) as f32, (py + 1) as f32);
        let rect = egui::Rect::from_min_max(p0, p1);
        painter.rect_filled(rect, 0.0, color);

        // White border on boundary edges
        let is_boundary = |dx: i32, dy: i32| -> bool {
            let nx = px as i32 + dx;
            let ny = py as i32 + dy;
            if nx < 0 || ny < 0 { return true; }
            !pixel_set.contains(&(nx as u32, ny as u32))
        };

        let feather = Color32::from_rgba_unmultiplied(255, 255, 255, 180);

        if is_boundary(0, -1) {
            painter.line_segment([p0, Pos2::new(p1.x, p0.y)], Stroke::new(1.0, feather));
        }
        if is_boundary(0, 1) {
            painter.line_segment([Pos2::new(p0.x, p1.y), p1], Stroke::new(1.0, feather));
        }
        if is_boundary(-1, 0) {
            painter.line_segment([p0, Pos2::new(p0.x, p1.y)], Stroke::new(1.0, feather));
        }
        if is_boundary(1, 0) {
            painter.line_segment([Pos2::new(p1.x, p0.y), p1], Stroke::new(1.0, feather));
        }
    }
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
