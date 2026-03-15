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

    // Hoist block-scoped variables into top-level scope
    let (rewritten, hoisted_names) = hoist_variables(script);
    for name in &hoisted_names {
        if !scope.contains(name) {
            scope.push(name.clone(), Dynamic::UNIT);
        }
    }

    // Evaluate
    let result = engine.eval_with_scope::<Dynamic>(&mut scope, &rewritten);

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
        // Skip hoisted variables that were never assigned (still unit)
        if value.is_unit() {
            continue;
        }
        let (type_name, dims) = describe_dynamic(value);
        vars.push(WorkspaceVar {
            name: name.to_string(),
            type_name,
            dims,
            value: value.clone(),
        });
    }
    vars
}

// ---------------------------------------------------------------------------
// Variable hoisting — lift block-scoped declarations to top-level scope
// ---------------------------------------------------------------------------

/// Scans the script for `let`/`const` declarations and `for` loop variables,
/// collects their names, and rewrites the script to remove the declaration
/// keywords so the variables bind to the pre-pushed scope entries instead.
fn hoist_variables(script: &str) -> (String, Vec<String>) {
    let mut names: Vec<String> = Vec::new();
    let mut result = String::with_capacity(script.len());
    let chars: Vec<char> = script.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Skip string literals
        if chars[i] == '"' {
            result.push(chars[i]);
            i += 1;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < len {
                    result.push(chars[i]);
                    i += 1;
                }
                result.push(chars[i]);
                i += 1;
            }
            if i < len {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Skip single-quoted strings
        if chars[i] == '\'' {
            result.push(chars[i]);
            i += 1;
            while i < len && chars[i] != '\'' {
                if chars[i] == '\\' && i + 1 < len {
                    result.push(chars[i]);
                    i += 1;
                }
                result.push(chars[i]);
                i += 1;
            }
            if i < len {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Skip line comments
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Skip block comments
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
            result.push(chars[i]);
            result.push(chars[i + 1]);
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                result.push(chars[i]);
                i += 1;
            }
            if i + 1 < len {
                result.push(chars[i]);
                result.push(chars[i + 1]);
                i += 2;
            }
            continue;
        }

        // Match `let` or `const` keyword followed by identifier
        if starts_with_keyword(&chars, i, "let") || starts_with_keyword(&chars, i, "const") {
            let kw_len = if chars[i] == 'l' { 3 } else { 5 };
            let after_kw = i + kw_len;
            // Must be followed by whitespace
            if after_kw < len && chars[after_kw].is_ascii_whitespace() {
                // Skip whitespace after keyword
                let mut j = after_kw;
                while j < len && chars[j].is_ascii_whitespace() {
                    j += 1;
                }
                // Read identifier
                if j < len && (chars[j].is_ascii_alphabetic() || chars[j] == '_') {
                    let name_start = j;
                    while j < len && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                        j += 1;
                    }
                    let var_name: String = chars[name_start..j].iter().collect();
                    if !names.contains(&var_name) {
                        names.push(var_name);
                    }
                    // Replace keyword with spaces (preserve column alignment)
                    for _ in 0..kw_len {
                        result.push(' ');
                    }
                    // Emit whitespace + identifier as-is
                    for &ch in &chars[after_kw..j] {
                        result.push(ch);
                    }
                    i = j;
                    continue;
                }
            }
        }

        // Match `for` keyword: `for varname in`
        if starts_with_keyword(&chars, i, "for") {
            let after_kw = i + 3;
            if after_kw < len && chars[after_kw].is_ascii_whitespace() {
                let mut j = after_kw;
                while j < len && chars[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < len && (chars[j].is_ascii_alphabetic() || chars[j] == '_') {
                    let name_start = j;
                    while j < len && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                        j += 1;
                    }
                    let var_name: String = chars[name_start..j].iter().collect();
                    // Check that the next non-whitespace token is `in`
                    let mut k = j;
                    while k < len && chars[k].is_ascii_whitespace() {
                        k += 1;
                    }
                    if starts_with_keyword(&chars, k, "in")
                        && !names.contains(&var_name)
                    {
                        names.push(var_name);
                    }
                }
            }
            // Emit `for` as-is (don't remove it)
        }

        result.push(chars[i]);
        i += 1;
    }

    (result, names)
}

/// Check if `chars[pos..]` starts with the given keyword, preceded by a
/// non-identifier char (or start of string).
fn starts_with_keyword(chars: &[char], pos: usize, keyword: &str) -> bool {
    let kw: Vec<char> = keyword.chars().collect();
    if pos + kw.len() > chars.len() {
        return false;
    }
    // Must not be preceded by an identifier char
    if pos > 0 && (chars[pos - 1].is_ascii_alphanumeric() || chars[pos - 1] == '_') {
        return false;
    }
    for (j, &kc) in kw.iter().enumerate() {
        if chars[pos + j] != kc {
            return false;
        }
    }
    // Must not be followed by an identifier char (beyond the keyword length)
    let after = pos + kw.len();
    if after < chars.len() && (chars[after].is_ascii_alphanumeric() || chars[after] == '_') {
        return false;
    }
    true
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
