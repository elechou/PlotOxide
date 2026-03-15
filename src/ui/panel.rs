use eframe::egui;
use egui::{Color32, Vec2};
use rfd::FileDialog;
use std::fs::File;
use std::io::Write;

use crate::action::Action;
use crate::i18n::t;
use crate::icons;
use crate::state::{AppMode, AppState};

pub fn draw_panel(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    let lang = state.lang;
    egui::SidePanel::left("left_panel")
        .resizable(false)
        .exact_width(300.0)
        .show(ctx, |ui| {
            ui.add_space(10.0);

            // Set explicit horizontal filling for children so frames all snap to the same uniform width
            ui.set_min_width(ui.available_width());

            ui.add_enabled_ui(state.texture.is_some(), |ui| {
                // Section 1: Calibration

                ui.strong(t(lang, "axes_calibration"));
                ui.add_space(10.0);

                // Warnings placed below text fields so their appearance/disappearance
                // doesn't shift the input fields and steal focus mid-edit
                if state.calib_pts.len() < 4 {
                    ui.colored_label(
                        Color32::RED,
                        format!(
                            "{} {} ({}/4)",
                            icons::ALERT,
                            t(lang, "place_calib_points"),
                            state.calib_pts.len()
                        ),
                    );
                }

                let magic_active = state.axis_mask.active
                    && state.axis_mask.mask_mode == crate::state::MaskMode::AxisCalib;

                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::Button::new(
                            egui::RichText::new(format!("{}  {}", icons::AXIS_BRUSH, t(lang, "smart_axis_brush"))).strong()
                                ).selected(magic_active),
                    )
                    .on_hover_text(t(lang, "hover_axis_mask"))
                    .clicked()
                {
                    actions.push(Action::MaskToggleForAxis);
                }

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    let btn_text = if state.mode == AppMode::AddCalib {
                        t(lang, "stop")
                    } else {
                        t(lang, "manually_place_axes")
                    };
                    let mut btn = egui::Button::new(btn_text);
                    if state.mode == AppMode::AddCalib {
                        btn = btn.fill(Color32::from_rgb(50, 120, 180));
                    }

                    if ui.add_sized([168.0, 20.0], btn).clicked() {
                        actions.push(Action::SetMode(if state.mode == AppMode::AddCalib {
                            AppMode::Select
                        } else {
                            AppMode::AddCalib
                        }));
                    }

                    let clear_btn = egui::Button::new(t(lang, "clear_axes"));
                    if ui.add_sized([108.0, 20.0], clear_btn).clicked() {
                        actions.push(Action::ClearCalib);
                    }
                });

                ui.add_space(10.0);

                ui.columns(2, |cols| {
                    cols[0].horizontal(|ui| {
                        ui.label("X1:");
                        let mut x1 = state.x1_val.clone();
                        if ui
                            .add_sized(
                                [ui.available_width(), 20.0],
                                egui::TextEdit::singleline(&mut x1),
                            )
                            .changed()
                        {
                            actions.push(Action::UpdateCalibAxis("x1".to_string(), x1));
                        }
                    });
                    cols[1].horizontal(|ui| {
                        ui.label("X2:");
                        let mut x2 = state.x2_val.clone();
                        if ui
                            .add_sized(
                                [ui.available_width(), 20.0],
                                egui::TextEdit::singleline(&mut x2),
                            )
                            .changed()
                        {
                            actions.push(Action::UpdateCalibAxis("x2".to_string(), x2));
                        }
                    });
                });

                ui.add_space(5.0);

                ui.columns(2, |cols| {
                    cols[0].horizontal(|ui| {
                        ui.label("Y1:");
                        let mut y1 = state.y1_val.clone();
                        if ui
                            .add_sized(
                                [ui.available_width(), 20.0],
                                egui::TextEdit::singleline(&mut y1),
                            )
                            .changed()
                        {
                            actions.push(Action::UpdateCalibAxis("y1".to_string(), y1));
                        }
                    });
                    cols[1].horizontal(|ui| {
                        ui.label("Y2:");
                        let mut y2 = state.y2_val.clone();
                        if ui
                            .add_sized(
                                [ui.available_width(), 20.0],
                                egui::TextEdit::singleline(&mut y2),
                            )
                            .changed()
                        {
                            actions.push(Action::UpdateCalibAxis("y2".to_string(), y2));
                        }
                    });
                });

                {
                    let filled = [&state.x1_val, &state.x2_val, &state.y1_val, &state.y2_val]
                        .iter()
                        .filter(|v| v.parse::<f64>().is_ok())
                        .count();
                    if filled < 4 {
                        ui.add_space(5.0);
                        ui.colored_label(
                            Color32::RED,
                            format!(
                                "{} {} ({}/4)",
                                icons::ALERT,
                                t(lang, "enter_tick_values"),
                                filled
                            ),
                        );
                    }
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let mut log_x = state.log_x;
                    if ui.checkbox(&mut log_x, t(lang, "log_x")).changed() {
                        actions.push(Action::UpdateLogScale(log_x, state.log_y));
                    }
                    let mut log_y = state.log_y;
                    if ui.checkbox(&mut log_y, t(lang, "log_y")).changed() {
                        actions.push(Action::UpdateLogScale(state.log_x, log_y));
                    }
                });

                ui.add_space(15.0);

                ui.separator();

                ui.add_space(15.0);

                // Section 2: Extraction
                ui.strong(t(lang, "data_extraction"));
                ui.add_space(10.0);

                ui.label(format!("{}: {}", t(lang, "total_datapoints"), state.data_pts.len()));

                ui.add_space(5.0);

                let mask_active = state.data_mask.active
                    && state.data_mask.mask_mode == crate::state::MaskMode::DataRecog;

                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::Button::new(
                            egui::RichText::new(format!("{}  {}", icons::DATA_BRUSH, t(lang, "smart_data_brush"))).strong()
                            ).selected(mask_active),
                    )
                    .on_hover_text(t(lang, "hover_data_mask"))
                    .clicked()
                {
                    actions.push(Action::MaskToggle);
                }

                ui.add_space(5.0);

                let _palette = [
                    Color32::from_rgb(0xe4, 0x1a, 0x1c), // Red
                    Color32::from_rgb(0x37, 0x7e, 0xb8), // Blue
                    Color32::from_rgb(0x4d, 0xaf, 0x4a), // Green
                    Color32::from_rgb(0x98, 0x4e, 0xa3), // Purple
                    Color32::from_rgb(0xff, 0x7f, 0x00), // Orange
                ];

                ui.horizontal(|ui| {
                    if ui
                        .add_sized([90.0, 20.0], egui::Button::new(format!("{} {}", icons::ADD_GROUP, t(lang, "add_group"))))
                        .clicked()
                    {
                        actions.push(Action::AddGroup);
                    }

                    if ui
                        .add_sized([90.0, 20.0], egui::Button::new(t(lang, "clear_all_data")))
                        .clicked()
                    {
                        actions.push(Action::RequestClearData);
                    }

                    if ui
                        .add_sized([87.0, 20.0], egui::Button::new(t(lang, "export_csv_btn")))
                        .clicked()
                    {
                        actions.push(Action::RequestExportCsv);
                    }
                });

                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .id_salt("data_panel_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let _num_groups = state.groups.len();
                        for (g_idx, group) in state.groups.iter().enumerate() {
                            let frame = egui::Frame::NONE
                                .inner_margin(4.0)
                                .corner_radius(4.0)
                                .fill(if state.active_group_idx == g_idx {
                                    Color32::from_gray(50)
                                } else {
                                    Color32::TRANSPARENT
                                });

                            let (_inner_resp, payload_opt) =
                                ui.dnd_drop_zone::<Vec<usize>, _>(frame, |ui| {
                                    ui.horizontal(|ui| {
                                        // Expand/Collapse toggle
                                        let is_collapsed = state.collapsed_groups.contains(&g_idx);
                                        let collapse_icon =
                                            if is_collapsed { icons::CHEVRON_RIGHT } else { icons::CHEVRON_DOWN };
                                        if ui
                                            .button(collapse_icon)
                                            .on_hover_text(if is_collapsed {
                                                t(lang, "expand")
                                            } else {
                                                t(lang, "collapse")
                                            })
                                            .clicked()
                                        {
                                            if is_collapsed {
                                                state.collapsed_groups.remove(&g_idx);
                                            } else {
                                                state.collapsed_groups.insert(g_idx);
                                            }
                                        }

                                        let is_active = state.active_group_idx == g_idx;
                                        let radio_icon = if is_active { icons::RADIO_ON } else { icons::RADIO_OFF };
                                        if ui
                                            .selectable_label(is_active, radio_icon)
                                            .on_hover_text(t(lang, "set_active_group"))
                                            .clicked()
                                        {
                                            actions.push(Action::SetActiveGroup(g_idx));
                                        }

                                        let mut col = group.color;
                                        if ui.color_edit_button_srgba(&mut col).changed() {
                                            actions.push(Action::UpdateGroupColor(g_idx, col));
                                        }

                                        let right_space = 24.0 /* trash */ + 60.0 /* count */;
                                        let text_width =
                                            (ui.available_width() - right_space).max(40.0);

                                        let mut name = group.name.clone();
                                        if ui
                                            .add_sized(
                                                [text_width, 20.0],
                                                egui::TextEdit::singleline(&mut name),
                                            )
                                            .changed()
                                        {
                                            actions.push(Action::UpdateGroupName(g_idx, name));
                                        }

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(icons::TRASH)
                                                    .on_hover_text(t(lang, "delete_group"))
                                                    .clicked()
                                                {
                                                    actions.push(Action::DeleteGroup(g_idx));
                                                }

                                                let count = state
                                                    .data_pts
                                                    .iter()
                                                    .filter(|p| p.group_id == g_idx)
                                                    .count();
                                                ui.label(format!("({} {})", count, t(lang, "pts")));
                                            },
                                        );
                                    });
                                });

                            if let Some(payload) = payload_opt {
                                actions.push(Action::MovePointsToGroup {
                                    indices: payload.to_vec(),
                                    new_group_id: g_idx,
                                });
                            }

                            ui.add_space(2.0);

                            let is_collapsed = state.collapsed_groups.contains(&g_idx);
                            if !is_collapsed {
                                let group_indices: Vec<usize> = state
                                    .data_pts
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, p)| p.group_id == g_idx)
                                    .map(|(idx, _)| idx)
                                    .collect();
                                let has_points = !group_indices.is_empty();

                                for (list_pos, &i) in group_indices.iter().enumerate() {
                                    let p = &state.data_pts[i];
                                    let is_selected = state.selected_data_indices.contains(&i);
                                    let is_hovered = state.hovered_data_idx == Some(i);

                                    let pt_id = egui::Id::new("pt").with(i);

                                    let drag_payload = if is_selected {
                                        let mut payload: Vec<usize> =
                                            state.selected_data_indices.iter().copied().collect();
                                        payload.sort();
                                        payload
                                    } else {
                                        vec![i]
                                    };

                                    let mut text_x = egui::RichText::new(format!("{:.8}", p.lx));
                                    let mut text_y = egui::RichText::new(format!("{:.8}", p.ly));

                                    let mut bg_color = Color32::TRANSPARENT;
                                    if is_selected {
                                        bg_color = group.color.linear_multiply(0.2);
                                        text_x = text_x.color(group.color).strong();
                                        text_y = text_y.color(group.color).strong();
                                    } else if is_hovered {
                                        let h_col = group.color.linear_multiply(0.8);
                                        text_x = text_x.color(h_col);
                                        text_y = text_y.color(h_col);
                                    }

                                    egui::Frame::NONE
                                        .fill(bg_color)
                                        .inner_margin(egui::Margin::same(2))
                                        .corner_radius(3.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let (handle_rect, handle_resp) = ui
                                                    .allocate_exact_size(
                                                        egui::vec2(16.0, 20.0),
                                                        egui::Sense::drag(),
                                                    );
                                                let handle_resp = handle_resp
                                                    .on_hover_cursor(egui::CursorIcon::Grab);
                                                ui.painter().text(
                                                    handle_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    icons::DRAG_HANDLE,
                                                    egui::FontId::proportional(14.0),
                                                    Color32::DARK_GRAY,
                                                );

                                                if handle_resp.dragged() {
                                                    egui::DragAndDrop::set_payload(
                                                        ui.ctx(),
                                                        drag_payload.clone(),
                                                    );
                                                    ui.ctx().set_cursor_icon(
                                                        egui::CursorIcon::Grabbing,
                                                    );

                                                    if let Some(mouse_pos) =
                                                        ui.ctx().pointer_interact_pos()
                                                    {
                                                        egui::Area::new(pt_id.with("drag_tooltip"))
                                                            .order(egui::Order::Tooltip)
                                                            .interactable(false)
                                                            .fixed_pos(
                                                                mouse_pos + egui::vec2(12.0, 12.0),
                                                            )
                                                            .show(ui.ctx(), |ui| {
                                                                egui::Frame::popup(ui.style())
                                                                    .show(ui, |ui| {
                                                                        if drag_payload.len() > 1 {
                                                                            ui.label(
                                                                            egui::RichText::new(
                                                                                format!(
                                                                                    "{} {}",
                                                                                    drag_payload
                                                                                        .len(),
                                                                                    t(lang, "points"),
                                                                                ),
                                                                            )
                                                                            .color(group.color)
                                                                            .strong(),
                                                                        );
                                                                        } else {
                                                                            ui.label(
                                                                            egui::RichText::new(
                                                                                format!(
                                                                                    "{:.4}, {:.4}",
                                                                                    p.lx, p.ly
                                                                                ),
                                                                            )
                                                                            .color(group.color)
                                                                            .strong(),
                                                                        );
                                                                        }
                                                                    });
                                                            });
                                                    }
                                                }

                                                let avail = ui.available_width();
                                                let trash_width = 24.0;
                                                let text_width =
                                                    ((avail - trash_width) / 2.0).max(0.0);

                                                let resp_x = ui.add_sized(
                                                    [text_width, 20.0],
                                                    egui::Button::new(text_x)
                                                        .selected(is_selected)
                                                        .frame(false),
                                                );
                                                let resp_y = ui.add_sized(
                                                    [text_width, 20.0],
                                                    egui::Button::new(text_y)
                                                        .selected(is_selected)
                                                        .frame(false),
                                                );

                                                let clicked = resp_x.clicked() || resp_y.clicked();
                                                if clicked {
                                                    let modifiers = ui.ctx().input(|i| i.modifiers);
                                                    let is_multi =
                                                        modifiers.command || modifiers.ctrl;
                                                    let is_shift = modifiers.shift;

                                                    if is_shift {
                                                        let selected_positions: Vec<usize> =
                                                            group_indices
                                                                .iter()
                                                                .enumerate()
                                                                .filter(|(_, &idx)| {
                                                                    state
                                                                        .selected_data_indices
                                                                        .contains(&idx)
                                                                })
                                                                .map(|(pos, _)| pos)
                                                                .collect();
                                                        let min_sel = selected_positions
                                                            .iter()
                                                            .min()
                                                            .copied()
                                                            .unwrap_or(list_pos);
                                                        let max_sel = selected_positions
                                                            .iter()
                                                            .max()
                                                            .copied()
                                                            .unwrap_or(list_pos);
                                                        let start = list_pos.min(min_sel);
                                                        let end = list_pos.max(max_sel);

                                                        let mut new_selection = Vec::new();
                                                        for pos in start..=end {
                                                            new_selection.push(group_indices[pos]);
                                                        }
                                                        actions.push(Action::SelectPoints(
                                                            new_selection,
                                                            true,
                                                        ));
                                                    } else {
                                                        actions.push(Action::SelectPoints(
                                                            vec![i],
                                                            is_multi,
                                                        ));
                                                    }
                                                }

                                                let mut btn_text = egui::RichText::new(icons::TRASH);
                                                if is_selected {
                                                    btn_text = btn_text.color(group.color);
                                                } else if is_hovered {
                                                    btn_text = btn_text
                                                        .color(group.color.linear_multiply(0.8));
                                                }

                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if ui
                                                            .add_sized(
                                                                [trash_width, 20.0],
                                                                egui::Button::new(btn_text)
                                                                    .frame(false),
                                                            )
                                                            .clicked()
                                                        {
                                                            actions
                                                                .push(Action::RemoveDataPoint(i));
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                }

                                if !has_points {
                                    ui.label(
                                        egui::RichText::new(t(lang, "drag_hint"))
                                            .color(Color32::DARK_GRAY)
                                            .small(),
                                    );
                                }
                            } // end if !is_collapsed
                            ui.add_space(4.0);
                        }
                    });
            });
        });
}

