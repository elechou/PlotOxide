use crate::core::{CalibPoint, DataPoint};
use crate::script::WorkspaceVar;
use eframe::egui::{Color32, TextureHandle, Vec2};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Mask types ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MaskTool {
    Pen,
    Eraser,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MaskMode {
    AxisCalib,
    DataRecog,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AxisHighlight {
    X,
    Y,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DataCurveMode {
    Continuous,
    Scatter,
}

#[derive(Clone, Debug)]
pub struct AxisDetectionResult {
    /// Detected X-axis: (start_img_x, start_img_y) to (end_img_x, end_img_y)
    pub x_axis: Option<((f32, f32), (f32, f32))>,
    /// Detected Y-axis: (start_img_x, start_img_y) to (end_img_x, end_img_y)
    pub y_axis: Option<((f32, f32), (f32, f32))>,
    /// Pixel coords of detected X-axis line (for highlighting)
    pub x_axis_pixels: Vec<(u32, u32)>,
    /// Pixel coords of detected Y-axis line (for highlighting)
    pub y_axis_pixels: Vec<(u32, u32)>,
    /// All detected tick positions on X-axis
    pub x_ticks: Vec<(f32, f32)>,
    /// All detected tick positions on Y-axis
    pub y_ticks: Vec<(f32, f32)>,
}

#[derive(Clone, Debug)]
pub struct DetectedColorGroup {
    pub color: [u8; 3],
    pub pixel_coords: Vec<(u32, u32)>,
    pub curve_mode: DataCurveMode,
    pub point_count: usize,
    /// Sampled points in image coordinates
    pub sampled_points: Vec<(f32, f32)>,
}

#[derive(Clone, Debug)]
pub struct DataDetectionResult {
    pub groups: Vec<DetectedColorGroup>,
}

#[derive(Clone)]
pub struct MaskState {
    /// Mask buffer: true = masked pixel (image coordinates)
    pub buffer: Vec<bool>,
    pub width: u32,
    pub height: u32,

    /// Is mask mode active (sub-toolbar visible)
    pub active: bool,
    /// Which mode: axis calibration or data recognition
    pub mask_mode: MaskMode,
    /// Current tool
    pub tool: MaskTool,
    /// Brush radius in image pixels
    pub brush_size: f32,
    /// Show/hide overlay
    pub visible: bool,
    /// Currently painting (mouse held down)
    pub painting: bool,
    /// Previous mouse position for interpolation
    pub last_paint_pos: Option<(f32, f32)>,

    /// Detected background color of the image
    pub bg_color: Option<[u8; 3]>,
    /// Cached axis detection result
    pub axis_result: Option<AxisDetectionResult>,
    /// Cached data detection result
    pub data_result: Option<DataDetectionResult>,

    /// User-adjustable color tolerance for data clustering (default 30.0)
    pub color_tolerance: f32,

    /// Which axis to highlight on hover
    pub highlight_axis: Option<AxisHighlight>,
    /// Which data color group to highlight on hover
    pub highlight_data_idx: Option<usize>,
}

impl Default for MaskState {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            width: 0,
            height: 0,
            active: false,
            mask_mode: MaskMode::AxisCalib,
            tool: MaskTool::Pen,
            brush_size: 20.0,
            visible: true,
            painting: false,
            last_paint_pos: None,
            bg_color: None,
            axis_result: None,
            data_result: None,
            color_tolerance: 30.0,
            highlight_axis: None,
            highlight_data_idx: None,
        }
    }
}

impl MaskState {
    /// Initialize or resize the mask buffer to match the image dimensions.
    pub fn ensure_buffer(&mut self, w: u32, h: u32) {
        if self.width != w || self.height != h {
            self.width = w;
            self.height = h;
            self.buffer = vec![false; (w as usize) * (h as usize)];
        }
    }

    /// Paint a filled circle onto the mask buffer.
    pub fn paint_circle(&mut self, cx: f32, cy: f32, radius: f32, value: bool) {
        let r = radius as i32;
        let cx_i = cx as i32;
        let cy_i = cy as i32;
        let w = self.width as i32;
        let h = self.height as i32;

        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let px = cx_i + dx;
                    let py = cy_i + dy;
                    if px >= 0 && px < w && py >= 0 && py < h {
                        self.buffer[(py as usize) * (self.width as usize) + (px as usize)] = value;
                    }
                }
            }
        }
    }

    /// Paint a line from (x0, y0) to (x1, y1) with the given brush radius.
    pub fn paint_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, radius: f32, value: bool) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        let steps = (dist / (radius * 0.3)).ceil().max(1.0) as usize;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let cx = x0 + dx * t;
            let cy = y0 + dy * t;
            self.paint_circle(cx, cy, radius, value);
        }
    }

    pub fn has_any_mask(&self) -> bool {
        self.buffer.iter().any(|&b| b)
    }
}

