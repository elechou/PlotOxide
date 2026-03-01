use rhai::{Array, Dynamic, Engine, Map, FLOAT, INT};
use std::string::String as StdString;

/// Convert a rhai Array of numeric values to Vec<f64>.
pub fn to_floats(arr: &Array) -> Vec<f64> {
    arr.iter()
        .filter_map(|v| {
            if v.is_float() {
                v.as_float().ok()
            } else if v.is_int() {
                v.as_int().ok().map(|i| i as f64)
            } else {
                None
            }
        })
        .collect()
}

/// Register all math / science helper functions into the rhai Engine.
pub fn register(engine: &mut Engine) {
    // ---- basic math ----
    engine.register_fn("abs", |x: FLOAT| -> FLOAT { x.abs() });
    engine.register_fn("abs", |x: INT| -> INT { x.abs() });
    engine.register_fn("sqrt", |x: FLOAT| -> FLOAT { x.sqrt() });
    engine.register_fn("ln", |x: FLOAT| -> FLOAT { x.ln() });
    engine.register_fn("log10", |x: FLOAT| -> FLOAT { x.log10() });
    engine.register_fn("log2", |x: FLOAT| -> FLOAT { x.log2() });
    engine.register_fn("exp", |x: FLOAT| -> FLOAT { x.exp() });
    engine.register_fn("pow", |x: FLOAT, y: FLOAT| -> FLOAT { x.powf(y) });
    engine.register_fn("pow", |x: FLOAT, y: INT| -> FLOAT { x.powi(y as i32) });
    engine.register_fn("sin", |x: FLOAT| -> FLOAT { x.sin() });
    engine.register_fn("cos", |x: FLOAT| -> FLOAT { x.cos() });
    engine.register_fn("tan", |x: FLOAT| -> FLOAT { x.tan() });
    engine.register_fn("asin", |x: FLOAT| -> FLOAT { x.asin() });
    engine.register_fn("acos", |x: FLOAT| -> FLOAT { x.acos() });
    engine.register_fn("atan", |x: FLOAT| -> FLOAT { x.atan() });
    engine.register_fn("atan2", |y: FLOAT, x: FLOAT| -> FLOAT { y.atan2(x) });
    engine.register_fn("floor", |x: FLOAT| -> FLOAT { x.floor() });
    engine.register_fn("ceil", |x: FLOAT| -> FLOAT { x.ceil() });
    engine.register_fn("round", |x: FLOAT| -> FLOAT { x.round() });
    engine.register_fn("PI", || -> FLOAT { std::f64::consts::PI });

    // ---- array aggregation ----
    engine.register_fn("sum", sum_array);
    engine.register_fn("mean", mean_array);
    engine.register_fn("min_val", min_array);
    engine.register_fn("max_val", max_array);
    engine.register_fn("std_dev", std_dev_array);
    engine.register_fn("variance", variance_array);

    // ---- column extraction from array-of-maps ----
    engine.register_fn("col", col_extract);

    // ---- regression / fitting ----
    engine.register_fn("linreg", linreg);
    engine.register_fn("polyfit", polyfit);
    engine.register_fn("lstsq", lstsq);

    // ---- array operations ----
    engine.register_fn("log10_array", log10_array);

    // ---- convenience ----
    engine.register_fn("pow10", |x: FLOAT| -> FLOAT { 10.0_f64.powf(x) });
    engine.register_fn("extract_number", extract_number);
    engine.register_fn("round_to", |x: FLOAT, decimals: INT| -> FLOAT {
        let factor = 10.0_f64.powi(decimals as i32);
        (x * factor).round() / factor
    });
}

// ---------------------------------------------------------------------------
// Array aggregation
// ---------------------------------------------------------------------------

fn sum_array(arr: Array) -> Dynamic {
    let vals = to_floats(&arr);
    Dynamic::from(vals.iter().sum::<f64>())
}

fn mean_array(arr: Array) -> Dynamic {
    let vals = to_floats(&arr);
    if vals.is_empty() {
        return Dynamic::from(0.0_f64);
    }
    Dynamic::from(vals.iter().sum::<f64>() / vals.len() as f64)
}

fn min_array(arr: Array) -> Dynamic {
    let vals = to_floats(&arr);
    Dynamic::from(vals.iter().copied().fold(f64::INFINITY, f64::min))
}

