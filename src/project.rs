use crate::state::{AppState, PointGroup, ProjectData, SerializableGroup, SerializableIdeState, SerializableMask};
use eframe::egui;
use rfd::FileDialog;
use std::io::{Read, Write};
use std::path::PathBuf;

// ── Save ───────────────────────────────────────────────────────────────

/// Build a `ProjectData` from the current `AppState`.
fn build_project_data(state: &AppState) -> ProjectData {
    ProjectData {
        version: 1,
        calib_pts: state.calib_pts.clone(),
        data_pts: state.data_pts.clone(),
        groups: state
            .groups
            .iter()
            .map(|g| {
                let c = g.color.to_array();
                SerializableGroup {
                    name: g.name.clone(),
                    color: c,
                }
            })
            .collect(),
        active_group_idx: state.active_group_idx,
        x1_val: state.x1_val.clone(),
        x2_val: state.x2_val.clone(),
        y1_val: state.y1_val.clone(),
        y2_val: state.y2_val.clone(),
        log_x: state.log_x,
        log_y: state.log_y,
        ide: SerializableIdeState {
            is_open: state.ide.is_open,
            code: state.ide.code.clone(),
            user_scripts: state.ide.user_scripts.clone(),
        },
        axis_mask: SerializableMask::from_mask(&state.axis_mask),
        data_mask: SerializableMask::from_mask(&state.data_mask),
        grid_removal_enabled: state.grid_removal.enabled,
        grid_removal_strength: state.grid_removal.strength,
    }
}

/// Collect the raw image bytes for saving.
/// Uses cached `raw_image_bytes` from AppState (set at load time).
fn collect_image_bytes(state: &AppState) -> Option<Vec<u8>> {
    // Use the cached bytes (set during LoadImage or clipboard paste)
    if let Some(bytes) = &state.raw_image_bytes {
        return Some(bytes.clone());
    }
    // Encode clipboard image lazily
    if let Some((rgba_bytes, w, h)) = &state.clipboard_rgba {
        let mut png_buf = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
            use image::ImageEncoder;
            let _ = encoder.write_image(rgba_bytes, *w, *h, image::ColorType::Rgba8);
        }
        return Some(png_buf);
    }
    // Fallback: try re-reading from the original file path
    if let Some(path) = &state.image_path {
        if path.to_string_lossy() != "Clipboard" {
            if let Ok(bytes) = std::fs::read(path) {
                return Some(bytes);
            }
        }
    }
    None
}

/// Save directly to the given path (Ctrl+S when project_path is set).
pub fn save_project_to_path(state: &AppState, path: &std::path::Path) {
    if let Err(e) = write_prdx(state, path) {
        eprintln!("Failed to save project: {}", e);
    }
}

/// Show a "Save As" dialog and write the `.prdx` ZIP file.
/// Returns the chosen path on success.
pub fn save_project_as(state: &AppState) -> Option<PathBuf> {
    let default_name = state
        .project_path
        .as_ref()
        .and_then(|p| p.file_stem())
        .map(|s| format!("{}.prdx", s.to_string_lossy()))
        .unwrap_or_else(|| "untitled.prdx".to_string());

    if let Some(path) = FileDialog::new()
        .set_file_name(&default_name)
        .add_filter("PlotRedox Project", &["prdx"])
        .save_file()
    {
        if let Err(e) = write_prdx(state, &path) {
            eprintln!("Failed to save project: {}", e);
            return None;
        }
        return Some(path);
    }
    None
}

/// Internal: write a `.prdx` ZIP to the given path.
fn write_prdx(state: &AppState, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Write manifest.json
    let project_data = build_project_data(state);
    let json = serde_json::to_string_pretty(&project_data)?;
    zip.start_file("manifest.json", options)?;
    zip.write_all(json.as_bytes())?;

    // Write image
    if let Some(img_bytes) = collect_image_bytes(state) {
        zip.start_file("image.png", options)?;
        zip.write_all(&img_bytes)?;
    }

    zip.finish()?;
    Ok(())
}

// ── Open ───────────────────────────────────────────────────────────────

/// Show an open dialog and read a `.prdx` file.
/// Returns (ProjectData, image_bytes, file_path) on success.
pub fn open_project() -> Option<(ProjectData, Vec<u8>, PathBuf)> {
    let path = FileDialog::new()
        .add_filter("PlotRedox Project", &["prdx"])
        .pick_file()?;
    read_prdx(&path).ok()
}

/// Open a `.prdx` file from a given path (used by drag-drop).
pub fn open_project_from_path(path: &std::path::Path) -> Option<(ProjectData, Vec<u8>, PathBuf)> {
    read_prdx(path).ok()
}

