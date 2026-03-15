use num_complex::Complex;
use rustfft::FftPlanner;

/// Remove periodic grid lines from an RGBA image.
///
/// For linear axes: FFT-based frequency domain filtering with narrow 2D notch filters
/// on the DC axes, preserving data curves whose energy is spread across the 2D spectrum.
///
/// For log axes: spatial 1D median-profile approach — grid lines show up as columns/rows
/// that are consistently darker than their neighbors, regardless of spacing pattern.
pub fn remove_grid(
    rgba: &[u8],
    w: u32,
    h: u32,
    strength: f32,
    log_x: bool,
    log_y: bool,
) -> Vec<u8> {
    let w = w as usize;
    let h = h as usize;
    let n = w * h;

    // RGBA → grayscale (luminance)
    let mut gray = vec![0.0f32; n];
    for i in 0..n {
        let r = rgba[i * 4] as f32;
        let g = rgba[i * 4 + 1] as f32;
        let b = rgba[i * 4 + 2] as f32;
        gray[i] = 0.299 * r + 0.587 * g + 0.114 * b;
    }

    // Compute per-pixel correction for each axis independently
    let vcorr = if log_x {
        spatial_grid_correction(&gray, w, h, false, strength)
    } else {
        fft_grid_correction(&gray, w, h, false, strength)
    };

    let hcorr = if log_y {
        spatial_grid_correction(&gray, w, h, true, strength)
    } else {
        fft_grid_correction(&gray, w, h, true, strength)
    };

    // Apply combined correction to RGB channels
    let mut output = rgba.to_vec();
    for i in 0..n {
        let corr = vcorr[i] + hcorr[i];
        for ch in 0..3 {
            let orig = rgba[i * 4 + ch] as f32;
            output[i * 4 + ch] = (orig - corr).clamp(0.0, 255.0) as u8;
        }
    }

    output
}

// ── FFT approach (linear axes) ─────────────────────────────────────────

/// Detect and compute grid correction using 1D FFT for a linear axis.
///
/// `is_horizontal`: true = detect horizontal grid lines (row-wise), false = vertical (column-wise).
///
/// Algorithm:
/// 1. Compute the mean profile along the axis (column means or row means)
/// 2. FFT the 1D profile → detect periodic peaks
/// 3. Extract the grid-only signal by keeping only peak frequencies
/// 4. Expand to a full-image correction
fn fft_grid_correction(
    gray: &[f32],
    w: usize,
    h: usize,
    is_horizontal: bool,
    strength: f32,
) -> Vec<f32> {
    let n = w * h;
    let profile_len = if is_horizontal { h } else { w };

    // 1. Compute mean profile
    let mut profile = vec![0.0f32; profile_len];
    if is_horizontal {
        // Row means → detect horizontal grid lines
        for row in 0..h {
            let mut sum = 0.0f32;
            for col in 0..w {
                sum += gray[row * w + col];
            }
            profile[row] = sum / w as f32;
        }
    } else {
        // Column means → detect vertical grid lines
        for col in 0..w {
            let mut sum = 0.0f32;
            for row in 0..h {
                sum += gray[row * w + col];
            }
            profile[col] = sum / h as f32;
        }
    }

    // 2. FFT the profile
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(profile_len);

    let mut freq: Vec<Complex<f32>> = profile.iter().map(|&v| Complex::new(v, 0.0)).collect();
    fft.process(&mut freq);

    // 3. Detect periodic peaks (grid frequencies)
    let mags: Vec<f32> = freq.iter().map(|c| c.norm()).collect();
    let peaks = find_periodic_peaks(&mags, 3.0);

    if peaks.is_empty() {
        return vec![0.0f32; n];
    }

    // 4. Extract grid-only signal: zero out everything except detected peaks
    let notch_sigma = 1.0 + strength * 2.0;
    let mut grid_freq = vec![Complex::new(0.0f32, 0.0); profile_len];
    for &peak in &peaks {
        // Include the peak and nearby bins with Gaussian falloff
        let extent = (notch_sigma * 3.0) as usize + 1;
        for d in 0..=extent {
            let weight = if d == 0 {
                1.0
            } else {
                (-(d as f32).powi(2) / (2.0 * notch_sigma * notch_sigma)).exp()
            };
            if weight < 0.01 {
                break;
            }
            let indices = if d == 0 {
                vec![peak]
            } else {
                vec![
                    (peak + d) % profile_len,
                    (peak + profile_len - d) % profile_len,
                ]
            };
            for idx in indices {
                grid_freq[idx] = freq[idx] * weight;
            }
        }
    }
    // Never include DC
    grid_freq[0] = Complex::new(0.0, 0.0);

    // Inverse FFT → grid-only profile
    let ifft = planner.plan_fft_inverse(profile_len);
    ifft.process(&mut grid_freq);
    let scale = 1.0 / profile_len as f32;
    let grid_profile: Vec<f32> = grid_freq.iter().map(|c| c.re * scale).collect();

    // 5. Expand to full image correction
    let suppress = 0.5 + strength * 0.5;
    let mut correction = vec![0.0f32; n];
    if is_horizontal {
        for row in 0..h {
            let val = grid_profile[row] * suppress;
            for col in 0..w {
                correction[row * w + col] = val;
            }
        }
    } else {
        for col in 0..w {
            let val = grid_profile[col] * suppress;
            for row in 0..h {
                correction[row * w + col] = val;
            }
        }
    }

    correction
}

