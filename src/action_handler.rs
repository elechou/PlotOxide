use crate::action::Action;
use crate::core::{CalibPoint, DataPoint};
use crate::state::{AppMode, AppState, HistorySnapshot, PointGroup};
use eframe::egui::Color32;
use std::sync::Arc;

/// Helper to get the active RGBA from parts (avoids borrow conflicts).
fn active_rgba_from_parts<'a>(
    grid_removal: &'a crate::state::GridRemovalState,
    decoded_rgba: &'a Option<Arc<Vec<u8>>>,
) -> Option<&'a Arc<Vec<u8>>> {
    if grid_removal.enabled {
        grid_removal
            .cleaned_rgba
            .as_ref()
            .or(decoded_rgba.as_ref())
    } else {
        decoded_rgba.as_ref()
    }
}

/// Spawn a background thread to compute grid removal.
/// Also called from project.rs during project load.
pub fn trigger_grid_removal_for(state: &mut AppState) {
    trigger_grid_removal(state);
}

fn trigger_grid_removal(state: &mut AppState) {
    if let (Some(ref rgba_arc), Some(tx)) = (&state.decoded_rgba, &state.mask_tx) {
        state.grid_removal.compute_generation += 1;
        state.grid_removal.is_computing = true;

        let rgba_clone = Arc::clone(rgba_arc);
        let w = state.img_size.x as u32;
        let h = state.img_size.y as u32;
        let strength = state.grid_removal.strength;
        let generation = state.grid_removal.compute_generation;
        let tx_clone = tx.clone();

        std::thread::spawn(move || {
            let cleaned = crate::recognition::grid_removal::remove_grid(
                &rgba_clone, w, h, strength,
            );
            let _ = tx_clone.send(Action::ApplyGridRemoval(Arc::new(cleaned), generation));
        });
    }
}

/// Re-trigger mask detection for the currently active mask after undo/redo.
fn trigger_mask_redetection(state: &mut AppState) {
    let mask = if state.mode == AppMode::AxisMask {
        &mut state.axis_mask
    } else if state.mode == AppMode::DataMask {
        &mut state.data_mask
    } else {
        return;
    };

    if mask.has_any_mask() {
        let rgba_src = active_rgba_from_parts(
            &state.grid_removal,
            &state.decoded_rgba,
        );
        if let (Some(rgba_arc), Some(tx)) = (rgba_src, &state.mask_tx) {
            mask.compute_generation += 1;
            mask.is_computing = true;

            let w = mask.width;
            let h = mask.height;
            let bg = mask.bg_color.unwrap_or([255, 255, 255]);
            let buffer_clone = mask.buffer.clone();
            let rgba_clone = std::sync::Arc::clone(rgba_arc);
            let mode_clone = mask.mask_mode;
            let color_tolerance = mask.color_tolerance;
            let generation = mask.compute_generation;
            let tx_clone = tx.clone();

            std::thread::spawn(move || {
                match mode_clone {
                    crate::state::MaskMode::AxisCalib => {
                        let result = crate::recognition::axis::analyze_mask_for_axes(
                            &rgba_clone,
                            &buffer_clone,
                            w,
                            h,
                            bg,
                        );
                        let _ = tx_clone.send(Action::ApplyAxisDetection(result, generation));
                    }
                    crate::state::MaskMode::DataRecog => {
                        let result = crate::recognition::data::analyze_mask_for_data(
                            &rgba_clone,
                            &buffer_clone,
                            w,
                            h,
                            bg,
                            color_tolerance,
                        );
                        let _ = tx_clone.send(Action::ApplyDataDetection(result, generation));
                    }
                }
            });
        }
    } else {
        mask.clear_detection_results();
        mask.is_computing = false;
    }
}

