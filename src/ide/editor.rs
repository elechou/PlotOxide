use crate::action::Action;
use crate::state::AppState;
use eframe::egui::{self, Color32};
use std::sync::LazyLock;

/// Custom SyntectSettings that replaces "base16-mocha.dark" with "base16-eighties.dark" colors.
pub(crate) static EIGHTIES_SETTINGS: LazyLock<egui_extras::syntax_highlighting::SyntectSettings> =
    LazyLock::new(|| {
        let ps = syntect::parsing::SyntaxSet::load_defaults_newlines();
        let mut ts = syntect::highlighting::ThemeSet::load_defaults();
        // Copy the Eighties theme data into the Mocha key so CodeTheme::dark() resolves to it
        if let Some(eighties) = ts.themes.get("base16-eighties.dark").cloned() {
            ts.themes.insert("base16-mocha.dark".to_string(), eighties);
        }
        egui_extras::syntax_highlighting::SyntectSettings { ps, ts }
    });

pub fn draw_editor(state: &mut AppState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    // Small left margin to prevent scrollbar / panel-resize-handle conflict
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: 6,
            right: 0,
            top: 0,
            bottom: 6,
        })
        .show(ui, |ui| {
            ui.strong("Code Editor");
            ui.add_space(4.0);

            let mut code = state.ide.code.clone();

            let theme = egui_extras::syntax_highlighting::CodeTheme::from_style(ui.style());
            let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
                let mut layout_job = egui_extras::syntax_highlighting::highlight_with(
                    ui.ctx(),
                    ui.style(),
                    &theme,
                    buf.as_str(),
                    "rs",
                    &EIGHTIES_SETTINGS,
                );
                layout_job.wrap.max_width = f32::INFINITY;
                ui.fonts_mut(|f| f.layout_job(layout_job))
            };

            let editor_bg = if theme.is_dark() {
                Color32::from_rgb(20, 20, 20)
            } else {
                Color32::from_rgb(240, 240, 240)
            };

            // Calculate rows to fill remaining height
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
            let available_rows = (ui.available_height() / row_height).floor() as usize;
            let desired_rows = available_rows.max(10) + 1;

            egui::ScrollArea::both()
                .id_salt("ide_editor_scroll")
                .show(ui, |ui| {
                    let editor = egui::TextEdit::multiline(&mut code)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_rows(desired_rows)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter)
                        .background_color(editor_bg);

                    ui.add(editor);
                });

            if code != state.ide.code {
                actions.push(Action::UpdateIDECode(code));
            }
        });
}
