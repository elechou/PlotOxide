use crate::ide::editor::EIGHTIES_SETTINGS;
use crate::state::AppState;
use eframe::egui;
use egui_extras::{Column, TableBuilder};

/// The embedded scripting help markdown (compiled into the binary).
const HELP_MD: &str = include_str!("../../docs/scripting_help.md");

/// Draw the floating help window if `state.ide.show_help` is true.
pub fn draw_help_window(state: &mut AppState, ctx: &egui::Context) {
    if !state.ide.show_help {
        return;
    }

    let mut open = state.ide.show_help;
    egui::Window::new("Scripting Reference")
        .open(&mut open)
        .default_width(850.0)
        .default_height(500.0)
        .vscroll(true)
        .resizable(true)
        .show(ctx, |ui| {
            render_help_markdown(ui, HELP_MD);
        });
    state.ide.show_help = open;
}

/// Render a simplified subset of markdown as egui widgets.
/// Supports: headings (#, ##, ###), code blocks (``` with syntax highlighting),
/// tables (rendered as egui_extras::TableBuilder), bold (**), `inline code`,
/// and plain paragraphs.
fn render_help_markdown(ui: &mut egui::Ui, md: &str) {
    let mut in_code_block = false;
    let mut code_buf = String::new();
    let mut code_lang = String::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_counter: usize = 0;

    for line in md.lines() {
        // Code block toggle
        if line.trim_start().starts_with("```") {
            // Flush any pending table
            if !table_rows.is_empty() {
                render_table(ui, &table_rows, table_counter);
                table_counter += 1;
                table_rows.clear();
            }

            if in_code_block {
                // End code block — render with syntax highlighting
                render_code_block(ui, &code_buf, &code_lang);
                code_buf.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                // Start code block — capture optional language hint
                let hint = line.trim_start().trim_start_matches('`').trim();
                code_lang = hint.to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            if !code_buf.is_empty() {
                code_buf.push('\n');
            }
            code_buf.push_str(line);
            continue;
        }

        let trimmed = line.trim();

        // Table rows: accumulate and defer rendering
        if trimmed.starts_with('|') {
            // Skip separator rows like |---|---|
            if trimmed.contains("---") {
                continue;
            }
            let cells: Vec<String> = trimmed
                .split('|')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
                .collect();
            if !cells.is_empty() {
                table_rows.push(cells);
            }
            continue;
        }

        // If we were accumulating table rows and hit a non-table line, flush them
        if !table_rows.is_empty() {
            render_table(ui, &table_rows, table_counter);
            table_counter += 1;
            table_rows.clear();
        }

        // Empty line → spacing
        if trimmed.is_empty() {
            ui.add_space(4.0);
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            ui.separator();
            continue;
        }

        // Headings (check longest prefix first to avoid ambiguity)
        if trimmed.starts_with("#### ") {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&trimmed[5..]).strong().size(14.0));
            ui.add_space(2.0);
            continue;
        }
        if trimmed.starts_with("### ") {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(&trimmed[4..]).strong().size(15.0));
            ui.add_space(2.0);
            continue;
        }
        if trimmed.starts_with("## ") {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(&trimmed[3..]).strong().size(17.0));
            ui.add_space(3.0);
            continue;
        }
        if trimmed.starts_with("# ") {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(&trimmed[2..]).strong().size(20.0));
            ui.add_space(4.0);
            continue;
        }

        // Blockquote / note
        if trimmed.starts_with("> ") {
            let content = &trimmed[2..];
            let bg_color = ui.visuals().faint_bg_color;
            let frame = egui::Frame::NONE
                .fill(bg_color)
                .inner_margin(6.0)
                .corner_radius(3.0);
            frame.show(ui, |ui| {
                render_inline(ui, content);
            });
            ui.add_space(2.0);
            continue;
        }

        // Bullet points
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            ui.horizontal(|ui| {
                ui.label("  •");
                render_inline(ui, content);
            });
            continue;
        }

        // Regular paragraph — render with inline formatting
        render_inline(ui, trimmed);
    }

    // Flush any trailing table
    if !table_rows.is_empty() {
        render_table(ui, &table_rows, table_counter);
    }
}

// ---------------------------------------------------------------------------
// Code block rendering with syntax highlighting
// ---------------------------------------------------------------------------

/// Map the markdown language hint to a syntect-compatible extension.
fn syntect_lang(hint: &str) -> &str {
    match hint {
        "rhai" | "rust" | "rs" => "rs",
        "js" | "javascript" => "js",
        "py" | "python" => "py",
        "toml" => "toml",
        "json" => "json",
        _ => "rs", // default to Rust-like highlighting (good for Rhai)
    }
}