pub fn handle(state: &mut AppState, action: Action) {
    // Mark dirty for data-modifying actions (before the action is consumed)
    let is_data_modifying = matches!(
        &action,
        Action::AddCalibPoint { .. }
            | Action::AddDataPoint { .. }
            | Action::MoveSelected { .. }
            | Action::NudgeSelected { .. }
            | Action::DeleteSelectedPoints
            | Action::RemoveDataPoint(_)
            | Action::MovePointsToGroup { .. }
            | Action::DeleteGroup(_)
            | Action::UpdateGroupName(..)
            | Action::UpdateGroupColor(..)
            | Action::AddGroup
            | Action::ClearData
            | Action::ClearCalib
            | Action::UpdateCalibAxis(..)
            | Action::UpdateLogScale(..)
            | Action::LoadImage(..)
            | Action::UpdateIDECode(_)
            | Action::AddUserScript(..)
    );
    if is_data_modifying {
        state.dirty = true;
    }

    // Determine if this action should push a history state BEFORE mutating
    match action {
        Action::MovePointsToGroup { .. }
        | Action::DeleteSelectedPoints
        | Action::RemoveDataPoint(_)
        | Action::DeleteGroup(_)
        | Action::UpdateGroupName(_, _)
        | Action::UpdateGroupColor(_, _)
        | Action::AddCalibPoint { .. }
        | Action::AddDataPoint { .. }
        | Action::ClearData
        | Action::ClearCalib => {
            state.save_snapshot();
        }
        Action::SetDraggingPoint { .. } => {
            state.save_snapshot();
        }
        Action::NudgeSelected { .. } => {
            state.save_snapshot();
        }
        _ => {}
    }

    match action {
        Action::MovePointsToGroup {
            indices,
            new_group_id,
        } => {
            let mut moved_pts = Vec::new();
            for &pt_idx in &indices {
                if pt_idx < state.data_pts.len() {
                    let mut pt = state.data_pts[pt_idx].clone();
                    pt.group_id = new_group_id;
                    moved_pts.push(pt);
                }
            }

            let mut to_remove = indices.clone();
            to_remove.sort_unstable_by(|a, b| b.cmp(a));
            for &idx in &to_remove {
                if idx < state.data_pts.len() {
                    state.data_pts.remove(idx);
                }
            }

            state.data_pts.extend(moved_pts);

            state.selected_data_indices.clear();
            let new_len = state.data_pts.len();
            for i in 0..indices.len() {
                state
                    .selected_data_indices
                    .insert(new_len - indices.len() + i);
            }
        }
        Action::DeleteSelectedPoints => {
            let mut to_remove: Vec<usize> = state.selected_data_indices.iter().copied().collect();
            to_remove.sort_unstable_by(|a, b| b.cmp(a));
            for idx in to_remove {
                if idx < state.data_pts.len() {
                    state.data_pts.remove(idx);
                }
            }
            state.selected_data_indices.clear();
        }
        Action::RemoveDataPoint(idx) => {
            if idx < state.data_pts.len() {
                state.data_pts.remove(idx);
                state.selected_data_indices.remove(&idx);
            }
        }
        Action::DeleteGroup(idx) => {
            if idx < state.groups.len() {
                state.groups.remove(idx);
                if state.active_group_idx == idx {
                    state.active_group_idx = 0;
                } else if state.active_group_idx > idx {
                    state.active_group_idx -= 1;
                }

                state.data_pts.retain(|p| p.group_id != idx);

                for p in &mut state.data_pts {
                    if p.group_id > idx {
                        p.group_id -= 1;
                    }
                }
                state.selected_data_indices.clear();
            }
        }
        Action::UpdateGroupName(idx, name) => {
            if let Some(g) = state.groups.get_mut(idx) {
                g.name = name;
            }
        }
        Action::UpdateGroupColor(idx, color) => {
            if let Some(g) = state.groups.get_mut(idx) {
                g.color = color;
            }
        }
        Action::SetActiveGroup(idx) => {
            state.active_group_idx = idx;
        }
        Action::AddCalibPoint { img_x, img_y } => {
            if state.calib_pts.len() < 4 {
                state.calib_pts.push(CalibPoint {
                    px: img_x,
                    py: img_y,
                });
                state.selected_calib_idx = Some(state.calib_pts.len() - 1);
                state.selected_data_indices.clear();

                if state.calib_pts.len() == 4 {
                    state.mode = AppMode::AddData;
                }
                crate::core::recalculate_data(
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
        }
        Action::AddDataPoint { img_x, img_y } => {
            if state.groups.is_empty() {
                state.groups.push(PointGroup {
                    name: "Group 1".to_string(),
                    color: Color32::from_rgb(0xd7, 0x30, 0x27), // Palette 1
                });
                state.active_group_idx = 0;
            }
            state.data_pts.push(DataPoint {
                px: img_x,
                py: img_y,
                lx: 0.0,
                ly: 0.0,
                group_id: state.active_group_idx,
            });
            state.selected_data_indices.clear();
            state.selected_data_indices.insert(state.data_pts.len() - 1);
            state.selected_calib_idx = None;
            crate::core::recalculate_data(
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
        Action::NudgeSelected { dx, dy } => {
            if let Some(idx) = state.selected_calib_idx {
                if let Some(p) = state.calib_pts.get_mut(idx) {
                    p.px += dx;
                    p.py += dy;
                }
            } else {
                for &s_idx in &state.selected_data_indices {
                    if let Some(p) = state.data_pts.get_mut(s_idx) {
                        p.px += dx;
                        p.py += dy;
                    }
                }
            }
        }
        Action::MoveSelected { dx, dy } => {
            if let Some(idx) = state.dragging_calib_idx {
                if let Some(p) = state.calib_pts.get_mut(idx) {
                    p.px += dx;
                    p.py += dy;
                }
            } else if let Some(idx) = state.dragging_data_idx {
                if state.selected_data_indices.contains(&idx) {
                    for &s_idx in &state.selected_data_indices {
                        if let Some(p) = state.data_pts.get_mut(s_idx) {
                            p.px += dx;
                            p.py += dy;
                        }
                    }
                } else {
                    if let Some(p) = state.data_pts.get_mut(idx) {
                        p.px += dx;
                        p.py += dy;
                    }
                }
            }
        }
        Action::RecalculateData => {
            crate::core::recalculate_data(
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
        Action::SelectCalibPoint(idx) => {
            state.selected_calib_idx = Some(idx);
            state.selected_data_indices.clear();
        }
        Action::SelectPoints(indices, is_multi) => {
            state.selected_calib_idx = None;
            let should_toggle = is_multi && indices.len() == 1;
            if !is_multi {
                state.selected_data_indices.clear();
            }
            for idx in indices {
                if should_toggle && state.selected_data_indices.contains(&idx) {
                    state.selected_data_indices.remove(&idx);
                } else {
                    state.selected_data_indices.insert(idx);
                }
            }
        }
        Action::SetDraggingPoint { is_calib, idx } => {
            if is_calib {
                state.dragging_calib_idx = idx;
            } else {
                state.dragging_data_idx = idx;
            }
        }
        Action::StopDragging => {
            state.dragging_calib_idx = None;
            state.dragging_data_idx = None;
            crate::core::recalculate_data(
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
        Action::SetHoveredCalib(idx) => {
            state.hovered_calib_idx = idx;
        }
        Action::SetHoveredData(idx) => {
            state.hovered_data_idx = idx;
        }
        Action::SetBoxStart(pos) => {
            state.box_start = pos;
        }
        Action::RequestClearData => {
            state.pending_clear_data = true;
        }
        Action::CancelClearData => {
            state.pending_clear_data = false;
        }
        Action::Undo => {
            if let Some(snapshot) = state.undo_stack.pop() {
                let redo_snap = HistorySnapshot {
                    calib_pts: state.calib_pts.clone(),
                    data_pts: state.data_pts.clone(),
                    groups: state.groups.clone(),
                    active_group_idx: state.active_group_idx,
                    axis_mask_buffer: if !state.axis_mask.buffer.is_empty() {
                        Some(state.axis_mask.buffer.clone())
                    } else {
                        None
                    },
                    data_mask_buffer: if !state.data_mask.buffer.is_empty() {
                        Some(state.data_mask.buffer.clone())
                    } else {
                        None
                    },
                };
                state.redo_stack.push(redo_snap);

                state.calib_pts = snapshot.calib_pts;
                state.data_pts = snapshot.data_pts;
                state.groups = snapshot.groups;
                state.active_group_idx = snapshot.active_group_idx;
                if let Some(buf) = snapshot.axis_mask_buffer {
                    state.axis_mask.buffer = buf;
                    state.axis_mask.texture_dirty = true;
                    state.axis_mask.clear_highlight_cache();
                }
                if let Some(buf) = snapshot.data_mask_buffer {
                    state.data_mask.buffer = buf;
                    state.data_mask.texture_dirty = true;
                    state.data_mask.clear_highlight_cache();
                }

                // Re-trigger mask detection for the restored buffer
                // (keeps old results visible until new ones arrive)
                trigger_mask_redetection(state);

                state.selected_data_indices.clear();
                crate::core::recalculate_data(
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
        }
        Action::Redo => {
            if let Some(snapshot) = state.redo_stack.pop() {
                let undo_snap = HistorySnapshot {
                    calib_pts: state.calib_pts.clone(),
                    data_pts: state.data_pts.clone(),
                    groups: state.groups.clone(),
                    active_group_idx: state.active_group_idx,
                    axis_mask_buffer: if !state.axis_mask.buffer.is_empty() {
                        Some(state.axis_mask.buffer.clone())
                    } else {
                        None
                    },
                    data_mask_buffer: if !state.data_mask.buffer.is_empty() {
                        Some(state.data_mask.buffer.clone())
                    } else {
                        None
                    },
                };
                state.undo_stack.push(undo_snap);

                state.calib_pts = snapshot.calib_pts;
                state.data_pts = snapshot.data_pts;
                state.groups = snapshot.groups;
                state.active_group_idx = snapshot.active_group_idx;
                if let Some(buf) = snapshot.axis_mask_buffer {
                    state.axis_mask.buffer = buf;
                    state.axis_mask.texture_dirty = true;
                    state.axis_mask.clear_highlight_cache();
                }
                if let Some(buf) = snapshot.data_mask_buffer {
                    state.data_mask.buffer = buf;
                    state.data_mask.texture_dirty = true;
                    state.data_mask.clear_highlight_cache();
                }

                // Re-trigger mask detection for the restored buffer
                // (keeps old results visible until new ones arrive)
                trigger_mask_redetection(state);

                state.selected_data_indices.clear();
                crate::core::recalculate_data(
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
        }
        Action::ToggleIDE => {
            state.ide.is_open = !state.ide.is_open;
        }
        Action::ToggleHelp => {
            state.ide.show_help = !state.ide.show_help;
        }
        Action::UpdateIDECode(code) => {
            state.ide.code = code;
        }
        Action::RunScript(script) => {
            let result = crate::script::run_script(state, &script);
            state.ide.output = result.output;
            state.ide.workspace_vars = result.workspace;
        }
        Action::LoadPresetScript(code) => {
            state.ide.code = code;
        }
        Action::AddUserScript(name, code) => {
            state.ide.user_scripts.push((name, code));
        }
        Action::OpenInspector(group_name) => {
            state.ide.open_inspectors.insert(group_name);
        }
        Action::CloseInspector(group_name) => {
            state.ide.open_inspectors.remove(&group_name);
        }
        Action::ClearData => {
            state.reset_extraction_data();
        }
        Action::ClearCalib => {
            state.reset_calibration_data();
            crate::core::recalculate_data(
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
        Action::UpdateCalibAxis(axis, val) => {
            match axis.as_str() {
                "x1" => state.x1_val = val,
                "x2" => state.x2_val = val,
                "y1" => state.y1_val = val,
                "y2" => state.y2_val = val,
                _ => {}
            }
            crate::core::recalculate_data(
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
        Action::UpdateLogScale(log_x, log_y) => {
            state.log_x = log_x;
            state.log_y = log_y;
            crate::core::recalculate_data(
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
        Action::AddGroup => {
            let new_idx = state.groups.len();
            let palette = [
                Color32::from_rgb(0xe4, 0x1a, 0x1c),
                Color32::from_rgb(0x37, 0x7e, 0xb8),
                Color32::from_rgb(0x4d, 0xaf, 0x4a),
                Color32::from_rgb(0x98, 0x4e, 0xa3),
                Color32::from_rgb(0xff, 0x7f, 0x00),
            ];
            let col = palette[new_idx % palette.len()];
            state.groups.push(PointGroup {
                name: format!("Group {}", new_idx + 1),
                color: col,
            });
            state.active_group_idx = new_idx;
        }
        Action::SetMode(mode) => {
            if state.mode == AppMode::AxisMask && mode != AppMode::AxisMask {
                state.axis_mask.active = false;
                state.axis_mask.painting = false;
                state.axis_mask.last_paint_pos = None;
                state.axis_mask.drag_origin = None;
                state.axis_mask.constrained_axis = None;
            } else if mode == AppMode::AxisMask {
                state.axis_mask.active = true;
            }
            if state.mode == AppMode::DataMask && mode != AppMode::DataMask {
                state.data_mask.active = false;
                state.data_mask.painting = false;
                state.data_mask.last_paint_pos = None;
                state.data_mask.drag_origin = None;
                state.data_mask.constrained_axis = None;
            } else if mode == AppMode::DataMask {
                state.data_mask.active = true;
            }
            // GridRemoval mode is exclusive with mask modes
            if mode == AppMode::GridRemoval {
                state.axis_mask.active = false;
                state.data_mask.active = false;
            }
            state.mode = mode;
        }
        Action::LoadImage(path, tex, size) => {
            let bytes = if path.to_string_lossy() != "Clipboard" {
                std::fs::read(&path).ok()
            } else {
                None
            };

            // Decode image to RGBA for mask analysis
            let decoded_rgba = bytes.as_ref().and_then(|b| {
                image::load_from_memory(b)
                    .ok()
                    .map(|img| std::sync::Arc::new(img.to_rgba8().into_raw()))
            });

            let mut new_state = AppState::default();

            // set up the mpsc channels
            let (tx, rx) = std::sync::mpsc::channel();
            new_state.mask_tx = Some(tx);
            new_state.mask_rx = Some(rx);

            if let Some(ref rgba) = decoded_rgba {
                let bg_col =
                    crate::recognition::detect_background_color(rgba, size.x as u32, size.y as u32);
                new_state.axis_mask.bg_color = Some(bg_col);
                new_state.data_mask.bg_color = Some(bg_col);
            }
            new_state.raw_image_bytes = bytes;
            new_state.decoded_rgba = decoded_rgba;
            new_state.image_path = Some(path);
            new_state.texture = Some(tex);
            new_state.img_size = size;

            *state = new_state;
        }
        Action::LoadClipboardImage(tex, size, bytes, w, h) => {
            let mut new_state = AppState::default();
            // set up the mpsc channels
            let (tx, rx) = std::sync::mpsc::channel();
            new_state.mask_tx = Some(tx);
            new_state.mask_rx = Some(rx);

            // Store decoded RGBA directly from clipboard data
            new_state.decoded_rgba = Some(std::sync::Arc::new(bytes.clone()));
            let bg_col = crate::recognition::detect_background_color(&bytes, w, h);
            new_state.axis_mask.bg_color = Some(bg_col);
            new_state.data_mask.bg_color = Some(bg_col);
            new_state.clipboard_rgba = Some((bytes, w, h));
            new_state.image_path = Some(std::path::PathBuf::from("Clipboard"));
            new_state.texture = Some(tex);
            new_state.img_size = size;

            *state = new_state;
        }
        Action::SetPendingImage(path, tex, size) => {
            state.pending_image = Some((path, tex, size));
        }
        Action::CancelPendingImage => {
            state.pending_image = None;
        }
        Action::ClearSelection => {
            state.selected_calib_idx = None;
            state.selected_data_indices.clear();
            state.dragging_calib_idx = None;
            state.dragging_data_idx = None;
        }
        Action::RequestCenter => {
            state.center_requested = true;
        }
        Action::CenterCanvas(canvas_rect) => {
            state.center_requested = false;
            if state.img_size.x > 0.0 && state.img_size.y > 0.0 {
                let scale_x = canvas_rect.width() / state.img_size.x;
                let scale_y = canvas_rect.height() / state.img_size.y;
                state.zoom = scale_x.min(scale_y) * 0.95; // 5% padding
                let scaled_size = state.img_size * state.zoom;
                state.pan = (canvas_rect.size() - scaled_size) / 2.0;
            } else {
                state.pan = eframe::egui::Vec2::ZERO;
                state.zoom = 1.0;
            }
        }
        Action::SetPanZoom { pan, zoom } => {
            state.pan = pan;
            state.zoom = zoom;
        }
        Action::RequestExportCsv => {
            crate::ui::panel::export_csv(state);
        }
        Action::SaveProject | Action::SaveProjectAs | Action::OpenProject | Action::NewProject => {}

        // ── Mask actions ──────────────────────────────────────────────
        Action::MaskToggle => {
            let was_active = state.data_mask.active;
            if was_active {
                state.data_mask.active = false;
                if state.mode == AppMode::DataMask {
                    state.mode = AppMode::Select;
                }
            } else {
                state.data_mask.active = true;
                state.data_mask.mask_mode = crate::state::MaskMode::DataRecog;
                state.mode = AppMode::DataMask;
                if state.texture.is_some() {
                    state
                        .data_mask
                        .ensure_buffer(state.img_size.x as u32, state.img_size.y as u32);
                }

                // Keep AxisMask disabled when enabling DataMask
                state.axis_mask.active = false;
            }
        }
        Action::MaskToggleForAxis => {
            let was_active = state.axis_mask.active;
            if was_active {
                state.axis_mask.active = false;
                if state.mode == AppMode::AxisMask {
                    state.mode = AppMode::Select;
                }
            } else {
                state.axis_mask.active = true;
                state.axis_mask.mask_mode = crate::state::MaskMode::AxisCalib;
                state.mode = AppMode::AxisMask;
                if state.texture.is_some() {
                    state
                        .axis_mask
                        .ensure_buffer(state.img_size.x as u32, state.img_size.y as u32);
                }

                // Keep DataMask disabled when enabling AxisMask
                state.data_mask.active = false;
            }
        }
        Action::MaskFinishCalib => {
            if state.mode == AppMode::AxisMask {
                state.axis_mask.active = false;
                state.mode = AppMode::Select;
            } else if state.mode == AppMode::DataMask {
                state.data_mask.active = false;
                state.mode = AppMode::Select;
            }
        }
        Action::MaskSetTool(tool) => {
            if state.mode == AppMode::AxisMask {
                state.axis_mask.tool = tool;
            } else {
                state.data_mask.tool = tool;
            }
        }
        Action::MaskSetBrushSize(size) => {
            if state.mode == AppMode::AxisMask {
                state.axis_mask.brush_size = size;
            } else {
                state.data_mask.brush_size = size;
            }
        }
        Action::MaskToggleVisibility => {
            if state.mode == AppMode::AxisMask {
                state.axis_mask.visible = !state.axis_mask.visible;
            } else {
                state.data_mask.visible = !state.data_mask.visible;
            }
        }
        Action::MaskClear => {
            let has_mask = if state.mode == AppMode::AxisMask {
                state.axis_mask.has_any_mask()
            } else {
                state.data_mask.has_any_mask()
            };
            if has_mask {
                state.save_snapshot();
            }
            let mask = if state.mode == AppMode::AxisMask {
                &mut state.axis_mask
            } else {
                &mut state.data_mask
            };
            mask.buffer.fill(false);
            mask.clear_detection_results();
            mask.texture_dirty = true;
            mask.last_stroke_end = None;
        }
        Action::MaskPaintStart => {
            state.save_snapshot();
            let mask = if state.mode == AppMode::AxisMask {
                &mut state.axis_mask
            } else {
                &mut state.data_mask
            };
            mask.painting = true;
            mask.last_paint_pos = None;
            mask.drag_origin = None;
            mask.constrained_axis = None;
            // Snapshot so we can diff new pixels during this stroke
            mask.stroke_snapshot = mask.buffer.clone();
        }
        Action::MaskPaintStroke { x, y } => {
            let mask = if state.mode == AppMode::AxisMask {
                &mut state.axis_mask
            } else {
                &mut state.data_mask
            };
            if mask.width == 0 || mask.height == 0 {
                return;
            }
            let value = mask.tool == crate::state::MaskTool::Pen;
            let radius = mask.brush_size;

            if mask.drag_origin.is_none() {
                mask.drag_origin = Some((x, y));
            }
            if let Some((lx, ly)) = mask.last_paint_pos {
                // Interpolate from last position to current
                mask.paint_line(lx, ly, x, y, radius, value);
            } else {
                // First point of stroke
                mask.paint_circle(x, y, radius, value);
            }
            mask.last_paint_pos = Some((x, y));
        }
        Action::MaskPaintEnd(ctx) => {
            let AppState {
                mode,
                decoded_rgba,
                axis_mask,
                data_mask,
                grid_removal,
                mask_tx,
                ..
            } = state;
            let mask = if *mode == AppMode::AxisMask {
                axis_mask
            } else {
                data_mask
            };
            mask.last_stroke_end = mask.last_paint_pos;
            mask.painting = false;
            mask.last_paint_pos = None;
            mask.drag_origin = None;
            mask.constrained_axis = None;
            mask.texture_dirty = true;
            mask.stroke_snapshot = Vec::new();

            if mask.has_any_mask() {
                let rgba_src = active_rgba_from_parts(grid_removal, decoded_rgba);
                if let (Some(rgba_arc), Some(tx)) = (rgba_src, mask_tx) {
                    mask.compute_generation += 1;
                    mask.is_computing = true;

                    let w = mask.width;
                    let h = mask.height;
                    let bg = mask.bg_color.unwrap_or([255, 255, 255]);
                    let buffer_clone = mask.buffer.clone();
                    let rgba_clone = std::sync::Arc::clone(rgba_arc);
                    let mode_clone = mask.mask_mode;
                    let color_tolerance = mask.color_tolerance;
                    let generation = mask.compute_generation;
                    let tx_clone = tx.clone();

                    std::thread::spawn(move || {
                        match mode_clone {
                            crate::state::MaskMode::AxisCalib => {
                                let result = crate::recognition::axis::analyze_mask_for_axes(
                                    &rgba_clone,
                                    &buffer_clone,
                                    w,
                                    h,
                                    bg,
                                );
                                let _ =
                                    tx_clone.send(Action::ApplyAxisDetection(result, generation));
                            }
                            crate::state::MaskMode::DataRecog => {
                                let result = crate::recognition::data::analyze_mask_for_data(
                                    &rgba_clone,
                                    &buffer_clone,
                                    w,
                                    h,
                                    bg,
                                    color_tolerance,
                                );
                                let _ =
                                    tx_clone.send(Action::ApplyDataDetection(result, generation));
                            }
                        }
                        ctx.request_repaint();
                    });
                }
            } else {
                mask.clear_detection_results();
                mask.is_computing = false;
            }
        }
        Action::MaskShiftClickLine { x, y } => {
            let mask = if state.mode == AppMode::AxisMask {
                &mut state.axis_mask
            } else {
                &mut state.data_mask
            };
            if mask.width == 0 || mask.height == 0 {
                return;
            }
            let value = mask.tool == crate::state::MaskTool::Pen;
            let radius = mask.brush_size;

            if let Some((lx, ly)) = mask.last_stroke_end {
                mask.paint_line(lx, ly, x, y, radius, value);
            } else {
                mask.paint_circle(x, y, radius, value);
            }
            mask.last_paint_pos = Some((x, y));
        }
        Action::MaskSetConstrainedAxis(axis) => {
            let mask = if state.mode == AppMode::AxisMask {
                &mut state.axis_mask
            } else {
                &mut state.data_mask
            };
            mask.constrained_axis = axis;
        }
        Action::ApplyAxisDetection(result, gen) => {
            if state.axis_mask.compute_generation == gen {
                state.axis_mask.set_axis_result(result);
                state.axis_mask.is_computing = false;
            }
        }
        Action::ApplyDataDetection(result, gen) => {
            if state.data_mask.compute_generation == gen {
                state.data_mask.set_data_result(result);
                state.data_mask.is_computing = false;
            }
        }
        Action::MaskSetColorTolerance(tol) => {
            let AppState {
                mode,
                decoded_rgba,
                axis_mask,
                data_mask,
                grid_removal,
                mask_tx,
                ..
            } = state;
            let mask = if *mode == AppMode::AxisMask {
                axis_mask
            } else {
                data_mask
            };
            mask.color_tolerance = tol;
            if mask.has_any_mask() {
                let rgba_src = active_rgba_from_parts(grid_removal, decoded_rgba);
                if let (Some(rgba_arc), Some(tx)) = (rgba_src, mask_tx) {
                    mask.compute_generation += 1;
                    mask.is_computing = true;

                    let w = mask.width;
                    let h = mask.height;
                    let bg = mask.bg_color.unwrap_or([255, 255, 255]);
                    let buffer_clone = mask.buffer.clone();
                    let rgba_clone = std::sync::Arc::clone(rgba_arc);
                    let generation = mask.compute_generation;
                    let tx_clone = tx.clone();

                    std::thread::spawn(move || {
                        let result = crate::recognition::data::analyze_mask_for_data(
                            &rgba_clone,
                            &buffer_clone,
                            w,
                            h,
                            bg,
                            tol,
                        );
                        let _ = tx_clone.send(Action::ApplyDataDetection(result, generation));
                        // Since we don't have egui context here, we don't request repaint
                        // It will repaint on mouse interactions anyway
                    });
                }
            }
        }

        // ── Mask Axis Detection ──
        Action::MaskSetAxisHighlight(hl) => {
            state.axis_mask.highlight_axis = hl;
        }
        Action::MaskApplyAxis(axis) => {
            // Clone the endpoints out before mutating state
            let endpoints = state
                .axis_mask
                .axis_result
                .as_ref()
                .and_then(|result| match axis {
                    crate::state::AxisHighlight::X => result.x_axis,
                    crate::state::AxisHighlight::Y => result.y_axis,
                });

            if let Some((start, end)) = endpoints {
                state.save_snapshot();
                match axis {
                    crate::state::AxisHighlight::X => {
                        let x1 = crate::core::CalibPoint {
                            px: start.0,
                            py: start.1,
                        };
                        let x2 = crate::core::CalibPoint {
                            px: end.0,
                            py: end.1,
                        };
                        while state.calib_pts.len() < 2 {
                            state
                                .calib_pts
                                .push(crate::core::CalibPoint { px: 0.0, py: 0.0 });
                        }
                        state.calib_pts[0] = x1;
                        state.calib_pts[1] = x2;
                    }
                    crate::state::AxisHighlight::Y => {
                        let y1 = crate::core::CalibPoint {
                            px: start.0,
                            py: start.1,
                        };
                        let y2 = crate::core::CalibPoint {
                            px: end.0,
                            py: end.1,
                        };
                        while state.calib_pts.len() < 4 {
                            state
                                .calib_pts
                                .push(crate::core::CalibPoint { px: 0.0, py: 0.0 });
                        }
                        state.calib_pts[2] = y1;
                        state.calib_pts[3] = y2;
                    }
                }
                crate::core::recalculate_data(
                    &state.calib_pts,
                    &mut state.data_pts,
                    &state.x1_val,
                    &state.x2_val,
                    &state.y1_val,
                    &state.y2_val,
                    state.log_x,
                    state.log_y,
                );
                state.dirty = true;
            }
        }

        // ── Mask Data Recognition ──
        Action::MaskSetDataHighlight(idx) => {
            state.data_mask.highlight_data_idx = idx;
        }
        Action::MaskSetDataMode(idx, mode) => {
            if let Some(ref mut result) = state.data_mask.data_result {
                if let Some(group) = result.groups.get_mut(idx) {
                    group.curve_mode = mode;
                    group.sampled_points = crate::recognition::data::sample_points_for_mode(
                        mode,
                        &group.pixel_coords,
                        group.point_count,
                        state.data_mask.width,
                    );
                }
            }
        }
        Action::MaskSetDataPoints(idx, count) => {
            if let Some(ref mut result) = state.data_mask.data_result {
                if let Some(group) = result.groups.get_mut(idx) {
                    group.point_count = count;
                    group.sampled_points = crate::recognition::data::sample_points_for_mode(
                        group.curve_mode,
                        &group.pixel_coords,
                        count,
                        state.data_mask.width,
                    );
                }
            }
        }
        // ── Grid Removal ──
        Action::GridRemovalToggle => {
            if state.mode == AppMode::GridRemoval {
                // Leave GridRemoval mode but keep enabled
                state.mode = AppMode::Select;
            } else {
                // Enter GridRemoval mode
                state.grid_removal.enabled = true;
                state.mode = AppMode::GridRemoval;
                // Deactivate mask modes
                state.axis_mask.active = false;
                state.data_mask.active = false;
                // Trigger computation if we don't have a cleaned image yet
                if state.grid_removal.cleaned_rgba.is_none() && state.decoded_rgba.is_some() {
                    trigger_grid_removal(state);
                }
            }
        }
        Action::GridRemovalSetStrength(s) => {
            state.grid_removal.strength = s;
            state.grid_removal.pending_strength = Some(s);
            state.grid_removal.pending_since = Some(std::time::Instant::now());
        }
        Action::GridRemovalDisable => {
            state.grid_removal.enabled = false;
            state.grid_removal.cleaned_rgba = None;
            state.grid_removal.cleaned_texture = None;
            if state.mode == AppMode::GridRemoval {
                state.mode = AppMode::Select;
            }
        }
        Action::ApplyGridRemoval(cleaned, gen) => {
            if state.grid_removal.compute_generation == gen {
                // Build texture from cleaned RGBA
                state.grid_removal.cleaned_rgba = Some(cleaned);
                // Mark texture as needing rebuild (done in UI draw)
                state.grid_removal.cleaned_texture = None;
                state.grid_removal.is_computing = false;
            }
        }

        Action::MaskAddData(idx) => {
            // Clone both the color and sampled points before mutating state
            let group_data = state.data_mask.data_result.as_ref().and_then(|result| {
                result
                    .groups
                    .get(idx)
                    .map(|group| (group.color, group.sampled_points.clone()))
            });

            if let Some((color, pts)) = group_data {
                if !pts.is_empty() {
                    state.save_snapshot();
                    // Create a new group with the detected color
                    let new_group_idx = state.groups.len();
                    let group_name = format!("Group {}", new_group_idx + 1);
                    state.groups.push(PointGroup {
                        name: group_name,
                        color: Color32::from_rgb(color[0], color[1], color[2]),
                    });
                    state.active_group_idx = new_group_idx;

                    for (px, py) in pts {
                        state.data_pts.push(crate::core::DataPoint {
                            px,
                            py,
                            lx: 0.0,
                            ly: 0.0,
                            group_id: new_group_idx,
                        });
                    }
                    crate::core::recalculate_data(
                        &state.calib_pts,
                        &mut state.data_pts,
                        &state.x1_val,
                        &state.x2_val,
                        &state.y1_val,
                        &state.y2_val,
                        state.log_x,
                        state.log_y,
                    );
                    state.dirty = true;
                }
            }
        }
    }
}
