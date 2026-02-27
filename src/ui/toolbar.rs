use crate::action::Action;
use crate::state::{AppMode, AppState};
use eframe::egui;

pub fn draw_toolbar(
    state: &AppState,
    ui: &mut egui::Ui,
    canvas_rect: egui::Rect,
    actions: &mut Vec<Action>,
) {
    let window = egui::Window::new("CAD Toolbar")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::RIGHT_TOP, [-20.0, 50.0]);

    window.show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(state.mode == AppMode::Select, "↖ Select")
                .on_hover_text("Select & Drag (ESC to cancel)")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Select));
                actions.push(Action::ClearSelection);
            }
            if ui
                .selectable_label(state.mode == AppMode::AddData, "➕ Add Data")
                .on_hover_text("Pick new points (disabled without 4 calib pts)")
                .clicked()
            {
                if state.calib_pts.len() == 4 {
                    actions.push(Action::SetMode(AppMode::AddData));
                }
            }

            let is_alt_pressed = ui.ctx().input(|i| i.modifiers.alt);
            if ui
                .selectable_label(state.mode == AppMode::Delete || is_alt_pressed, "❌ Delete")
                .on_hover_text("Click points to delete them")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Delete));
            }
            if ui
                .selectable_label(state.mode == AppMode::Pan, "✋ Pan")
                .on_hover_text("Left-click and drag to pan canvas")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Pan));
            }
            if ui
                .button("⛶ Center")
                .on_hover_text("Center canvas to fit window")
                .clicked()
            {
                actions.push(Action::CenterCanvas(canvas_rect));
            }
        });
    });
}
