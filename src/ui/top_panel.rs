use crate::action::Action;
use crate::state::AppState;
use eframe::egui;

pub fn draw(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
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
            });

            ui.menu_button("Edit", |ui| {
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
            ui.add(egui::Label::new("|").selectable(false));
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
}

/// Helper: paste image from clipboard and push LoadImage actions directly
pub fn paste_clipboard_image(
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
