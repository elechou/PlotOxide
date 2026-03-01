// ============================================================
//  PlotOxide — Rhai Scripting Basics
// ============================================================
//  This script demonstrates the core language features and
//  the helper functions available for working with your data.
// ============================================================

// ---- 1. Variables ----
// Use `let` to declare variables (like Python's `x = 5`).
let greeting = "Hello from Rhai!";
let pi_val   = PI();            // built-in constant
let n        = 42;              // integer (i64)
let x        = 3.14;           // float   (f64)

print("--- Variables ---");
print(greeting);
print(`PI = ${pi_val}`);
print(`n  = ${n},  x = ${x}`);

// ---- 2. Arithmetic & Math ----
print("\n--- Math ---");
print(`sqrt(2)  = ${sqrt(2.0)}`);
print(`ln(e)    = ${ln(exp(1.0))}`);
print(`log10(100) = ${log10(100.0)}`);
print(`pow(2, 10) = ${pow(2.0, 10)}`);
print(`sin(PI()/6) = ${sin(PI() / 6.0)}`);

// ---- 3. Arrays & Loops ----
print("\n--- Arrays ---");
let nums = [10.0, 20.0, 30.0, 40.0, 50.0];
print(`nums = ${nums}`);
print(`sum  = ${sum(nums)},  mean = ${mean(nums)}`);
print(`min  = ${min_val(nums)},  max = ${max_val(nums)}`);

// `for` loop — iterates like Python's `for x in list:`
print("\nSquares:");
for v in nums {
    print(`  ${v}^2 = ${pow(v, 2)}`);
}

// ---- 4. Working with Data ----
// `data` is a Map of group-name → array-of-points.
// Each point has fields: .x  .y  (logical/calibrated)
//                        .px .py (pixel coordinates)
print("\n--- Data Groups ---");
let group_names = data.keys();
print(`Groups: ${group_names}`);

for name in group_names {
    let pts = data[name];
    print(`  "${name}": ${pts.len()} points`);

    if pts.len() > 0 {
        let xs = col(pts, "x");
        let ys = col(pts, "y");
        print(`    x range: ${min_val(xs)} .. ${max_val(xs)}`);
        print(`    y range: ${min_val(ys)} .. ${max_val(ys)}`);
    }
}

// ---- 5. If / Else ----
print("\n--- Conditional ---");
let total_pts = 0;
for name in data.keys() {
    total_pts += data[name].len();
}
if total_pts == 0 {
    print("No data points yet — add some on the canvas!");
} else {
    print(`Total data points across all groups: ${total_pts}`);
}

// ---- 6. Linear Regression ----
print("\n--- Linear Regression (first group) ---");
if group_names.len() > 0 {
    let first = data[group_names[0]];
    if first.len() >= 2 {
        let xs = col(first, "x");
        let ys = col(first, "y");
        let fit = linreg(xs, ys);
        print(`  slope     = ${fit.slope}`);
        print(`  intercept = ${fit.intercept}`);
        print(`  R²        = ${fit.r_squared}`);
    } else {
        print("  Need at least 2 points for regression.");
    }
}
