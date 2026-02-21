use std::sync::atomic::{AtomicU32, Ordering};

use super::filters::BiquadFilter;

/// Independent Tone control with LowShelf (~100 Hz) and HighShelf (~10 kHz) filters,
/// separate from the parametric EQ stages.
pub struct ToneNode {
    bass_gain_bits: AtomicU32,
    treble_gain_bits: AtomicU32,
    sample_rate: f32,
    left_bass: BiquadFilter,
    right_bass: BiquadFilter,
    left_treble: BiquadFilter,
    right_treble: BiquadFilter,
    needs_update: std::sync::atomic::AtomicBool,
}

const BASS_FREQ: f32 = 100.0;
const TREBLE_FREQ: f32 = 10_000.0;
const SHELF_SLOPE: f32 = 1.0;

impl ToneNode {
    pub fn new(sample_rate: f32) -> Self {
        let mut node = Self {
            bass_gain_bits: AtomicU32::new(0.0_f32.to_bits()),
            treble_gain_bits: AtomicU32::new(0.0_f32.to_bits()),
            sample_rate: sample_rate.max(8_000.0),
            left_bass: BiquadFilter::new(),
            right_bass: BiquadFilter::new(),
            left_treble: BiquadFilter::new(),
            right_treble: BiquadFilter::new(),
            needs_update: std::sync::atomic::AtomicBool::new(true),
        };
        node.recalculate();
        node
    }

