use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const EQ_BANDS_MIN: usize = 10;
const EQ_BANDS_MAX: usize = 15;

#[derive(Clone, Copy)]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
    HighPass,
    LowPass,
}

#[derive(Clone, Copy)]
struct BiquadCoefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoefficients {
    fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

pub struct BiquadFilter {
    coeffs: BiquadCoefficients,
    z1: f32,
    z2: f32,
}

impl BiquadFilter {
    pub fn new() -> Self {
        Self {
            coeffs: BiquadCoefficients::identity(),
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn process_sample(&mut self, sample: f32) -> f32 {
        let y = self.coeffs.b0 * sample + self.z1;
        self.z1 = self.coeffs.b1 * sample - self.coeffs.a1 * y + self.z2;
        self.z2 = self.coeffs.b2 * sample - self.coeffs.a2 * y;
        y
    }

    pub fn set_peaking(&mut self, sample_rate: f32, frequency: f32, gain_db: f32, q_factor: f32) {
        self.coeffs = peaking_coefficients(sample_rate, frequency, gain_db, q_factor);
    }

    pub fn set_low_shelf(&mut self, sample_rate: f32, frequency: f32, gain_db: f32, slope: f32) {
        self.coeffs = low_shelf_coefficients(sample_rate, frequency, gain_db, slope);
    }

    pub fn set_high_shelf(&mut self, sample_rate: f32, frequency: f32, gain_db: f32, slope: f32) {
        self.coeffs = high_shelf_coefficients(sample_rate, frequency, gain_db, slope);
    }

    pub fn set_high_pass(&mut self, sample_rate: f32, frequency: f32, q_factor: f32) {
        self.coeffs = high_pass_coefficients(sample_rate, frequency, q_factor);
    }

    pub fn set_low_pass(&mut self, sample_rate: f32, frequency: f32, q_factor: f32) {
        self.coeffs = low_pass_coefficients(sample_rate, frequency, q_factor);
    }
}

impl Default for BiquadFilter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SoftLimiter {
    threshold: f32,
}

impl SoftLimiter {
    pub fn new() -> Self {
        Self {
            threshold: db_to_gain(-0.1),
        }
    }

    pub fn process_sample(&self, sample: f32) -> f32 {
        let abs = sample.abs();
        if abs <= self.threshold {
            return sample;
        }

        let over = abs - self.threshold;
        let compressed = self.threshold + over / (1.0 + over / (1.0 - self.threshold));
        compressed.min(1.0).copysign(sample)
    }
}

impl Default for SoftLimiter {
    fn default() -> Self {
        Self::new()
    }
}

struct EqBand {
    filter_type: FilterType,
    frequency_bits: AtomicU32,
    gain_db_bits: AtomicU32,
    q_factor_bits: AtomicU32,
}

impl EqBand {
    fn new(filter_type: FilterType, frequency: f32, gain_db: f32, q_factor: f32) -> Self {
        Self {
            filter_type,
            frequency_bits: AtomicU32::new(frequency.to_bits()),
            gain_db_bits: AtomicU32::new(gain_db.to_bits()),
            q_factor_bits: AtomicU32::new(q_factor.to_bits()),
        }
    }

    fn frequency(&self) -> f32 {
        f32::from_bits(self.frequency_bits.load(Ordering::Relaxed))
    }

    fn gain_db(&self) -> f32 {
        f32::from_bits(self.gain_db_bits.load(Ordering::Relaxed))
    }

    fn q_factor(&self) -> f32 {
        f32::from_bits(self.q_factor_bits.load(Ordering::Relaxed))
    }

    fn update(&self, frequency: f32, gain_db: f32, q_factor: f32) -> bool {
        let new_frequency = frequency.to_bits();
        let new_gain = gain_db.to_bits();
        let new_q = q_factor.to_bits();

        let old_frequency = self.frequency_bits.swap(new_frequency, Ordering::SeqCst);
        let old_gain = self.gain_db_bits.swap(new_gain, Ordering::SeqCst);
        let old_q = self.q_factor_bits.swap(new_q, Ordering::SeqCst);

        old_frequency != new_frequency || old_gain != new_gain || old_q != new_q
    }
}

pub struct ParametricEQ {
    sample_rate: f32,
    bands: Vec<EqBand>,
    left_filters: Vec<BiquadFilter>,
    right_filters: Vec<BiquadFilter>,
    needs_recalculation: AtomicBool,
}

impl ParametricEQ {
    pub fn new(bands: usize, sample_rate: f32) -> Self {
        let band_count = bands.clamp(EQ_BANDS_MIN, EQ_BANDS_MAX);
        let mut eq_bands = Vec::with_capacity(band_count);
        for index in 0..band_count {
            eq_bands.push(EqBand::new(
                FilterType::Peaking,
                default_band_frequency(index, band_count),
                0.0,
                1.0,
            ));
        }

        let mut eq = Self {
            sample_rate: sample_rate.max(8_000.0),
            bands: eq_bands,
            left_filters: (0..band_count).map(|_| BiquadFilter::new()).collect(),
            right_filters: (0..band_count).map(|_| BiquadFilter::new()).collect(),
            needs_recalculation: AtomicBool::new(true),
        };
        eq.recalculate_if_needed();
        eq
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sanitized = sample_rate.max(8_000.0);
        if (sanitized - self.sample_rate).abs() > f32::EPSILON {
            self.sample_rate = sanitized;
            self.needs_recalculation.store(true, Ordering::SeqCst);
        }
    }

    pub fn update_band(
        &self,
        index: usize,
        frequency: f32,
        gain_db: f32,
        q_factor: f32,
    ) -> Result<(), String> {
        let Some(band) = self.bands.get(index) else {
            return Err(format!(
                "Band index out of range: {index} (valid: 0 to {})",
                self.bands.len().saturating_sub(1),
            ));
        };

        let frequency = sanitize_frequency(frequency, self.sample_rate);
        let gain_db = gain_db.clamp(-24.0, 24.0);
        let q_factor = sanitize_q(q_factor);

        if band.update(frequency, gain_db, q_factor) {
            self.needs_recalculation.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    pub fn process_stereo_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        self.recalculate_if_needed();
        let mut left_sample = left;
        let mut right_sample = right;
        for filter in &mut self.left_filters {
            left_sample = filter.process_sample(left_sample);
        }
        for filter in &mut self.right_filters {
            right_sample = filter.process_sample(right_sample);
        }
        (left_sample, right_sample)
    }

    fn recalculate_if_needed(&mut self) {
        if !self.needs_recalculation.swap(false, Ordering::SeqCst) {
            return;
        }

        for (index, band) in self.bands.iter().enumerate() {
            let frequency = sanitize_frequency(band.frequency(), self.sample_rate);
            let gain_db = band.gain_db().clamp(-24.0, 24.0);
            let q_factor = sanitize_q(band.q_factor());
            let coeffs = match band.filter_type {
                FilterType::Peaking => {
                    peaking_coefficients(self.sample_rate, frequency, gain_db, q_factor)
                }
                FilterType::LowShelf => {
                    low_shelf_coefficients(self.sample_rate, frequency, gain_db, q_factor)
                }
                FilterType::HighShelf => {
                    high_shelf_coefficients(self.sample_rate, frequency, gain_db, q_factor)
                }
                FilterType::HighPass => {
                    high_pass_coefficients(self.sample_rate, frequency, q_factor)
                }
                FilterType::LowPass => low_pass_coefficients(self.sample_rate, frequency, q_factor),
            };

            self.left_filters[index].coeffs = coeffs;
            self.right_filters[index].coeffs = coeffs;
        }
    }

    /// Returns band parameters as Vec of (frequency, gain_db, q_factor) tuples.
    pub fn get_bands(&self) -> Vec<(f32, f32, f32)> {
        self.bands
            .iter()
            .map(|b| (b.frequency(), b.gain_db(), b.q_factor()))
            .collect()
    }

    /// Computes the combined magnitude response (dB) at logarithmically spaced frequencies.
    /// Returns Vec of (frequency_hz, magnitude_db) pairs.
    pub fn compute_frequency_response(&self, num_points: usize) -> Vec<(f32, f32)> {
        let min_hz: f32 = 20.0;
        let max_hz: f32 = (self.sample_rate * 0.5).min(20_000.0);
        let n = num_points.max(2);
        let mut result = Vec::with_capacity(n);

        for i in 0..n {
            let ratio = i as f32 / (n - 1) as f32;
            let freq = min_hz * (max_hz / min_hz).powf(ratio);
            let w = 2.0 * std::f32::consts::PI * freq / self.sample_rate;
            let cos_w = w.cos();
            let cos_2w = (2.0 * w).cos();

            let mut total_mag_sq: f64 = 1.0;

            for band in &self.bands {
                let band_freq = sanitize_frequency(band.frequency(), self.sample_rate);
                let gain_db = band.gain_db().clamp(-24.0, 24.0);
                let q = sanitize_q(band.q_factor());
                let coeffs = match band.filter_type {
                    FilterType::Peaking => {
                        peaking_coefficients(self.sample_rate, band_freq, gain_db, q)
                    }
                    FilterType::LowShelf => {
                        low_shelf_coefficients(self.sample_rate, band_freq, gain_db, q)
                    }
                    FilterType::HighShelf => {
                        high_shelf_coefficients(self.sample_rate, band_freq, gain_db, q)
                    }
                    FilterType::HighPass => {
                        high_pass_coefficients(self.sample_rate, band_freq, q)
                    }
                    FilterType::LowPass => {
                        low_pass_coefficients(self.sample_rate, band_freq, q)
                    }
                };

                // |H(e^jw)|^2 = (b0^2 + b1^2 + b2^2 + 2*(b0*b1+b1*b2)*cos(w) + 2*b0*b2*cos(2w))
                //              / (1    + a1^2 + a2^2 + 2*(a1+a1*a2)*cos(w)     + 2*a2*cos(2w))
                let num = (coeffs.b0 * coeffs.b0 + coeffs.b1 * coeffs.b1 + coeffs.b2 * coeffs.b2) as f64
                    + 2.0 * (coeffs.b0 * coeffs.b1 + coeffs.b1 * coeffs.b2) as f64 * cos_w as f64
                    + 2.0 * (coeffs.b0 * coeffs.b2) as f64 * cos_2w as f64;
                let den = (1.0 + coeffs.a1 * coeffs.a1 + coeffs.a2 * coeffs.a2) as f64
                    + 2.0 * (coeffs.a1 + coeffs.a1 * coeffs.a2) as f64 * cos_w as f64
                    + 2.0 * coeffs.a2 as f64 * cos_2w as f64;

                if den.abs() > 1e-12 {
                    total_mag_sq *= num / den;
                }
            }

            let mag_db = 10.0 * total_mag_sq.max(1e-12).log10();
            result.push((freq, mag_db as f32));
        }

        result
    }
}

impl Default for ParametricEQ {
    fn default() -> Self {
        Self::new(10, 48_000.0)
    }
}

fn default_band_frequency(index: usize, total: usize) -> f32 {
    let min_hz = 32.0_f32;
    let max_hz = 16_000.0_f32;
    if total <= 1 {
        return min_hz;
    }
    let ratio = index as f32 / (total - 1) as f32;
    min_hz * (max_hz / min_hz).powf(ratio)
}

fn sanitize_frequency(frequency: f32, sample_rate: f32) -> f32 {
    let nyquist = (sample_rate * 0.5) - 1.0;
    frequency.clamp(10.0, nyquist.max(10.0))
}

fn sanitize_q(q_factor: f32) -> f32 {
    q_factor.clamp(0.1, 18.0)
}

fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

fn normalize(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> BiquadCoefficients {
    let inv_a0 = if a0.abs() > f32::EPSILON {
        1.0 / a0
    } else {
        1.0
    };
    BiquadCoefficients {
        b0: b0 * inv_a0,
        b1: b1 * inv_a0,
        b2: b2 * inv_a0,
        a1: a1 * inv_a0,
        a2: a2 * inv_a0,
    }
}

fn peaking_coefficients(
    sample_rate: f32,
    frequency: f32,
    gain_db: f32,
    q_factor: f32,
) -> BiquadCoefficients {
    let w0 = 2.0 * std::f32::consts::PI * sanitize_frequency(frequency, sample_rate) / sample_rate;
    let alpha = w0.sin() / (2.0 * sanitize_q(q_factor));
    let a = db_to_gain(gain_db / 2.0);
    let cos_w0 = w0.cos();

    normalize(
        1.0 + alpha * a,
        -2.0 * cos_w0,
        1.0 - alpha * a,
        1.0 + alpha / a,
        -2.0 * cos_w0,
        1.0 - alpha / a,
    )
}

fn low_shelf_coefficients(
    sample_rate: f32,
    frequency: f32,
    gain_db: f32,
    slope: f32,
) -> BiquadCoefficients {
    let w0 = 2.0 * std::f32::consts::PI * sanitize_frequency(frequency, sample_rate) / sample_rate;
    let a = db_to_gain(gain_db / 2.0);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let s = sanitize_q(slope);
    let alpha = sin_w0 * 0.5 * ((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).sqrt();
    let beta = 2.0 * a.sqrt() * alpha;

    normalize(
        a * ((a + 1.0) - (a - 1.0) * cos_w0 + beta),
        2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
        a * ((a + 1.0) - (a - 1.0) * cos_w0 - beta),
        (a + 1.0) + (a - 1.0) * cos_w0 + beta,
        -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
        (a + 1.0) + (a - 1.0) * cos_w0 - beta,
    )
}

fn high_shelf_coefficients(
    sample_rate: f32,
    frequency: f32,
    gain_db: f32,
    slope: f32,
) -> BiquadCoefficients {
    let w0 = 2.0 * std::f32::consts::PI * sanitize_frequency(frequency, sample_rate) / sample_rate;
    let a = db_to_gain(gain_db / 2.0);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let s = sanitize_q(slope);
    let alpha = sin_w0 * 0.5 * ((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).sqrt();
    let beta = 2.0 * a.sqrt() * alpha;

    normalize(
        a * ((a + 1.0) + (a - 1.0) * cos_w0 + beta),
        -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
        a * ((a + 1.0) + (a - 1.0) * cos_w0 - beta),
        (a + 1.0) - (a - 1.0) * cos_w0 + beta,
        2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
        (a + 1.0) - (a - 1.0) * cos_w0 - beta,
    )
}

fn high_pass_coefficients(sample_rate: f32, frequency: f32, q_factor: f32) -> BiquadCoefficients {
    let w0 = 2.0 * std::f32::consts::PI * sanitize_frequency(frequency, sample_rate) / sample_rate;
    let alpha = w0.sin() / (2.0 * sanitize_q(q_factor));
    let cos_w0 = w0.cos();

    normalize(
        (1.0 + cos_w0) * 0.5,
        -(1.0 + cos_w0),
        (1.0 + cos_w0) * 0.5,
        1.0 + alpha,
        -2.0 * cos_w0,
        1.0 - alpha,
    )
}

fn low_pass_coefficients(sample_rate: f32, frequency: f32, q_factor: f32) -> BiquadCoefficients {
    let w0 = 2.0 * std::f32::consts::PI * sanitize_frequency(frequency, sample_rate) / sample_rate;
    let alpha = w0.sin() / (2.0 * sanitize_q(q_factor));
    let cos_w0 = w0.cos();

    normalize(
        (1.0 - cos_w0) * 0.5,
        1.0 - cos_w0,
        (1.0 - cos_w0) * 0.5,
        1.0 + alpha,
        -2.0 * cos_w0,
        1.0 - alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::{BiquadFilter, ParametricEQ, SoftLimiter};

    #[test]
    fn biquad_stays_finite_after_configuration() {
        let mut filter = BiquadFilter::new();
        filter.set_peaking(48_000.0, 1_000.0, 6.0, 1.0);
        let processed = filter.process_sample(0.5);
        assert!(processed.is_finite());
    }

    #[test]
    fn eq_marks_dirty_only_when_values_change() {
        let eq = ParametricEQ::new(10, 48_000.0);
        let initial_frequency = eq.bands[0].frequency();
        assert!(!eq
            .needs_recalculation
            .load(std::sync::atomic::Ordering::SeqCst));
        eq.update_band(0, initial_frequency, 0.0, 1.0)
            .expect("band should exist");
        assert!(!eq
            .needs_recalculation
            .load(std::sync::atomic::Ordering::SeqCst));
        eq.update_band(0, initial_frequency + 10.0, 1.0, 1.1)
            .expect("band should exist");
        assert!(eq
            .needs_recalculation
            .load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn soft_limiter_caps_extreme_levels() {
        let limiter = SoftLimiter::new();
        assert!(limiter.process_sample(2.0) <= 1.0);
        assert!(limiter.process_sample(-2.0) >= -1.0);
    }
}
