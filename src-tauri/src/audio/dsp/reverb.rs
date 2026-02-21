use std::sync::atomic::{AtomicU32, Ordering};

use super::filters::BiquadFilter;

/// Algorithmic reverb node inspired by Freeverb / Schroeder.
/// Uses parallel comb filters fed into series all-pass filters.
pub struct ReverbNode {
    room_size_bits: AtomicU32,
    damping_bits: AtomicU32,
    predelay_ms_bits: AtomicU32,
    lowpass_freq_bits: AtomicU32,
    decay_bits: AtomicU32,
    wet_mix_bits: AtomicU32,

    sample_rate: f32,
    combs_l: Vec<CombFilter>,
    combs_r: Vec<CombFilter>,
    allpasses_l: Vec<AllPassFilter>,
    allpasses_r: Vec<AllPassFilter>,
    predelay_buffer_l: Vec<f32>,
    predelay_buffer_r: Vec<f32>,
    predelay_pos: usize,
    predelay_len: usize,
    lp_left: BiquadFilter,
    lp_right: BiquadFilter,
    needs_update: std::sync::atomic::AtomicBool,
}

/// Preset reverb configurations.
#[derive(Clone, Debug)]
pub struct ReverbPreset {
    pub name: &'static str,
    pub room_size: f32,
    pub damping: f32,
    pub predelay_ms: f32,
    pub lowpass_filter: f32,
    pub decay: f32,
    pub wet_mix: f32,
}

pub const PRESET_STUDIO: ReverbPreset = ReverbPreset {
    name: "Estudio",
    room_size: 0.3,
    damping: 0.6,
    predelay_ms: 5.0,
    lowpass_filter: 8000.0,
    decay: 0.3,
    wet_mix: 0.15,
};

pub const PRESET_LARGE_ROOM: ReverbPreset = ReverbPreset {
    name: "Sala Grande",
    room_size: 0.75,
    damping: 0.4,
    predelay_ms: 20.0,
    lowpass_filter: 6000.0,
    decay: 0.6,
    wet_mix: 0.3,
};

pub const PRESET_CLUB: ReverbPreset = ReverbPreset {
    name: "Club",
    room_size: 0.55,
    damping: 0.5,
    predelay_ms: 12.0,
    lowpass_filter: 7000.0,
    decay: 0.45,
    wet_mix: 0.25,
};

pub const PRESET_CHURCH: ReverbPreset = ReverbPreset {
    name: "Iglesia",
    room_size: 0.9,
    damping: 0.25,
    predelay_ms: 35.0,
    lowpass_filter: 4500.0,
    decay: 0.8,
    wet_mix: 0.4,
};

pub fn get_preset(name: &str) -> Option<&'static ReverbPreset> {
    let normalized = name.trim().to_lowercase();
    match normalized.as_str() {
        "estudio" | "studio" => Some(&PRESET_STUDIO),
        "sala grande" | "large room" => Some(&PRESET_LARGE_ROOM),
        "club" => Some(&PRESET_CLUB),
        "iglesia" | "church" => Some(&PRESET_CHURCH),
        _ => None,
    }
}

/// Comb filter lengths in samples at 44100 Hz (Freeverb reference).
/// These are scaled to the actual sample rate at runtime.
const COMB_LENGTHS_REF: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_LENGTHS_REF: [usize; 4] = [556, 441, 341, 225];
const REF_RATE: f32 = 44100.0;
/// Small stereo spread offset to decorrelate L/R channels.
const STEREO_SPREAD: usize = 23;

fn scale_len(base: usize, sample_rate: f32) -> usize {
    ((base as f32 * sample_rate / REF_RATE) as usize).max(1)
}

struct CombFilter {
    buffer: Vec<f32>,
    pos: usize,
    feedback: f32,
    damp1: f32,
    damp2: f32,
    filter_state: f32,
}

impl CombFilter {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            pos: 0,
            feedback: 0.5,
            damp1: 0.5,
            damp2: 0.5,
            filter_state: 0.0,
        }
    }

    fn set_params(&mut self, feedback: f32, damp: f32) {
        self.feedback = feedback;
        self.damp1 = damp;
        self.damp2 = 1.0 - damp;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.pos];
        self.filter_state = output * self.damp2 + self.filter_state * self.damp1;
        self.buffer[self.pos] = input + self.filter_state * self.feedback;
        self.pos = (self.pos + 1) % self.buffer.len();
        output
    }
}

struct AllPassFilter {
    buffer: Vec<f32>,
    pos: usize,
}

impl AllPassFilter {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            pos: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.pos];
        let output = -input + buffered;
        self.buffer[self.pos] = input + buffered * 0.5;
        self.pos = (self.pos + 1) % self.buffer.len();
        output
    }
}

impl ReverbNode {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(8_000.0);

