use crate::core::{CalibPoint, DataPoint};
use crate::script::WorkspaceVar;
use eframe::egui::{Color32, TextureHandle, Vec2};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

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
pub enum ConstrainedAxis {
    Horizontal,
    Vertical,
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

#[derive(Clone, Copy, Debug)]
pub struct PixelBounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl PixelBounds {
    pub fn width(self) -> usize {
        (self.max_x - self.min_x + 1) as usize
    }

    pub fn height(self) -> usize {
        (self.max_y - self.min_y + 1) as usize
    }
}

#[derive(Clone)]
pub struct CachedHighlight {
    pub bounds: PixelBounds,
    pub contours: Vec<Vec<(u32, u32)>>,
    pub texture: TextureHandle,
}

#[derive(Clone, Default)]
pub struct MaskHighlightCache {
    pub axis_x: Option<CachedHighlight>,
    pub axis_y: Option<CachedHighlight>,
    pub data_groups: Vec<Option<CachedHighlight>>,
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
    /// Endpoint of the last completed stroke (for Shift+Click line drawing)
    pub last_stroke_end: Option<(f32, f32)>,
    /// Drag start point in image coords (for Shift+Drag axis constraint)
    pub drag_origin: Option<(f32, f32)>,
    /// Locked axis during Shift+Drag
    pub constrained_axis: Option<ConstrainedAxis>,

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
    /// Cached region textures + contours for hover highlights.
    pub highlight_cache: MaskHighlightCache,

    /// Cached GPU texture for mask overlay (avoids moiré from per-pixel rects)
    pub mask_texture: Option<eframe::egui::TextureHandle>,
    /// Whether the texture needs rebuilding from the buffer
    pub texture_dirty: bool,

    /// Snapshot of the mask buffer at stroke start, used to detect which pixels
    /// are new during the current stroke (so only new pixels need rect rendering).
    pub stroke_snapshot: Vec<bool>,

    /// Is a background thread currently computing the mask result?
    pub is_computing: bool,
    /// Generation counter to drop stale background thread results
    pub compute_generation: u64,
    /// Grid removal generation when the current results were computed
    pub result_grid_gen: u64,
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
            last_stroke_end: None,
            drag_origin: None,
            constrained_axis: None,
            bg_color: None,
            axis_result: None,
            data_result: None,
            color_tolerance: 60.0,
            highlight_axis: None,
            highlight_data_idx: None,
            highlight_cache: MaskHighlightCache::default(),
            mask_texture: None,
            texture_dirty: false,
            stroke_snapshot: Vec::new(),
            is_computing: false,
            compute_generation: 0,
            result_grid_gen: 0,
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
            self.texture_dirty = true;
            self.mask_texture = None;
            self.clear_highlight_cache();
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
        // NOTE: texture_dirty is NOT set here for performance.
        // It is set when painting ends (MaskPaintEnd action) so the texture
        // is rebuilt only once, not every frame during painting.
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

    pub fn clear_highlight_cache(&mut self) {
        self.highlight_axis = None;
        self.highlight_data_idx = None;
        self.highlight_cache = MaskHighlightCache::default();
    }

    pub fn clear_detection_results(&mut self) {
        self.axis_result = None;
        self.data_result = None;
        self.clear_highlight_cache();
    }

    pub fn set_axis_result(&mut self, result: AxisDetectionResult) {
        self.axis_result = Some(result);
        self.data_result = None;
        self.highlight_data_idx = None;
        self.highlight_cache = MaskHighlightCache::default();
    }

    pub fn set_data_result(&mut self, result: DataDetectionResult) {
        self.highlight_axis = None;
        self.highlight_data_idx = None;
        self.highlight_cache = MaskHighlightCache {
            axis_x: None,
            axis_y: None,
            data_groups: vec![None; result.groups.len()],
        };
        self.data_result = Some(result);
        self.axis_result = None;
    }

    /// Rebuild the GPU texture from the bool buffer if dirty.
    /// Call this once per frame before drawing.
    pub fn rebuild_texture_if_dirty(&mut self, ctx: &eframe::egui::Context, name: &str) {
        if !self.texture_dirty || self.buffer.is_empty() {
            return;
        }
        self.texture_dirty = false;

        let w = self.width as usize;
        let h = self.height as usize;

        // Build RGBA pixels: masked pixels get semi-transparent red-orange
        let mut pixels = vec![0u8; w * h * 4];
        for i in 0..self.buffer.len() {
            if self.buffer[i] {
                let off = i * 4;
                pixels[off] = 220;
                pixels[off + 1] = 80;
                pixels[off + 2] = 40;
                pixels[off + 3] = 90;
            }
            // else: fully transparent (already zeroed)
        }

        let color_image = eframe::egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
        let handle = ctx.load_texture(name, color_image, eframe::egui::TextureOptions::LINEAR);
        self.mask_texture = Some(handle);
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

/// Run-length encoded mask buffer for compact serialization.
/// Stores alternating runs of `false` and `true` values, starting with `false`.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SerializableMask {
    pub width: u32,
    pub height: u32,
    /// RLE counts: alternating runs starting with `false`.
    pub rle: Vec<u32>,
    pub color_tolerance: f32,
}

impl SerializableMask {
    pub fn from_mask(mask: &MaskState) -> Option<Self> {
        if mask.buffer.is_empty() || !mask.has_any_mask() {
            return None;
        }
        let mut rle = Vec::new();
        let mut current = false; // always start counting `false`
        let mut count: u32 = 0;
        for &val in &mask.buffer {
            if val == current {
                count += 1;
            } else {
                rle.push(count);
                current = val;
                count = 1;
            }
        }
        rle.push(count);
        Some(Self {
            width: mask.width,
            height: mask.height,
            rle,
            color_tolerance: mask.color_tolerance,
        })
    }

