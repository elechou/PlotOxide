pub mod math;

use crate::state::AppState;
use rhai::{Array, Dynamic, Engine, Map, Scope};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// WorkspaceVar — extracted from rhai Scope after script evaluation
// ---------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct WorkspaceVar {
    pub name: String,
    pub type_name: String,
    pub dims: String,
    pub value: Dynamic,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Result of running a script: the combined output text and a snapshot of all
/// variables visible in the rhai scope after evaluation.
pub struct ScriptResult {
    pub output: String,
    pub workspace: Vec<WorkspaceVar>,
}

/// Evaluates a Rhai script against the current AppState.
///
/// Data points are exposed as a single `data` Map keyed by group name,
/// e.g. `data["20kHz"][0].x`.
pub fn run_script(state: &AppState, script: &str) -> ScriptResult {
    let mut engine = Engine::new();

    // Register math helpers
    math::register(&mut engine);

    // Capture print() output
    let output_buffer = Arc::new(Mutex::new(String::new()));
    let out_ref = Arc::clone(&output_buffer);
    engine.on_print(move |s| {
        let mut out = out_ref.lock().unwrap();
        out.push_str(s);
        out.push('\n');
    });

    // Build scope with `data` variable
    let mut scope = Scope::new();
    let data_map = build_data_map(state);
    scope.push("data", data_map);

    // Evaluate
    let result = engine.eval_with_scope::<Dynamic>(&mut scope, script);

    // Collect output
    let print_str = {
        let mut out = output_buffer.lock().unwrap();
        std::mem::take(&mut *out)
    };

    let output = match result {
        Ok(v) => {
            if v.is_unit() {
                print_str.trim_end().to_string()
            } else {
                let val_str = format!("{}", v);
                if print_str.is_empty() {
                    val_str
                } else {
                    format!("{}\n{}", print_str.trim_end(), val_str)
                }
            }
        }
        Err(e) => {
            let err_str = format!("{}", e);
            if print_str.is_empty() {
                err_str
            } else {
                format!("{}\n{}", print_str.trim_end(), err_str)
            }
        }
    };

    // Extract workspace variables from scope
    let workspace = extract_workspace(&scope);

    ScriptResult { output, workspace }
}

// ---------------------------------------------------------------------------
// Build the `data` Map from AppState groups
// ---------------------------------------------------------------------------
fn build_data_map(state: &AppState) -> Map {
    let mut data_map = Map::new();
    for (i, group) in state.groups.iter().enumerate() {
        let mut group_arr = Array::new();
        for p in &state.data_pts {
            if p.group_id == i {
                let mut pt_map = Map::new();
                pt_map.insert("x".into(), Dynamic::from(p.lx));
                pt_map.insert("y".into(), Dynamic::from(p.ly));
                pt_map.insert("px".into(), Dynamic::from(p.px));
                pt_map.insert("py".into(), Dynamic::from(p.py));
                group_arr.push(Dynamic::from_map(pt_map));
            }
        }
        data_map.insert(group.name.clone().into(), Dynamic::from_array(group_arr));
    }
    data_map
}

// ---------------------------------------------------------------------------
// Extract workspace variables from the Scope after eval
// ---------------------------------------------------------------------------
fn extract_workspace(scope: &Scope) -> Vec<WorkspaceVar> {
    let mut vars = Vec::new();
    for (name, _is_const, value) in scope.iter_raw() {
        let (type_name, dims) = describe_dynamic(&value);
        vars.push(WorkspaceVar {
            name: name.to_string(),
            type_name,
            dims,
            value: value.clone(),
        });
    }
    vars
}

/// Produce a human-readable type name and dimensions string for a Dynamic value.
fn describe_dynamic(val: &Dynamic) -> (String, String) {
    if val.is_array() {
        let arr = val.clone().try_cast::<Array>().unwrap_or_default();
        let len = arr.len();
        if let Some(first) = arr.first() {
            if first.is_map() {
                let cols = first
                    .clone()
                    .try_cast::<Map>()
                    .map(|m| m.len())
                    .unwrap_or(0);
                return ("Array<Map>".into(), format!("[{}×{}]", len, cols));
            }
        }
        ("Array".into(), format!("[{}]", len))
    } else if val.is_map() {
        let m = val.clone().try_cast::<Map>().unwrap_or_default();
        ("Map".into(), format!("{} keys", m.len()))
    } else if val.is_string() {
        let s = val.clone().try_cast::<String>().unwrap_or_default();
        ("String".into(), format!("len {}", s.len()))
    } else if val.is_int() {
        ("i64".into(), "scalar".into())
    } else if val.is_float() {
        ("f64".into(), "scalar".into())
    } else if val.is_bool() {
        ("bool".into(), "scalar".into())
    } else if val.is_unit() {
        ("()".into(), "—".into())
    } else {
        (val.type_name().to_string(), "—".into())
    }
}
