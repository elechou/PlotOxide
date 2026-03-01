use crate::state::AppState;
use eframe::egui;

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
        .default_width(600.0)
        .default_height(500.0)
        .vscroll(true)
        .resizable(true)
        .show(ctx, |ui| {
            render_help_markdown(ui, HELP_MD);
        });
    state.ide.show_help = open;
}

/// Render a simplified subset of markdown as egui widgets.
/// Supports: headings (#, ##, ###), code blocks (```), tables, bold (**), `inline code`,
/// and plain paragraphs.
fn render_help_markdown(ui: &mut egui::Ui, md: &str) {
    let mut in_code_block = false;
    let mut code_buf = String::new();

    for line in md.lines() {
        // Code block toggle
        if line.trim_start().starts_with("```") {
            if in_code_block {
                // End code block — render accumulated code
                let frame = egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(30, 30, 30))
                    .inner_margin(8.0)
                    .corner_radius(4.0);
                frame.show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(&code_buf)
                            .font(egui::FontId::monospace(13.0))
                            .color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)),
                    );
                });
                ui.add_space(4.0);
                code_buf.clear();
                in_code_block = false;
            } else {
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

        // Headings
        if trimmed.starts_with("### ") {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(&trimmed[4..])
                    .strong()
                    .size(15.0)
                    .color(egui::Color32::from_rgb(0xE0, 0xE0, 0xE0)),
            );
            ui.add_space(2.0);
            continue;
        }
        if trimmed.starts_with("## ") {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(&trimmed[3..])
                    .strong()
                    .size(17.0)
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(3.0);
            continue;
        }
        if trimmed.starts_with("# ") {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(&trimmed[2..])
                    .strong()
                    .size(20.0)
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(4.0);
            continue;
        }

        // Table rows (simplified: render as monospace)
        if trimmed.starts_with('|') {
            // Skip separator rows like |---|---|
            if trimmed.contains("---") {
                continue;
            }
            ui.label(
                egui::RichText::new(trimmed)
                    .font(egui::FontId::monospace(12.0))
                    .color(egui::Color32::from_rgb(0xCC, 0xCC, 0xCC)),
            );
            continue;
        }

        // Blockquote / note
        if trimmed.starts_with("> ") {
            let content = &trimmed[2..];
            let frame = egui::Frame::NONE
                .fill(egui::Color32::from_rgb(40, 50, 60))
                .inner_margin(6.0)
                .corner_radius(3.0);
            frame.show(ui, |ui| {
                ui.label(
                    egui::RichText::new(content)
                        .italics()
                        .color(egui::Color32::from_rgb(0xAA, 0xCC, 0xEE)),
                );
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
}

/// Render a single line with basic inline formatting:
/// `code`, **bold**, *italic*
fn render_inline(ui: &mut egui::Ui, text: &str) {
    // For simplicity, render as a single label with inline code highlighted
    // A more sophisticated approach would parse and layout each segment
    let mut job = egui::text::LayoutJob::default();
    let default_color = egui::Color32::from_rgb(0xCC, 0xCC, 0xCC);
    let code_color = egui::Color32::from_rgb(0xE0, 0x6C, 0x75);
    let code_bg = egui::Color32::from_rgb(45, 45, 45);
    let bold_color = egui::Color32::from_rgb(0xEE, 0xEE, 0xEE);

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
                        color: default_color,
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
                        color: default_color,
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
                    color: bold_color,
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
                color: default_color,
                ..Default::default()
            },
        );
    }

    if !job.is_empty() {
        ui.label(job);
    }
}