    pub fn to_buffer(&self) -> Vec<bool> {
        let total = (self.width as usize) * (self.height as usize);
        let mut buf = Vec::with_capacity(total);
        let mut current = false;
        for &count in &self.rle {
            for _ in 0..count {
                buf.push(current);
            }
            current = !current;
        }
        buf.truncate(total);
        buf
    }
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
    pub axis_mask: Option<SerializableMask>,
    pub data_mask: Option<SerializableMask>,
    pub grid_removal_enabled: bool,
    pub grid_removal_strength: f32,
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
    /// Axis mask buffer snapshot for unified undo/redo
    pub axis_mask_buffer: Option<Vec<bool>>,
    /// Data mask buffer snapshot for unified undo/redo
    pub data_mask_buffer: Option<Vec<bool>>,
}

// ── Grid Removal state ─────────────────────────────────────────────────
#[derive(Clone)]
pub struct GridRemovalState {
    pub enabled: bool,
    pub strength: f32,
    pub cleaned_rgba: Option<Arc<Vec<u8>>>,
    pub cleaned_texture: Option<TextureHandle>,
    pub is_computing: bool,
    pub compute_generation: u64,
    pub pending_strength: Option<f32>,
    pub pending_since: Option<std::time::Instant>,
}

impl Default for GridRemovalState {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: 0.5,
            cleaned_rgba: None,
            cleaned_texture: None,
            is_computing: false,
            compute_generation: 0,
            pending_strength: None,
            pending_since: None,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AppMode {
    Select,
    AddCalib,
    AddData,
    Delete,
    Pan,
    AxisMask,
    DataMask,
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
    pub lang: crate::i18n::Lang,
    pub mode: AppMode,

    // Image loading
    pub image_path: Option<PathBuf>,
    pub texture: Option<TextureHandle>,
    pub img_size: Vec2,
    pub raw_image_bytes: Option<Vec<u8>>,
    pub clipboard_rgba: Option<(Vec<u8>, u32, u32)>, // For lazily encoding pasted images
    pub pending_image: Option<(PathBuf, TextureHandle, Vec2)>,
    /// Decoded RGBA pixel data (w * h * 4 bytes) for mask analysis
    pub decoded_rgba: Option<std::sync::Arc<Vec<u8>>>,

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
    pub axis_mask: MaskState,
    pub data_mask: MaskState,

    // Grid Removal State
    pub grid_removal: GridRemovalState,

    // Thread communication channel for async background detection
    pub mask_rx: Option<std::sync::mpsc::Receiver<crate::action::Action>>,
    pub mask_tx: Option<std::sync::mpsc::Sender<crate::action::Action>>,
}

/// An open inspector window — either a top-level workspace variable or a
/// drill-down into a nested Array/Map value.
#[derive(Clone)]
pub struct InspectorEntry {
    /// Unique key for dedup, e.g. "data" or "data.x[0]"
    pub key: String,
    /// Window title
    pub label: String,
    /// `Some(val)` for sub-inspectors; `None` means look up from workspace_vars
    pub value: Option<rhai::Dynamic>,
}

#[derive(Clone)]
pub struct IdeState {
    pub is_open: bool,
    pub code: String,
    pub output: String,
    pub workspace_vars: Vec<WorkspaceVar>,
    pub open_inspectors: Vec<InspectorEntry>,
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
            open_inspectors: Vec::new(),
            show_help: false,
            user_scripts: Vec::new(),
            output_fraction: 0.5,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            lang: crate::i18n::Lang::default(),
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
            axis_mask: MaskState {
                mask_mode: MaskMode::AxisCalib,
                ..Default::default()
            },
            data_mask: MaskState {
                mask_mode: MaskMode::DataRecog,
                ..Default::default()
            },
            grid_removal: GridRemovalState::default(),
            mask_rx: None,
            mask_tx: None,
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
        self.x1_val = "".to_string();
        self.x2_val = "".to_string();
        self.y1_val = "".to_string();
        self.y2_val = "".to_string();
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
            axis_mask_buffer: if !self.axis_mask.buffer.is_empty() {
                Some(self.axis_mask.buffer.clone())
            } else {
                None
            },
            data_mask_buffer: if !self.data_mask.buffer.is_empty() {
                Some(self.data_mask.buffer.clone())
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
