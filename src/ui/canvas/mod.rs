pub mod keyboard;
pub mod mouse;

use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

use crate::action::Action;
use crate::state::{AppMode, AppState};
use crate::ui::toolbar::draw_toolbar;

pub fn draw_canvas(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

        // Coordinate transforms
        let rect_min = response.rect.min;
        let pan = state.pan;
        let zoom = state.zoom;

        let to_screen =
            |px: f32, py: f32| -> Pos2 { rect_min + pan + Vec2::new(px * zoom, py * zoom) };

        // --- Input handling ---
        keyboard::handle_keyboard(state, ctx, &response, actions);
        mouse::handle_mouse(state, ctx, &response, actions, &to_screen);

        // --- Drawing ---

        // Draw Image (use cleaned texture if grid removal is active)
        let display_texture = if state.grid_removal.enabled {
            state
                .grid_removal
                .cleaned_texture
                .as_ref()
                .or(state.texture.as_ref())
        } else {
            state.texture.as_ref()
        };
        if let Some(texture) = display_texture {
            let rect = Rect::from_min_size(rect_min + pan, state.img_size * zoom);
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

        // --- Draw Mask Overlay (after image, before points) ---
        crate::recognition::mask::draw_mask_overlay(state, &painter, &to_screen, zoom);

        // --- Draw Mask Highlights (axis/data hover) ---
        crate::recognition::mask::draw_mask_highlights(state, ctx, &painter, &to_screen, zoom);

        // Draw Box Selection Rectangle
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

        // --- Draw Axes & Reference Lines ---
        const GOOGLE_BLUE: Color32 = Color32::from_rgb(0x42, 0x85, 0xF4);
        const GOOGLE_GREEN: Color32 = Color32::from_rgb(0x34, 0xA8, 0x53);

        draw_axes(&painter, state, &to_screen, GOOGLE_BLUE, GOOGLE_GREEN);

        // --- Draw Data Points ---
        for (i, p) in state.data_pts.iter().enumerate() {
            let sp = to_screen(p.px, p.py);
            let is_selected = state.selected_data_indices.contains(&i);
            let is_hovered = state.hovered_data_idx == Some(i);

            let draw_color = state
                .groups
                .get(p.group_id)
                .map(|g| g.color)
                .unwrap_or(Color32::WHITE);

            draw_point_target(&painter, sp, draw_color, is_selected, is_hovered);
        }

        // --- Draw Calibration Points ---
        let calib_colors = [GOOGLE_BLUE, GOOGLE_BLUE, GOOGLE_GREEN, GOOGLE_GREEN];
        let calib_labels = ["X1", "X2", "Y1", "Y2"];
        for (i, p) in state.calib_pts.iter().enumerate() {
            let sp = to_screen(p.px, p.py);
            let col = calib_colors[i];
            let is_selected = state.selected_calib_idx == Some(i);

            let cross_size = if is_selected { 14.0 } else { 12.0 };
            let cross_stroke = Stroke::new(if is_selected { 5.0 } else { 4.0 }, col);
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

            let (text_pos, text_align) = if i < 2 {
                (sp + Vec2::new(10.0, -15.0), egui::Align2::LEFT_BOTTOM)
            } else {
                (sp + Vec2::new(-10.0, -15.0), egui::Align2::RIGHT_BOTTOM)
            };

            painter.text(
                text_pos + Vec2::new(1.0, 1.0),
                text_align,
                calib_labels[i],
                egui::FontId::proportional(16.0),
                Color32::BLACK,
            );
            painter.text(
                text_pos,
                text_align,
                calib_labels[i],
                egui::FontId::proportional(16.0),
                col,
            );
        }

        // Center canvas if requested
        if state.center_requested {
            actions.push(Action::CenterCanvas(response.rect));
        }

        // --- Toolbar overlay ---
        draw_toolbar(state, ui, response.rect, actions);

        // --- Sub-toolbar (mask brush + results, or grid removal) ---
        crate::ui::sub_toolbar::draw_sub_toolbar(state, ui, actions);

        // --- Mask brush cursor ---
        crate::recognition::mask::draw_mask_cursor(state, &painter, ctx, &response, zoom);

        // --- Cursor style ---
        let is_alt_pressed = ctx.input(|i| i.modifiers.alt);
        let is_space_held = ctx.input(|i| i.key_down(egui::Key::Space));
        let effective_mode = if is_space_held {
            AppMode::Pan
        } else if is_alt_pressed
            && (state.mode == AppMode::Select || state.mode == AppMode::AddData)
        {
            AppMode::Delete
        } else {
            state.mode
        };

        if response.hovered() {
            match effective_mode {
                AppMode::Delete => ctx.set_cursor_icon(egui::CursorIcon::Crosshair),
                AppMode::Pan => ctx.set_cursor_icon(egui::CursorIcon::Grab),
                AppMode::AddData | AppMode::AddCalib => {
                    ctx.set_cursor_icon(egui::CursorIcon::Crosshair)
                }
                AppMode::AxisMask | AppMode::DataMask => {
                    // Hide default cursor — we draw a custom brush circle
                    ctx.set_cursor_icon(egui::CursorIcon::None);
                }
                _ if state.box_start.is_some() => ctx.set_cursor_icon(egui::CursorIcon::Crosshair),
                _ => {}
            }
        }
    });
}

