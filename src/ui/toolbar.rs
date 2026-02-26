use crate::state::{AppMode, AppState};
use eframe::egui;

pub fn draw_toolbar(state: &mut AppState, ui: &mut egui::Ui, canvas_rect: egui::Rect) {
    let window = egui::Window::new("CAD Toolbar")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::RIGHT_TOP, [-20.0, 20.0]);

    window.show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(state.mode == AppMode::Select, "↖ Select")
                .on_hover_text("Select & Drag (ESC to cancel)")
                .clicked()
            {
                state.mode = AppMode::Select;
                state.selected_calib_idx = None;
            }
            if ui
                .selectable_label(state.mode == AppMode::AddData, "🎯 Add Data")
                .on_hover_text("Pick new points (disabled without 4 calib pts)")
                .clicked()
            {
                if state.calib_pts.len() == 4 {
                    state.mode = AppMode::AddData;
                }
            }
            if ui
                .selectable_label(state.mode == AppMode::Delete, "❌ Delete")
                .on_hover_text("Click points to delete them")
                .clicked()
            {
                state.mode = AppMode::Delete;
            }
            if ui
                .selectable_label(state.mode == AppMode::Pan, "✋ Pan")
                .on_hover_text("Left-click and drag to pan canvas")
                .clicked()
            {
                state.mode = AppMode::Pan;
            }
            if ui
                .button("🎯 Center")
                .on_hover_text("Center canvas to fit window")
                .clicked()
            {
                if state.img_size.x > 0.0 && state.img_size.y > 0.0 {
                    let scale_x = canvas_rect.width() / state.img_size.x;
                    let scale_y = canvas_rect.height() / state.img_size.y;
                    state.zoom = scale_x.min(scale_y) * 0.95; // 5% padding

                    let scaled_size = state.img_size * state.zoom;
                    // Compute pan to center the scaled image in the canvas
                    // pan is the top-left offset relative to canvas_rect.min
                    state.pan = (canvas_rect.size() - scaled_size) / 2.0;
                } else {
                    state.pan = egui::Vec2::ZERO;
                    state.zoom = 1.0;
                }
            }
        });
    });
}
