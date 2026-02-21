use rustfft::{num_complex::Complex, FftPlanner};

const FFT_SIZE: usize = 2048;

/// Computes FFT magnitude spectrum from interleaved stereo audio samples.
/// Returns `FFT_SIZE / 2` magnitude values in dB (normalized).
pub fn compute_spectrum(samples: &[f32]) -> Vec<f32> {
    let mono = to_mono(samples);
    if mono.len() < FFT_SIZE {
        return vec![-100.0; FFT_SIZE / 2];
    }

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    // Take last FFT_SIZE samples and apply Hann window
    let start = mono.len().saturating_sub(FFT_SIZE);
    let mut buffer: Vec<Complex<f32>> = mono[start..start + FFT_SIZE]
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let window =
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
            Complex::new(s * window, 0.0)
        })
        .collect();

    fft.process(&mut buffer);

    // Convert to magnitude in dB (only positive frequencies)
    let half = FFT_SIZE / 2;
    let norm = 1.0 / FFT_SIZE as f32;
    buffer[..half]
        .iter()
        .map(|c| {
            let magnitude = c.norm() * norm;
            20.0 * magnitude.max(1e-10).log10()
        })
        .collect()
}

fn to_mono(interleaved: &[f32]) -> Vec<f32> {
    if interleaved.len() < 2 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(2)
        .map(|ch| (ch[0] + ch.get(1).copied().unwrap_or(ch[0])) * 0.5)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_floor() {
        let result = compute_spectrum(&[]);
        assert_eq!(result.len(), FFT_SIZE / 2);
        assert!(result.iter().all(|&v| v == -100.0));
    }

    #[test]
    fn sine_wave_produces_peak() {
        let sample_rate = 48000.0_f32;
        let freq = 1000.0_f32;
        let num_samples = FFT_SIZE * 2;
        let samples: Vec<f32> = (0..num_samples)
            .flat_map(|i| {
                let t = i as f32 / sample_rate;
                let s = (2.0 * std::f32::consts::PI * freq * t).sin();
                vec![s, s] // stereo interleaved
            })
            .collect();

        let spectrum = compute_spectrum(&samples);
        assert_eq!(spectrum.len(), FFT_SIZE / 2);

        // Find the bin with maximum magnitude
        let expected_bin = (freq / sample_rate * FFT_SIZE as f32) as usize;
        let max_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;

        // Peak should be near the expected frequency bin (±2 bins for windowing)
        assert!(
            (max_bin as i32 - expected_bin as i32).unsigned_abs() <= 2,
            "Expected peak near bin {expected_bin}, got bin {max_bin}"
        );
    }

    #[test]
    fn spectrum_values_are_finite() {
        let samples: Vec<f32> = (0..FFT_SIZE * 2).map(|i| (i as f32 * 0.01).sin()).collect();
        let spectrum = compute_spectrum(&samples);
        assert!(spectrum.iter().all(|v| v.is_finite()));
    }
}
