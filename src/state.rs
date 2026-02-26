use std::path::PathBuf;
use eframe::egui::{Vec2, TextureHandle};
use crate::core::{CalibPoint, DataPoint};

#[derive(PartialEq, Clone, Copy)]
pub enum AppMode {
    Idle,
    AddCalib,
    AddData,
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
    
    // Points
    pub calib_pts: Vec<CalibPoint>,
    pub data_pts: Vec<DataPoint>,
    
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
    pub selected_data_idx: Option<usize>,
    pub hovered_calib_idx: Option<usize>,
    pub hovered_data_idx: Option<usize>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::Idle,
            image_path: None,
            texture: None,
            img_size: Vec2::ZERO,
            pan: Vec2::ZERO,
            zoom: 1.0,
            calib_pts: Vec::new(),
            data_pts: Vec::new(),
            x1_val: "0.0".to_string(),
            x2_val: "10.0".to_string(),
            y1_val: "0.0".to_string(),
            y2_val: "10.0".to_string(),
            log_x: false,
            log_y: false,
            dragging_calib_idx: None,
            dragging_data_idx: None,
            selected_calib_idx: None,
            selected_data_idx: None,
            hovered_calib_idx: None,
            hovered_data_idx: None,
        }
    }
}
