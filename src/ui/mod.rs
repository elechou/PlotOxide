pub mod canvas;
pub mod panel;
pub mod toolbar;

use crate::action::Action;
use crate::state::AppState;
use eframe::egui;

pub fn draw_ui(state: &AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    // Top Panel: Unified Toolbar
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("PlotDigitizer");
            ui.add_space(20.0);

            let is_dark = ctx.style().visuals.dark_mode;
            let icon = if is_dark { "☀" } else { "🌙" };
            if ui.button(icon).clicked() {
                if is_dark {
                    ctx.set_visuals(egui::Visuals::light());
                } else {
                    ctx.set_visuals(egui::Visuals::dark());
                }
            }
            ui.add_space(10.0);

            if ui.button("Load Image").clicked() {
                crate::ui::panel::load_image(ctx, actions);
            }
            if ui.button("Export CSV").clicked() {
                actions.push(Action::RequestExportCsv);
            }
        });
        ui.add_space(8.0);
    });

    // Left Sidebar for Control Panels
    panel::draw_panel(state, ctx, actions);

    // Central Image Viewport Canvas & Toolbar
    canvas::draw_canvas(state, ctx, actions);
}
