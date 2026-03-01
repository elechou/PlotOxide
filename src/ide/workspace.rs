use crate::action::Action;
use crate::state::AppState;
use eframe::egui::{self, Color32, RichText};

pub fn draw_workspace(state: &AppState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    egui::SidePanel::left("ide_workspace")
        .resizable(false)
        .exact_width(180.0)
        .show_inside(ui, |ui| {
            ui.strong("Workspace");
            ui.add_space(4.0);

            if state.ide.workspace_vars.is_empty() {
                ui.label(
                    RichText::new("Run a script to populate variables")
                        .italics()
                        .color(Color32::GRAY),
                );
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for var in &state.ide.workspace_vars {
                    let response = ui
                        .horizontal(|ui| {
                            // Variable name — bold blue
                            ui.label(
                                RichText::new(&var.name)
                                    .strong()
                                    .color(Color32::from_rgb(0x61, 0xAF, 0xEF)),
                            );
                            ui.add_space(4.0);
                            // Type + dims — dimmed
                            ui.label(
                                RichText::new(format!("{} {}", var.type_name, var.dims))
                                    .small()
                                    .color(Color32::GRAY),
                            );
                        })
                        .response;

                    let interact_response =
                        ui.interact(response.rect, ui.id().with(&var.name), egui::Sense::click());

                    if interact_response.clicked() {
                        actions.push(Action::OpenInspector(var.name.clone()));
                    }
                    interact_response
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Click to inspect");
                }
            });
        });
}