/// Render a fenced code block with syntax highlighting inside a dark frame.
fn render_code_block(ui: &mut egui::Ui, code: &str, lang_hint: &str) {
    let theme = egui_extras::syntax_highlighting::CodeTheme::from_style(ui.style());
    let code_bg = if theme.is_dark() {
        egui::Color32::from_rgb(20, 20, 20)
    } else {
        egui::Color32::from_rgb(240, 240, 240)
    };

    let frame = egui::Frame::NONE
        .fill(code_bg)
        .inner_margin(8.0)
        .corner_radius(4.0);

    frame.show(ui, |ui| {
        let ext = syntect_lang(lang_hint);
        let mut layout_job = egui_extras::syntax_highlighting::highlight_with(
            ui.ctx(),
            ui.style(),
            &theme,
            code,
            ext,
            &EIGHTIES_SETTINGS,
        );
        // Prevent wrapping so code lines stay intact
        layout_job.wrap.max_width = f32::INFINITY;
        ui.label(layout_job);
    });
    ui.add_space(4.0);
}

// ---------------------------------------------------------------------------
// Table rendering
// ---------------------------------------------------------------------------

/// Render accumulated table rows as a proper egui_extras table.
/// The first row is treated as the header.
fn render_table(ui: &mut egui::Ui, rows: &[Vec<String>], table_index: usize) {
    if rows.is_empty() {
        return;
    }

    let header = &rows[0];
    let body_rows = if rows.len() > 1 { &rows[1..] } else { &[] };
    let num_cols = header.len();
    if num_cols == 0 {
        return;
    }

    ui.add_space(4.0);

    // Wrap in push_id so each table gets a completely unique egui ID scope.
    ui.push_id(("help_table", table_index), |ui| {
        let available_width = ui.available_width();
        let col_width = (available_width / num_cols as f32).max(80.0);

        let mut builder = TableBuilder::new(ui)
            .striped(true)
            .vscroll(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

        for i in 0..num_cols {
            if i == num_cols - 1 {
                builder = builder.column(Column::remainder().at_least(60.0));
            } else {
                builder = builder.column(Column::initial(col_width).at_least(60.0));
            }
        }

        builder
            .header(20.0, |mut hdr| {
                for cell_text in header {
                    hdr.col(|ui| {
                        ui.strong(cell_text);
                    });
                }
            })
            .body(|body| {
                body.rows(20.0, body_rows.len(), |mut row| {
                    let idx = row.index();
                    let row_data = &body_rows[idx];
                    for (col_idx, cell_text) in row_data.iter().enumerate() {
                        if col_idx < num_cols {
                            row.col(|ui| {
                                render_inline(ui, cell_text);
                            });
                        }
                    }
                    // Fill remaining columns if this row has fewer cells
                    for _ in row_data.len()..num_cols {
                        row.col(|_ui| {});
                    }
                });
            });
    }); // ui.push_id
    ui.add_space(4.0);
}

// ---------------------------------------------------------------------------
// Inline formatting
// ---------------------------------------------------------------------------

/// Render a single line with basic inline formatting:
/// `code`, **bold**, *italic*
fn render_inline(ui: &mut egui::Ui, text: &str) {
    // For simplicity, render as a single label with inline code highlighted
    let mut job = egui::text::LayoutJob::default();
    let code_color = egui::Color32::from_rgb(0xE0, 0x6C, 0x75);
    let code_bg = ui.visuals().extreme_bg_color;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut segment = String::new();

    while i < chars.len() {
        // Inline code
        if chars[i] == '`' {
            // Flush pending text
            if !segment.is_empty() {
                job.append(
                    &segment,
                    0.0,
                    egui::TextFormat {
                        ..Default::default()
                    },
                );
                segment.clear();
            }
            i += 1;
            let mut code_text = String::new();
            while i < chars.len() && chars[i] != '`' {
                code_text.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // skip closing `
            }
            job.append(
                &code_text,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::monospace(13.0),
                    color: code_color,
                    background: code_bg,
                    ..Default::default()
                },
            );
            continue;
        }

        // Bold **text**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if !segment.is_empty() {
                job.append(
                    &segment,
                    0.0,
                    egui::TextFormat {
                        ..Default::default()
                    },
                );
                segment.clear();
            }
            i += 2;
            let mut bold_text = String::new();
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                bold_text.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2; // skip closing **
            }
            job.append(
                &bold_text,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(14.0),
                    ..Default::default()
                },
            );
            continue;
        }

        segment.push(chars[i]);
        i += 1;
    }

    // Flush remaining text
    if !segment.is_empty() {
        job.append(
            &segment,
            0.0,
            egui::TextFormat {
                ..Default::default()
            },
        );
    }

    if !job.is_empty() {
        ui.label(job);
    }
}