/// Internal: read a `.prdx` ZIP and return (ProjectData, image_bytes, path).
fn read_prdx(
    path: &std::path::Path,
) -> Result<(ProjectData, Vec<u8>, PathBuf), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Read manifest
    let project_data: ProjectData = {
        let mut manifest = archive.by_name("manifest.json")?;
        let mut json_str = String::new();
        manifest.read_to_string(&mut json_str)?;
        serde_json::from_str(&json_str)?
    };

    // Read image
    let image_bytes = {
        let mut img_file = archive.by_name("image.png")?;
        let mut buf = Vec::new();
        img_file.read_to_end(&mut buf)?;
        buf
    };

    Ok((project_data, image_bytes, path.to_path_buf()))
}

// ── Restore state from ProjectData ─────────────────────────────────────

/// Apply a `ProjectData` + image bytes to the `AppState`, rebuilding the texture.
pub fn apply_project(
    state: &mut AppState,
    data: ProjectData,
    image_bytes: &[u8],
    file_path: PathBuf,
    ctx: &egui::Context,
) {
    // Rebuild texture from image bytes and decode RGBA for mask analysis
    let mut img_w: u32 = 0;
    let mut img_h: u32 = 0;
    if let Ok(img) = image::load_from_memory(image_bytes) {
        let img = img.to_rgba8();
        img_w = img.width();
        img_h = img.height();
        let size = [img_w as usize, img_h as usize];
        let pixels = img.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
        let handle = ctx.load_texture("main_image", color_image, Default::default());
        state.texture = Some(handle);
        state.img_size = egui::Vec2::new(size[0] as f32, size[1] as f32);
        state.decoded_rgba = Some(std::sync::Arc::new(img.into_raw()));
    }

    state.image_path = Some(file_path);
    state.raw_image_bytes = Some(image_bytes.to_vec());

    // Set up mpsc channels for async mask detection
    let (tx, rx) = std::sync::mpsc::channel();
    state.mask_tx = Some(tx);
    state.mask_rx = Some(rx);

    // Detect background color
    if let Some(ref rgba) = state.decoded_rgba {
        let bg_col = crate::recognition::detect_background_color(rgba, img_w, img_h);
        state.axis_mask.bg_color = Some(bg_col);
        state.data_mask.bg_color = Some(bg_col);
    }

    // Core data
    state.calib_pts = data.calib_pts;
    state.data_pts = data.data_pts;
    state.groups = data
        .groups
        .into_iter()
        .map(|g| PointGroup {
            name: g.name,
            color: egui::Color32::from_rgba_unmultiplied(
                g.color[0], g.color[1], g.color[2], g.color[3],
            ),
        })
        .collect();
    // Ensure at least one group
    if state.groups.is_empty() {
        state.groups.push(PointGroup {
            name: "Group 1".to_string(),
            color: egui::Color32::from_rgb(0xd7, 0x30, 0x27),
        });
    }
    state.active_group_idx = data.active_group_idx;

    // Calibration
    state.x1_val = data.x1_val;
    state.x2_val = data.x2_val;
    state.y1_val = data.y1_val;
    state.y2_val = data.y2_val;
    state.log_x = data.log_x;
    state.log_y = data.log_y;

    // IDE
    state.ide.is_open = data.ide.is_open;
    state.ide.code = data.ide.code;
    state.ide.user_scripts = data.ide.user_scripts;

    // Reset transient state
    state.pan = egui::Vec2::ZERO;
    state.zoom = 1.0;
    state.center_requested = true;
    state.selected_calib_idx = None;
    state.selected_data_indices.clear();
    state.dragging_calib_idx = None;
    state.dragging_data_idx = None;
    state.hovered_calib_idx = None;
    state.hovered_data_idx = None;
    state.box_start = None;
    state.pending_image = None;
    state.pending_action = None;
    state.pending_clear_data = false;
    state.show_clipboard_empty = false;
    state.dirty = false;
    state.undo_stack.clear();
    state.redo_stack.clear();
    state.ide.output.clear();
    state.ide.workspace_vars.clear();
    state.ide.open_inspectors.clear();
    state.ide.show_help = false;

    // Restore grid removal settings
    state.grid_removal.enabled = data.grid_removal_enabled;
    state.grid_removal.strength = data.grid_removal_strength;
    if state.grid_removal.enabled && state.decoded_rgba.is_some() {
        crate::action_handler::trigger_grid_removal_for(state);
    }

    // Restore mask buffers and trigger re-detection
    if let Some(ref mask_data) = data.axis_mask {
        let buf = mask_data.to_buffer();
        state.axis_mask.width = mask_data.width;
        state.axis_mask.height = mask_data.height;
        state.axis_mask.color_tolerance = mask_data.color_tolerance;
        state.axis_mask.buffer = buf;
        state.axis_mask.texture_dirty = true;
        trigger_mask_detection_for(
            &mut state.axis_mask,
            &state.decoded_rgba,
            &state.mask_tx,
        );
    }
    if let Some(ref mask_data) = data.data_mask {
        let buf = mask_data.to_buffer();
        state.data_mask.width = mask_data.width;
        state.data_mask.height = mask_data.height;
        state.data_mask.color_tolerance = mask_data.color_tolerance;
        state.data_mask.buffer = buf;
        state.data_mask.texture_dirty = true;
        trigger_mask_detection_for(
            &mut state.data_mask,
            &state.decoded_rgba,
            &state.mask_tx,
        );
    }

    // Recalculate logical coordinates
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

/// Trigger background mask detection for a single mask.
fn trigger_mask_detection_for(
    mask: &mut crate::state::MaskState,
    decoded_rgba: &Option<std::sync::Arc<Vec<u8>>>,
    mask_tx: &Option<std::sync::mpsc::Sender<crate::action::Action>>,
) {
    if !mask.has_any_mask() {
        return;
    }
    if let (Some(ref rgba_arc), Some(tx)) = (decoded_rgba, mask_tx) {
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
                    let _ = tx_clone.send(crate::action::Action::ApplyAxisDetection(result, generation));
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
                    let _ = tx_clone.send(crate::action::Action::ApplyDataDetection(result, generation));
                }
            }
        });
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CalibPoint, DataPoint};

    /// Round-trip: serialize → deserialize → compare.
    #[test]
    fn project_data_round_trip() {
        let data = ProjectData {
            version: 1,
            calib_pts: vec![
                CalibPoint { px: 10.0, py: 20.0 },
                CalibPoint { px: 30.0, py: 40.0 },
            ],
            data_pts: vec![DataPoint {
                px: 50.0,
                py: 60.0,
                lx: 1.5,
                ly: 2.5,
                group_id: 0,
            }],
            groups: vec![SerializableGroup {
                name: "Test Group".to_string(),
                color: [255, 128, 0, 255],
            }],
            active_group_idx: 0,
            x1_val: "0.0".to_string(),
            x2_val: "10.0".to_string(),
            y1_val: "0.0".to_string(),
            y2_val: "10.0".to_string(),
            log_x: false,
            log_y: true,
            ide: SerializableIdeState {
                is_open: true,
                code: "print(42);".to_string(),
                user_scripts: vec![("script1".to_string(), "let x = 1;".to_string())],
            },
            axis_mask: None,
            data_mask: None,
            grid_removal_enabled: false,
            grid_removal_strength: 0.5,
        };

        let json = serde_json::to_string(&data).expect("serialize");
        let restored: ProjectData = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.version, 1);
        assert_eq!(restored.calib_pts.len(), 2);
        assert_eq!(restored.data_pts.len(), 1);
        assert_eq!(restored.groups[0].name, "Test Group");
        assert_eq!(restored.groups[0].color, [255, 128, 0, 255]);
        assert_eq!(restored.x2_val, "10.0");
        assert!(restored.log_y);
        assert!(!restored.log_x);
        assert_eq!(restored.ide.code, "print(42);");
        assert_eq!(restored.ide.user_scripts.len(), 1);
    }

    /// Forward-compatibility: a JSON with missing fields should still deserialize
    /// using default values, thanks to `#[serde(default)]`.
    #[test]
    fn project_data_forward_compat() {
        // Imagine a minimal old file with only version and calib_pts
        let json = r#"{"version": 1, "calib_pts": [{"px": 5.0, "py": 6.0}]}"#;
        let restored: ProjectData = serde_json::from_str(json).expect("deserialize old format");

        assert_eq!(restored.version, 1);
        assert_eq!(restored.calib_pts.len(), 1);
        assert!(restored.data_pts.is_empty());
        assert!(restored.groups.is_empty());
        assert_eq!(restored.x1_val, ""); // Default empty string
        assert!(!restored.log_x);
        assert!(!restored.ide.is_open);
    }

    /// Empty JSON object should deserialize to all defaults.
    #[test]
    fn project_data_empty_json() {
        let json = "{}";
        let restored: ProjectData = serde_json::from_str(json).expect("deserialize empty");
        assert_eq!(restored.version, 0);
        assert!(restored.calib_pts.is_empty());
        assert!(restored.data_pts.is_empty());
    }
}
