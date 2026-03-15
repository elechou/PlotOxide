pub mod editor;
pub mod help;
pub mod inspector;
pub mod presets;
pub mod workspace;

use crate::action::Action;
use crate::icons;
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
                    egui::RichText::new(format!("{} Run Script", icons::PLAY))
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
                    if ui.button(format!("{} Help", icons::HELP_CIRCLE)).clicked() {
                        actions.push(Action::ToggleHelp);
                    }
                });
            });
            ui.separator();

            // Workspace (left) — fixed width
            workspace::draw_workspace(state, ui, actions);

            // Editor and Output proportional split
            let available = ui.available_rect_before_wrap();
            let mut fraction = state.ide.output_fraction;
            let handle_width = 1.0;

            let content_width = (available.width() - handle_width).max(10.0);

            ui.horizontal(|ui| {
                let editor_width = (content_width * (1.0 - fraction)).max(50.0);
                let output_width = (content_width * fraction).max(50.0);

                // Editor (left proportionally)
                ui.allocate_ui_with_layout(
                    egui::vec2(editor_width, available.height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        editor::draw_editor(state, ui, actions);
                    },
                );

                // Drag handle (center)
                let (handle_rect, handle_resp) = ui.allocate_exact_size(
                    egui::vec2(handle_width, available.height()),
                    egui::Sense::drag(),
                );

                // Draw a visual separator
                let color = if handle_resp.hovered() || handle_resp.dragged() {
                    ui.visuals().widgets.active.bg_fill
                } else {
                    ui.visuals().widgets.noninteractive.bg_stroke.color
                };
                ui.painter().vline(
                    handle_rect.center().x,
                    handle_rect.y_range(),
                    egui::Stroke::new(1.0, color),
                );

                if handle_resp.hovered() || handle_resp.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }

                if handle_resp.dragged() {
                    let delta = handle_resp.drag_delta().x;
                    // dragged right => editor bigger, output smaller => fraction smaller
                    fraction -= delta / content_width;
                    fraction = fraction.clamp(0.2, 0.8);
                    state.ide.output_fraction = fraction;
                }

                // Output (right proportionally)
                ui.allocate_ui_with_layout(
                    egui::vec2(output_width, available.height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        egui::Frame::NONE.show(ui, |ui| {
                            ui.strong("Output");
                            ui.add_space(4.0);
                            egui::ScrollArea::vertical()
                                .id_salt("ide_output_scroll")
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
                    },
                );
            });
        });
}
