use eframe::egui;
use egui::{Color32, Vec2};
use rfd::FileDialog;
use std::fs::File;
use std::io::Write;

use crate::core::recalculate_data;
use crate::state::{AppMode, AppState};

pub fn draw_panel(state: &mut AppState, ctx: &egui::Context) {
    egui::SidePanel::left("left_panel")
        .resizable(false)
        .exact_width(320.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);

                // Set explicit horizontal filling for children so frames all snap to the same uniform width
                ui.set_min_width(ui.available_width());

                // Unified card style
                let frame = egui::Frame::group(ui.style())
                    .fill(Color32::from_gray(35))
                    .inner_margin(12.0)
                    .corner_radius(5.0);

                // Section 1: Calibration
                frame.show(ui, |ui| {
                    ui.strong("1. Axes Calibration");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        let btn_text = if state.mode == AppMode::AddCalib {
                            "Stop Calib"
                        } else {
                            "Place Calib Points"
                        };
                        let mut btn = egui::Button::new(btn_text);
                        if state.mode == AppMode::AddCalib {
                            btn = btn.fill(Color32::from_rgb(180, 50, 50));
                        }
                        if ui.add_sized([160.0, 30.0], btn).clicked() {
                            state.mode = if state.mode == AppMode::AddCalib {
                                AppMode::Select
                            } else {
                                AppMode::AddCalib
                            };
                        }
                        ui.label(format!("Points: {}/4", state.calib_pts.len()));
                    });

                    if ui.button("Clear Calib").clicked() {
                        state.calib_pts.clear();
                        recalculate_data(
                            &state.calib_pts,
                            &mut state.data_pts,
                            &state.x1_val,
                            &state.x2_val,
                            &state.y1_val,
                            &state.y2_val,
                            state.log_x,
                            state.log_y,
                        );
                    }

                    ui.add_space(10.0);
                    egui::Grid::new("calib_grid")
                        .num_columns(2)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("X1 (Min):");
                            ui.text_edit_singleline(&mut state.x1_val);
                            ui.end_row();
                            ui.label("X2 (Max):");
                            ui.text_edit_singleline(&mut state.x2_val);
                            ui.end_row();
                            ui.label("Y1 (Min):");
                            ui.text_edit_singleline(&mut state.y1_val);
                            ui.end_row();
                            ui.label("Y2 (Max):");
                            ui.text_edit_singleline(&mut state.y2_val);
                            ui.end_row();
                        });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut state.log_x, "Log X").changed() {
                            recalculate_data(
                                &state.calib_pts,
                                &mut state.data_pts,
                                &state.x1_val,
                                &state.x2_val,
                                &state.y1_val,
                                &state.y2_val,
                                state.log_x,
                                state.log_y,
                            );
                        }
                        if ui.checkbox(&mut state.log_y, "Log Y").changed() {
                            recalculate_data(
                                &state.calib_pts,
                                &mut state.data_pts,
                                &state.x1_val,
                                &state.x2_val,
                                &state.y1_val,
                                &state.y2_val,
                                state.log_x,
                                state.log_y,
                            );
                        }
                    });
                });

                ui.add_space(15.0);

                // Section 2: Extraction
                frame.show(ui, |ui| {
                    ui.strong("2. Data Extraction");
                    ui.add_space(10.0);

                    if state.calib_pts.len() < 4 {
                        ui.colored_label(Color32::RED, "⚠ Calibrate 4 axes points first");
                        ui.add_space(5.0);
                    }

                    ui.label(format!("Total Datapoints: {}", state.data_pts.len()));

                    let palette = [
                        Color32::from_rgb(0xe4, 0x1a, 0x1c), // Red
                        Color32::from_rgb(0x37, 0x7e, 0xb8), // Blue
                        Color32::from_rgb(0x4d, 0xaf, 0x4a), // Green
                        Color32::from_rgb(0x98, 0x4e, 0xa3), // Purple
                        Color32::from_rgb(0xff, 0x7f, 0x00), // Orange
                    ];

                    ui.horizontal(|ui| {
                        if ui.button("➕ Add Group").clicked() {
                            let new_idx = state.groups.len();
                            let col = palette[new_idx % palette.len()];
                            state.groups.push(crate::state::PointGroup {
                                name: format!("Group {}", new_idx + 1),
                                color: col,
                            });
                            state.active_group_idx = new_idx;
                        }
                        if ui.button("Clear All Data").clicked() {
                            state.data_pts.clear();
                        }
                    });

                    ui.add_space(10.0);
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            let mut to_remove_group = None;
                            let mut to_remove_data = None;
                            let mut move_point = None;

                            let num_groups = state.groups.len();
                            for (g_idx, group) in state.groups.iter_mut().enumerate() {
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
                                            let mut is_active = state.active_group_idx == g_idx;
                                            if ui.toggle_value(&mut is_active, "🔴").on_hover_text("Set Active Group").clicked() {
                                                state.active_group_idx = g_idx;
                                            }
                                            ui.color_edit_button_srgba(&mut group.color);
                                            
                                            let right_space = if num_groups > 1 { 24.0 /* trash */ + 50.0 /* assign */ + 60.0 /* count */ } else { 50.0 + 60.0 };
                                            let text_width = (ui.available_width() - right_space).max(40.0);
                                            
                                            ui.add_sized(
                                                [text_width, 20.0],
                                                egui::TextEdit::singleline(&mut group.name),
                                            );

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if num_groups > 1 {
                                                        if ui.button("🗑").on_hover_text("Delete Group").clicked() {
                                                            to_remove_group = Some(g_idx);
                                                        }
                                                    }
                                                    
                                                    // Batch assign currently selected points to this group
                                                    ui.add_enabled_ui(!state.selected_data_indices.is_empty(), |ui| {
                                                        if ui.button("Assign").on_hover_text("Assign selected points to this group").clicked() {
                                                            let payload: Vec<usize> = state.selected_data_indices.iter().copied().collect();
                                                            move_point = Some((payload, g_idx));
                                                        }
                                                    });

                                                    let count = state
                                                        .data_pts
                                                        .iter()
                                                        .filter(|p| p.group_id == g_idx)
                                                        .count();
                                                    ui.label(format!("({} pts)", count));
                                                },
                                            );
                                        });
                                    });

                                if let Some(payload) = payload_opt {
                                    move_point = Some((payload.to_vec(), g_idx));
                                }

                                ui.add_space(4.0);

                                let group_indices: Vec<usize> = state.data_pts.iter()
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
                                        let mut payload: Vec<usize> = state.selected_data_indices.iter().copied().collect();
                                        payload.sort();
                                        payload
                                    } else {
                                        vec![i]
                                    };

                                    let mut text_x = egui::RichText::new(format!("{:.8}", p.lx));
                                    let mut text_y = egui::RichText::new(format!("{:.8}", p.ly));

                                    let mut bg_color = Color32::TRANSPARENT;
                                    if is_selected {
                                        bg_color = group.color.linear_multiply(0.2); // Soft highlight background
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
                                                let (handle_rect, handle_resp) = ui.allocate_exact_size(egui::vec2(16.0, 20.0), egui::Sense::drag());
                                                let handle_resp = handle_resp.on_hover_cursor(egui::CursorIcon::Grab);
                                                ui.painter().text(
                                                    handle_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "☰",
                                                    egui::FontId::proportional(14.0),
                                                    Color32::DARK_GRAY,
                                                );

                                                if handle_resp.dragged() {
                                                    egui::DragAndDrop::set_payload(ui.ctx(), drag_payload.clone());
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                                    
                                                    if let Some(mouse_pos) = ui.ctx().pointer_interact_pos() {
                                                        egui::Area::new(pt_id.with("drag_tooltip"))
                                                            .order(egui::Order::Tooltip)
                                                            .interactable(false)
                                                            .fixed_pos(mouse_pos + egui::vec2(12.0, 12.0))
                                                            .show(ui.ctx(), |ui| {
                                                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                                                    if drag_payload.len() > 1 {
                                                                        ui.label(
                                                                            egui::RichText::new(format!("{} points", drag_payload.len()))
                                                                                .color(group.color)
                                                                                .strong()
                                                                        );
                                                                    } else {
                                                                        ui.label(
                                                                            egui::RichText::new(format!("{:.4}, {:.4}", p.lx, p.ly))
                                                                                .color(group.color)
                                                                                .strong()
                                                                        );
                                                                    }
                                                                });
                                                            });
                                                    }
                                                }

                                                let avail = ui.available_width();
                                                let trash_width = 24.0;
                                                let text_width = ((avail - trash_width) / 2.0).max(0.0);

                                                    let resp_x = ui.add_sized([text_width, 20.0], egui::Button::new(text_x).selected(is_selected).frame(false));
                                                    let resp_y = ui.add_sized([text_width, 20.0], egui::Button::new(text_y).selected(is_selected).frame(false));
                                                    
                                                    let clicked = resp_x.clicked() || resp_y.clicked();
                                                    if clicked {
                                                        state.selected_calib_idx = None;
                                                        
                                                        let modifiers = ui.ctx().input(|i| i.modifiers);
                                                        let is_multi = modifiers.command || modifiers.ctrl;
                                                        let is_shift = modifiers.shift;
                                                        
                                                        if is_shift {
                                                            let selected_positions: Vec<usize> = group_indices.iter().enumerate()
                                                                .filter(|(_, &idx)| state.selected_data_indices.contains(&idx))
                                                                .map(|(pos, _)| pos).collect();
                                                            let min_sel = selected_positions.iter().min().copied().unwrap_or(list_pos);
                                                            let max_sel = selected_positions.iter().max().copied().unwrap_or(list_pos);
                                                            let start = list_pos.min(min_sel);
                                                            let end = list_pos.max(max_sel);
                                                            for pos in start..=end {
                                                                state.selected_data_indices.insert(group_indices[pos]);
                                                            }
                                                        } else if is_multi {
                                                            if is_selected {
                                                                state.selected_data_indices.remove(&i);
                                                            } else {
                                                                state.selected_data_indices.insert(i);
                                                            }
                                                        } else {
                                                            state.selected_data_indices.clear();
                                                            state.selected_data_indices.insert(i);
                                                        }
                                                    }

                                                    let mut btn_text = egui::RichText::new("🗑");
                                                    if is_selected {
                                                        btn_text = btn_text.color(group.color);
                                                    } else if is_hovered {
                                                        btn_text = btn_text.color(group.color.linear_multiply(0.8));
                                                    }

                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        // Disable dragging for the trash bin so it's clickable reliably
                                                        if ui.add_sized([trash_width, 20.0], egui::Button::new(btn_text).frame(false)).clicked() {
                                                            to_remove_data = Some(i);
                                                        }
                                                    });
                                            });
                                        });
                                }

                                if !has_points {
                                    ui.label(
                                        egui::RichText::new(
                                            "   (Drag points here or click canvas to add)",
                                        )
                                        .color(Color32::DARK_GRAY)
                                        .small(),
                                    );
                                }

                                ui.add_space(8.0);
                            }

                            if let Some((payload_indices, new_g_idx)) = move_point {
                                // Extract the points being moved
                                let mut moved_pts = Vec::new();
                                for &pt_idx in payload_indices.iter() {
                                    if pt_idx < state.data_pts.len() {
                                        let mut pt = state.data_pts[pt_idx].clone();
                                        pt.group_id = new_g_idx;
                                        moved_pts.push(pt);
                                    }
                                }

                                // Remove them from the existing array in reverse order
                                let mut to_remove: Vec<usize> = payload_indices.to_vec();
                                to_remove.sort_unstable_by(|a, b| b.cmp(a));
                                for &idx in to_remove.iter() {
                                    if idx < state.data_pts.len() {
                                        state.data_pts.remove(idx);
                                    }
                                }

                                // Append to the end
                                state.data_pts.extend(moved_pts);

                                // Update selection indices to the newly appended indices
                                state.selected_data_indices.clear();
                                let new_len = state.data_pts.len();
                                for i in 0..payload_indices.len() {
                                    state.selected_data_indices.insert(new_len - payload_indices.len() + i);
                                }
                            }
                            
                            if let Some(idx) = to_remove_data {
                                state.data_pts.remove(idx);
                                state.selected_data_indices.remove(&idx);
                            }
                            if let Some(idx) = to_remove_group {
                                state.groups.remove(idx);
                                // Cleanup deleted group_id and fallback active pointer
                                if state.active_group_idx == idx {
                                    state.active_group_idx = 0;
                                } else if state.active_group_idx > idx {
                                    state.active_group_idx -= 1;
                                }

                                // Re-assign orphaned points or delete them. We will re-assign to active group.
                                for p in &mut state.data_pts {
                                    if p.group_id == idx {
                                        p.group_id = state.active_group_idx;
                                    } else if p.group_id > idx {
                                        p.group_id -= 1;
                                    }
                                }
                            }
                        });
                });
            });
        });
}

pub fn load_image(state: &mut AppState, ctx: &egui::Context) {
    if let Some(path) = FileDialog::new()
        .add_filter("image", &["png", "jpg", "jpeg"])
        .pick_file()
    {
        if let Ok(img) = image::open(&path) {
            let img = img.to_rgba8();
            let size = [img.width() as _, img.height() as _];
            let pixels = img.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            state.texture = Some(ctx.load_texture("main_image", color_image, Default::default()));
            state.img_size = Vec2::new(size[0] as f32, size[1] as f32);
            state.image_path = Some(path);
            state.pan = Vec2::ZERO;
            state.zoom = 1.0;
            state.center_requested = true;
            state.calib_pts.clear();
            state.data_pts.clear();
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
