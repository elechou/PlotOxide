use eframe::egui;
use egui::Color32;

use crate::action::Action;
use crate::i18n::{t, Lang};
use crate::icons;
use crate::state::{AppState, AxisHighlight, MaskMode, MaskState, MaskTool};

// ────────────────────────────────────────────────────────────────
//  Sub-Toolbar: secondary toolbar that appears below the main
//  CAD toolbar for mask brush tools, results panel, and grid removal.
// ────────────────────────────────────────────────────────────────

/// Entry point: draws whichever sub-toolbar is currently active.
pub fn draw_sub_toolbar(state: &mut AppState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    if state.axis_mask.active || state.data_mask.active {
        draw_mask_window(state, ui, actions);
    }
    // Grid removal housekeeping (debounce + texture build) runs always
    handle_grid_removal_housekeeping(state, ui);
}

// ────────────────────────────────────────────────────────────────
//  Mask Sub-Toolbar + Results Panel (combined in one window)
// ────────────────────────────────────────────────────────────────

fn draw_mask_window(state: &mut AppState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let lang = state.lang;
    let screen = ui.ctx().viewport_rect();
    let window = egui::Window::new("Mask Sub-Toolbar")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .default_width(280.0)
        .pivot(egui::Align2::RIGHT_TOP)
        .default_pos([screen.max.x - 5.0, 60.0])
        .order(egui::Order::Foreground);

    window.show(ui.ctx(), |ui| {
        // ── Brush / Eraser toolbar row ──
        let mask = if state.axis_mask.active {
            &state.axis_mask
        } else {
            &state.data_mask
        };
        draw_mask_tools(mask, lang, ui, actions);

        // ── Results panel (if results exist) ──
        let mask = if state.axis_mask.active {
            &state.axis_mask
        } else {
            &state.data_mask
        };

        let has_axis = mask.axis_result.is_some();
        let has_data = mask.data_result.is_some();

        if has_axis || has_data {
            ui.separator();
            match mask.mask_mode {
                MaskMode::AxisCalib => draw_axis_section(mask, lang, ui, actions),
                MaskMode::DataRecog => draw_data_section(mask, lang, ui, actions),
            }
        }

        // Grid removal controls (only in data mask mode)
        if state.data_mask.active {
            ui.separator();
            draw_grid_removal_section(state, lang, ui, actions);
        }
    });
}

fn draw_mask_tools(mask: &MaskState, lang: Lang, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(mask.tool == MaskTool::Pen, format!("{} {}", icons::BRUSH, t(lang, "brush")))
            .on_hover_text(t(lang, "paint_mask"))
            .clicked()
        {
            actions.push(Action::MaskSetTool(MaskTool::Pen));
        }
        if ui
            .selectable_label(
                mask.tool == MaskTool::Eraser,
                format!("{} {}", icons::ERASER, t(lang, "eraser")),
            )
            .on_hover_text(t(lang, "erase_mask"))
            .clicked()
        {
            actions.push(Action::MaskSetTool(MaskTool::Eraser));
        }

        // Compact brush size slider
        let mut size = mask.brush_size;
        let slider = egui::Slider::new(&mut size, 2.0..=80.0)
            .step_by(1.0)
            .show_value(false);
        if ui
            .add_sized([40.0, 18.0], slider)
            .on_hover_text(format!("Brush size: {:.0} px", mask.brush_size))
            .changed()
        {
            actions.push(Action::MaskSetBrushSize(size));
        }

        // Visibility toggle
        let vis_icon = if mask.visible {
            icons::EYE
        } else {
            icons::EYE_OFF
        };
        if ui
            .add_sized(
                egui::vec2(22.0, 18.0),
                egui::Button::new(vis_icon).selected(!mask.visible),
            )
            .on_hover_text(if mask.visible {
                t(lang, "hide_mask")
            } else {
                t(lang, "show_mask")
            })
            .clicked()
        {
            actions.push(Action::MaskToggleVisibility);
        }

        // Clear button
        if ui
            .button(icons::TRASH)
            .on_hover_text(t(lang, "clear_mask"))
            .clicked()
        {
            actions.push(Action::MaskClear);
        }
    });
}

// ────────────────────────────────────────────────────────────────
//  Axis Results Section
// ────────────────────────────────────────────────────────────────

