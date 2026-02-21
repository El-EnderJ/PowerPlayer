use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::filters::BiquadFilter;

/// Speed of sound in air (m/s).
const SPEED_OF_SOUND: f32 = 343.0;
/// Maximum ITD delay in samples (capped to avoid excessive buffer usage).
const MAX_DELAY_SAMPLES: usize = 128;
/// Number of early reflection taps per source.
const NUM_REFLECTIONS: usize = 6;

/// Names for the four stem sources used in spatial positioning.
pub const SOURCE_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];

/// 3-D position in the virtual room.
#[derive(Clone, Copy, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn distance_to(&self, other: &Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Azimuth angle (radians) from `self` looking towards `other`, projected on the XY plane.
    fn azimuth_to(&self, other: &Vec3) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        dy.atan2(dx)
    }
}

/// Parameters for a single sound source inside the virtual room.
struct SpatialSource {
    /// Atomic x, y, z packed as f32 bits for lock-free updates.
    x_bits: AtomicU32,
    y_bits: AtomicU32,
    z_bits: AtomicU32,
    active: AtomicBool,

    // Per-source processing state (mutated only on the audio thread)
    delay_line_l: Vec<f32>,
    delay_line_r: Vec<f32>,
    delay_pos: usize,
    itd_delay_l: usize,
    itd_delay_r: usize,
    gain_l: f32,
    gain_r: f32,
    /// Simple low-pass filter for ILD shadow on the far ear.
    shadow_filter_l: BiquadFilter,
    shadow_filter_r: BiquadFilter,
    /// Early reflection taps (delay in samples, attenuation).
    reflection_taps: Vec<(usize, f32)>,
    reflection_buffer_l: Vec<f32>,
    reflection_buffer_r: Vec<f32>,
    reflection_pos: usize,
}

impl SpatialSource {
    fn new(pos: Vec3) -> Self {
        let max_ref_delay = 4800_usize; // ~100 ms at 48 kHz
        Self {
            x_bits: AtomicU32::new(pos.x.to_bits()),
            y_bits: AtomicU32::new(pos.y.to_bits()),
            z_bits: AtomicU32::new(pos.z.to_bits()),
            active: AtomicBool::new(true),
            delay_line_l: vec![0.0; MAX_DELAY_SAMPLES],
            delay_line_r: vec![0.0; MAX_DELAY_SAMPLES],
            delay_pos: 0,
            itd_delay_l: 0,
            itd_delay_r: 0,
            gain_l: 1.0,
            gain_r: 1.0,
            shadow_filter_l: BiquadFilter::new(),
            shadow_filter_r: BiquadFilter::new(),
            reflection_taps: Vec::new(),
            reflection_buffer_l: vec![0.0; max_ref_delay],
            reflection_buffer_r: vec![0.0; max_ref_delay],
            reflection_pos: 0,
        }
    }

    fn position(&self) -> Vec3 {
        Vec3 {
            x: f32::from_bits(self.x_bits.load(Ordering::Relaxed)),
            y: f32::from_bits(self.y_bits.load(Ordering::Relaxed)),
            z: f32::from_bits(self.z_bits.load(Ordering::Relaxed)),
        }
    }

    fn set_position(&self, pos: Vec3) {
        self.x_bits.store(pos.x.to_bits(), Ordering::SeqCst);
        self.y_bits.store(pos.y.to_bits(), Ordering::SeqCst);
        self.z_bits.store(pos.z.to_bits(), Ordering::SeqCst);
    }
}

/// Virtual room for spatial audio processing using simplified HRTF (binaural pan).
///
/// Processing chain per source:
///   1. Compute ITD (inter-aural time difference) from azimuth → per-ear delay.
///   2. Compute ILD (inter-aural level difference) → per-ear gain + head-shadow LP filter.
///   3. Distance attenuation (inverse-distance).
///   4. Early reflections from virtual walls.
///
/// The node accepts a normal stereo frame and outputs a binaural stereo frame.
pub struct SpatialRoomNode {
    enabled: AtomicBool,
    needs_update: AtomicBool,

