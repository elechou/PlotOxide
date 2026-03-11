use crate::action::Action;
use crate::state::AppState;
use eframe::egui;

pub fn draw(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    // Unsaved-changes modal — shown when a destructive action is pending while dirty
    let mut do_save_then_proceed = false;
    let mut do_proceed_no_save = false;
    if state.pending_action.is_some() {
        let modal = egui::Modal::new(egui::Id::new("modal_unsaved")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.vertical_centered(|ui| {
                ui.heading("⚠ Unsaved Changes");
            });
            ui.add_space(8.0);
            ui.label("Your project has unsaved changes.");
            ui.label("Would you like to save before proceeding?");
            ui.add_space(10.0);
            ui.separator();

            egui::Sides::new().show(
                ui,
                |_ui| {},
                |ui| {
                    if ui.button("Cancel").clicked() {
                        ui.close();
                    }
                    let dont_save_btn = egui::Button::new(
                        egui::RichText::new("Don't Save").color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(200, 50, 50));
                    if ui.add(dont_save_btn).clicked() {
                        do_proceed_no_save = true;
                        ui.close();
                    }
                    let save_btn =
                        egui::Button::new(egui::RichText::new("Save").color(egui::Color32::WHITE))
                            .fill(egui::Color32::from_rgb(0, 150, 0));
                    if ui.add(save_btn).clicked() {
                        do_save_then_proceed = true;
                        ui.close();
                    }
                },
            );
        });
        if modal.should_close() && !do_save_then_proceed && !do_proceed_no_save {
            state.pending_action = None;
        }
    }
    // Execute deferred actions after modal closure to avoid borrow conflicts
    if do_save_then_proceed {
        actions.push(Action::SaveProject);
        execute_pending_action(state, ctx, actions);
    }
    if do_proceed_no_save {
        execute_pending_action(state, ctx, actions);
    }

    // Modal dialog for overwriting existing workspace (drag-drop / keyboard paste flow)
    if let Some((path, tex, size)) = &state.pending_image {
        if state.texture.is_some() {
            let modal = egui::Modal::new(egui::Id::new("modal_load_image")).show(ctx, |ui| {
                ui.set_width(350.0);
                ui.vertical_centered(|ui| {
                    ui.heading("⚠ Warning");
                });
                ui.add_space(8.0);
                ui.label("Loading a new image will clear your current workspace.");
                ui.label("Are you sure you want to proceed?");
                ui.add_space(10.0);
                ui.separator();

                egui::Sides::new().show(
                    ui,
                    |_ui| {},
                    |ui| {
                        if ui.button("Cancel").clicked() {
                            ui.close();
                        }
                        let confirm_btn = egui::Button::new(
                            egui::RichText::new("Confirm").color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(200, 50, 50));
                        if ui.add(confirm_btn).clicked() {
                            actions.push(Action::LoadImage(path.clone(), tex.clone(), *size));
                            actions.push(Action::RequestCenter);
                        }
                    },
                );
            });
            if modal.should_close() {
                actions.push(Action::CancelPendingImage);
            }
        } else {
            // Workspace is empty, load directly without warning
            actions.push(Action::LoadImage(path.clone(), tex.clone(), *size));
            actions.push(Action::RequestCenter);
        }
    }

    // Modal dialog for clearing all data
    if state.pending_clear_data {
        let modal = egui::Modal::new(egui::Id::new("modal_clear_data")).show(ctx, |ui| {
            ui.set_width(350.0);
            ui.vertical_centered(|ui| {
                ui.heading("⚠ Clear Data");
            });
            ui.add_space(8.0);
            ui.label("Are you sure you want to clear all extracted data points?");
            ui.label("This action cannot be undone.");
            ui.add_space(10.0);
            ui.separator();

            egui::Sides::new().show(
                ui,
                |_ui| {},
                |ui| {
                    if ui.button("Cancel").clicked() {
                        ui.close();
                    }
                    let confirm_btn = egui::Button::new(
                        egui::RichText::new("Confirm").color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(200, 50, 50));
                    if ui.add(confirm_btn).clicked() {
                        actions.push(Action::ClearData);
                    }
                },
            );
        });
        if modal.should_close() {
            actions.push(Action::CancelClearData);
        }
    }

    // Modal dialog for clipboard with no image
    if state.show_clipboard_empty {
        let modal = egui::Modal::new(egui::Id::new("modal_clipboard_empty")).show(ctx, |ui| {
            ui.set_width(320.0);
            ui.vertical_centered(|ui| {
                ui.heading("ℹ No Image Found");
            });
            ui.add_space(8.0);
            ui.label("No image was found in the clipboard.");
            ui.label("Please copy an image first, then try again.");
            ui.add_space(10.0);
            ui.separator();
            ui.vertical_centered(|ui| {
                if ui.button("OK").clicked() {
                    ui.close();
                }
            });
        });
        if modal.should_close() {
            state.show_clipboard_empty = false;
        }
    }

    // About dialog
    if state.show_about {
        let modal = egui::Modal::new(egui::Id::new("modal_about")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading("PlotRedox");
                ui.add_space(4.0);
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(8.0);
            });
            ui.label(env!("CARGO_PKG_DESCRIPTION"));
            ui.add_space(8.0);
            ui.label("Authors: Qiu Shou");
            ui.label("License: MIT");
            ui.add_space(4.0);
            ui.hyperlink_to("GitHub Repository", "https://github.com/elechou/PlotOxide");
            ui.add_space(10.0);
            ui.separator();
            ui.vertical_centered(|ui| {
                if ui.button("OK").clicked() {
                    ui.close();
                }
            });
        });
        if modal.should_close() {
            state.show_about = false;
        }
    }
}

/// Execute the deferred pending action after unsaved-changes modal resolution.
fn execute_pending_action(
    state: &mut crate::state::AppState,
    ctx: &egui::Context,
    actions: &mut Vec<Action>,
) {
    use crate::state::PendingAction;
    if let Some(pending) = state.pending_action.take() {
        // Mark not dirty so the action can proceed cleanly
        state.dirty = false;
        match pending {
            PendingAction::NewProject => {
                actions.push(Action::NewProject);
            }
            PendingAction::LoadImage(path, tex, size) => {
                actions.push(Action::LoadImage(path, tex, size));
                actions.push(Action::RequestCenter);
            }
            PendingAction::LoadClipboardImage(tex, size, bytes, w, h) => {
                actions.push(Action::LoadClipboardImage(tex, size, bytes, w, h));
                actions.push(Action::RequestCenter);
            }
            PendingAction::OpenProject(data, img_bytes, path) => {
                crate::project::apply_project(state, data, &img_bytes, path.clone(), ctx);
                state.project_path = Some(path);
                state.dirty = false;
            }
            PendingAction::CloseApp => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