        let combs_l: Vec<CombFilter> = COMB_LENGTHS_REF
            .iter()
            .map(|&len| CombFilter::new(scale_len(len, sr)))
            .collect();
        let combs_r: Vec<CombFilter> = COMB_LENGTHS_REF
            .iter()
            .map(|&len| CombFilter::new(scale_len(len + STEREO_SPREAD, sr)))
            .collect();
        let allpasses_l: Vec<AllPassFilter> = ALLPASS_LENGTHS_REF
            .iter()
            .map(|&len| AllPassFilter::new(scale_len(len, sr)))
            .collect();
        let allpasses_r: Vec<AllPassFilter> = ALLPASS_LENGTHS_REF
            .iter()
            .map(|&len| AllPassFilter::new(scale_len(len + STEREO_SPREAD, sr)))
            .collect();

        let predelay_len = 1; // will be recomputed on first update
        let mut lp_left = BiquadFilter::new();
        let mut lp_right = BiquadFilter::new();
        lp_left.set_low_pass(sr, 8000.0, 0.707);
        lp_right.set_low_pass(sr, 8000.0, 0.707);

        Self {
            room_size_bits: AtomicU32::new(0.5_f32.to_bits()),
            damping_bits: AtomicU32::new(0.5_f32.to_bits()),
            predelay_ms_bits: AtomicU32::new(10.0_f32.to_bits()),
            lowpass_freq_bits: AtomicU32::new(8000.0_f32.to_bits()),
            decay_bits: AtomicU32::new(0.5_f32.to_bits()),
            wet_mix_bits: AtomicU32::new(0.0_f32.to_bits()),
            sample_rate: sr,
            combs_l,
            combs_r,
            allpasses_l,
            allpasses_r,
            predelay_buffer_l: vec![0.0; predelay_len],
            predelay_buffer_r: vec![0.0; predelay_len],
            predelay_pos: 0,
            predelay_len,
            lp_left,
            lp_right,
            needs_update: std::sync::atomic::AtomicBool::new(true),
        }
    }

    pub fn set_room_size(&self, val: f32) {
        self.room_size_bits
            .store(val.clamp(0.0, 1.0).to_bits(), Ordering::SeqCst);
        self.needs_update
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_damping(&self, val: f32) {
        self.damping_bits
            .store(val.clamp(0.0, 1.0).to_bits(), Ordering::SeqCst);
        self.needs_update
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_predelay_ms(&self, val: f32) {
        self.predelay_ms_bits
            .store(val.clamp(0.0, 200.0).to_bits(), Ordering::SeqCst);
        self.needs_update
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_lowpass_filter(&self, freq: f32) {
        self.lowpass_freq_bits
            .store(freq.clamp(200.0, 20_000.0).to_bits(), Ordering::SeqCst);
        self.needs_update
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_decay(&self, val: f32) {
        self.decay_bits
            .store(val.clamp(0.0, 1.0).to_bits(), Ordering::SeqCst);
        self.needs_update
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_wet_mix(&self, val: f32) {
        self.wet_mix_bits
            .store(val.clamp(0.0, 1.0).to_bits(), Ordering::SeqCst);
    }

    pub fn load_preset(&self, preset: &ReverbPreset) {
        self.set_room_size(preset.room_size);
        self.set_damping(preset.damping);
        self.set_predelay_ms(preset.predelay_ms);
        self.set_lowpass_filter(preset.lowpass_filter);
        self.set_decay(preset.decay);
        self.set_wet_mix(preset.wet_mix);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sr = sample_rate.max(8_000.0);
        if (sr - self.sample_rate).abs() > f32::EPSILON {
            self.sample_rate = sr;
            // Rebuild comb and allpass with new lengths
            self.combs_l = COMB_LENGTHS_REF
                .iter()
                .map(|&len| CombFilter::new(scale_len(len, sr)))
                .collect();
            self.combs_r = COMB_LENGTHS_REF
                .iter()
                .map(|&len| CombFilter::new(scale_len(len + STEREO_SPREAD, sr)))
                .collect();
            self.allpasses_l = ALLPASS_LENGTHS_REF
                .iter()
                .map(|&len| AllPassFilter::new(scale_len(len, sr)))
                .collect();
            self.allpasses_r = ALLPASS_LENGTHS_REF
                .iter()
                .map(|&len| AllPassFilter::new(scale_len(len + STEREO_SPREAD, sr)))
                .collect();
            self.needs_update
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn process_stereo_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        if self
            .needs_update
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            self.recalculate();
        }

        let wet_mix = f32::from_bits(self.wet_mix_bits.load(Ordering::Relaxed));
        if wet_mix < f32::EPSILON {
            return (left, right);
        }

        // Predelay
        let pre_l = self.predelay_buffer_l[self.predelay_pos];
        let pre_r = self.predelay_buffer_r[self.predelay_pos];
        self.predelay_buffer_l[self.predelay_pos] = left;
        self.predelay_buffer_r[self.predelay_pos] = right;
        self.predelay_pos = (self.predelay_pos + 1) % self.predelay_len;

        // Parallel comb filters
        let mut wet_l = 0.0_f32;
        let mut wet_r = 0.0_f32;
        for comb in &mut self.combs_l {
            wet_l += comb.process(pre_l);
        }
        for comb in &mut self.combs_r {
            wet_r += comb.process(pre_r);
        }

        // Series all-pass filters
        for ap in &mut self.allpasses_l {
            wet_l = ap.process(wet_l);
        }
        for ap in &mut self.allpasses_r {
            wet_r = ap.process(wet_r);
        }

        // Lowpass
        wet_l = self.lp_left.process_sample(wet_l);
        wet_r = self.lp_right.process_sample(wet_r);

        let dry = 1.0 - wet_mix;
        (left * dry + wet_l * wet_mix, right * dry + wet_r * wet_mix)
    }

    fn recalculate(&mut self) {
        let room = f32::from_bits(self.room_size_bits.load(Ordering::Relaxed));
        let damp = f32::from_bits(self.damping_bits.load(Ordering::Relaxed));
        let decay = f32::from_bits(self.decay_bits.load(Ordering::Relaxed));
        let predelay_ms = f32::from_bits(self.predelay_ms_bits.load(Ordering::Relaxed));
        let lp_freq = f32::from_bits(self.lowpass_freq_bits.load(Ordering::Relaxed));

        // feedback = room_size * decay (scaled into useful range 0..0.98)
        let feedback = (room * 0.28 + 0.7) * decay;
        let feedback = feedback.clamp(0.0, 0.98);

        for comb in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            comb.set_params(feedback, damp);
        }

        // Resize predelay buffer
        let new_len = ((predelay_ms / 1000.0) * self.sample_rate).ceil() as usize;
        let new_len = new_len.max(1);
        if new_len != self.predelay_len {
            self.predelay_buffer_l = vec![0.0; new_len];
            self.predelay_buffer_r = vec![0.0; new_len];
            self.predelay_pos = 0;
            self.predelay_len = new_len;
        }

        self.lp_left.set_low_pass(self.sample_rate, lp_freq, 0.707);
        self.lp_right
            .set_low_pass(self.sample_rate, lp_freq, 0.707);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverb_dry_is_passthrough() {
        let mut reverb = ReverbNode::new(48_000.0);
        reverb.set_wet_mix(0.0);
        let (l, r) = reverb.process_stereo_frame(0.5, -0.3);
        assert!((l - 0.5).abs() < f32::EPSILON);
        assert!((r - (-0.3)).abs() < f32::EPSILON);
    }

    #[test]
    fn reverb_wet_produces_tail() {
        let mut reverb = ReverbNode::new(48_000.0);
        reverb.set_wet_mix(0.5);
        reverb.set_predelay_ms(1.0);
        // Feed an impulse then silence; comb filter lengths at 48kHz are ~1200+ samples
        let _ = reverb.process_stereo_frame(1.0, 1.0);
        for _ in 0..2000 {
            let _ = reverb.process_stereo_frame(0.0, 0.0);
        }
        let (l, r) = reverb.process_stereo_frame(0.0, 0.0);
        // After feeding silence, the reverb tail should still have some energy
        assert!(
            l.abs() > 0.0001 || r.abs() > 0.0001,
            "expected reverb tail energy, got l={l} r={r}"
        );
    }

    #[test]
    fn preset_loading_sets_params() {
        let reverb = ReverbNode::new(48_000.0);
        reverb.load_preset(&PRESET_CHURCH);
        let room = f32::from_bits(reverb.room_size_bits.load(Ordering::Relaxed));
        let wet = f32::from_bits(reverb.wet_mix_bits.load(Ordering::Relaxed));
        assert!((room - 0.9).abs() < f32::EPSILON);
        assert!((wet - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn get_preset_resolves_known_names() {
        assert!(get_preset("Estudio").is_some());
        assert!(get_preset("studio").is_some());
        assert!(get_preset("Sala Grande").is_some());
        assert!(get_preset("Club").is_some());
        assert!(get_preset("Iglesia").is_some());
        assert!(get_preset("church").is_some());
        assert!(get_preset("Unknown").is_none());
    }

    #[test]
    fn reverb_params_are_clamped() {
        let reverb = ReverbNode::new(48_000.0);
        reverb.set_room_size(5.0);
        reverb.set_damping(-1.0);
        reverb.set_predelay_ms(999.0);
        reverb.set_lowpass_filter(50.0);
        reverb.set_decay(2.0);
        reverb.set_wet_mix(-0.5);
        assert_eq!(
            f32::from_bits(reverb.room_size_bits.load(Ordering::Relaxed)),
            1.0
        );
        assert_eq!(
            f32::from_bits(reverb.damping_bits.load(Ordering::Relaxed)),
            0.0
        );
        assert_eq!(
            f32::from_bits(reverb.predelay_ms_bits.load(Ordering::Relaxed)),
            200.0
        );
        assert_eq!(
            f32::from_bits(reverb.lowpass_freq_bits.load(Ordering::Relaxed)),
            200.0
        );
        assert_eq!(
            f32::from_bits(reverb.decay_bits.load(Ordering::Relaxed)),
            1.0
        );
        assert_eq!(
            f32::from_bits(reverb.wet_mix_bits.load(Ordering::Relaxed)),
            0.0
        );
    }
}
