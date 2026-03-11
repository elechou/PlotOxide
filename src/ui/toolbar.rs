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
        .anchor(egui::Align2::RIGHT_TOP, [-5.0, 25.0]);

    window.show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(state.mode == AppMode::Select, "\u{2196} Select")
                .on_hover_text("Select & Drag")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Select));
                actions.push(Action::ClearSelection);
            }
            if ui
                .selectable_label(state.mask.active, "\u{1F17E} Mask")
                .on_hover_text("Paint mask for axis detection & data recognition")
                .clicked()
            {
                actions.push(Action::MaskToggle);
            }
            if ui
                .selectable_label(state.mode == AppMode::AddData, "\u{2795} Add Data")
                .on_hover_text("Pick new points")
                .clicked()
            {
                if state.calib_pts.len() == 4 {
                    actions.push(Action::SetMode(AppMode::AddData));
                }
            }

            let is_alt_pressed = ui.ctx().input(|i| i.modifiers.alt);
            if ui
                .selectable_label(
                    state.mode == AppMode::Delete || is_alt_pressed,
                    "\u{2796} Delete",
                )
                .on_hover_text("Click points to delete them (or hold Alt)")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Delete));
            }
            let is_space_pressed = ui.ctx().input(|i| i.key_down(egui::Key::Space));
            if ui
                .selectable_label(
                    state.mode == AppMode::Pan || is_space_pressed,
                    "\u{270B} Pan",
                )
                .on_hover_text("Left-click and drag to pan canvas (or hold Space)")
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
