use crate::action::Action;
use crate::i18n::t;
use crate::icons;
use crate::state::{AppMode, AppState};
use eframe::egui;

pub fn draw_toolbar(
    state: &AppState,
    ui: &mut egui::Ui,
    canvas_rect: egui::Rect,
    actions: &mut Vec<Action>,
) {
    let lang = state.lang;
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
                    format!("{} {}", icons::CURSOR_DEFAULT, t(lang, "select")),
                )
                .on_hover_text(t(lang, "hover_select_drag"))
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Select));
                actions.push(Action::ClearSelection);
            }
            if ui
                .selectable_label(
                    state.mode == AppMode::AddData,
                    format!("{} {}", icons::PLUS, t(lang, "add_data")),
                )
                .on_hover_text(t(lang, "hover_pick_points"))
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::AddData));
            }

            if ui
                .selectable_label(
                    state.mode == AppMode::Delete,
                    format!("{} {}", icons::MINUS, t(lang, "delete")),
                )
                .on_hover_text(t(lang, "hover_delete_points"))
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Delete));
            }

            let magic_active = state.axis_mask.active
                && state.axis_mask.mask_mode == crate::state::MaskMode::AxisCalib;
            if ui
                .selectable_label(
                    magic_active,
                    format!("{} {}", icons::AXIS_BRUSH, t(lang, "axis_brush")),
                )
                .on_hover_text(t(lang, "hover_axis_mask"))
                .clicked()
            {
                actions.push(Action::MaskToggleForAxis);
            }

            let mask_active = state.data_mask.active
                && state.data_mask.mask_mode == crate::state::MaskMode::DataRecog;
            if ui
                .selectable_label(
                    mask_active,
                    format!("{} {}", icons::DATA_BRUSH, t(lang, "data_brush")),
                )
                .on_hover_text(t(lang, "hover_data_mask"))
                .clicked()
            {
                actions.push(Action::MaskToggle);
            }

            let is_space_pressed = ui.ctx().input(|i| i.key_down(egui::Key::Space));
            if ui
                .selectable_label(
                    state.mode == AppMode::Pan || is_space_pressed,
                    format!("{} {}", icons::HAND, t(lang, "pan")),
                )
                .on_hover_text(t(lang, "hover_pan"))
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Pan));
            }
            if ui
                .button(format!("{} {}", icons::FIT_SCREEN, t(lang, "center")))
                .on_hover_text(t(lang, "hover_center"))
                .clicked()
            {
                actions.push(Action::CenterCanvas(canvas_rect));
            }
        });
    });
}
