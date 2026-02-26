#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod state;
mod ui;

use eframe::egui;
use state::AppState;

struct PlotDigitizerApp {
    state: AppState,
}

impl Default for PlotDigitizerApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

impl PlotDigitizerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize look
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        cc.egui_ctx.set_style(style);
        
        Self::default()
    }
}

impl eframe::App for PlotDigitizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::draw_ui(&mut self.state, ctx);
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "PlotDigitizer",
        options,
        Box::new(|cc| Ok(Box::new(PlotDigitizerApp::new(cc)))),
    )
}