// ── Spatial approach (log axes) ────────────────────────────────────────

/// Detect and compute grid correction using spatial median-profile approach.
///
/// Works for any grid spacing (linear or log) because it doesn't assume periodicity.
/// Grid lines show up as columns/rows that are consistently darker than their neighbors.
///
/// `is_horizontal`: true = detect horizontal grid lines, false = vertical.
fn spatial_grid_correction(
    gray: &[f32],
    w: usize,
    h: usize,
    is_horizontal: bool,
    strength: f32,
) -> Vec<f32> {
    let n = w * h;
    let profile_len = if is_horizontal { h } else { w };
    let line_len = if is_horizontal { w } else { h };

    // 1. Compute median brightness for each column/row
    let mut profile = vec![0.0f32; profile_len];
    let mut buf = vec![0.0f32; line_len];
    for (i, prof) in profile.iter_mut().enumerate() {
        for (j, b) in buf.iter_mut().enumerate() {
            let idx = if is_horizontal { i * w + j } else { j * w + i };
            *b = gray[idx];
        }
        buf.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        *prof = buf[line_len / 2];
    }

    // 2. Smooth the profile to get the local background trend
    // Use a large sigma (~5% of the dimension) to smooth over grid spacing
    let sigma = (profile_len as f32 * 0.05).max(10.0);
    let background = gaussian_smooth_1d(&profile, sigma);

    // 3. Grid contribution = profile - background
    // Grid lines are darker → negative dips. Only keep negative values (darker than background).
    let mut grid_dip = vec![0.0f32; profile_len];
    for i in 0..profile_len {
        let diff = profile[i] - background[i];
        if diff < 0.0 {
            grid_dip[i] = diff; // negative value = grid line
        }
    }

    // 4. Expand to full image correction
    // correction = -grid_dip * strength (positive: make pixels brighter to compensate)
    let suppress = 0.5 + strength * 0.5;
    let mut correction = vec![0.0f32; n];
    if is_horizontal {
        for row in 0..h {
            let val = grid_dip[row] * suppress;
            for col in 0..w {
                correction[row * w + col] = val;
            }
        }
    } else {
        for col in 0..w {
            let val = grid_dip[col] * suppress;
            for row in 0..h {
                correction[row * w + col] = val;
            }
        }
    }

    correction
}

/// 1D Gaussian smoothing.
fn gaussian_smooth_1d(data: &[f32], sigma: f32) -> Vec<f32> {
    let n = data.len();
    let radius = (sigma * 3.0).ceil() as usize;
    let mut result = vec![0.0f32; n];

    // Precompute kernel
    let mut kernel = Vec::with_capacity(2 * radius + 1);
    let mut kernel_sum = 0.0f32;
    for d in 0..=radius {
        let w = (-(d as f32).powi(2) / (2.0 * sigma * sigma)).exp();
        kernel.push(w);
        kernel_sum += if d == 0 { w } else { 2.0 * w };
    }

    for i in 0..n {
        let mut sum = 0.0f32;
        let mut wsum = 0.0f32;
        for d in 0..=radius {
            let weight = kernel[d];
            // Handle boundaries with clamping
            if d == 0 {
                sum += data[i] * weight;
                wsum += weight;
            } else {
                if i >= d {
                    sum += data[i - d] * weight;
                    wsum += weight;
                }
                if i + d < n {
                    sum += data[i + d] * weight;
                    wsum += weight;
                }
            }
        }
        result[i] = sum / wsum;
    }

    let _ = kernel_sum; // used for normalization reference
    result
}

// ── Peak detection ─────────────────────────────────────────────────────

/// Find periodic peaks in a 1D magnitude spectrum.
/// Skips DC (index 0) and very low frequencies.
/// Returns indices of detected peaks significantly above the local median.
fn find_periodic_peaks(mags: &[f32], threshold_factor: f32) -> Vec<usize> {
    let n = mags.len();
    if n < 8 {
        return vec![];
    }

    let half = n / 2;
    let search_start = 3;
    let search_range = search_start..half;

    let mut sorted: Vec<f32> = search_range.clone().map(|i| mags[i]).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];

    if median < 1e-6 {
        return vec![];
    }

    let threshold = median * threshold_factor;

    let mut peaks = Vec::new();
    for i in search_range {
        let prev = mags[i - 1];
        let next = mags[(i + 1).min(half - 1)];
        if mags[i] > threshold && mags[i] >= prev && mags[i] >= next {
            peaks.push(i);
            let mirror = n - i;
            if mirror < n && mirror != i {
                peaks.push(mirror);
            }
        }
    }

    peaks
}
