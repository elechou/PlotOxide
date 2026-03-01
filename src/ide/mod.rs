pub mod editor;
pub mod help;
pub mod inspector;
pub mod presets;
pub mod workspace;

use crate::action::Action;
use crate::state::AppState;
use eframe::egui;

pub fn draw_ide(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    if !state.ide.is_open {
        return;
    }

    // Windows for table inspectors
    inspector::draw_inspectors(state, ctx, actions);

    // Help window (floating, independent of IDE panel)
    help::draw_help_window(state, ctx);

    // Bottom Panel IDE
    egui::TopBottomPanel::bottom("ide_panel")
        .resizable(true)
        .min_height(250.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.strong("Script IDE");

                // Presets dropdown + Export (right-aligned, appear left of heading)
                presets::draw_presets(state, ui, actions);

                ui.add_space(10.0);

                // ▶ Run Script (green triangle)
                let run_btn = egui::Button::new(
                    egui::RichText::new("▶ Run Script")
                        .color(egui::Color32::from_rgb(0x4E, 0xC9, 0x4E))
                        .strong(),
                )
                .min_size(egui::vec2(0.0, 0.0));
                if ui.add(run_btn).clicked() {
                    actions.push(Action::RunScript(state.ide.code.clone()));
                }

                // Right-aligned: Help, then Script IDE heading, then presets/export
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Rightmost: Help
                    if ui.button("\u{2139} Help").clicked() {
                        actions.push(Action::ToggleHelp);
                    }
                });
            });
            ui.separator();

            // Workspace (left) — fixed width
            workspace::draw_workspace(state, ui, actions);

            // Calculate 50/50 split for editor and output
            let half_width = ui.available_width() / 2.0;

            // Output (right side of remaining space)
            let max_output_width = (ui.available_width() - 150.0).max(100.0);
            egui::SidePanel::right("ide_output")
                .resizable(true)
                .default_width(half_width)
                .min_width(100.0)
                .max_width(max_output_width)
                .show_inside(ui, |ui| {
                    ui.strong("Output");
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut safe_out = state.ide.output.clone();
                            if safe_out.len() > 5000 {
                                safe_out.truncate(5000);
                                safe_out.push_str("\n... (Output truncated)");
                            }
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(safe_out)
                                        .font(egui::FontId::monospace(14.0)),
                                )
                                .wrap(),
                            );
                        });
                });

            // Editor (center)
            editor::draw_editor(state, ui, actions);
        });
}
