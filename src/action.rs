use eframe::egui::Color32;
use std::path::PathBuf;

pub enum Action {
    // Data Group / List Operations
    MovePointsToGroup {
        indices: Vec<usize>,
        new_group_id: usize,
    },
    DeleteSelectedPoints,
    DeleteGroup(usize),
    UpdateGroupName(usize, String),
    UpdateGroupColor(usize, Color32),
    SetActiveGroup(usize),
    RemoveDataPoint(usize),

    // Canvas / Viewport Operations
    AddCalibPoint {
        img_x: f32,
        img_y: f32,
    },
    AddDataPoint {
        img_x: f32,
        img_y: f32,
    },
    MoveSelected {
        dx: f32,
        dy: f32,
    },
    RecalculateData,
    SelectCalibPoint(usize),
    SelectPoints(Vec<usize>, bool /* is_multi */),
    SetDraggingPoint {
        is_calib: bool,
        idx: Option<usize>,
    },
    StopDragging,
    SetHoveredCalib(Option<usize>),
    SetHoveredData(Option<usize>),
    SetBoxStart(Option<eframe::egui::Pos2>),

    // Global App Commands
    SetMode(crate::state::AppMode),
    LoadImage(PathBuf, eframe::egui::TextureHandle, eframe::egui::Vec2),
    SetPendingImage(PathBuf, eframe::egui::TextureHandle, eframe::egui::Vec2),
    CancelPendingImage,
    Undo,
    Redo,
    ToggleIDE,
    ToggleHelp,
    UpdateIDECode(String),
    RunScript(String),
    LoadPresetScript(String),      // code to load into editor
    AddUserScript(String, String), // (name, code)
    OpenInspector(String),
    CloseInspector(String),
    ClearSelection,
    RequestCenter,
    CenterCanvas(eframe::egui::Rect),
    SetPanZoom {
        pan: eframe::egui::Vec2,
        zoom: f32,
    },
    RequestExportCsv,

    // Additional Panel Commands
    AddGroup,
    RequestClearData,
    CancelClearData,
    ClearData,
    ClearCalib,
    UpdateCalibAxis(String, String), // axis identifier, value
    UpdateLogScale(bool, bool),      // log_x, log_y
}
