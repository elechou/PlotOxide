use crate::action::Action;
use crate::state::{AppState, InspectorEntry};
use eframe::egui;
use rhai::{Array, Dynamic, Map};

/// Draw inspector windows for all open variables.
pub fn draw_inspectors(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    // Snapshot entries to iterate (we'll collect mutations separately)
    let entries: Vec<InspectorEntry> = state.ide.open_inspectors.clone();
    let mut new_entries: Vec<InspectorEntry> = Vec::new();

    for entry in &entries {
        let mut is_open = true;

        // Resolve value: sub-inspector has it inline, top-level looks up workspace
        let val = if let Some(ref v) = entry.value {
            v.clone()
        } else {
            match state
                .ide
                .workspace_vars
                .iter()
                .find(|v| v.name == entry.key)
            {
                Some(v) => v.value.clone(),
                None => {
                    // Variable no longer exists — close inspector
                    actions.push(Action::CloseInspector(entry.key.clone()));
                    continue;
                }
            }
        };

        let entry_key = entry.key.clone();
        let mut pending: Vec<InspectorEntry> = Vec::new();

        egui::Window::new(format!("Inspector: {}", entry.label))
            .id(egui::Id::new(&entry.key))
            .open(&mut is_open)
            .default_size([200.0, 250.0])
            .resizable(true)
            .vscroll(false)
            .show(ctx, |ui| {
                draw_value_inspector(ui, &val, &entry_key, &mut pending);
            });

        if !is_open {
            actions.push(Action::CloseInspector(entry.key.clone()));
        }

        new_entries.extend(pending);
    }

    // Add any new sub-inspector entries (dedup by key)
    for entry in new_entries {
        if !state.ide.open_inspectors.iter().any(|e| e.key == entry.key) {
            state.ide.open_inspectors.push(entry);
        }
    }
}

/// Render an inspector view for any Dynamic value.
fn draw_value_inspector(
    ui: &mut egui::Ui,
    val: &Dynamic,
    path: &str,
    pending: &mut Vec<InspectorEntry>,
) {
    if val.is_array() {
        let arr = val.clone().try_cast::<Array>().unwrap_or_default();
        if arr.is_empty() {
            ui.label("Empty array");
            return;
        }

        // Check if it's an array of maps (table-like)
        if arr[0].is_map() {
            draw_array_of_maps_table(ui, &arr, path, pending);
        } else {
            draw_scalar_array_table(ui, &arr, path, pending);
        }
    } else if val.is_map() {
        let map = val.clone().try_cast::<Map>().unwrap_or_default();
        draw_map_table(ui, &map, path, pending);
    } else {
        // Scalar value — just show it
        ui.heading(format!("{}", val));
    }
}

/// Render a cell value: scalars as labels, complex values as clickable links.
fn render_cell_value(
    ui: &mut egui::Ui,
    val: &Dynamic,
    path: &str,
    label: &str,
    pending: &mut Vec<InspectorEntry>,
) {
    if val.is_array() || val.is_map() {
        let text = format_dynamic_short(val);
        if ui.link(&text).clicked() {
            pending.push(InspectorEntry {
                key: path.to_string(),
                label: label.to_string(),
                value: Some(val.clone()),
            });
        }
    } else {
        ui.label(format_dynamic_short(val));
    }
}

/// Table view for an array of maps (like data["group"])
fn draw_array_of_maps_table(
    ui: &mut egui::Ui,
    arr: &[Dynamic],
    path: &str,
    pending: &mut Vec<InspectorEntry>,
) {
    use egui_extras::{Column, TableBuilder};

    // Collect column names from the first element
    let first_map = arr[0].clone().try_cast::<Map>().unwrap_or_default();
    let mut col_names: Vec<String> = first_map.keys().map(|k| k.to_string()).collect();
    col_names.sort();

    ui.label(format!(
        "Array<Map>  [{} rows \u{00d7} {} cols]",
        arr.len(),
        col_names.len()
    ));
    ui.separator();

    let mut builder = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto().at_least(40.0)); // index column

    for _ in &col_names {
        builder = builder.column(Column::auto().at_least(50.0));
    }

    // We need to collect pending entries from inside the closure
    let path_owned = path.to_string();
    let col_names_clone = col_names.clone();

    // Use a shared vec for pending entries from table body
    let pending_cell = std::cell::RefCell::new(Vec::new());

    builder
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("idx");
            });
            for name in &col_names {
                header.col(|ui| {
                    ui.strong(name);
                });
            }
        })
        .body(|body| {
            body.rows(18.0, arr.len(), |mut row| {
                let idx = row.index();
                row.col(|ui| {
                    ui.label(format!("{}", idx));
                });

                let map = arr[idx].clone().try_cast::<Map>().unwrap_or_default();
                for name in &col_names_clone {
                    row.col(|ui| {
                        let val = map.get(name.as_str()).cloned().unwrap_or(Dynamic::UNIT);
                        let cell_path = format!("{}[{}].{}", path_owned, idx, name);
                        let cell_label = format!("{}[{}].{}", path_owned, idx, name);
                        let mut cell_pending = Vec::new();
                        render_cell_value(ui, &val, &cell_path, &cell_label, &mut cell_pending);
                        pending_cell.borrow_mut().extend(cell_pending);
                    });
                }
            });
        });

    pending.extend(pending_cell.into_inner());
}

