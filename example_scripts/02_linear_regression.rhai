// ============================================================
//  PlotOxide — Linear Regression (y = k·x) for All Groups
// ============================================================
//  Fits  y = k·x  (forced through origin) plus a standard
//  y = a + b·x with intercept for comparison.
// ============================================================

print("=== Linear Fit for All Groups ===\n");

for name in data.keys() {
    let pts = data[name];
    if pts.len() < 2 {
        print(`[${name}]  ⚠ Need ≥ 2 points (have ${pts.len()}), skipped.`);
        continue;
    }

    let xs = col(pts, "x");
    let ys = col(pts, "y");

    // --- y = a + b·x ---
    let fit = linreg(xs, ys);

    print(`[${name}]  ${pts.len()} pts`);
    print(`  y = ${fit.slope}·x + ${fit.intercept} (with intercept) R² = ${fit.r_squared}`);
    print("");
}