// ── Serializable mirror types (no egui dependency) ─────────────────────

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SerializableGroup {
    pub name: String,
    pub color: [u8; 4],
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SerializableIdeState {
    pub is_open: bool,
    pub code: String,
    pub user_scripts: Vec<(String, String)>,
}

/// The on-disk representation of a project.
/// All fields have `#[serde(default)]` so that adding new fields in the future
/// is fully backwards-compatible: old files simply get the default value.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectData {
    pub version: u32,
    pub calib_pts: Vec<CalibPoint>,
    pub data_pts: Vec<DataPoint>,
    pub groups: Vec<SerializableGroup>,
    pub active_group_idx: usize,
    pub x1_val: String,
    pub x2_val: String,
    pub y1_val: String,
    pub y2_val: String,
    pub log_x: bool,
    pub log_y: bool,
    pub ide: SerializableIdeState,
}

// ── Runtime types ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PointGroup {
    pub name: String,
    pub color: Color32,
}

#[derive(Clone)]
pub struct HistorySnapshot {
    pub calib_pts: Vec<CalibPoint>,
    pub data_pts: Vec<DataPoint>,
    pub groups: Vec<PointGroup>,
    pub active_group_idx: usize,
    /// Mask buffer snapshot for unified undo/redo
    pub mask_buffer: Option<Vec<bool>>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AppMode {
    Select,
    AddCalib,
    AddData,
    Delete,
    Pan,
    Mask,
}

/// Actions that are deferred until unsaved-changes confirmation is resolved.
pub enum PendingAction {
    NewProject,
    LoadImage(
        std::path::PathBuf,
        eframe::egui::TextureHandle,
        eframe::egui::Vec2,
    ),
    LoadClipboardImage(
        eframe::egui::TextureHandle,
        eframe::egui::Vec2,
        Vec<u8>,
        u32,
        u32,
    ),
    OpenProject(crate::state::ProjectData, Vec<u8>, std::path::PathBuf),
    CloseApp,
}

pub struct AppState {
    pub mode: AppMode,

    // Image loading
    pub image_path: Option<PathBuf>,
    pub texture: Option<TextureHandle>,
    pub img_size: Vec2,
    pub raw_image_bytes: Option<Vec<u8>>,
    pub clipboard_rgba: Option<(Vec<u8>, u32, u32)>, // For lazily encoding pasted images
    pub pending_image: Option<(PathBuf, TextureHandle, Vec2)>,
    /// Decoded RGBA pixel data (w * h * 4 bytes) for mask analysis
    pub decoded_rgba: Option<Vec<u8>>,

    // Viewport transform (Panning & Zooming)
    pub pan: Vec2,
    pub zoom: f32,

    // Points & Groups
    pub calib_pts: Vec<CalibPoint>,
    pub data_pts: Vec<DataPoint>,
    pub groups: Vec<PointGroup>,
    pub active_group_idx: usize,

    // Calibration Settings
    pub x1_val: String,
    pub x2_val: String,
    pub y1_val: String,
    pub y2_val: String,
    pub log_x: bool,
    pub log_y: bool,

    // Interaction state
    pub dragging_calib_idx: Option<usize>,
    pub dragging_data_idx: Option<usize>,
    pub selected_calib_idx: Option<usize>,
    pub selected_data_indices: std::collections::HashSet<usize>,
    pub hovered_calib_idx: Option<usize>,
    pub hovered_data_idx: Option<usize>,

    // Transient Box Select state
    pub box_start: Option<eframe::egui::Pos2>,
    pub center_requested: bool,
    pub pending_clear_data: bool,

    // UI collapse state for group data lists
    pub collapsed_groups: std::collections::HashSet<usize>,