// ────────────────────────────────────────────────────────────────
//  Helper: draw data-point target circle
// ────────────────────────────────────────────────────────────────
fn draw_point_target(
    painter: &egui::Painter,
    sp: Pos2,
    col: Color32,
    is_selected: bool,
    is_hovered: bool,
) {
    let radius = if is_selected {
        7.0
    } else if is_hovered {
        6.0
    } else {
        5.0
    };

    let alpha = col.a();
    let black_border = Color32::from_rgba_unmultiplied(0, 0, 0, alpha);
    let white_border = Color32::from_rgba_unmultiplied(255, 255, 255, alpha);

    painter.circle_filled(sp, radius, col);
    painter.circle_stroke(sp, radius, Stroke::new(1.5, white_border));
    painter.circle_stroke(sp, radius + 1.5, Stroke::new(1.5, black_border));
}

// ────────────────────────────────────────────────────────────────
//  Helper: draw calibration axes & reference lines
// ────────────────────────────────────────────────────────────────
fn draw_axes(
    painter: &egui::Painter,
    state: &AppState,
    to_screen: &dyn Fn(f32, f32) -> Pos2,
    google_blue: Color32,
    google_green: Color32,
) {
    if state.calib_pts.is_empty() {
        return;
    }

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

    let draw_dashed_line_inf = |painter: &egui::Painter, p1: Pos2, dir: Vec2, stroke: Stroke| {
        if dir.x.is_nan() || dir.y.is_nan() {
            return;
        }
        let dash_len = 6.0;
        let gap_len = 4.0;
        let dash_period = dash_len + gap_len;

        let center = painter.clip_rect().center();
        let to_center = center - p1;
        let t_center = to_center.x * dir.x + to_center.y * dir.y;

        let t_start = t_center - 4000.0;
        let mut t = (t_start / dash_period).floor() * dash_period;
        let t_end = t_center + 4000.0;

        while t < t_end {
            let start = p1 + dir * t;
            let end = p1 + dir * (t + dash_len);
            painter.line_segment([start, end], stroke);
            t += dash_period;
        }
    };

    // X-Axis logic: calib_pts[0] and [1]
    if p_len >= 2 {
        let x1_pos = to_screen(state.calib_pts[0].px, state.calib_pts[0].py);
        let x2_pos = to_screen(state.calib_pts[1].px, state.calib_pts[1].py);

        let dir_x1x2 = (x2_pos - x1_pos).normalized();
        draw_dashed_line_inf(
            painter,
            x1_pos,
            dir_x1x2,
            Stroke::new(2.0, google_blue.linear_multiply(0.5)),
        );

        draw_dashed_line_inf(
            painter,
            x1_pos,
            Vec2::new(1.0, 0.0),
            Stroke::new(2.0, google_blue.linear_multiply(0.5)),
        );

        let proj_x2 = Pos2::new(x2_pos.x, x1_pos.y);
        draw_dashed_line(
            painter,
            x2_pos,
            proj_x2,
            Stroke::new(2.0, google_blue.linear_multiply(0.5)),
        );

        painter.line_segment(
            [Pos2::new(x1_pos.x, x1_pos.y), Pos2::new(x2_pos.x, x1_pos.y)],
            Stroke::new(3.0, google_blue),
        );

        let text_color = google_blue;
        let x1_val_pos = Pos2::new(x1_pos.x, x1_pos.y + 15.0);
        painter.text(
            x1_val_pos + Vec2::new(1.0, 1.0),
            egui::Align2::CENTER_TOP,
            &state.x1_val,
            egui::FontId::proportional(16.0),
            Color32::BLACK,
        );
        painter.text(
            x1_val_pos,
            egui::Align2::CENTER_TOP,
            &state.x1_val,
            egui::FontId::proportional(16.0),
            text_color,
        );

        let x2_val_pos = Pos2::new(x2_pos.x, x1_pos.y + 15.0);
        painter.text(
            x2_val_pos + Vec2::new(1.0, 1.0),
            egui::Align2::CENTER_TOP,
            &state.x2_val,
            egui::FontId::proportional(16.0),
            Color32::BLACK,
        );
        painter.text(
            x2_val_pos,
            egui::Align2::CENTER_TOP,
            &state.x2_val,
            egui::FontId::proportional(16.0),
            text_color,
        );
    }

    // Y-Axis logic
    if p_len >= 4 {
        let y1_pos = to_screen(state.calib_pts[2].px, state.calib_pts[2].py);
        let y2_pos = to_screen(state.calib_pts[3].px, state.calib_pts[3].py);

        let dir_y1y2 = (y2_pos - y1_pos).normalized();
        draw_dashed_line_inf(
            painter,
            y1_pos,
            dir_y1y2,
            Stroke::new(2.0, google_green.linear_multiply(0.5)),
        );

        draw_dashed_line_inf(
            painter,
            y1_pos,
            Vec2::new(0.0, 1.0),
            Stroke::new(2.0, google_green.linear_multiply(0.5)),
        );

        let proj_y2 = Pos2::new(y1_pos.x, y2_pos.y);
        draw_dashed_line(
            painter,
            y2_pos,
            proj_y2,
            Stroke::new(2.0, google_green.linear_multiply(0.5)),
        );

        painter.line_segment(
            [Pos2::new(y1_pos.x, y1_pos.y), Pos2::new(y1_pos.x, y2_pos.y)],
            Stroke::new(3.0, google_green),
        );

        let text_color = google_green;
        let y1_val_pos = Pos2::new(y1_pos.x - 15.0, y1_pos.y);
        painter.text(
            y1_val_pos + Vec2::new(1.0, 1.0),
            egui::Align2::RIGHT_CENTER,
            &state.y1_val,
            egui::FontId::proportional(16.0),
            Color32::BLACK,
        );
        painter.text(
            y1_val_pos,
            egui::Align2::RIGHT_CENTER,
            &state.y1_val,
            egui::FontId::proportional(16.0),
            text_color,
        );

        let y2_val_pos = Pos2::new(y1_pos.x - 15.0, y2_pos.y);
        painter.text(
            y2_val_pos + Vec2::new(1.0, 1.0),
            egui::Align2::RIGHT_CENTER,
            &state.y2_val,
            egui::FontId::proportional(16.0),
            Color32::BLACK,
        );
        painter.text(
            y2_val_pos,
            egui::Align2::RIGHT_CENTER,
            &state.y2_val,
            egui::FontId::proportional(16.0),
            text_color,
        );
    }
}
