use crate::action::Action;
use crate::state::AppState;
use eframe::egui;

// Include the auto-generated script list from build.rs
include!(concat!(env!("OUT_DIR"), "/embedded_scripts.rs"));

/// Draw the presets ComboBox and import/export buttons.
pub fn draw_presets(state: &mut AppState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let combo = egui::ComboBox::from_id_salt("script_presets")
        .selected_text("Script Templates…")
        .width(160.0);

    combo.show_ui(ui, |ui| {
        // Built-in presets (auto-discovered from example_scripts/)
        for (name, code) in BUILTIN_SCRIPTS {
            if ui.selectable_label(false, *name).clicked() {
                actions.push(Action::LoadPresetScript(code.to_string()));
            }
        }

        // User-imported scripts
        if !state.ide.user_scripts.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("User Scripts")
                    .small()
                    .color(egui::Color32::GRAY),
            );
            let user_scripts = state.ide.user_scripts.clone();
            for (name, code) in &user_scripts {
                if ui.selectable_label(false, format!("📄 {}", name)).clicked() {
                    actions.push(Action::LoadPresetScript(code.clone()));
                }
            }
        }

        ui.separator();

        // Import
        if ui.selectable_label(false, "Import Script…").clicked() {
            import_script(actions);
        }
    });

    // Export button
    if ui
        .button("Export")
        .on_hover_text("Save current script to a .rhai file")
        .clicked()
    {
        export_script(&state.ide.code);
    }
}

/// Open a file dialog to import a .rhai script.
fn import_script(actions: &mut Vec<Action>) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Rhai Script", &["rhai", "txt"])
        .pick_file()
    {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Imported".to_string());
            actions.push(Action::LoadPresetScript(contents.clone()));
            actions.push(Action::AddUserScript(name, contents));
        }
    }
}

/// Open a file dialog to export the current script.
fn export_script(code: &str) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Rhai Script", &["rhai"])
        .set_file_name("my_script.rhai")
        .save_file()
    {
        let _ = std::fs::write(path, code);
    }
}