pub fn load_image(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    if let Some(path) = FileDialog::new()
        .add_filter("image", &["png", "jpg", "jpeg"])
        .pick_file()
    {
        process_image_file(state, path, ctx, actions);
    }
}

pub fn process_image_file(
    state: &mut AppState,
    path: std::path::PathBuf,
    ctx: &egui::Context,
    actions: &mut Vec<Action>,
) {
    if let Ok(img) = image::open(&path) {
        let img = img.to_rgba8();
        let size = [img.width() as _, img.height() as _];
        let pixels = img.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
        let handle = ctx.load_texture("main_image", color_image, Default::default());

        if state.dirty {
            let img_size = eframe::egui::Vec2::new(size[0] as f32, size[1] as f32);
            state.pending_action = Some(crate::state::PendingAction::LoadImage(
                path, handle, img_size,
            ));
        } else {
            actions.push(Action::LoadImage(
                path,
                handle,
                Vec2::new(size[0] as f32, size[1] as f32),
            ));
            actions.push(Action::RequestCenter);
        }
    }
}

pub fn export_csv(state: &AppState) {
    if let Some(path) = FileDialog::new()
        .set_file_name("extracted_data.csv")
        .add_filter("csv", &["csv"])
        .save_file()
    {
        if let Ok(mut file) = File::create(path) {
            let _ = writeln!(file, "Group,X,Y");
            for (g_idx, group) in state.groups.iter().enumerate() {
                for p in state.data_pts.iter().filter(|p| p.group_id == g_idx) {
                    let _ = writeln!(file, "\"{}\",{:.8},{:.8}", group.name, p.lx, p.ly);
                }
            }
        }
    }
}
