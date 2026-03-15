use crate::action::Action;
use crate::icons;
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
                .selectable_label(
                    state.mode == AppMode::Select,
                    format!("{} Select", icons::CURSOR_DEFAULT),
                )
                .on_hover_text("Select & Drag")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Select));
                actions.push(Action::ClearSelection);
            }
            if ui
                .selectable_label(
                    state.mode == AppMode::AddData,
                    format!("{} Add Data", icons::PLUS),
                )
                .on_hover_text("Pick new points")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::AddData));
            }

            if ui
                .selectable_label(
                    state.mode == AppMode::Delete,
                    format!("{} Delete", icons::MINUS),
                )
                .on_hover_text("Click points to delete them")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Delete));
            }

            let magic_active = state.axis_mask.active
                && state.axis_mask.mask_mode == crate::state::MaskMode::AxisCalib;
            if ui
                .selectable_label(
                    magic_active,
                    format!("{} Axis Brush", icons::AXIS_BRUSH),
                )
                .on_hover_text("Auto-detect axes by painting a mask")
                .clicked()
            {
                actions.push(Action::MaskToggleForAxis);
            }

            let mask_active = state.data_mask.active
                && state.data_mask.mask_mode == crate::state::MaskMode::DataRecog;
            if ui
                .selectable_label(
                    mask_active,
                    format!("{} Data Brush", icons::DATA_BRUSH),
                )
                .on_hover_text(
                    "Auto-extract data points using color recognition via a painted mask",
                )
                .clicked()
            {
                actions.push(Action::MaskToggle);
            }

            let grid_active = state.mode == AppMode::GridRemoval;
            if ui
                .selectable_label(
                    grid_active,
                    format!("{} Grid", icons::GRID),
                )
                .on_hover_text("Remove grid lines from image using FFT filtering")
                .clicked()
            {
                actions.push(Action::GridRemovalToggle);
            }

            let is_space_pressed = ui.ctx().input(|i| i.key_down(egui::Key::Space));
            if ui
                .selectable_label(
                    state.mode == AppMode::Pan || is_space_pressed,
                    format!("{} Pan", icons::HAND),
                )
                .on_hover_text("Left-click and drag to pan canvas (or hold Space)")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Pan));
            }
            if ui
                .button(format!("{} Center", icons::FIT_SCREEN))
                .on_hover_text("Center canvas to fit window")
                .clicked()
            {
                actions.push(Action::CenterCanvas(canvas_rect));
            }
        });
    });
}