    pub fn set_bass(&self, gain_db: f32) {
        let clamped = gain_db.clamp(-12.0, 12.0);
        self.bass_gain_bits
            .store(clamped.to_bits(), Ordering::SeqCst);
        self.needs_update
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_treble(&self, gain_db: f32) {
        let clamped = gain_db.clamp(-12.0, 12.0);
        self.treble_gain_bits
            .store(clamped.to_bits(), Ordering::SeqCst);
        self.needs_update
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sr = sample_rate.max(8_000.0);
        if (sr - self.sample_rate).abs() > f32::EPSILON {
            self.sample_rate = sr;
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
        let l = self.left_treble.process_sample(self.left_bass.process_sample(left));
        let r = self.right_treble.process_sample(self.right_bass.process_sample(right));
        (l, r)
    }

    fn recalculate(&mut self) {
        let bass_db = f32::from_bits(self.bass_gain_bits.load(Ordering::Relaxed));
        let treble_db = f32::from_bits(self.treble_gain_bits.load(Ordering::Relaxed));
        self.left_bass
            .set_low_shelf(self.sample_rate, BASS_FREQ, bass_db, SHELF_SLOPE);
        self.right_bass
            .set_low_shelf(self.sample_rate, BASS_FREQ, bass_db, SHELF_SLOPE);
        self.left_treble
            .set_high_shelf(self.sample_rate, TREBLE_FREQ, treble_db, SHELF_SLOPE);
        self.right_treble
            .set_high_shelf(self.sample_rate, TREBLE_FREQ, treble_db, SHELF_SLOPE);
    }
}

/// Stereo balance control. `balance` ranges from -1.0 (full left) to 1.0 (full right).
pub struct BalanceNode {
    balance_bits: AtomicU32,
}

impl BalanceNode {
    pub fn new() -> Self {
        Self {
            balance_bits: AtomicU32::new(0.0_f32.to_bits()),
        }
    }

    pub fn set_balance(&self, balance: f32) {
        let clamped = balance.clamp(-1.0, 1.0);
        self.balance_bits
            .store(clamped.to_bits(), Ordering::SeqCst);
    }

    pub fn process_stereo_frame(&self, left: f32, right: f32) -> (f32, f32) {
        let balance = f32::from_bits(self.balance_bits.load(Ordering::Relaxed));
        let l_gain = 1.0_f32.min(1.0 - balance);
        let r_gain = 1.0_f32.min(1.0 + balance);
        (left * l_gain, right * r_gain)
    }
}

impl Default for BalanceNode {
    fn default() -> Self {
        Self::new()
    }
}

/// Basic crossfeed stereo expansion node. Delays one channel by a few milliseconds,
/// applies a low-pass filter, and mixes it with the opposite channel.
pub struct StereoExpansionNode {
    amount_bits: AtomicU32,
    sample_rate: f32,
    delay_buffer_l: Vec<f32>,
    delay_buffer_r: Vec<f32>,
    delay_pos: usize,
    delay_len: usize,
    lp_left: BiquadFilter,
    lp_right: BiquadFilter,
}

const CROSSFEED_DELAY_MS: f32 = 0.3;
const CROSSFEED_LP_FREQ: f32 = 700.0;

impl StereoExpansionNode {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(8_000.0);
        let delay_len = ((CROSSFEED_DELAY_MS / 1000.0) * sr).ceil() as usize;
        let delay_len = delay_len.max(1);
        let mut lp_left = BiquadFilter::new();
        let mut lp_right = BiquadFilter::new();
        lp_left.set_low_pass(sr, CROSSFEED_LP_FREQ, 0.707);
        lp_right.set_low_pass(sr, CROSSFEED_LP_FREQ, 0.707);
        Self {
            amount_bits: AtomicU32::new(0.0_f32.to_bits()),
            sample_rate: sr,
            delay_buffer_l: vec![0.0; delay_len],
            delay_buffer_r: vec![0.0; delay_len],
            delay_pos: 0,
            delay_len,
            lp_left,
            lp_right,
        }
    }

    pub fn set_amount(&self, amount: f32) {
        let clamped = amount.clamp(0.0, 1.0);
        self.amount_bits
            .store(clamped.to_bits(), Ordering::SeqCst);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sr = sample_rate.max(8_000.0);
        if (sr - self.sample_rate).abs() > f32::EPSILON {
            self.sample_rate = sr;
            self.delay_len = ((CROSSFEED_DELAY_MS / 1000.0) * sr).ceil() as usize;
            self.delay_len = self.delay_len.max(1);
            self.delay_buffer_l = vec![0.0; self.delay_len];
            self.delay_buffer_r = vec![0.0; self.delay_len];
            self.delay_pos = 0;
            self.lp_left.set_low_pass(sr, CROSSFEED_LP_FREQ, 0.707);
            self.lp_right.set_low_pass(sr, CROSSFEED_LP_FREQ, 0.707);
        }
    }

    pub fn process_stereo_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        let amount = f32::from_bits(self.amount_bits.load(Ordering::Relaxed));
        if amount < f32::EPSILON {
            return (left, right);
        }

        // Read delayed samples from the opposite channel
        let delayed_l = self.delay_buffer_l[self.delay_pos];
        let delayed_r = self.delay_buffer_r[self.delay_pos];

        // Write current samples into delay buffer
        self.delay_buffer_l[self.delay_pos] = left;
        self.delay_buffer_r[self.delay_pos] = right;
        self.delay_pos = (self.delay_pos + 1) % self.delay_len;

        // Apply low-pass to the delayed crossfeed
        let cross_l = self.lp_left.process_sample(delayed_r) * amount;
        let cross_r = self.lp_right.process_sample(delayed_l) * amount;

        let out_l = left + cross_l;
        let out_r = right + cross_r;
        (out_l, out_r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_node_flat_is_passthrough() {
        let mut tone = ToneNode::new(48_000.0);
        // bass=0, treble=0 should be near-identity
        let (l, r) = tone.process_stereo_frame(0.5, -0.3);
        assert!((l - 0.5).abs() < 0.01, "expected ~0.5, got {l}");
        assert!((r - (-0.3)).abs() < 0.01, "expected ~-0.3, got {r}");
    }

    #[test]
    fn tone_node_clamps_gain() {
        let tone = ToneNode::new(48_000.0);
        tone.set_bass(20.0);
        tone.set_treble(-20.0);
        // values should be clamped to ±12
        let bass = f32::from_bits(tone.bass_gain_bits.load(Ordering::Relaxed));
        let treble = f32::from_bits(tone.treble_gain_bits.load(Ordering::Relaxed));
        assert_eq!(bass, 12.0);
        assert_eq!(treble, -12.0);
    }

    #[test]
    fn balance_center_is_passthrough() {
        let node = BalanceNode::new();
        let (l, r) = node.process_stereo_frame(0.5, 0.5);
        assert!((l - 0.5).abs() < f32::EPSILON);
        assert!((r - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn balance_hard_left_mutes_right() {
        let node = BalanceNode::new();
        node.set_balance(-1.0);
        let (l, r) = node.process_stereo_frame(0.8, 0.8);
        assert!((l - 0.8).abs() < f32::EPSILON);
        assert!(r.abs() < f32::EPSILON);
    }

    #[test]
    fn balance_hard_right_mutes_left() {
        let node = BalanceNode::new();
        node.set_balance(1.0);
        let (l, r) = node.process_stereo_frame(0.8, 0.8);
        assert!(l.abs() < f32::EPSILON);
        assert!((r - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn expansion_zero_amount_is_passthrough() {
        let mut node = StereoExpansionNode::new(48_000.0);
        node.set_amount(0.0);
        let (l, r) = node.process_stereo_frame(0.5, -0.3);
        assert!((l - 0.5).abs() < f32::EPSILON);
        assert!((r - (-0.3)).abs() < f32::EPSILON);
    }

    #[test]
    fn expansion_nonzero_mixes_crossfeed() {
        let mut node = StereoExpansionNode::new(48_000.0);
        node.set_amount(0.5);
        // Process multiple frames to fill the delay buffer
        for _ in 0..100 {
            node.process_stereo_frame(1.0, 0.0);
        }
        let (l, r) = node.process_stereo_frame(1.0, 0.0);
        // Right channel should pick up some crossfeed from left
        assert!(r.abs() > 0.01, "expected crossfeed in right, got {r}");
    }
}
