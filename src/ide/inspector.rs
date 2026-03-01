use crate::action::Action;
use crate::state::AppState;
use eframe::egui;
use rhai::{Array, Dynamic, Map};

/// Draw inspector windows for all open variables.
pub fn draw_inspectors(state: &mut AppState, ctx: &egui::Context, actions: &mut Vec<Action>) {
    let open_vars: Vec<String> = state.ide.open_inspectors.iter().cloned().collect();

    for var_name in open_vars {
        let mut is_open = true;

        // Find the variable in workspace_vars
        let var_value = state
            .ide
            .workspace_vars
            .iter()
            .find(|v| v.name == var_name)
            .map(|v| v.value.clone());

        let val = match var_value {
            Some(v) => v,
            None => {
                // Variable no longer exists — close inspector
                actions.push(Action::CloseInspector(var_name));
                continue;
            }
        };

        egui::Window::new(format!("Inspector: {}", var_name))
            .open(&mut is_open)
            .default_size([400.0, 400.0])
            .vscroll(false)
            .show(ctx, |ui| {
                draw_value_inspector(ui, &val);
            });

        if !is_open {
            actions.push(Action::CloseInspector(var_name));
        }
    }
}

/// Render an inspector view for any Dynamic value.
fn draw_value_inspector(ui: &mut egui::Ui, val: &Dynamic) {
    if val.is_array() {
        let arr = val.clone().try_cast::<Array>().unwrap_or_default();
        if arr.is_empty() {
            ui.label("Empty array");
            return;
        }

        // Check if it's an array of maps (table-like)
        if arr[0].is_map() {
            draw_array_of_maps_table(ui, &arr);
        } else {
            draw_scalar_array_table(ui, &arr);
        }
    } else if val.is_map() {
        let map = val.clone().try_cast::<Map>().unwrap_or_default();
        draw_map_table(ui, &map);
    } else {
        // Scalar value — just show it
        ui.heading(format!("{}", val));
    }
}

/// Table view for an array of maps (like data["group"])
fn draw_array_of_maps_table(ui: &mut egui::Ui, arr: &[Dynamic]) {
    use egui_extras::{Column, TableBuilder};

    // Collect column names from the first element
    let first_map = arr[0].clone().try_cast::<Map>().unwrap_or_default();
    let mut col_names: Vec<String> = first_map.keys().map(|k| k.to_string()).collect();
    col_names.sort();

    ui.label(format!(
        "Array<Map>  [{} rows × {} cols]",
        arr.len(),
        col_names.len()
    ));
    ui.separator();

    let mut builder = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(50.0).at_least(40.0)); // index column

    for _ in &col_names {
        builder = builder.column(Column::remainder().at_least(60.0));
    }

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
                for name in &col_names {
                    row.col(|ui| {
                        let val = map.get(name.as_str()).cloned().unwrap_or(Dynamic::UNIT);
                        ui.label(format_dynamic_short(&val));
                    });
                }
            });
        });
}

/// Table view for an array of scalars
fn draw_scalar_array_table(ui: &mut egui::Ui, arr: &[Dynamic]) {
    use egui_extras::{Column, TableBuilder};

    ui.label(format!("Array  [{}]", arr.len()));
    ui.separator();

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(50.0).at_least(40.0))
        .column(Column::remainder().at_least(80.0))
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
                    ui.label(format_dynamic_short(&arr[idx]));
                });
            });
        });
}

/// Table view for a Map (key-value pairs)
fn draw_map_table(ui: &mut egui::Ui, map: &rhai::Map) {
    use egui_extras::{Column, TableBuilder};

    let mut entries: Vec<(String, Dynamic)> = map
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    ui.label(format!("Map  [{} keys]", entries.len()));
    ui.separator();

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(120.0).at_least(60.0))
        .column(Column::remainder().at_least(80.0))
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
                    ui.label(format_dynamic_short(val));
                });
            });
        });
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
