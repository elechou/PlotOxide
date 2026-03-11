pub mod canvas;
pub mod modals;
pub mod panel;
pub mod toolbar;
pub mod top_panel;

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

        // Undo / Redo — route to mask undo when mask is painting
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
    top_panel::draw(state, ctx, actions);

    // Left Sidebar for Control Panels (full height — drawn before IDE bottom panel)
    panel::draw_panel(state, ctx, actions);

    // IDE Bottom Panel (drawn after left panel, before CentralPanel,
    // so CentralPanel correctly fills remaining space)
    crate::ide::draw_ide(state, ctx, actions);

    // Central Image Viewport Canvas & Toolbar (CentralPanel — must be last)
    canvas::draw_canvas(state, ctx, actions);

    // Dialogs / Modals
    modals::draw(state, ctx, actions);
}