/// Table view for an array of scalars
fn draw_scalar_array_table(
    ui: &mut egui::Ui,
    arr: &[Dynamic],
    path: &str,
    pending: &mut Vec<InspectorEntry>,
) {
    use egui_extras::{Column, TableBuilder};

    ui.label(format!("Array  [{}]", arr.len()));
    ui.separator();

    let path_owned = path.to_string();
    let pending_cell = std::cell::RefCell::new(Vec::new());

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto().at_least(40.0))
        .column(Column::remainder().at_least(60.0))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("idx");
            });
            header.col(|ui| {
                ui.strong("value");
            });
        })
        .body(|body| {
            body.rows(18.0, arr.len(), |mut row| {
                let idx = row.index();
                row.col(|ui| {
                    ui.label(format!("{}", idx));
                });
                row.col(|ui| {
                    let cell_path = format!("{}[{}]", path_owned, idx);
                    let cell_label = format!("{}[{}]", path_owned, idx);
                    let mut cell_pending = Vec::new();
                    render_cell_value(ui, &arr[idx], &cell_path, &cell_label, &mut cell_pending);
                    pending_cell.borrow_mut().extend(cell_pending);
                });
            });
        });

    pending.extend(pending_cell.into_inner());
}

/// Table view for a Map (key-value pairs)
fn draw_map_table(
    ui: &mut egui::Ui,
    map: &rhai::Map,
    path: &str,
    pending: &mut Vec<InspectorEntry>,
) {
    use egui_extras::{Column, TableBuilder};

    let mut entries: Vec<(String, Dynamic)> = map
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    ui.label(format!("Map  [{} keys]", entries.len()));
    ui.separator();

    let path_owned = path.to_string();
    let pending_cell = std::cell::RefCell::new(Vec::new());

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto().at_least(60.0))
        .column(Column::remainder().at_least(60.0))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("key");
            });
            header.col(|ui| {
                ui.strong("value");
            });
        })
        .body(|body| {
            body.rows(18.0, entries.len(), |mut row| {
                let idx = row.index();
                let (key, val) = &entries[idx];
                row.col(|ui| {
                    ui.label(key);
                });
                row.col(|ui| {
                    let cell_path = format!("{}.{}", path_owned, key);
                    let cell_label = format!("{}.{}", path_owned, key);
                    let mut cell_pending = Vec::new();
                    render_cell_value(ui, val, &cell_path, &cell_label, &mut cell_pending);
                    pending_cell.borrow_mut().extend(cell_pending);
                });
            });
        });

    pending.extend(pending_cell.into_inner());
}

/// Format a Dynamic value for display in a table cell.
fn format_dynamic_short(val: &Dynamic) -> String {
    if val.is_float() {
        format!("{:.6}", val.as_float().unwrap_or(0.0))
    } else if val.is_int() {
        format!("{}", val.as_int().unwrap_or(0))
    } else if val.is_string() {
        format!(
            "\"{}\"",
            val.clone().try_cast::<String>().unwrap_or_default()
        )
    } else if val.is_bool() {
        format!("{}", val.as_bool().unwrap_or(false))
    } else if val.is_array() {
        let arr = val.clone().try_cast::<Array>().unwrap_or_default();
        format!("Array[{}]", arr.len())
    } else if val.is_map() {
        let m = val.clone().try_cast::<Map>().unwrap_or_default();
        format!("Map{{{} keys}}", m.len())
    } else if val.is_unit() {
        "()".into()
    } else {
        format!("{}", val)
    }
}
