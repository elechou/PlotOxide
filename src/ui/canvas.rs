use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

use crate::action::Action;

use crate::state::{AppMode, AppState};
use crate::ui::toolbar::draw_toolbar;

pub fn draw_canvas(state: &AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    // Central Image Viewport Canvas
    egui::CentralPanel::default().show(ctx, |ui| {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            actions.push(Action::SetMode(AppMode::Select));
            actions.push(Action::ClearSelection);
        }

        let can_delete = !ctx.wants_keyboard_input() || response.has_focus();
        if (ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)))
            && can_delete
        {
            actions.push(Action::DeleteSelectedPoints);
        }

        // Zoom/Pan
        if response.hovered() {
            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            let mut new_zoom = state.zoom;
            let mut new_pan = state.pan;

            if scroll != 0.0 {
                let zoom_delta = (scroll * 0.005).exp();
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    let rect_pos = response.rect.min;
                    let mouse_rel = mouse_pos - rect_pos - state.pan;
                    new_zoom *= zoom_delta;
                    let new_mouse_rel = mouse_rel * zoom_delta;
                    new_pan -= new_mouse_rel - mouse_rel;
                }
            }

            let mut is_panning = response.dragged_by(egui::PointerButton::Middle)
                || response.dragged_by(egui::PointerButton::Secondary);

            if state.mode == AppMode::Pan && response.dragged_by(egui::PointerButton::Primary) {
                is_panning = true;
            }

            if is_panning {
                new_pan += response.drag_delta();
            }

            if new_zoom != state.zoom || new_pan != state.pan {
                actions.push(Action::SetPanZoom {
                    pan: new_pan,
                    zoom: new_zoom,
                });
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
            actions.push(Action::MoveSelected {
                dx: img_nudge_x,
                dy: img_nudge_y,
            });
            actions.push(Action::RecalculateData);
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

            if state.hovered_calib_idx != hover_hit_calib {
                actions.push(Action::SetHoveredCalib(hover_hit_calib));
            }
            if state.hovered_data_idx != hover_hit_data {
                actions.push(Action::SetHoveredData(hover_hit_data));
            }

            if response.drag_started_by(egui::PointerButton::Primary) {
                if state.mode == AppMode::Select
                    || state.mode == AppMode::AddCalib
                    || state.mode == AppMode::AddData
                {
                    if let Some(idx) = press_hit_calib {
                        actions.push(Action::SetDraggingPoint {
                            is_calib: true,
                            idx: Some(idx),
                        });
                        actions.push(Action::SelectCalibPoint(idx));
                    } else if let Some(idx) = press_hit_data {
                        actions.push(Action::SetDraggingPoint {
                            is_calib: false,
                            idx: Some(idx),
                        });
                        let is_multi = ctx.input(|i| {
                            i.modifiers.shift || i.modifiers.ctrl || i.modifiers.command
                        });
                        if !state.selected_data_indices.contains(&idx) {
                            actions.push(Action::SelectPoints(vec![idx], is_multi));
                        }
                        response.request_focus();
                    } else if state.mode == AppMode::Select {
                        // Dragging on empty space starts box selection natively in Select Mode
                        if let Some(pos) = press_origin {
                            actions.push(Action::SetBoxStart(Some(pos)));
                        }
                    }
                }
            }

            if response.clicked_by(egui::PointerButton::Primary) {
                if state.mode == AppMode::Delete {
                    if let Some(idx) = press_hit_data {
                        actions.push(Action::RemoveDataPoint(idx));
                    }
                } else if state.mode == AppMode::Select {
                    if let Some(idx) = press_hit_calib {
                        actions.push(Action::SelectCalibPoint(idx));
                    } else if let Some(idx) = press_hit_data {
                        let is_multi = ctx.input(|i| {
                            i.modifiers.shift || i.modifiers.command || i.modifiers.ctrl
                        });
                        actions.push(Action::SelectPoints(vec![idx], is_multi));
                        response.request_focus();
                    } else {
                        actions.push(Action::ClearSelection);
                    }
                } else if state.texture.is_some() {
                    let (img_x, img_y) = to_image(mouse_pos, state.pan, state.zoom);

                    if state.mode == AppMode::AddCalib && state.calib_pts.len() < 4 {
                        actions.push(Action::AddCalibPoint { img_x, img_y });
                        response.request_focus();
                    } else if state.mode == AppMode::AddData {
                        actions.push(Action::AddDataPoint { img_x, img_y });
                        response.request_focus();
                    }
                }
            }

            if response.dragged_by(egui::PointerButton::Primary) && state.mode != AppMode::Pan {
                let drag_delta = response.drag_delta() / state.zoom;
                actions.push(Action::MoveSelected {
                    dx: drag_delta.x,
                    dy: drag_delta.y,
                });
                actions.push(Action::RecalculateData);
            }

            if response.drag_stopped() {
                if state.box_start.is_some() {
                    if let Some(start_pos) = state.box_start {
                        let end_pos = mouse_pos;
                        let box_rect = Rect::from_two_pos(start_pos, end_pos);

                        let is_multi = ctx.input(|i| {
                            i.modifiers.shift || i.modifiers.command || i.modifiers.ctrl
                        });

                        // Select all points inside the drawn box
                        let mut selected = Vec::new();
                        for (i, p) in state.data_pts.iter().enumerate() {
                            let sp = to_screen(p.px, p.py, state.pan, state.zoom);
                            if box_rect.contains(sp) {
                                selected.push(i);
                            }
                        }

                        if !selected.is_empty() || !is_multi {
                            actions.push(Action::SelectPoints(selected, is_multi));
                        }
                    }
                    actions.push(Action::SetBoxStart(None));
                }

                if state.dragging_calib_idx.is_some() || state.dragging_data_idx.is_some() {
                    actions.push(Action::StopDragging);
                }
            }
        } else {
            if state.hovered_calib_idx.is_some() {
                actions.push(Action::SetHoveredCalib(None));
            }
            if state.hovered_data_idx.is_some() {
                actions.push(Action::SetHoveredData(None));
            }
            if response.drag_stopped() && state.box_start.is_some() {
                actions.push(Action::SetBoxStart(None));
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

        // Draw Axes based on Calib Points
        if !state.calib_pts.is_empty() {
            let p_len = state.calib_pts.len();

            let draw_dashed_line = |painter: &egui::Painter, p1: Pos2, p2: Pos2, stroke: Stroke| {
                let dir = (p2 - p1).normalized();
                if dir.x.is_nan() || dir.y.is_nan() {
                    return;
                }
                let length = p1.distance(p2);
                let dash_len = 6.0;
                let gap_len = 4.0;
                let mut t = 0.0;
                while t < length {
                    let start = p1 + dir * t;
                    let end = p1 + dir * (t + dash_len).min(length);
                    painter.line_segment([start, end], stroke);
                    t += dash_len + gap_len;
                }
            };

            let draw_dashed_line_inf =
                |painter: &egui::Painter, p1: Pos2, dir: Vec2, stroke: Stroke| {
                    if dir.x.is_nan() || dir.y.is_nan() {
                        return;
                    }
                    let dash_len = 6.0;
                    let gap_len = 4.0;
                    let mut t = -4000.0;
                    while t < 4000.0 {
                        let start = p1 + dir * t;
                        let end = p1 + dir * (t + dash_len);
                        painter.line_segment([start, end], stroke);
                        t += dash_len + gap_len;
                    }
                };

            // X-Axis logic: calib_pts[0] and [1]
            if p_len >= 2 {
                let x1_pos = to_screen(
                    state.calib_pts[0].px,
                    state.calib_pts[0].py,
                    state.pan,
                    state.zoom,
                );
                let x2_pos = to_screen(
                    state.calib_pts[1].px,
                    state.calib_pts[1].py,
                    state.pan,
                    state.zoom,
                );

                // 1. Dashed line through X1 and X2 (slope)
                let dir_x1x2 = (x2_pos - x1_pos).normalized();
                draw_dashed_line_inf(
                    &painter,
                    x1_pos,
                    dir_x1x2,
                    Stroke::new(1.0, GOOGLE_BLUE.linear_multiply(0.5)),
                );

                // 2. Dashed horizontal line through X1
                draw_dashed_line_inf(
                    &painter,
                    x1_pos,
                    Vec2::new(1.0, 0.0),
                    Stroke::new(1.0, GOOGLE_BLUE.linear_multiply(0.5)),
                );

                // 3. Dashed vertical line passing through X2 to X1's horizontal baseline
                let proj_x2 = Pos2::new(x2_pos.x, x1_pos.y);
                draw_dashed_line(
                    &painter,
                    x2_pos,
                    proj_x2,
                    Stroke::new(1.0, GOOGLE_BLUE.linear_multiply(0.5)),
                );

                // 4. Thick solid horizontal line from X1.x to X2.x along X1's baseline
                painter.line_segment(
                    [Pos2::new(x1_pos.x, x1_pos.y), Pos2::new(x2_pos.x, x1_pos.y)],
                    Stroke::new(3.0, GOOGLE_BLUE),
                );

                // Add text below X1 and X2 on the thick line
                let text_color = GOOGLE_BLUE;
                painter.text(
                    Pos2::new(x1_pos.x, x1_pos.y + 15.0),
                    egui::Align2::CENTER_TOP,
                    &state.x1_val,
                    egui::FontId::proportional(14.0),
                    text_color,
                );
                painter.text(
                    Pos2::new(x2_pos.x, x1_pos.y + 15.0),
                    egui::Align2::CENTER_TOP,
                    &state.x2_val,
                    egui::FontId::proportional(14.0),
                    text_color,
                );
            }

            // Y-Axis logic
            if p_len >= 4 {
                let y1_pos = to_screen(
                    state.calib_pts[2].px,
                    state.calib_pts[2].py,
                    state.pan,
                    state.zoom,
                );
                let y2_pos = to_screen(
                    state.calib_pts[3].px,
                    state.calib_pts[3].py,
                    state.pan,
                    state.zoom,
                );

                // 1. Dashed line through Y1 and Y2 (slope)
                let dir_y1y2 = (y2_pos - y1_pos).normalized();
                draw_dashed_line_inf(
                    &painter,
                    y1_pos,
                    dir_y1y2,
                    Stroke::new(1.0, GOOGLE_GREEN.linear_multiply(0.5)),
                );

                // 2. Dashed vertical line through Y1
                draw_dashed_line_inf(
                    &painter,
                    y1_pos,
                    Vec2::new(0.0, 1.0),
                    Stroke::new(1.0, GOOGLE_GREEN.linear_multiply(0.5)),
                );

                // 3. Dashed horizontal line passing through Y2 to Y1's vertical baseline
                let proj_y2 = Pos2::new(y1_pos.x, y2_pos.y);
                draw_dashed_line(
                    &painter,
                    y2_pos,
                    proj_y2,
                    Stroke::new(1.0, GOOGLE_GREEN.linear_multiply(0.5)),
                );

                // 4. Thick solid vertical line from Y1.y to Y2.y along Y1's vertical baseline
                painter.line_segment(
                    [Pos2::new(y1_pos.x, y1_pos.y), Pos2::new(y1_pos.x, y2_pos.y)],
                    Stroke::new(3.0, GOOGLE_GREEN),
                );

                // Add text left to Y1 and Y2
                let text_color = GOOGLE_GREEN;
                painter.text(
                    Pos2::new(y1_pos.x - 15.0, y1_pos.y),
                    egui::Align2::RIGHT_CENTER,
                    &state.y1_val,
                    egui::FontId::proportional(14.0),
                    text_color,
                );
                painter.text(
                    Pos2::new(y1_pos.x - 15.0, y2_pos.y),
                    egui::Align2::RIGHT_CENTER,
                    &state.y2_val,
                    egui::FontId::proportional(14.0),
                    text_color,
                );
            }
        }

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
            actions.push(Action::CenterCanvas(response.rect));
        }

        // Draw CAD Toolbar layer
        draw_toolbar(state, ui, response.rect, actions);

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