    // Room dimensions in metres
    width_bits: AtomicU32,
    length_bits: AtomicU32,
    height_bits: AtomicU32,
    damping_bits: AtomicU32,

    sample_rate: f32,

    /// Listener is always at the centre of the room by default.
    listener: Vec3,

    /// Four sources: Vocals (0), Drums (1), Bass (2), Other (3).
    sources: Vec<SpatialSource>,
}

impl SpatialRoomNode {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(8_000.0);
        let default_width: f32 = 8.0;
        let default_length: f32 = 10.0;
        let default_height: f32 = 3.5;

        let listener = Vec3::new(default_width / 2.0, default_length / 2.0, 1.7);

        // Default positions: front-left, front-right, front-center, rear
        let default_positions = [
            Vec3::new(2.0, 7.0, 1.7), // Vocals: front-center-left
            Vec3::new(6.0, 7.0, 1.7), // Drums: front-center-right
            Vec3::new(4.0, 8.0, 1.7), // Bass: front-center
            Vec3::new(4.0, 3.0, 1.7), // Other: rear
        ];

        let sources: Vec<SpatialSource> = default_positions
            .iter()
            .map(|pos| SpatialSource::new(*pos))
            .collect();

        let mut node = Self {
            enabled: AtomicBool::new(false),
            needs_update: AtomicBool::new(true),
            width_bits: AtomicU32::new(default_width.to_bits()),
            length_bits: AtomicU32::new(default_length.to_bits()),
            height_bits: AtomicU32::new(default_height.to_bits()),
            damping_bits: AtomicU32::new(0.5_f32.to_bits()),
            sample_rate: sr,
            listener,
            sources,
        };
        node.recalculate();
        node
    }

    // ── Public setters (lock-free, called from UI thread) ──────────────

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_room_size(&self, width: f32, length: f32, height: f32) {
        self.width_bits
            .store(width.clamp(2.0, 50.0).to_bits(), Ordering::SeqCst);
        self.length_bits
            .store(length.clamp(2.0, 50.0).to_bits(), Ordering::SeqCst);
        self.height_bits
            .store(height.clamp(2.0, 20.0).to_bits(), Ordering::SeqCst);
        self.needs_update.store(true, Ordering::SeqCst);
    }

    pub fn set_damping(&self, val: f32) {
        self.damping_bits
            .store(val.clamp(0.0, 1.0).to_bits(), Ordering::SeqCst);
        self.needs_update.store(true, Ordering::SeqCst);
    }

    pub fn set_source_position(&self, index: usize, x: f32, y: f32, z: f32) {
        if let Some(src) = self.sources.get(index) {
            src.set_position(Vec3::new(x, y, z));
            self.needs_update.store(true, Ordering::SeqCst);
        }
    }

    pub fn set_source_active(&self, index: usize, active: bool) {
        if let Some(src) = self.sources.get(index) {
            src.active.store(active, Ordering::SeqCst);
        }
    }

    pub fn source_positions(&self) -> Vec<(f32, f32, f32, bool)> {
        self.sources
            .iter()
            .map(|s| {
                let p = s.position();
                (p.x, p.y, p.z, s.active.load(Ordering::Relaxed))
            })
            .collect()
    }

    /// Distributes 4 sources in a 180° arc in front of the listener,
    /// ordered by approximate frequency content (bass → other).
    pub fn auto_orchestra(&self) {
        let w = f32::from_bits(self.width_bits.load(Ordering::Relaxed));
        let l = f32::from_bits(self.length_bits.load(Ordering::Relaxed));
        let h = f32::from_bits(self.height_bits.load(Ordering::Relaxed));
        let cx = w / 2.0;
        let cy = l / 2.0;
        let radius = (w.min(l) / 2.0) * 0.75;

        // Order by typical spectral content: bass, drums, other, vocals
        // Arc angles: -90° (left) to +90° (right) mapped to 4 positions
        let order: [usize; 4] = [2, 1, 3, 0]; // bass, drums, other, vocals
        let angles: [f32; 4] = [-60.0, -20.0, 20.0, 60.0];

        for (slot, &src_idx) in order.iter().enumerate() {
            let angle_rad = angles[slot].to_radians();
            let x = cx + radius * angle_rad.sin();
            let y = cy + radius * angle_rad.cos();
            let z = (h * 0.5).min(2.0);
            if let Some(src) = self.sources.get(src_idx) {
                src.set_position(Vec3::new(x, y, z));
            }
        }
        self.needs_update.store(true, Ordering::SeqCst);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sr = sample_rate.max(8_000.0);
        if (sr - self.sample_rate).abs() > f32::EPSILON {
            self.sample_rate = sr;
            self.needs_update.store(true, Ordering::SeqCst);
        }
    }

    // ── Audio-thread processing ────────────────────────────────────────

    /// Process a single stereo frame.  When spatial mode is disabled the
    /// function returns `(left, right)` immediately (zero CPU cost bypass).
    pub fn process_stereo_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        if !self.enabled.load(Ordering::Relaxed) {
            return (left, right);
        }

        if self.needs_update.swap(false, Ordering::SeqCst) {
            self.recalculate();
        }

        // Mix the input mono-down per source, then spatialise each independently.
        let mono = (left + right) * 0.5;
        let mut out_l = 0.0_f32;
        let mut out_r = 0.0_f32;

        for src in &mut self.sources {
            if !src.active.load(Ordering::Relaxed) {
                continue;
            }

            // ── Direct path with ITD delay ──
            let dl_len = src.delay_line_l.len();
            src.delay_line_l[src.delay_pos] = mono;
            src.delay_line_r[src.delay_pos] = mono;

            let read_l = (src.delay_pos + dl_len - src.itd_delay_l) % dl_len;
            let read_r = (src.delay_pos + dl_len - src.itd_delay_r) % dl_len;

            let direct_l = src.delay_line_l[read_l] * src.gain_l;
            let direct_r = src.delay_line_r[read_r] * src.gain_r;

            // Apply head-shadow low-pass on the far ear
            let direct_l = src.shadow_filter_l.process_sample(direct_l);
            let direct_r = src.shadow_filter_r.process_sample(direct_r);

            out_l += direct_l;
            out_r += direct_r;

            // ── Early reflections ──
            let rb_len = src.reflection_buffer_l.len();
            src.reflection_buffer_l[src.reflection_pos] = mono;
            src.reflection_buffer_r[src.reflection_pos] = mono;

            for &(tap_delay, tap_gain) in &src.reflection_taps {
                let tap_idx = (src.reflection_pos + rb_len - tap_delay) % rb_len;
                out_l += src.reflection_buffer_l[tap_idx] * tap_gain * src.gain_l;
                out_r += src.reflection_buffer_r[tap_idx] * tap_gain * src.gain_r;
            }

            src.delay_pos = (src.delay_pos + 1) % dl_len;
            src.reflection_pos = (src.reflection_pos + 1) % rb_len;
        }

        (out_l, out_r)
    }

    // ── Internal recalculation ─────────────────────────────────────────

    fn recalculate(&mut self) {
        let width = f32::from_bits(self.width_bits.load(Ordering::Relaxed));
        let length = f32::from_bits(self.length_bits.load(Ordering::Relaxed));
        let height = f32::from_bits(self.height_bits.load(Ordering::Relaxed));
        let damping = f32::from_bits(self.damping_bits.load(Ordering::Relaxed));
        let listener = self.listener;
        let sr = self.sample_rate;

        // Approximate head radius for ITD computation (Woodworth formula).
        let head_radius: f32 = 0.0875; // metres

        for src in &mut self.sources {
            let pos = src.position();
            let dist = listener.distance_to(&pos).max(0.1);
            let azimuth = listener.azimuth_to(&pos); // radians

            // ── ITD (inter-aural time difference) ──
            // Woodworth approximation: ITD = (r/c) * (sin(θ) + θ)  for |θ| ≤ π/2
            let abs_az = azimuth.abs().min(std::f32::consts::FRAC_PI_2);
            let itd_seconds = (head_radius / SPEED_OF_SOUND) * (abs_az.sin() + abs_az);
            let itd_samples = (itd_seconds * sr).round() as usize;
            let itd_clamped = itd_samples.min(MAX_DELAY_SAMPLES - 1);

            if azimuth >= 0.0 {
                // Source is to the right → right ear is nearer
                src.itd_delay_l = itd_clamped;
                src.itd_delay_r = 0;
            } else {
                src.itd_delay_l = 0;
                src.itd_delay_r = itd_clamped;
            }

            // ── ILD (inter-aural level difference) ──
            // Simplified: up to ~6 dB attenuation on the far ear at 90°.
            let ild_db = 6.0 * abs_az.sin();
            let near_gain = 1.0 / dist; // inverse-distance attenuation
            let far_gain = near_gain * 10.0_f32.powf(-ild_db / 20.0);

            if azimuth >= 0.0 {
                src.gain_r = near_gain;
                src.gain_l = far_gain;
            } else {
                src.gain_l = near_gain;
                src.gain_r = far_gain;
            }

            // Head-shadow low-pass: cut-off frequency lowers as azimuth increases.
            let shadow_cutoff = 20_000.0 - 12_000.0 * abs_az.sin();
            let shadow_cutoff = shadow_cutoff.clamp(2_000.0, 20_000.0);
            if azimuth >= 0.0 {
                src.shadow_filter_l
                    .set_low_pass(sr, shadow_cutoff, 0.707);
                // Near ear gets identity (no filtering)
                src.shadow_filter_r
                    .set_low_pass(sr, 20_000.0, 0.707);
            } else {
                src.shadow_filter_r
                    .set_low_pass(sr, shadow_cutoff, 0.707);
                src.shadow_filter_l
                    .set_low_pass(sr, 20_000.0, 0.707);
            }

            // ── Early reflections ──
            // Compute image sources for 6 walls (±x, ±y, ±z).
            let walls: [(Vec3, f32); NUM_REFLECTIONS] = [
                // left wall  (x=0)
                (Vec3::new(-pos.x, pos.y, pos.z), width),
                // right wall (x=width)
                (Vec3::new(2.0 * width - pos.x, pos.y, pos.z), width),
                // front wall (y=length)
                (Vec3::new(pos.x, 2.0 * length - pos.y, pos.z), length),
                // back wall  (y=0)
                (Vec3::new(pos.x, -pos.y, pos.z), length),
                // ceiling    (z=height)
                (Vec3::new(pos.x, pos.y, 2.0 * height - pos.z), height),
                // floor      (z=0)
                (Vec3::new(pos.x, pos.y, -pos.z), height),
            ];

            src.reflection_taps.clear();
            let max_buf = src.reflection_buffer_l.len();
            for (image, _dim) in &walls {
                let ref_dist = listener.distance_to(image).max(0.1);
                let delay_sec = ref_dist / SPEED_OF_SOUND;
                let delay_samples = (delay_sec * sr).round() as usize;
                if delay_samples == 0 || delay_samples >= max_buf {
                    continue;
                }
                // Attenuation = 1/distance × (1 - damping) to simulate absorption
                let atten = (1.0 / ref_dist) * (1.0 - damping * 0.7);
                src.reflection_taps.push((delay_samples, atten.max(0.0)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_spatial_is_passthrough() {
        let mut node = SpatialRoomNode::new(48_000.0);
        node.set_enabled(false);
        let (l, r) = node.process_stereo_frame(0.5, -0.3);
        assert!((l - 0.5).abs() < f32::EPSILON);
        assert!((r - (-0.3)).abs() < f32::EPSILON);
    }

    #[test]
    fn enabled_spatial_produces_output() {
        let mut node = SpatialRoomNode::new(48_000.0);
        node.set_enabled(true);
        // Process several frames to fill delay lines
        for _ in 0..200 {
            node.process_stereo_frame(0.5, 0.5);
        }
        let (l, r) = node.process_stereo_frame(0.5, 0.5);
        assert!(l.is_finite());
        assert!(r.is_finite());
        // Output should be non-zero when sources are active
        assert!(l.abs() > 0.0 || r.abs() > 0.0);
    }

    #[test]
    fn room_size_is_clamped() {
        let node = SpatialRoomNode::new(48_000.0);
        node.set_room_size(0.5, 0.5, 0.5);
        assert_eq!(f32::from_bits(node.width_bits.load(Ordering::Relaxed)), 2.0);
        assert_eq!(
            f32::from_bits(node.length_bits.load(Ordering::Relaxed)),
            2.0
        );
        assert_eq!(
            f32::from_bits(node.height_bits.load(Ordering::Relaxed)),
            2.0
        );

        node.set_room_size(100.0, 100.0, 100.0);
        assert_eq!(
            f32::from_bits(node.width_bits.load(Ordering::Relaxed)),
            50.0
        );
        assert_eq!(
            f32::from_bits(node.length_bits.load(Ordering::Relaxed)),
            50.0
        );
        assert_eq!(
            f32::from_bits(node.height_bits.load(Ordering::Relaxed)),
            20.0
        );
    }

    #[test]
    fn source_position_roundtrips() {
        let node = SpatialRoomNode::new(48_000.0);
        node.set_source_position(0, 1.0, 2.0, 3.0);
        let positions = node.source_positions();
        assert_eq!(positions.len(), 4);
        assert!((positions[0].0 - 1.0).abs() < f32::EPSILON);
        assert!((positions[0].1 - 2.0).abs() < f32::EPSILON);
        assert!((positions[0].2 - 3.0).abs() < f32::EPSILON);
        assert!(positions[0].3); // active by default
    }

    #[test]
    fn auto_orchestra_updates_positions() {
        let node = SpatialRoomNode::new(48_000.0);
        let before = node.source_positions();
        node.auto_orchestra();
        let after = node.source_positions();
        // At least some position should have changed
        let changed = before
            .iter()
            .zip(after.iter())
            .any(|(a, b)| (a.0 - b.0).abs() > 0.01 || (a.1 - b.1).abs() > 0.01);
        assert!(changed, "auto_orchestra should move at least one source");
    }

    #[test]
    fn inactive_source_produces_no_output() {
        let mut node = SpatialRoomNode::new(48_000.0);
        node.set_enabled(true);
        // Deactivate all sources
        for i in 0..4 {
            node.set_source_active(i, false);
        }
        for _ in 0..100 {
            node.process_stereo_frame(0.5, 0.5);
        }
        let (l, r) = node.process_stereo_frame(0.5, 0.5);
        assert!(
            l.abs() < f32::EPSILON && r.abs() < f32::EPSILON,
            "all sources inactive should produce silence, got l={l} r={r}"
        );
    }

    #[test]
    fn damping_is_clamped() {
        let node = SpatialRoomNode::new(48_000.0);
        node.set_damping(-1.0);
        assert_eq!(
            f32::from_bits(node.damping_bits.load(Ordering::Relaxed)),
            0.0
        );
        node.set_damping(5.0);
        assert_eq!(
            f32::from_bits(node.damping_bits.load(Ordering::Relaxed)),
            1.0
        );
    }
}
