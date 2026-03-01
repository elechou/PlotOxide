pub mod canvas;
pub mod panel;
pub mod toolbar;

use crate::action::Action;
use crate::state::AppState;
use eframe::egui;

pub fn draw_ui(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    // Top Panel: Unified Toolbar
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("PlotOxide");
            ui.add_space(20.0);

            let is_dark = ctx.style().visuals.dark_mode;
            let icon = if is_dark { "\u{2600}" } else { "\u{1F319}" };
            if ui.button(icon).clicked() {
                if is_dark {
                    ctx.set_visuals(egui::Visuals::light());
                } else {
                    ctx.set_visuals(egui::Visuals::dark());
                }
            }
            ui.add_space(10.0);

            if ui.button("Load Image").clicked() {
                if state.texture.is_some() {
                    state.pending_load_kind = Some("file".to_string());
                } else {
                    crate::ui::panel::load_image(ctx, actions);
                }
            }
            if ui.button("Paste Image").clicked() {
                if state.texture.is_some() {
                    state.pending_load_kind = Some("clipboard".to_string());
                } else {
                    paste_clipboard_image(state, ctx, actions);
                }
            }
            ui.add_space(10.0);

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
        ui.add_space(8.0);
    });

    // Left Sidebar for Control Panels (full height — drawn before IDE bottom panel)
    panel::draw_panel(state, ctx, actions);

    // IDE Bottom Panel (drawn after left panel, before CentralPanel,
    // so CentralPanel correctly fills remaining space)
    crate::ide::draw_ide(state, ctx, actions);

    // Central Image Viewport Canvas & Toolbar (CentralPanel — must be last)
    canvas::draw_canvas(state, ctx, actions);

    // Parse drag&drop or paste instructions
    let mut dropped_path = None;
    let mut paste_requested = false;

    // When a text widget (TextEdit, DragValue, etc.) has keyboard focus,
    // skip canvas/global shortcuts so they don't conflict with text editing.
    let text_focused = ctx.wants_keyboard_input();

    ctx.input_mut(|i| {
        if let Some(file) = i.raw.dropped_files.first() {
            if let Some(path) = &file.path {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    dropped_path = Some(path.clone());
                }
            }
        }

        if !text_focused {
            let shortcut_cmd = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::V);
            let shortcut_ctrl = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::V);
            let has_paste_event = i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)));
            if has_paste_event
                || i.consume_shortcut(&shortcut_cmd)
                || i.consume_shortcut(&shortcut_ctrl)
            {
                paste_requested = true;
            }

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
        }
    });

    if let Some(path) = dropped_path {
        crate::ui::panel::process_image_file(path, ctx, actions);
    } else if paste_requested {
        use clipboard_rs::{common::RustImage, Clipboard, ClipboardContext};
        let mut found_image = false;
        if let Ok(ctx_cb) = ClipboardContext::new() {
            if ctx_cb.has(clipboard_rs::ContentFormat::Image) {
                if let Ok(image) = ctx_cb.get_image() {
                    let (w, h) = image.get_size();
                    let size = [w as usize, h as usize];
                    let rgba = image
                        .to_rgba8()
                        .expect("Failed to convert clipboard image to RGBA");
                    let bytes = rgba.into_raw(); // Vec<u8> in RGBA format
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &bytes);
                    let handle =
                        ctx.load_texture("clipboard_image", color_image, Default::default());
                    actions.push(Action::SetPendingImage(
                        std::path::PathBuf::from("Clipboard"),
                        handle,
                        eframe::egui::Vec2::new(size[0] as f32, size[1] as f32),
                    ));
                    found_image = true;
                }
            }
        }
        if !found_image {
            state.show_clipboard_empty = true;
        }
    }
    // Pre-load confirmation modal (shown BEFORE file picker / clipboard paste)
    let mut do_load_file = false;
    let mut do_paste_clip = false;
    if state.pending_load_kind.is_some() {
        let kind = state.pending_load_kind.clone().unwrap();
        let modal = egui::Modal::new(egui::Id::new("modal_preload")).show(ctx, |ui| {
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
                        if kind == "file" {
                            do_load_file = true;
                        } else {
                            do_paste_clip = true;
                        }
                        ui.close();
                    }
                },
            );
        });
        if modal.should_close() {
            state.pending_load_kind = None;
        }
    }
    // Execute after modal closure to avoid borrow conflicts
    if do_load_file {
        state.pending_load_kind = None;
        crate::ui::panel::load_image(ctx, actions);
    }
    if do_paste_clip {
        state.pending_load_kind = None;
        paste_clipboard_image(state, ctx, actions);
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
                actions.push(Action::LoadImage(
                    std::path::PathBuf::from("Clipboard"),
                    handle,
                    img_size,
                ));
                actions.push(Action::RequestCenter);
                actions.push(Action::SetMode(crate::state::AppMode::AddCalib));
                found = true;
            }
        }
    }
    if !found {
        state.show_clipboard_empty = true;
    }
}
