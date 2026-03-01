#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod ide;
mod script;
mod state;
mod ui;

use eframe::egui;
use state::AppState;

struct PlotOxideApp {
    state: AppState,
}

impl Default for PlotOxideApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

impl PlotOxideApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize look
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        cc.egui_ctx.set_style(style);

        Self::default()
    }
}

impl eframe::App for PlotOxideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut actions = Vec::new();
        ui::draw_ui(&mut self.state, ctx, &mut actions);
        for action in actions {
            self.state.update(action);
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PlotOxide",
        options,
        Box::new(|cc| Ok(Box::new(PlotOxideApp::new(cc)))),
    )
}
pub mod action;
