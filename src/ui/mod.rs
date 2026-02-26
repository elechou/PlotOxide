pub mod canvas;
pub mod panel;
pub mod toolbar;

use crate::state::AppState;
use eframe::egui;

pub fn draw_ui(state: &mut AppState, ctx: &egui::Context) {
    // Top Panel: Unified Toolbar
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("PlotDigitizer");
            ui.add_space(20.0);

            if ui.button("Load Image").clicked() {
                crate::ui::panel::load_image(state, ctx);
            }
            if ui.button("Export CSV").clicked() {
                crate::ui::panel::export_csv(state);
            }
        });
        ui.add_space(8.0);
    });

    // Left Sidebar for Control Panels
    panel::draw_panel(state, ctx);

    // Central Image Viewport Canvas & Toolbar
    canvas::draw_canvas(state, ctx);
}
