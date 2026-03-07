pub mod canvas;
pub mod panel;
pub mod toolbar;

use crate::action::Action;
use crate::state::AppState;
use eframe::egui;

pub fn draw_ui(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    // ── Global shortcut detection ──────────────────────────────────────
    // MUST run BEFORE any widget rendering.
    let mut dropped_image_path = None;
    let mut dropped_prdx_path = None;
    ctx.input_mut(|i| {
        // Drag & drop
        if let Some(file) = i.raw.dropped_files.first() {
            if let Some(path) = &file.path {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    dropped_image_path = Some(path.clone());
                } else if ext == "prdx" {
                    dropped_prdx_path = Some(path.clone());
                }
            }
        }

        // Save project: Ctrl+S / Cmd+S
        let save_cmd = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);
        let save_ctrl = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::S);
        if i.consume_shortcut(&save_cmd) || i.consume_shortcut(&save_ctrl) {
            actions.push(Action::SaveProject);
        }

        // Undo / Redo
        let undo_cmd = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z);
        let undo_ctrl = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z);
        let redo_cmd = egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            egui::Key::Z,
        );
        let redo_ctrl = egui::KeyboardShortcut::new(
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            egui::Key::Z,
        );

        if i.consume_shortcut(&redo_cmd) || i.consume_shortcut(&redo_ctrl) {
            actions.push(Action::Redo);
        } else if i.consume_shortcut(&undo_cmd) || i.consume_shortcut(&undo_ctrl) {
            actions.push(Action::Undo);
        }
    });

    // Process drag-drop results.
    if let Some(path) = dropped_image_path {
        if path.extension().is_some_and(|ext| ext == "prdx") {
            if let Some((data, img_bytes, path)) = crate::project::open_project_from_path(&path) {
                if state.dirty {
                    state.pending_action = Some(crate::state::PendingAction::OpenProject(
                        data, img_bytes, path,
                    ));
                } else {
                    let proj_path = path.clone();
                    crate::project::apply_project(state, data, &img_bytes, path, ctx);
                    state.project_path = Some(proj_path);
                    state.dirty = false;
                }
            }
        } else {
            crate::ui::panel::process_image_file(state, path, ctx, actions);
        }
    }
    if let Some(path) = dropped_prdx_path {
        if let Some((data, img_bytes, file_path)) = crate::project::open_project_from_path(&path) {
            if state.dirty {
                state.pending_action = Some(crate::state::PendingAction::OpenProject(
                    data, img_bytes, file_path,
                ));
            } else {
                let proj_path = file_path.clone();
                crate::project::apply_project(state, data, &img_bytes, file_path, ctx);
                state.project_path = Some(proj_path);
                state.dirty = false;
            }
        }
    }

    // ── UI rendering ──────────────────────────────────────────────────
    // Top Panel: Menu Bar + Quick-Access Toolbar
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            // ── Menu Bar ──────────────────────────────────────────
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    actions.push(Action::NewProject);
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new("Open Project…").shortcut_text(""))
                    .clicked()
                {
                    actions.push(Action::OpenProject);
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new("Save Project").shortcut_text("Ctrl+S"))
                    .clicked()
                {
                    actions.push(Action::SaveProject);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Save Project As…").shortcut_text(""))
                    .clicked()
                {
                    actions.push(Action::SaveProjectAs);
                    ui.close();
                }
                ui.separator();
                if ui.button("Load Image…").clicked() {
                    crate::ui::panel::load_image(state, ctx, actions);
                    ui.close();
                }
                if ui.button("Paste Image").clicked() {
                    paste_clipboard_image(state, ctx, actions);
                    ui.close();
                }
                ui.separator();
                if ui.button("Export CSV…").clicked() {
                    actions.push(Action::RequestExportCsv);
                    ui.close();
                }
            });

            ui.menu_button("About", |ui| {
                if ui.button("About PlotRedox").clicked() {
                    state.show_about = true;
                    ui.close();
                }
            });

            ui.add_space(8.0);
            ui.label("||");
            ui.add_space(8.0);

            // ── Quick-Access Toolbar Buttons ──────────────────────
            let is_dark = ctx.style().visuals.dark_mode;
            let icon = if is_dark { "\u{1F506}" } else { "\u{1F319}" };
            if ui.button(icon).on_hover_text("Toggle Theme").clicked() {
                if is_dark {
                    ctx.set_visuals(egui::Visuals::light());
                } else {
                    ctx.set_visuals(egui::Visuals::dark());
                }
            }

            if ui
                .button("\u{1F4BE}")
                .on_hover_text("Save Project")
                .clicked()
            {
                actions.push(Action::SaveProject);
            }

            if ui
                .button("Load Image")
                .on_hover_text("Load image from file")
                .clicked()
            {
                crate::ui::panel::load_image(state, ctx, actions);
            }
            if ui
                .button("Paste Image")
                .on_hover_text("Paste image from clipboard")
                .clicked()
            {
                paste_clipboard_image(state, ctx, actions);
            }

            // Right-aligned IDE toggle
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .selectable_label(state.ide.is_open, "\u{1F5B3} Script IDE")
                    .clicked()
                {
                    actions.push(Action::ToggleIDE);
                }
            });
        });
    });

    // Left Sidebar for Control Panels (full height — drawn before IDE bottom panel)
    panel::draw_panel(state, ctx, actions);

    // IDE Bottom Panel (drawn after left panel, before CentralPanel,
    // so CentralPanel correctly fills remaining space)
    crate::ide::draw_ide(state, ctx, actions);

    // Central Image Viewport Canvas & Toolbar (CentralPanel — must be last)
    canvas::draw_canvas(state, ctx, actions);

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
                            actions.push(Action::SetMode(crate::state::AppMode::AddCalib));
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
            actions.push(Action::SetMode(crate::state::AppMode::AddCalib));
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
                ui.heading("\u{2139} No Image Found");
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
                actions.push(Action::SetMode(crate::state::AppMode::AddCalib));
            }
            PendingAction::LoadClipboardImage(tex, size, bytes, w, h) => {
                actions.push(Action::LoadClipboardImage(tex, size, bytes, w, h));
                actions.push(Action::RequestCenter);
                actions.push(Action::SetMode(crate::state::AppMode::AddCalib));
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

/// Helper: paste image from clipboard and push LoadImage actions directly
fn paste_clipboard_image(
    state: &mut crate::state::AppState,
    ctx: &egui::Context,
    actions: &mut Vec<Action>,
) {
    use clipboard_rs::{common::RustImage, Clipboard, ClipboardContext};
    let mut found = false;
    if let Ok(ctx_cb) = ClipboardContext::new() {
        if ctx_cb.has(clipboard_rs::ContentFormat::Image) {
            if let Ok(image) = ctx_cb.get_image() {
                let (w, h) = image.get_size();
                let size = [w as usize, h as usize];
                let rgba = image
                    .to_rgba8()
                    .expect("Failed to convert clipboard image to RGBA");
                let bytes = rgba.into_raw();

                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &bytes);
                let handle = ctx.load_texture("clipboard_image", color_image, Default::default());
                let img_size = eframe::egui::Vec2::new(size[0] as f32, size[1] as f32);

                if state.dirty {
                    state.pending_action = Some(crate::state::PendingAction::LoadClipboardImage(
                        handle, img_size, bytes, w as u32, h as u32,
                    ));
                } else {
                    actions.push(Action::LoadClipboardImage(
                        handle, img_size, bytes, w as u32, h as u32,
                    ));
                    actions.push(Action::RequestCenter);
                    actions.push(Action::SetMode(crate::state::AppMode::AddCalib));
                }
                found = true;
            }
        }
    }
    if !found {
        state.show_clipboard_empty = true;
    }
}
