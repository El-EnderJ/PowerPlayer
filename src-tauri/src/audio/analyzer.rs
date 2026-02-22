use std::path::Path;

use super::decoder;

pub fn extract_waveform(path: &Path, points: usize) -> Result<Vec<f32>, String> {
    if points == 0 {
        return Ok(Vec::new());
    }
    let decoded = decoder::decode_file(path)?;
    Ok(compute_waveform(&decoded.samples, decoded.channels as usize, points))
}

fn compute_waveform(samples: &[f32], channels: usize, points: usize) -> Vec<f32> {
    if points == 0 {
        return Vec::new();
    }
    if samples.is_empty() || channels == 0 {
        return vec![0.0; points];
    }

    let frames = samples.len() / channels;
    if frames == 0 {
        return vec![0.0; points];
    }

    let mut out = Vec::with_capacity(points);
    for i in 0..points {
        let start = i * frames / points;
        let end = ((i + 1) * frames / points).min(frames);
        if start >= end {
            out.push(0.0);
            continue;
        }

        let mut sum_sq = 0.0_f32;
        let mut count = 0usize;
        for frame in start..end {
            let base = frame * channels;
            let mut mono = 0.0_f32;
            for ch in 0..channels {
                mono += samples[base + ch];
            }
            mono /= channels as f32;
            sum_sq += mono * mono;
            count += 1;
        }

        let rms = if count > 0 {
            (sum_sq / count as f32).sqrt()
        } else {
            0.0
        };
        out.push(rms);
    }

    let max = out.iter().copied().fold(0.0_f32, f32::max);
    if max > 0.0 {
        for value in &mut out {
            *value /= max;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::compute_waveform;

    #[test]
    fn waveform_has_requested_points_and_is_normalized() {
        let samples = vec![0.0_f32, 0.1, 0.3, 0.6, 0.9, 0.2, 0.1, 0.0];
        let out = compute_waveform(&samples, 1, 4);
        assert_eq!(out.len(), 4);
        let max = out.iter().copied().fold(0.0_f32, f32::max);
        assert!((max - 1.0).abs() < 1e-6);
    }

    #[test]
    fn waveform_handles_empty_input() {
        let out = compute_waveform(&[], 2, 5);
        assert_eq!(out, vec![0.0; 5]);
    }
}
