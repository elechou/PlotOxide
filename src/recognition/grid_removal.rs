/// Remove grid lines from an RGBA image using spatial median-profile detection.
///
/// Grid lines show up as columns/rows that are consistently darker than their
/// neighbors. By computing the median brightness profile, smoothing to get the
/// local background, and subtracting only the negative dips (darker = grid line),
/// we remove grid lines while preserving data curves.
///
/// This approach works for both linear and log-scale grid spacing because it
/// does not assume periodicity — it directly detects darker columns/rows.
pub fn remove_grid(rgba: &[u8], w: u32, h: u32, strength: f32) -> Vec<u8> {
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

    // Detect and correct vertical grid lines (column-wise)
    let vcorr = spatial_grid_correction(&gray, w, h, false, strength);
    // Detect and correct horizontal grid lines (row-wise)
    let hcorr = spatial_grid_correction(&gray, w, h, true, strength);

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

/// Detect and compute grid correction using spatial median-profile approach.
///
/// `is_horizontal`: true = detect horizontal grid lines (row-wise median),
///                  false = detect vertical grid lines (column-wise median).
///
/// Algorithm:
/// 1. Compute median brightness for each column/row
/// 2. Smooth the profile with Gaussian to get local background trend
/// 3. Grid contribution = profile - background (only negative dips = darker lines)
/// 4. Expand correction to full image
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
    // Sigma ~5% of dimension — large enough to smooth over grid spacing
    let sigma = (profile_len as f32 * 0.05).max(10.0);
    let background = gaussian_smooth_1d(&profile, sigma);

    // 3. Grid contribution = profile - background
    // Only keep negative values (darker than background = grid line)
    let mut grid_dip = vec![0.0f32; profile_len];
    for i in 0..profile_len {
        let diff = profile[i] - background[i];
        if diff < 0.0 {
            grid_dip[i] = diff; // negative
        }
    }

    // 4. Expand to full image correction
    // grid_dip is negative for grid lines → correction is positive (brighten)
    // Map strength so that 0.5 = full compensation (1.0×), 1.0 = over-compensate (2.0×)
    let suppress = strength * 2.0;
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

/// 1D Gaussian smoothing with boundary clamping.
fn gaussian_smooth_1d(data: &[f32], sigma: f32) -> Vec<f32> {
    let n = data.len();
    let radius = (sigma * 3.0).ceil() as usize;
    let mut result = vec![0.0f32; n];

    // Precompute kernel weights
    let mut kernel = Vec::with_capacity(radius + 1);
    for d in 0..=radius {
        kernel.push((-(d as f32).powi(2) / (2.0 * sigma * sigma)).exp());
    }

    for i in 0..n {
        let mut sum = 0.0f32;
        let mut wsum = 0.0f32;
        for d in 0..=radius {
            let weight = kernel[d];
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

    result
}
