pub mod canvas;
pub mod panel;
pub mod toolbar;

use crate::action::Action;
use crate::state::AppState;
use eframe::egui;

pub fn draw_ui(state: &AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    // Top Panel: Unified Toolbar
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("PlotDigitizer");
            ui.add_space(20.0);

            let is_dark = ctx.style().visuals.dark_mode;
            let icon = if is_dark { "☀" } else { "🌙" };
            if ui.button(icon).clicked() {
                if is_dark {
                    ctx.set_visuals(egui::Visuals::light());
                } else {
                    ctx.set_visuals(egui::Visuals::dark());
                }
            }
            ui.add_space(10.0);

            if ui.button("Load Image").clicked() {
                crate::ui::panel::load_image(ctx, actions);
            }
            if ui.button("Paste Image").clicked() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(image_data) = clipboard.get_image() {
                        let size = [image_data.width as usize, image_data.height as usize];
                        let color_image =
                            egui::ColorImage::from_rgba_unmultiplied(size, &image_data.bytes);
                        let handle =
                            ctx.load_texture("clipboard_image", color_image, Default::default());
                        actions.push(Action::SetPendingImage(
                            std::path::PathBuf::from("Clipboard"),
                            handle,
                            eframe::egui::Vec2::new(size[0] as f32, size[1] as f32),
                        ));
                    }
                }
            }
        });
        ui.add_space(8.0);
    });

    // Left Sidebar for Control Panels
    panel::draw_panel(state, ctx, actions);

    // Central Image Viewport Canvas & Toolbar
    canvas::draw_canvas(state, ctx, actions);

    // Parse drag&drop or paste instructions
    let mut dropped_path = None;
    let mut paste_requested = false;

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
    });

    if let Some(path) = dropped_path {
        crate::ui::panel::process_image_file(path, ctx, actions);
    } else if paste_requested {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(image_data) = clipboard.get_image() {
                let size = [image_data.width as usize, image_data.height as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &image_data.bytes);
                let handle = ctx.load_texture("clipboard_image", color_image, Default::default());
                actions.push(Action::SetPendingImage(
                    std::path::PathBuf::from("Clipboard"),
                    handle,
                    eframe::egui::Vec2::new(size[0] as f32, size[1] as f32),
                ));
            }
        }
    }

    // Modal dialog for overwriting existing workspace
    if let Some((path, tex, size)) = &state.pending_image {
        if state.texture.is_some() {
            let mut is_open = true;
            egui::Window::new("⚠ Warning")
                .collapsible(false)
                .resizable(false)
                .open(&mut is_open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("Loading a new image will clear your current workspace.");
                        ui.label("Are you sure you want to proceed?");
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            let avail = ui.available_width();
                            let btn_w = 80.0;
                            let spacing = 20.0;
                            ui.add_space((avail - (btn_w * 2.0 + spacing)) / 2.0);

                            let confirm_btn = egui::Button::new(
                                egui::RichText::new("Confirm").color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(200, 50, 50));

                            if ui.add_sized([btn_w, 24.0], confirm_btn).clicked() {
                                actions.push(Action::LoadImage(path.clone(), tex.clone(), *size));
                                actions.push(Action::RequestCenter);
                                actions.push(Action::SetMode(crate::state::AppMode::AddCalib));
                            }

                            ui.add_space(spacing);

                            if ui
                                .add_sized([btn_w, 24.0], egui::Button::new("Cancel"))
                                .clicked()
                            {
                                actions.push(Action::CancelPendingImage);
                            }
                        });
                    });
                });
            if !is_open {
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
        let mut is_open = true;
        egui::Window::new("⚠ Clear Data")
            .collapsible(false)
            .resizable(false)
            .open(&mut is_open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label("Are you sure you want to clear all extracted data points?");
                    ui.label("This action cannot be undone.");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let avail = ui.available_width();
                        let btn_w = 80.0;
                        let spacing = 20.0;
                        ui.add_space((avail - (btn_w * 2.0 + spacing)) / 2.0);

                        let confirm_btn = egui::Button::new(
                            egui::RichText::new("Confirm").color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(200, 50, 50));

                        if ui.add_sized([btn_w, 24.0], confirm_btn).clicked() {
                            actions.push(Action::ClearData);
                        }

                        ui.add_space(spacing);

                        if ui
                            .add_sized([btn_w, 24.0], egui::Button::new("Cancel"))
                            .clicked()
                        {
                            actions.push(Action::CancelClearData);
                        }
                    });
                });
            });
        if !is_open {
            actions.push(Action::CancelClearData);
        }
    }
}