    // Pending action requiring unsaved-changes confirmation
    pub pending_action: Option<PendingAction>,

    // Show "no image in clipboard" modal
    pub show_clipboard_empty: bool,

    // Current project file path (for Ctrl+S save-in-place)
    pub project_path: Option<PathBuf>,

    // Show About dialog
    pub show_about: bool,

    // Project state tracking
    pub dirty: bool,

    // History
    pub undo_stack: Vec<HistorySnapshot>,
    pub redo_stack: Vec<HistorySnapshot>,

    // IDE State
    pub ide: IdeState,

    // Mask State
    pub mask: MaskState,
}

#[derive(Clone)]
pub struct IdeState {
    pub is_open: bool,
    pub code: String,
    pub output: String,
    pub workspace_vars: Vec<WorkspaceVar>,
    pub open_inspectors: std::collections::HashSet<String>,
    pub show_help: bool,
    pub user_scripts: Vec<(String, String)>, // (name, code)
    pub output_fraction: f32,
}

impl Default for IdeState {
    fn default() -> Self {
        Self {
            is_open: false,
            code: String::new(),
            output: String::new(),
            workspace_vars: Vec::new(),
            open_inspectors: std::collections::HashSet::new(),
            show_help: false,
            user_scripts: Vec::new(),
            output_fraction: 0.5,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::Select,
            image_path: None,
            texture: None,
            img_size: Vec2::ZERO,
            raw_image_bytes: None,
            clipboard_rgba: None,
            pending_image: None,
            decoded_rgba: None,
            pan: Vec2::ZERO,
            zoom: 1.0,
            calib_pts: Vec::new(),
            data_pts: Vec::new(),
            groups: Vec::new(),
            active_group_idx: 0,
            x1_val: "".to_string(),
            x2_val: "".to_string(),
            y1_val: "".to_string(),
            y2_val: "".to_string(),
            log_x: false,
            log_y: false,
            dragging_calib_idx: None,
            dragging_data_idx: None,
            selected_calib_idx: None,
            selected_data_indices: std::collections::HashSet::new(),
            hovered_calib_idx: None,
            hovered_data_idx: None,
            box_start: None,
            center_requested: false,
            pending_clear_data: false,
            collapsed_groups: std::collections::HashSet::new(),
            pending_action: None,
            show_clipboard_empty: false,
            project_path: None,
            show_about: false,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            ide: IdeState::default(),
            mask: MaskState::default(),
        }
    }
}

impl AppState {
    /// Returns the project display name (filename with extension or "untitled.prdx").
    pub fn project_name(&self) -> String {
        self.project_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled.prdx".to_string())
    }

    /// Returns the window title: "PlotRedox — name" with "*" if dirty.
    pub fn window_title(&self) -> String {
        let name = self.project_name();
        if self.dirty {
            format!("PlotRedox — {}*", name)
        } else {
            format!("PlotRedox — {}", name)
        }
    }

    pub fn reset_extraction_data(&mut self) {
        // Clear all data points
        self.data_pts.clear();

        // Reset groups to empty
        self.groups = Vec::new();
        self.active_group_idx = 0;

        // Clear data interaction state
        self.selected_data_indices.clear();
        self.dragging_data_idx = None;
        self.hovered_data_idx = None;
        self.box_start = None;
        self.pending_clear_data = false;

        // Clear history
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn reset_calibration_data(&mut self) {
        self.calib_pts.clear();

        // Reset calibration axis values
        self.x1_val = "0.0".to_string();
        self.x2_val = "10.0".to_string();
        self.y1_val = "0.0".to_string();
        self.y2_val = "10.0".to_string();
        self.log_x = false;
        self.log_y = false;

        // Clear calib interaction state
        self.selected_calib_idx = None;
        self.dragging_calib_idx = None;
        self.hovered_calib_idx = None;
    }

    pub fn save_snapshot(&mut self) {
        let snapshot = HistorySnapshot {
            calib_pts: self.calib_pts.clone(),
            data_pts: self.data_pts.clone(),
            groups: self.groups.clone(),
            active_group_idx: self.active_group_idx,
            mask_buffer: if self.mask.active {
                Some(self.mask.buffer.clone())
            } else {
                None
            },
        };
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn update(&mut self, action: crate::action::Action) {
        crate::action_handler::handle(self, action);
    }
}