fn max_array(arr: Array) -> Dynamic {
    let vals = to_floats(&arr);
    Dynamic::from(vals.iter().copied().fold(f64::NEG_INFINITY, f64::max))
}

fn variance_array(arr: Array) -> Dynamic {
    let vals = to_floats(&arr);
    if vals.len() < 2 {
        return Dynamic::from(0.0_f64);
    }
    let n = vals.len() as f64;
    let mean = vals.iter().sum::<f64>() / n;
    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    Dynamic::from(var)
}

fn std_dev_array(arr: Array) -> Dynamic {
    let var = variance_array(arr);
    Dynamic::from(var.as_float().unwrap_or(0.0).sqrt())
}

// ---------------------------------------------------------------------------
// col(array_of_maps, "field") → array of values
// ---------------------------------------------------------------------------

fn col_extract(arr: Array, field: &str) -> Array {
    arr.iter()
        .map(|item| {
            if let Some(map) = item.clone().try_cast::<Map>() {
                map.get(field).cloned().unwrap_or(Dynamic::UNIT)
            } else {
                Dynamic::UNIT
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// linreg(x_array, y_array) → #{ slope, intercept, r_squared }
// ---------------------------------------------------------------------------

fn linreg(x_arr: Array, y_arr: Array) -> Dynamic {
    let xs = to_floats(&x_arr);
    let ys = to_floats(&y_arr);
    let n = xs.len().min(ys.len());
    if n < 2 {
        let mut m = Map::new();
        m.insert("slope".into(), Dynamic::from(f64::NAN));
        m.insert("intercept".into(), Dynamic::from(f64::NAN));
        m.insert("r_squared".into(), Dynamic::from(f64::NAN));
        return Dynamic::from_map(m);
    }

    let n_f = n as f64;
    let sum_x: f64 = xs[..n].iter().sum();
    let sum_y: f64 = ys[..n].iter().sum();
    let sum_xy: f64 = xs[..n].iter().zip(&ys[..n]).map(|(a, b)| a * b).sum();
    let sum_x2: f64 = xs[..n].iter().map(|x| x * x).sum();

    let denom = n_f * sum_x2 - sum_x * sum_x;
    let slope = if denom.abs() < 1e-30 {
        f64::NAN
    } else {
        (n_f * sum_xy - sum_x * sum_y) / denom
    };
    let intercept = (sum_y - slope * sum_x) / n_f;

    // R²
    let mean_y = sum_y / n_f;
    let ss_tot: f64 = ys[..n].iter().map(|y| (y - mean_y).powi(2)).sum();
    let ss_res: f64 = xs[..n]
        .iter()
        .zip(&ys[..n])
        .map(|(x, y)| {
            let pred = intercept + slope * x;
            (y - pred).powi(2)
        })
        .sum();
    let r_squared = if ss_tot.abs() < 1e-30 {
        f64::NAN
    } else {
        1.0 - ss_res / ss_tot
    };

    let mut m = Map::new();
    m.insert("slope".into(), Dynamic::from(slope));
    m.insert("intercept".into(), Dynamic::from(intercept));
    m.insert("r_squared".into(), Dynamic::from(r_squared));
    Dynamic::from_map(m)
}

// ---------------------------------------------------------------------------
// polyfit(x_array, y_array, degree) → #{ coeffs: [...], r_squared }
// ---------------------------------------------------------------------------

fn polyfit(x_arr: Array, y_arr: Array, degree: INT) -> Dynamic {
    use nalgebra::{DMatrix, DVector};

    let xs = to_floats(&x_arr);
    let ys = to_floats(&y_arr);
    let n = xs.len().min(ys.len());
    let deg = degree.max(1) as usize;

    if n <= deg {
        let mut m = Map::new();
        m.insert("coeffs".into(), Dynamic::from_array(vec![]));
        m.insert("r_squared".into(), Dynamic::from(f64::NAN));
        return Dynamic::from_map(m);
    }

    // Build Vandermonde matrix  [1, x, x², …, x^deg]
    let cols = deg + 1;
    let mut a_data = vec![0.0_f64; n * cols];
    for i in 0..n {
        let mut xp = 1.0_f64;
        for j in 0..cols {
            a_data[j * n + i] = xp; // column-major
            xp *= xs[i];
        }
    }
    let a = DMatrix::from_vec(n, cols, a_data);
    let b = DVector::from_vec(ys[..n].to_vec());

    // Normal equations: (AᵀA) c = Aᵀb
    let ata = a.transpose() * &a;
    let atb = a.transpose() * &b;

    let coeffs_vec = match ata.lu().solve(&atb) {
        Some(c) => c,
        None => {
            let mut m = Map::new();
            m.insert("coeffs".into(), Dynamic::from_array(vec![]));
            m.insert("r_squared".into(), Dynamic::from(f64::NAN));
            return Dynamic::from_map(m);
        }
    };

    // R²
    let mean_y: f64 = ys[..n].iter().sum::<f64>() / n as f64;
    let ss_tot: f64 = ys[..n].iter().map(|y| (y - mean_y).powi(2)).sum();
    let mut ss_res = 0.0_f64;
    for i in 0..n {
        let mut pred = 0.0_f64;
        let mut xp = 1.0_f64;
        for j in 0..cols {
            pred += coeffs_vec[j] * xp;
            xp *= xs[i];
        }
        ss_res += (ys[i] - pred).powi(2);
    }
    let r_squared = if ss_tot.abs() < 1e-30 {
        f64::NAN
    } else {
        1.0 - ss_res / ss_tot
    };

    let coeffs_dyn: Array = coeffs_vec.iter().map(|c| Dynamic::from(*c)).collect();

    let mut m = Map::new();
    m.insert("coeffs".into(), Dynamic::from_array(coeffs_dyn));
    m.insert("r_squared".into(), Dynamic::from(r_squared));
    Dynamic::from_map(m)
}

// ---------------------------------------------------------------------------
// lstsq(A_2d_array, b_array) → array of coefficients
// A is an array of row-arrays, b is a 1D array.
// Solves the least-squares problem  A·x ≈ b  via normal equations.
// ---------------------------------------------------------------------------

fn lstsq(a_rows: Array, b_arr: Array) -> Dynamic {
    use nalgebra::{DMatrix, DVector};

    let b_vals = to_floats(&b_arr);
    let n = b_vals.len(); // number of rows

    if n == 0 || a_rows.len() != n {
        return Dynamic::from_array(vec![]);
    }

    // Determine number of columns from first row
    let first_row = match a_rows[0].clone().try_cast::<Array>() {
        Some(r) => r,
        None => return Dynamic::from_array(vec![]),
    };
    let cols = first_row.len();
    if cols == 0 || n < cols {
        return Dynamic::from_array(vec![]);
    }

    // Build matrix A (column-major for nalgebra)
    let mut a_data = vec![0.0_f64; n * cols];
    for i in 0..n {
        let row = match a_rows[i].clone().try_cast::<Array>() {
            Some(r) => r,
            None => return Dynamic::from_array(vec![]),
        };
        let row_vals = to_floats(&row);
        for j in 0..cols {
            a_data[j * n + i] = if j < row_vals.len() { row_vals[j] } else { 0.0 };
        }
    }

    let a = DMatrix::from_vec(n, cols, a_data);
    let b = DVector::from_vec(b_vals);

    // Normal equations: (AᵀA) c = Aᵀb
    let ata = a.transpose() * &a;
    let atb = a.transpose() * &b;

    match ata.lu().solve(&atb) {
        Some(c) => {
            let result: Array = c.iter().map(|v| Dynamic::from(*v)).collect();
            Dynamic::from_array(result)
        }
        None => Dynamic::from_array(vec![]),
    }
}

// ---------------------------------------------------------------------------
// log10_array(arr) → element-wise log₁₀
// ---------------------------------------------------------------------------

fn log10_array(arr: Array) -> Array {
    to_floats(&arr)
        .into_iter()
        .map(|v| Dynamic::from(v.log10()))
        .collect()
}

// ---------------------------------------------------------------------------
// extract_number(s) → first numeric value found in the string
// ---------------------------------------------------------------------------

fn extract_number(s: StdString) -> Dynamic {
    // Find first sequence of digits (optionally with decimal point)
    let mut start = None;
    let mut end = 0;
    for (i, ch) in s.char_indices() {
        if ch.is_ascii_digit() || (ch == '.' && start.is_some()) {
            if start.is_none() {
                start = Some(i);
            }
            end = i + ch.len_utf8();
        } else if start.is_some() {
            break;
        }
    }
    match start {
        Some(s_idx) => {
            if let Ok(v) = s[s_idx..end].parse::<f64>() {
                Dynamic::from(v)
            } else {
                Dynamic::from(0.0_f64)
            }
        }
        None => Dynamic::from(0.0_f64),
    }
}
