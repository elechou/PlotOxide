use eframe::egui;
use egui::Color32;

use crate::action::Action;
use crate::state::{AppState, AxisHighlight, MaskMode};

// ────────────────────────────────────────────────────────────────
//  Results Panel: appears below the mask sub-toolbar
//  Shows detected axes (AxisCalib mode) or data colors (DataRecog mode)
// ────────────────────────────────────────────────────────────────

pub fn draw_results_panel(state: &AppState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let mask = if state.axis_mask.active {
        &state.axis_mask
    } else if state.data_mask.active {
        &state.data_mask
    } else {
        return;
    };

    let has_axis = mask.axis_result.is_some();
    let has_data = mask.data_result.is_some();

    if !has_axis && !has_data {
        return;
    }

    let window = egui::Window::new("Mask Results")
        .collapsible(true)
        .resizable(false)
        .title_bar(false)
        .default_pos([325.0, 62.0])
        .default_width(200.0);

    window.show(ui.ctx(), |ui| {
        ui.set_min_width(150.0);

        match mask.mask_mode {
            MaskMode::AxisCalib => draw_axis_section(mask, ui, actions),
            MaskMode::DataRecog => draw_data_section(mask, ui, actions),
        }
    });
}

fn draw_axis_section(mask: &crate::state::MaskState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    if let Some(ref axis_result) = mask.axis_result {
        let has_x = axis_result.x_axis.is_some();
        let has_y = axis_result.y_axis.is_some();

        if !has_x && !has_y {
            ui.colored_label(Color32::GRAY, "No axes detected. Paint over axis lines.");
            return;
        }

        ui.label(
            egui::RichText::new("Axes Calibration")
                .strong()
                .color(Color32::WHITE),
        );
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
                        ui.colored_label(Color32::from_rgb(0x42, 0x85, 0xF4), "X-Axis");
                        let tick_info = format!("({} ticks)", axis_result.x_ticks.len());
                        ui.label(egui::RichText::new(tick_info).small().color(Color32::GRAY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Apply").clicked() {
                                actions.push(Action::MaskApplyAxis(AxisHighlight::X));
                            }
                        });
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
                        ui.colored_label(Color32::from_rgb(0x34, 0xA8, 0x53), "Y-Axis");
                        let tick_info = format!("({} ticks)", axis_result.y_ticks.len());
                        ui.label(egui::RichText::new(tick_info).small().color(Color32::GRAY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Apply").clicked() {
                                actions.push(Action::MaskApplyAxis(AxisHighlight::Y));
                            }
                        });
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

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        // Finish button
        if ui
            .add(
                egui::Button::new("✅ Finish Calibration")
                    .min_size(egui::vec2(ui.available_width(), 28.0)),
            )
            .on_hover_text("Confirm calibration and switch to data collection mode")
            .clicked()
        {
            actions.push(Action::MaskFinishCalib);
        }
    }
}

fn draw_data_section(mask: &crate::state::MaskState, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    // Tolerance slider
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Tolerance:")
                .small()
                .color(Color32::LIGHT_GRAY),
        );
        let mut tol = mask.color_tolerance;
        let slider = egui::Slider::new(&mut tol, 10.0..=120.0)
            .step_by(1.0)
            .show_value(false);
        if ui.add_sized([50.0, 18.0], slider).changed() {
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
            ui.colored_label(Color32::GRAY, "No data colors detected.");
            return;
        }

        ui.label(
            egui::RichText::new("Data Recognition")
                .strong()
                .color(Color32::WHITE),
        );
        ui.add_space(2.0);

        // Calculate total non-background pixels for percentage display
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

                        // Pixel count (percentage)
                        let percent = if total_pixels > 0 {
                            (group.pixel_coords.len() as f32 / total_pixels as f32) * 100.0
                        } else {
                            0.0
                        };

                        // Fixed width label to prevent jittering when percentages change slightly
                        ui.allocate_ui_with_layout(
                            egui::vec2(25.0, ui.available_height()),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", percent))
                                        .small()
                                        .color(Color32::LIGHT_GRAY),
                                );
                            },
                        );

                        // Mode selector
                        let mode_label = match group.curve_mode {
                            crate::state::DataCurveMode::Continuous => "Curve",
                            crate::state::DataCurveMode::Scatter => "Scatter",
                        };
                        let combo_id = egui::Id::new("data_mode").with(idx);
                        egui::ComboBox::from_id_salt(combo_id)
                            .selected_text(mode_label)
                            .width(65.0)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        group.curve_mode == crate::state::DataCurveMode::Continuous,
                                        "Curve",
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
                                        "Scatter",
                                    )
                                    .clicked()
                                {
                                    actions.push(Action::MaskSetDataMode(
                                        idx,
                                        crate::state::DataCurveMode::Scatter,
                                    ));
                                }
                            });

                        // Point count (only for Continuous)
                        if group.curve_mode == crate::state::DataCurveMode::Continuous {
                            let mut pts = group.point_count;
                            if ui
                                .add(egui::DragValue::new(&mut pts).range(2..=200).suffix(" Pts"))
                                .changed()
                            {
                                actions.push(Action::MaskSetDataPoints(idx, pts));
                            }
                        }

                        // Add button
                        if ui
                            .button("Add")
                            .on_hover_text("Create new group with this color and add points")
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

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        // Finish button
        if ui
            .add(egui::Button::new("✅ Finish").min_size(egui::vec2(ui.available_width(), 28.0)))
            .on_hover_text("Clear mask and finish data recognition")
            .clicked()
        {
            actions.push(Action::MaskFinishCalib);
        }
    }
}
