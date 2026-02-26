use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

use crate::core::{recalculate_data, CalibPoint, DataPoint};
use crate::state::{AppMode, AppState};
use crate::ui::toolbar::draw_toolbar;

pub fn draw_canvas(state: &mut AppState, ctx: &egui::Context) {
    // Central Image Viewport Canvas
    egui::CentralPanel::default().show(ctx, |ui| {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.mode = AppMode::Select;
            state.selected_calib_idx = None;
            state.selected_data_indices.clear();
            state.dragging_calib_idx = None;
            state.dragging_data_idx = None;
        }

        let can_delete = !ctx.wants_keyboard_input() || response.has_focus();
        if (ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)))
            && can_delete
        {
            let mut to_remove: Vec<usize> = state.selected_data_indices.iter().copied().collect();
            to_remove.sort_unstable_by(|a, b| b.cmp(a));
            for idx in to_remove {
                if idx < state.data_pts.len() {
                    state.data_pts.remove(idx);
                }
            }
            state.selected_data_indices.clear();
        }

        // Zoom/Pan
        if response.hovered() {
            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let zoom_delta = (scroll * 0.005).exp();
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    let rect_pos = response.rect.min;
                    let mouse_rel = mouse_pos - rect_pos - state.pan;
                    state.zoom *= zoom_delta;
                    let new_mouse_rel = mouse_rel * zoom_delta;
                    state.pan -= new_mouse_rel - mouse_rel;
                }
            }
            let mut is_panning = response.dragged_by(egui::PointerButton::Middle)
                || response.dragged_by(egui::PointerButton::Secondary);

            if state.mode == AppMode::Pan && response.dragged_by(egui::PointerButton::Primary) {
                is_panning = true;
            }

            if is_panning {
                state.pan += response.drag_delta();
            }
        }

        // Draw Image
        if let Some(texture) = &state.texture {
            let rect =
                Rect::from_min_size(response.rect.min + state.pan, state.img_size * state.zoom);
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "No Image Loaded.",
                egui::FontId::proportional(20.0),
                Color32::GRAY,
            );
        }

        // Coordinate transforms
        let rect_min = response.rect.min;
        let to_screen = |px: f32, py: f32, pan: Vec2, zoom: f32| -> Pos2 {
            rect_min + pan + Vec2::new(px * zoom, py * zoom)
        };
        let to_image = |pos: Pos2, pan: Vec2, zoom: f32| -> (f32, f32) {
            let pt = pos - rect_min - pan;
            (pt.x / zoom, pt.y / zoom)
        };

        let threshold = 15.0; // Px radius for clicking

        // Global Keyboard Nudging
        let mut moved = false;
        let mut nudge_x = 0.0;
        let mut nudge_y = 0.0;
        if response.hovered() || response.has_focus() {
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                nudge_y -= 1.0;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                nudge_y += 1.0;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                nudge_x -= 1.0;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                nudge_x += 1.0;
            }
        }

        if nudge_x != 0.0 || nudge_y != 0.0 {
            let img_nudge_x = nudge_x / state.zoom;
            let img_nudge_y = nudge_y / state.zoom;
            if let Some(idx) = state.selected_calib_idx {
                state.calib_pts[idx].px += img_nudge_x;
                state.calib_pts[idx].py += img_nudge_y;
                moved = true;
            } else if !state.selected_data_indices.is_empty() {
                for &idx in &state.selected_data_indices {
                    if idx < state.data_pts.len() {
                        state.data_pts[idx].px += img_nudge_x;
                        state.data_pts[idx].py += img_nudge_y;
                    }
                }
                moved = true;
            }
            if moved {
                recalculate_data(
                    &state.calib_pts,
                    &mut state.data_pts,
                    &state.x1_val,
                    &state.x2_val,
                    &state.y1_val,
                    &state.y2_val,
                    state.log_x,
                    state.log_y,
                );
            }
        }

        // Handle Clicks
        let mouse_pos = ctx
            .input(|i| i.pointer.hover_pos())
            .or_else(|| ctx.input(|i| i.pointer.interact_pos()));
        let press_origin = ctx.input(|i| i.pointer.press_origin());

        if let Some(mouse_pos) = mouse_pos {
            let find_hit = |pos: Pos2| -> (Option<usize>, Option<usize>) {
                for (i, p) in state.calib_pts.iter().enumerate() {
                    if to_screen(p.px, p.py, state.pan, state.zoom).distance(pos) < threshold {
                        return (Some(i), None);
                    }
                }
                for (i, p) in state.data_pts.iter().enumerate() {
                    if to_screen(p.px, p.py, state.pan, state.zoom).distance(pos) < threshold {
                        return (None, Some(i));
                    }
                }
                (None, None)
            };

            let (hover_hit_calib, hover_hit_data) = find_hit(mouse_pos);
            let (press_hit_calib, press_hit_data) = if let Some(origin) = press_origin {
                find_hit(origin)
            } else {
                (hover_hit_calib, hover_hit_data)
            };

            state.hovered_calib_idx = hover_hit_calib;
            state.hovered_data_idx = hover_hit_data;

            if response.drag_started_by(egui::PointerButton::Primary) {
                if state.mode == AppMode::Select
                    || state.mode == AppMode::AddCalib
                    || state.mode == AppMode::AddData
                {
                    if let Some(idx) = press_hit_calib {
                        state.dragging_calib_idx = Some(idx);
                        state.selected_calib_idx = Some(idx);
                        state.selected_data_indices.clear();
                        response.request_focus();
                    } else if let Some(idx) = press_hit_data {
                        state.dragging_data_idx = Some(idx);
                        if !state.selected_data_indices.contains(&idx) {
                            if !ctx.input(|i| {
                                i.modifiers.shift || i.modifiers.ctrl || i.modifiers.command
                            }) {
                                state.selected_data_indices.clear();
                            }
                            state.selected_data_indices.insert(idx);
                        }
                        state.selected_calib_idx = None;
                        response.request_focus();
                    } else if state.mode == AppMode::Select {
                        // Dragging on empty space starts box selection natively in Select Mode
                        if let Some(pos) = press_origin {
                            state.box_start = Some(pos);
                        }
                    }
                }
            }

            if response.clicked_by(egui::PointerButton::Primary) {
                if state.mode == AppMode::Delete {
                    if let Some(idx) = press_hit_data {
                        state.data_pts.remove(idx);
                        state.selected_data_indices.remove(&idx);

                        // Decrement higher selected indices to align with shifted data array
                        let mut new_indices = std::collections::HashSet::new();
                        for &selected in &state.selected_data_indices {
                            if selected > idx {
                                new_indices.insert(selected - 1);
                            } else {
                                new_indices.insert(selected);
                            }
                        }
                        state.selected_data_indices = new_indices;
                    }
                } else if state.mode == AppMode::Select {
                    if let Some(idx) = press_hit_calib {
                        state.selected_calib_idx = Some(idx);
                        state.selected_data_indices.clear();
                        response.request_focus();
                    } else if let Some(idx) = press_hit_data {
                        let modifiers = ctx.input(|i| i.modifiers);
                        if modifiers.shift || modifiers.command || modifiers.ctrl {
                            if state.selected_data_indices.contains(&idx) {
                                state.selected_data_indices.remove(&idx);
                            } else {
                                state.selected_data_indices.insert(idx);
                            }
                        } else {
                            state.selected_data_indices.clear();
                            state.selected_data_indices.insert(idx);
                        }
                        state.selected_calib_idx = None;
                        response.request_focus();
                    } else {
                        state.selected_calib_idx = None;
                        state.selected_data_indices.clear();
                    }
                } else if state.texture.is_some() {
                    let (img_x, img_y) = to_image(mouse_pos, state.pan, state.zoom);

                    if state.mode == AppMode::AddCalib && state.calib_pts.len() < 4 {
                        state.calib_pts.push(CalibPoint {
                            px: img_x,
                            py: img_y,
                        });
                        state.selected_calib_idx = Some(state.calib_pts.len() - 1);
                        state.selected_data_indices.clear();
                        response.request_focus();

                        if state.calib_pts.len() == 4 {
                            state.mode = AppMode::AddData;
                        }
                        recalculate_data(
                            &state.calib_pts,
                            &mut state.data_pts,
                            &state.x1_val,
                            &state.x2_val,
                            &state.y1_val,
                            &state.y2_val,
                            state.log_x,
                            state.log_y,
                        );
                    } else if state.mode == AppMode::AddData {
                        state.data_pts.push(DataPoint {
                            px: img_x,
                            py: img_y,
                            lx: 0.0,
                            ly: 0.0,
                            group_id: state.active_group_idx,
                        });
                        state.selected_data_indices.clear();
                        state.selected_data_indices.insert(state.data_pts.len() - 1);
                        state.selected_calib_idx = None;
                        response.request_focus();
                        recalculate_data(
                            &state.calib_pts,
                            &mut state.data_pts,
                            &state.x1_val,
                            &state.x2_val,
                            &state.y1_val,
                            &state.y2_val,
                            state.log_x,
                            state.log_y,
                        );
                    }
                }
            }

            if response.dragged_by(egui::PointerButton::Primary) && state.mode != AppMode::Pan {
                let drag_delta = response.drag_delta() / state.zoom;
                if let Some(idx) = state.dragging_calib_idx {
                    state.calib_pts[idx].px += drag_delta.x;
                    state.calib_pts[idx].py += drag_delta.y;
                } else if state.dragging_data_idx.is_some() {
                    // Multi-dragging for data points
                    for &idx in &state.selected_data_indices {
                        if idx < state.data_pts.len() {
                            state.data_pts[idx].px += drag_delta.x;
                            state.data_pts[idx].py += drag_delta.y;
                        }
                    }
                }
            }

            if response.drag_stopped() {
                if state.box_start.is_some() {
                    if let Some(start_pos) = state.box_start {
                        let end_pos = mouse_pos;
                        let box_rect = Rect::from_two_pos(start_pos, end_pos);

                        // If not holding shift/ctrl, clear selection before box checking
                        if !ctx
                            .input(|i| i.modifiers.shift || i.modifiers.command || i.modifiers.ctrl)
                        {
                            state.selected_data_indices.clear();
                        }

                        // Select all points inside the drawn box
                        for (i, p) in state.data_pts.iter().enumerate() {
                            let sp = to_screen(p.px, p.py, state.pan, state.zoom);
                            if box_rect.contains(sp) {
                                state.selected_data_indices.insert(i);
                            }
                        }
                    }
                    state.box_start = None;
                }

                if state.dragging_calib_idx.is_some() || state.dragging_data_idx.is_some() {
                    state.dragging_calib_idx = None;
                    state.dragging_data_idx = None;
                    recalculate_data(
                        &state.calib_pts,
                        &mut state.data_pts,
                        &state.x1_val,
                        &state.x2_val,
                        &state.y1_val,
                        &state.y2_val,
                        state.log_x,
                        state.log_y,
                    );
                }
            }
        } else {
            state.hovered_calib_idx = None;
            state.hovered_data_idx = None;
            if response.drag_stopped() {
                state.box_start = None;
            }
        }

        // Render Box Selection Rectangle implicitly from AppMode::Select or AddData dragging on empty space
        if state.box_start.is_some() {
            if let Some(start_pos) = state.box_start {
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    let box_rect = Rect::from_two_pos(start_pos, mouse_pos);
                    painter.rect_filled(
                        box_rect,
                        0.0,
                        Color32::from_rgba_unmultiplied(50, 150, 250, 40),
                    );
                    painter.rect_stroke(
                        box_rect,
                        0.0,
                        Stroke::new(1.0, Color32::from_rgb(50, 150, 250)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }

        const GOOGLE_BLUE: Color32 = Color32::from_rgb(0x42, 0x85, 0xF4);
        const GOOGLE_GREEN: Color32 = Color32::from_rgb(0x34, 0xA8, 0x53);
        // const GOOGLE_RED: Color32 = Color32::from_rgb(0xEA, 0x43, 0x35);

        let draw_point_target = |sp: Pos2, col: Color32, is_selected: bool, is_hovered: bool| {
            let (r_blk, r_wht, r_in) = if is_selected {
                (12.0, 9.0, 6.0)
            } else if is_hovered {
                (10.0, 8.0, 6.0)
            } else {
                (9.0, 7.0, 5.0)
            };

            painter.circle_filled(sp, r_blk, Color32::BLACK);
            painter.circle_filled(sp, r_wht, Color32::WHITE);
            painter.circle_filled(sp, r_in, col);
        };

        // Draw Points
        for (i, p) in state.data_pts.iter().enumerate() {
            let sp = to_screen(p.px, p.py, state.pan, state.zoom);
            let is_selected = state.selected_data_indices.contains(&i);

            // Delete mode cursor visual override
            let is_hovered = state.hovered_data_idx == Some(i);

            let draw_color = state
                .groups
                .get(p.group_id)
                .map(|g| g.color)
                .unwrap_or(Color32::WHITE);

            if state.mode == AppMode::Delete && is_hovered {
                draw_point_target(sp, Color32::BLACK, true, false);
            } else {
                draw_point_target(sp, draw_color, is_selected, is_hovered);
            }
        }

        let calib_colors = [GOOGLE_BLUE, GOOGLE_BLUE, GOOGLE_GREEN, GOOGLE_GREEN];
        let calib_labels = ["X1", "X2", "Y1", "Y2"];
        for (i, p) in state.calib_pts.iter().enumerate() {
            let sp = to_screen(p.px, p.py, state.pan, state.zoom);
            let col = calib_colors[i];
            let is_selected = state.selected_calib_idx == Some(i);
            let is_hovered = state.hovered_calib_idx == Some(i);

            let cross_size = if is_selected { 14.0 } else { 10.0 };
            let cross_stroke = Stroke::new(if is_selected { 3.0 } else { 2.0 }, col);
            painter.line_segment(
                [
                    sp - Vec2::new(cross_size, cross_size),
                    sp + Vec2::new(cross_size, cross_size),
                ],
                cross_stroke,
            );
            painter.line_segment(
                [
                    sp - Vec2::new(cross_size, -cross_size),
                    sp + Vec2::new(cross_size, -cross_size),
                ],
                cross_stroke,
            );

            draw_point_target(sp, col, is_selected, is_hovered);

            let text_pos = sp + Vec2::new(10.0, -15.0);
            painter.text(
                text_pos + Vec2::new(1.0, 1.0),
                egui::Align2::LEFT_BOTTOM,
                calib_labels[i],
                egui::FontId::proportional(14.0),
                Color32::BLACK,
            );
            painter.text(
                text_pos,
                egui::Align2::LEFT_BOTTOM,
                calib_labels[i],
                egui::FontId::proportional(14.0),
                col,
            );
        }

        if state.center_requested {
            if state.img_size.x > 0.0 && state.img_size.y > 0.0 {
                let scale_x = response.rect.width() / state.img_size.x;
                let scale_y = response.rect.height() / state.img_size.y;
                state.zoom = scale_x.min(scale_y) * 0.95; // 5% padding

                let scaled_size = state.img_size * state.zoom;
                state.pan = (response.rect.size() - scaled_size) / 2.0;
            } else {
                state.pan = egui::Vec2::ZERO;
                state.zoom = 1.0;
            }
            state.center_requested = false;
        }

        // Draw CAD Toolbar layer
        draw_toolbar(state, ui, response.rect);

        // Delete mode custom cursor
        if state.mode == AppMode::Delete && response.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Default);
        } else if state.mode == AppMode::Pan && response.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        } else if state.box_start.is_some() && response.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Cell);
        }
    });
}
