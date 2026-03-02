#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod ide;
mod script;
mod state;
mod ui;

use eframe::egui;
use state::AppState;

struct PlotRedoxApp {
    state: AppState,
}

impl Default for PlotRedoxApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

impl PlotRedoxApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize look
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        cc.egui_ctx.set_style(style);

        Self::default()
    }
}

impl eframe::App for PlotRedoxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut actions = Vec::new();
        ui::draw_ui(&mut self.state, ctx, &mut actions);
        for action in actions {
            self.state.update(action);
        }
    }
}

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(icon_bytes).ok()?;
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    if let Some(icon) = load_icon() {
        options.viewport = options.viewport.with_icon(icon);
    }

    eframe::run_native(
        "PlotRedox",
        options,
        Box::new(|cc| Ok(Box::new(PlotRedoxApp::new(cc)))),
    )
}
pub mod action;
