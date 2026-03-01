// ============================================================
//  PlotOxide — Steinmetz Parameter Fitting
// ============================================================
//  Fits the Steinmetz equation in log₁₀ space:
//
//    log₁₀(Pv) = log₁₀(k) + α·log₁₀(f) + β·log₁₀(B)
//
//  Assumptions:
//    - Each group name contains the frequency, e.g. "20kHz"
//      (the first number is extracted as f in Hz×1000 for 'k',
//       or raw number — adjust the multiplier below).
//    - Point x-values = B (flux density in mT, converted to T)
//    - Point y-values = Pv (power loss density)
//
//  The script collects ALL points from ALL groups, builds the
//  design matrix, and solves via lstsq (OLS).
// ============================================================

// Adjust this multiplier if your group-name number is in kHz:
//   If names are like "20kHz" and the number 20 means 20000 Hz,
//   set freq_multiplier = 1000.0.
//   If the number is already in Hz, set to 1.0.
let freq_multiplier = 1000.0;

// B conversion: mT → T
let b_scale = 1.0e-3;

print("=== Steinmetz Parameter Fitting (OLS in log₁₀ space) ===\n");

// ---- Collect all valid (f, B, P) triples ----
let all_log_f = [];
let all_log_B = [];
let all_log_P = [];
let total_pts = 0;

for name in data.keys() {
    let f_val = extract_number(name);
    if f_val <= 0.0 {
        print(`[${name}]  ⚠ Could not extract frequency, skipped.`);
        continue;
    }
    let f_hz = f_val * freq_multiplier;

    let pts = data[name];
    if pts.len() == 0 { continue; }

    let lf = log10(f_hz);
    let count = 0;

    for pt in pts {
        let b_val = pt.x * b_scale;   // mT → T
        let p_val = pt.y;

        if b_val > 0.0 && p_val > 0.0 {
            all_log_f.push(lf);
            all_log_B.push(log10(b_val));
            all_log_P.push(log10(p_val));
            count += 1;
        }
    }
    print(`[${name}]  f = ${f_hz} Hz,  ${count} valid points`);
    total_pts += count;
}

print(`\nTotal valid data points: ${total_pts}`);

if total_pts < 3 {
    print("\n⚠ Need at least 3 valid points for fitting. Aborting.");
} else {
    // ---- Build design matrix A: each row = [1, log10(f), log10(B)] ----
    let A = [];
    let i = 0;
    while i < total_pts {
        A.push([1.0, all_log_f[i], all_log_B[i]]);
        i += 1;
    }

    // ---- Solve: A·x = log10(P) ----
    let result = lstsq(A, all_log_P);

    let log_k = result[0];
    let alpha  = result[1];
    let beta   = result[2];
    let k      = pow10(log_k);

    print("\n─── Results ───");
    print(`  k     = ${k}`);
    print(`  alpha = ${alpha}`);
    print(`  beta  = ${beta}`);
    print("");
    print(`  Pv = ${k} · f^${alpha} · B^${beta}`);
}
