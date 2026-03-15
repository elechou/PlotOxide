use crate::action::Action;
use crate::icons;
use crate::state::{AppMode, AppState};
use eframe::egui;

pub fn draw_toolbar(
    state: &AppState,
    ui: &mut egui::Ui,
    canvas_rect: egui::Rect,
    actions: &mut Vec<Action>,
) {
    let window = egui::Window::new("CAD Toolbar")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::RIGHT_TOP, [-5.0, 25.0]);

    window.show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    state.mode == AppMode::Select,
                    format!("{} Select", icons::CURSOR_DEFAULT),
                )
                .on_hover_text("Select & Drag")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Select));
                actions.push(Action::ClearSelection);
            }
            if ui
                .selectable_label(
                    state.mode == AppMode::AddData,
                    format!("{} Add Data", icons::PLUS),
                )
                .on_hover_text("Pick new points")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::AddData));
            }

            if ui
                .selectable_label(
                    state.mode == AppMode::Delete,
                    format!("{} Delete", icons::MINUS),
                )
                .on_hover_text("Click points to delete them")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Delete));
            }

            let magic_active = state.axis_mask.active
                && state.axis_mask.mask_mode == crate::state::MaskMode::AxisCalib;
            if ui
                .selectable_label(
                    magic_active,
                    format!("{} Axis Brush", icons::AXIS_BRUSH),
                )
                .on_hover_text("Auto-detect axes by painting a mask")
                .clicked()
            {
                actions.push(Action::MaskToggleForAxis);
            }

            let mask_active = state.data_mask.active
                && state.data_mask.mask_mode == crate::state::MaskMode::DataRecog;
            if ui
                .selectable_label(
                    mask_active,
                    format!("{} Data Brush", icons::DATA_BRUSH),
                )
                .on_hover_text(
                    "Auto-extract data points using color recognition via a painted mask",
                )
                .clicked()
            {
                actions.push(Action::MaskToggle);
            }

            let grid_active = state.mode == AppMode::GridRemoval;
            if ui
                .selectable_label(
                    grid_active,
                    format!("{} Grid", icons::GRID),
                )
                .on_hover_text("Remove grid lines from image using FFT filtering")
                .clicked()
            {
                actions.push(Action::GridRemovalToggle);
            }

            let is_space_pressed = ui.ctx().input(|i| i.key_down(egui::Key::Space));
            if ui
                .selectable_label(
                    state.mode == AppMode::Pan || is_space_pressed,
                    format!("{} Pan", icons::HAND),
                )
                .on_hover_text("Left-click and drag to pan canvas (or hold Space)")
                .clicked()
            {
                actions.push(Action::SetMode(AppMode::Pan));
            }
            if ui
                .button(format!("{} Center", icons::FIT_SCREEN))
                .on_hover_text("Center canvas to fit window")
                .clicked()
            {
                actions.push(Action::CenterCanvas(canvas_rect));
            }
        });
    });
}

pub fn draw_grid_removal_toolbar(
    state: &mut crate::state::AppState,
    ui: &mut egui::Ui,
    actions: &mut Vec<Action>,
) {
    if state.mode != AppMode::GridRemoval {
        return;
    }

    let window = egui::Window::new("Grid Removal")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::RIGHT_TOP, [-5.0, 60.0]);

    window.show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            if state.grid_removal.is_computing {
                ui.spinner();
                ui.label("Processing...");
            }
            
            ui.label("Strength:");
            let mut strength = state.grid_removal.strength;
            let slider = egui::Slider::new(&mut strength, 0.0..=1.0).step_by(0.01);
            if ui.add(slider).changed() {
                actions.push(Action::GridRemovalSetStrength(strength));
            }

            ui.separator();

            if ui.button("Disable").on_hover_text("Turn off grid removal and restore original image").clicked() {
                actions.push(Action::GridRemovalDisable);
            }
        });
    });

    // Debounce: check if pending strength should trigger recomputation
    if let (Some(_pending), Some(since)) = (state.grid_removal.pending_strength, state.grid_removal.pending_since) {
        if since.elapsed() > std::time::Duration::from_millis(300) && !state.grid_removal.is_computing {
            state.grid_removal.pending_strength = None;
            state.grid_removal.pending_since = None;
            // Trigger recomputation via action handler
            crate::action_handler::trigger_grid_removal_for(state);
        } else {
            // Request repaint to check debounce timer again
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    // Build cleaned texture if needed
    if state.grid_removal.enabled && state.grid_removal.cleaned_rgba.is_some() && state.grid_removal.cleaned_texture.is_none() {
        if let Some(ref cleaned) = state.grid_removal.cleaned_rgba {
            let w = state.img_size.x as usize;
            let h = state.img_size.y as usize;
            if cleaned.len() == w * h * 4 {
                let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], cleaned);
                let handle = ui.ctx().load_texture("grid_cleaned", color_image, egui::TextureOptions::LINEAR);
                state.grid_removal.cleaned_texture = Some(handle);
            }
        }
    }
}
