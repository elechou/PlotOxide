use crate::core::{CalibPoint, DataPoint};
use eframe::egui::{Color32, TextureHandle, Vec2};
use std::path::PathBuf;

#[derive(Clone)]
pub struct PointGroup {
    pub name: String,
    pub color: Color32,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AppMode {
    Select,
    AddCalib,
    AddData,
    Delete,
    Pan,
}

pub struct AppState {
    pub mode: AppMode,

    // Image loading
    pub image_path: Option<PathBuf>,
    pub texture: Option<TextureHandle>,
    pub img_size: Vec2,

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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::Select,
            image_path: None,
            texture: None,
            img_size: Vec2::ZERO,
            pan: Vec2::ZERO,
            zoom: 1.0,
            calib_pts: Vec::new(),
            data_pts: Vec::new(),
            groups: vec![PointGroup {
                name: "Group 1".to_string(),
                color: Color32::from_rgb(0xd7, 0x30, 0x27), // Palette 1
            }],
            active_group_idx: 0,
            x1_val: "0.0".to_string(),
            x2_val: "10.0".to_string(),
            y1_val: "0.0".to_string(),
            y2_val: "10.0".to_string(),
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
        }
    }
}
