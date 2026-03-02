// ============================================================
//  PlotRedox — Quadratic Regression for All Groups
// ============================================================
//  Fits  y = a₀ + a₁·x + a₂·x²  to every group and prints
//  the coefficients plus R² for each.
// ============================================================

print("=== Quadratic (2nd-order) Polynomial Fit ===\n");

for name in data.keys() {
    let pts = data[name];
    if pts.len() < 3 {
        print(`[${name}]  ⚠ Need ≥ 3 points (have ${pts.len()}), skipped.`);
        continue;
    }

    let xs = col(pts, "x");
    let ys = col(pts, "y");

    // polyfit(x, y, degree) → #{ coeffs: [a0, a1, a2], r_squared }
    let fit = polyfit(xs, ys, 2);

    let a0 = fit.coeffs[0];
    let a1 = fit.coeffs[1];
    let a2 = fit.coeffs[2];

    print(`[${name}]  ${pts.len()} pts`);
    print(`y = ${a0} + ${a1}·x + ${a2}·x²`);
    print(`R² = ${fit.r_squared}`);
}
