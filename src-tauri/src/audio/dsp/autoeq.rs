#[derive(Clone, Copy)]
pub struct EqBandConfig {
    pub frequency: f32,
    pub gain_db: f32,
    pub q_factor: f32,
}

const TEN_BAND_FREQUENCIES: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];

const FLAT_GAINS: [f32; 10] = [0.0; 10];
// Baseline 10-band compensation derived from AutoEQ-style Harman-target correction profile.
const SONY_WH_1000XM4_GAINS: [f32; 10] = [-2.4, -1.6, -0.8, 0.2, 1.1, 1.8, 2.2, 1.5, 0.4, -0.9];

fn build_profile(gains: [f32; 10]) -> Vec<EqBandConfig> {
    TEN_BAND_FREQUENCIES
        .iter()
        .zip(gains.iter())
        .map(|(frequency, gain_db)| EqBandConfig {
            frequency: *frequency,
            gain_db: *gain_db,
            q_factor: 1.0,
        })
        .collect()
}

pub fn profile_for_model(model: &str) -> Option<Vec<EqBandConfig>> {
    let normalized = model.trim().to_lowercase();
    if normalized.contains("sony wh-1000xm4") || normalized.contains("wh1000xm4") {
        return Some(build_profile(SONY_WH_1000XM4_GAINS));
    }
    if normalized == "flat" || normalized == "harman target" {
        return Some(build_profile(FLAT_GAINS));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{profile_for_model, TEN_BAND_FREQUENCIES};

    #[test]
    fn known_profile_returns_ten_bands() {
        let profile = profile_for_model("Sony WH-1000XM4").expect("profile should exist");
        assert_eq!(profile.len(), 10);
        assert_eq!(profile[0].frequency, TEN_BAND_FREQUENCIES[0]);
    }

    #[test]
    fn unknown_profile_returns_none() {
        assert!(profile_for_model("Unknown Model").is_none());
    }
}