fn draw_axis_section(mask: &MaskState, lang: Lang, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    if let Some(ref axis_result) = mask.axis_result {
        let has_x = axis_result.x_axis.is_some();
        let has_y = axis_result.y_axis.is_some();

        if !has_x && !has_y {
            ui.label(t(lang, "no_axes_detected"));
            return;
        }

        ui.label(egui::RichText::new(t(lang, "axes_calibration_title")).strong());
        ui.add_space(2.0);

        // X-Axis row
        if has_x {
            let x_resp = egui::Frame::NONE
                .inner_margin(4.0)
                .corner_radius(3.0)
                .fill(if mask.highlight_axis == Some(AxisHighlight::X) {
                    Color32::from_rgba_unmultiplied(66, 133, 244, 40)
                } else {
                    Color32::TRANSPARENT
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(0x42, 0x85, 0xF4), t(lang, "x_axis"));
                        let tick_info = format!("({} {})", axis_result.x_ticks.len(), t(lang, "ticks"));
                        ui.label(egui::RichText::new(tick_info).small());
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.button(t(lang, "apply")).clicked() {
                                    actions.push(Action::MaskApplyAxis(AxisHighlight::X));
                                }
                            },
                        );
                    });
                });

            if x_resp.response.hovered() {
                if mask.highlight_axis != Some(AxisHighlight::X) {
                    actions.push(Action::MaskSetAxisHighlight(Some(AxisHighlight::X)));
                }
            } else if mask.highlight_axis == Some(AxisHighlight::X) {
                actions.push(Action::MaskSetAxisHighlight(None));
            }
        }

        // Y-Axis row
        if has_y {
            let y_resp = egui::Frame::NONE
                .inner_margin(4.0)
                .corner_radius(3.0)
                .fill(if mask.highlight_axis == Some(AxisHighlight::Y) {
                    Color32::from_rgba_unmultiplied(52, 168, 83, 40)
                } else {
                    Color32::TRANSPARENT
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(0x34, 0xA8, 0x53), t(lang, "y_axis"));
                        let tick_info = format!("({} {})", axis_result.y_ticks.len(), t(lang, "ticks"));
                        ui.label(egui::RichText::new(tick_info).small());
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.button(t(lang, "apply")).clicked() {
                                    actions.push(Action::MaskApplyAxis(AxisHighlight::Y));
                                }
                            },
                        );
                    });
                });

            if y_resp.response.hovered() {
                if mask.highlight_axis != Some(AxisHighlight::Y) {
                    actions.push(Action::MaskSetAxisHighlight(Some(AxisHighlight::Y)));
                }
            } else if mask.highlight_axis == Some(AxisHighlight::Y) {
                actions.push(Action::MaskSetAxisHighlight(None));
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────
//  Data Results Section
// ────────────────────────────────────────────────────────────────

fn draw_data_section(mask: &MaskState, lang: Lang, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    ui.label(egui::RichText::new(t(lang, "data_recognition")).strong());
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t(lang, "color_tolerance")));
        let mut tol = mask.color_tolerance;
        let slider = egui::Slider::new(&mut tol, 10.0..=120.0)
            .step_by(1.0)
            .show_value(false);
        if ui.add_sized([10.0, 18.0], slider).changed() {
            actions.push(Action::MaskSetColorTolerance(tol));
        }
        let mut tol_drag = mask.color_tolerance;
        if ui
            .add(
                egui::DragValue::new(&mut tol_drag)
                    .range(10.0..=120.0)
                    .speed(0.5),
            )
            .changed()
        {
            actions.push(Action::MaskSetColorTolerance(tol_drag));
        }
    });

    ui.add_space(4.0);

    if let Some(ref data_result) = mask.data_result {
        if data_result.groups.is_empty() {
            ui.label(t(lang, "no_data_detected"));
            return;
        }

        let total_pixels: usize = data_result
            .groups
            .iter()
            .map(|g| g.pixel_coords.len())
            .sum();

        for (idx, group) in data_result.groups.iter().enumerate() {
            let is_highlighted = mask.highlight_data_idx == Some(idx);

            let group_resp = egui::Frame::NONE
                .inner_margin(4.0)
                .corner_radius(3.0)
                .fill(if is_highlighted {
                    Color32::from_rgba_unmultiplied(
                        group.color[0],
                        group.color[1],
                        group.color[2],
                        40,
                    )
                } else {
                    Color32::TRANSPARENT
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Color swatch
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            rect,
                            3.0,
                            Color32::from_rgb(group.color[0], group.color[1], group.color[2]),
                        );

                        // Pixel percentage
                        let percent = if total_pixels > 0 {
                            (group.pixel_coords.len() as f32 / total_pixels as f32) * 100.0
                        } else {
                            0.0
                        };
                        ui.allocate_ui_with_layout(
                            egui::vec2(25.0, ui.available_height()),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(egui::RichText::new(format!("{:.1}%", percent)).small());
                            },
                        );

                        // Mode selector
                        let mode_label = match group.curve_mode {
                            crate::state::DataCurveMode::Continuous => t(lang, "curve"),
                            crate::state::DataCurveMode::Scatter => t(lang, "scatter"),
                        };
                        let combo_id = egui::Id::new("data_mode").with(idx);
                        egui::ComboBox::from_id_salt(combo_id)
                            .selected_text(mode_label)
                            .width(65.0)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        group.curve_mode
                                            == crate::state::DataCurveMode::Continuous,
                                        t(lang, "curve"),
                                    )
                                    .clicked()
                                {
                                    actions.push(Action::MaskSetDataMode(
                                        idx,
                                        crate::state::DataCurveMode::Continuous,
                                    ));
                                }
                                if ui
                                    .selectable_label(
                                        group.curve_mode == crate::state::DataCurveMode::Scatter,
                                        t(lang, "scatter"),
                                    )
                                    .clicked()
                                {
                                    actions.push(Action::MaskSetDataMode(
                                        idx,
                                        crate::state::DataCurveMode::Scatter,
                                    ));
                                }
                            });

                        // Point count (Continuous only)
                        if group.curve_mode == crate::state::DataCurveMode::Continuous {
                            let mut pts = group.point_count;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut pts)
                                        .range(2..=200)
                                        .suffix(t(lang, "pts_suffix")),
                                )
                                .changed()
                            {
                                actions.push(Action::MaskSetDataPoints(idx, pts));
                            }
                        }

                        // Add button
                        if ui
                            .button(t(lang, "add_as_group"))
                            .on_hover_text(t(lang, "hover_add_group"))
                            .clicked()
                        {
                            actions.push(Action::MaskAddData(idx));
                        }
                    });
                });

            if group_resp.response.hovered() {
                if mask.highlight_data_idx != Some(idx) {
                    actions.push(Action::MaskSetDataHighlight(Some(idx)));
                }
            } else if mask.highlight_data_idx == Some(idx) {
                actions.push(Action::MaskSetDataHighlight(None));
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────
//  Grid Removal (integrated into data mask sub-toolbar)
// ────────────────────────────────────────────────────────────────

fn draw_grid_removal_section(
    state: &mut AppState,
    lang: crate::i18n::Lang,
    ui: &mut egui::Ui,
    actions: &mut Vec<Action>,
) {
    ui.horizontal(|ui| {
        let mut enabled = state.grid_removal.enabled;
        if ui
            .checkbox(&mut enabled, format!("{} {}", icons::GRID, t(lang, "grid")))
            .changed()
        {
            if enabled {
                actions.push(Action::GridRemovalEnable);
            } else {
                actions.push(Action::GridRemovalDisable);
            }
        }
    });

    if state.grid_removal.enabled {
        ui.horizontal(|ui| {
            if state.grid_removal.is_computing {
                ui.spinner();
            }
            ui.label(t(lang, "strength"));
            let mut strength = state.grid_removal.strength;
            let slider = egui::Slider::new(&mut strength, 0.0..=1.0).step_by(0.01);
            if ui.add(slider).changed() {
                actions.push(Action::GridRemovalSetStrength(strength));
            }
        });
    }
}

fn handle_grid_removal_housekeeping(state: &mut AppState, ui: &mut egui::Ui) {
    // Debounce: check if pending strength should trigger recomputation
    if let (Some(_pending), Some(since)) = (
        state.grid_removal.pending_strength,
        state.grid_removal.pending_since,
    ) {
        if since.elapsed() > std::time::Duration::from_millis(300)
            && !state.grid_removal.is_computing
        {
            state.grid_removal.pending_strength = None;
            state.grid_removal.pending_since = None;
            crate::action_handler::trigger_grid_removal_for(state);
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    // Build cleaned texture if needed
    if state.grid_removal.enabled
        && state.grid_removal.cleaned_rgba.is_some()
        && state.grid_removal.cleaned_texture.is_none()
    {
        if let Some(ref cleaned) = state.grid_removal.cleaned_rgba {
            let w = state.img_size.x as usize;
            let h = state.img_size.y as usize;
            if cleaned.len() == w * h * 4 {
                let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], cleaned);
                let handle = ui
                    .ctx()
                    .load_texture("grid_cleaned", color_image, egui::TextureOptions::LINEAR);
                state.grid_removal.cleaned_texture = Some(handle);
            }
        }
    }
}
