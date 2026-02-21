use std::path::{Path, PathBuf};

/// The four stem types produced by the separation engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StemKind {
    Vocals,
    Drums,
    Bass,
    Other,
}

impl StemKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            StemKind::Vocals => "vocals",
            StemKind::Drums => "drums",
            StemKind::Bass => "bass",
            StemKind::Other => "other",
        }
    }

    pub fn all() -> &'static [StemKind] {
        &[
            StemKind::Vocals,
            StemKind::Drums,
            StemKind::Bass,
            StemKind::Other,
        ]
    }
}

/// Paths to the four cached stem WAV files for a given track.
#[derive(Clone, Debug)]
pub struct StemPaths {
    pub vocals: PathBuf,
    pub drums: PathBuf,
    pub bass: PathBuf,
    pub other: PathBuf,
}

/// Configuration for the stem separation engine.
pub struct StemSeparator {
    cache_dir: PathBuf,
    /// Whether to prefer GPU execution (true) or CPU-only (false).
    prefer_gpu: bool,
}

/// Progress of an ongoing stem analysis.
#[derive(Clone, Debug, serde::Serialize)]
pub struct StemProgress {
    pub track_id: String,
    pub percent: f32,
    pub stage: String,
}

impl StemSeparator {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            prefer_gpu: true,
        }
    }

    pub fn set_prefer_gpu(&mut self, prefer: bool) {
        self.prefer_gpu = prefer;
    }

    /// Returns the cache directory for a given track (based on SHA-256 hash of path).
    fn track_cache_dir(&self, track_path: &str) -> PathBuf {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(track_path.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        self.cache_dir.join(&hash[..16])
    }

    /// Returns the expected stem file path inside the cache directory.
    fn stem_path(dir: &Path, kind: StemKind) -> PathBuf {
        dir.join(format!("{}.wav", kind.as_str()))
    }

    /// Check whether all four stems are already cached for this track.
    pub fn is_cached(&self, track_path: &str) -> bool {
        let dir = self.track_cache_dir(track_path);
        StemKind::all()
            .iter()
            .all(|kind| Self::stem_path(&dir, *kind).exists())
    }

    /// Get cached stem paths (returns None if not fully cached).
    pub fn cached_paths(&self, track_path: &str) -> Option<StemPaths> {
        let dir = self.track_cache_dir(track_path);
        let vocals = Self::stem_path(&dir, StemKind::Vocals);
        let drums = Self::stem_path(&dir, StemKind::Drums);
        let bass = Self::stem_path(&dir, StemKind::Bass);
        let other = Self::stem_path(&dir, StemKind::Other);

        if vocals.exists() && drums.exists() && bass.exists() && other.exists() {
            Some(StemPaths {
                vocals,
                drums,
                bass,
                other,
            })
        } else {
            None
        }
    }

    /// Analyze a track and produce 4 stems.
    ///
    /// **Step A**: If cached, return paths immediately.
    /// **Step B**: Load audio, split into chunks.
    /// **Step C**: Run ONNX model (or fallback).
    /// **Phase sync**: Ensure stems sum to original.
    ///
    /// The `progress_cb` is called with 0.0..1.0 and a stage description
    /// so the UI can display progress.
    pub fn analyze_spatial_stems(
        &self,
        track_path: &str,
        progress_cb: impl Fn(StemProgress),
    ) -> Result<StemPaths, String> {
        // Step A: cache check
        if let Some(paths) = self.cached_paths(track_path) {
            progress_cb(StemProgress {
                track_id: track_path.to_string(),
                percent: 1.0,
                stage: "Cached".to_string(),
            });
            return Ok(paths);
        }

        let dir = self.track_cache_dir(track_path);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create stem cache dir: {e}"))?;

        progress_cb(StemProgress {
            track_id: track_path.to_string(),
            percent: 0.05,
            stage: "Loading audio...".to_string(),
        });

        // Step B: Load raw audio (stereo f32 PCM) via symphonia
        let (samples, sample_rate, channels) = load_audio_f32(track_path)?;

        progress_cb(StemProgress {
            track_id: track_path.to_string(),
            percent: 0.15,
            stage: "Separating stems...".to_string(),
        });

        // Step C: Attempt ONNX model, fallback to center-cancel
        let stem_buffers = match self.run_onnx_separation(&samples, sample_rate, channels, |p| {
            progress_cb(StemProgress {
                track_id: track_path.to_string(),
                percent: 0.15 + p * 0.7,
                stage: "AI processing...".to_string(),
            });
        }) {
            Ok(buffers) => buffers,
            Err(_) => {
                progress_cb(StemProgress {
                    track_id: track_path.to_string(),
                    percent: 0.2,
                    stage: "Fallback: center cancellation...".to_string(),
                });
                center_cancel_fallback(&samples, channels)?
            }
        };

        progress_cb(StemProgress {
            track_id: track_path.to_string(),
            percent: 0.9,
            stage: "Writing stems...".to_string(),
        });

        // Write stems as 32-bit float WAV
        let kinds = StemKind::all();
        for (i, kind) in kinds.iter().enumerate() {
            let path = Self::stem_path(&dir, *kind);
            write_wav_f32(&path, &stem_buffers[i], sample_rate, channels)?;
        }

        progress_cb(StemProgress {
            track_id: track_path.to_string(),
            percent: 1.0,
            stage: "Complete".to_string(),
        });

        Ok(StemPaths {
            vocals: Self::stem_path(&dir, StemKind::Vocals),
            drums: Self::stem_path(&dir, StemKind::Drums),
            bass: Self::stem_path(&dir, StemKind::Bass),
            other: Self::stem_path(&dir, StemKind::Other),
        })
    }

    /// Attempt to run ONNX-based stem separation.
    ///
    /// This is a structural placeholder: it defines the correct data flow
    /// (chunk audio → build input tensor → run inference → reassemble)
    /// but will return Err if no ONNX runtime is available, triggering
    /// the center-cancellation fallback.
    fn run_onnx_separation(
        &self,
        _samples: &[f32],
        _sample_rate: u32,
        _channels: u16,
        _progress_cb: impl Fn(f32),
    ) -> Result<[Vec<f32>; 4], String> {
        // ONNX runtime integration point.
        // When onnxruntime crate is added:
        //   1. Load model from cache_dir / "spleeter_4stems.onnx"
        //   2. SessionBuilder::new()?.with_execution_providers([CUDAExecutionProvider, CPUExecutionProvider])
        //   3. Chunk input into ~10-20 second windows
        //   4. Run each chunk, accumulate output tensors
        //   5. Normalize and return 4 stem buffers
        Err("ONNX runtime not available – using fallback".to_string())
    }
}

// ── Fallback: Center Cancellation / Side Extraction ────────────────────

/// Mathematical stem separation without AI.
///
/// Technique: For stereo input, the center (mid) channel approximates vocals
/// while the side channel captures panned instruments.
/// Bass is extracted with a low-pass filter on the mid signal.
/// This is a best-effort last-resort when no ONNX model is available.
fn center_cancel_fallback(
    samples: &[f32],
    channels: u16,
) -> Result<[Vec<f32>; 4], String> {
    if channels < 2 {
        return Err("Center cancellation requires stereo input".to_string());
    }

    let frame_count = samples.len() / channels as usize;
    let mut vocals = Vec::with_capacity(samples.len());
    let mut drums = Vec::with_capacity(samples.len());
    let mut bass = Vec::with_capacity(samples.len());
    let mut other = Vec::with_capacity(samples.len());

    // Simple single-pole low-pass for bass extraction
    let bass_alpha = 0.02_f32; // ~140 Hz at 44.1 kHz (f_c ≈ alpha * sr / 2π)
    let mut bass_state_l = 0.0_f32;
    let mut bass_state_r = 0.0_f32;

    for i in 0..frame_count {
        let l = samples[i * 2];
        let r = samples[i * 2 + 1];

        let mid = (l + r) * 0.5;
        let side = (l - r) * 0.5;

        // Bass: low-passed mid
        bass_state_l = bass_state_l + bass_alpha * (mid - bass_state_l);
        bass_state_r = bass_state_r + bass_alpha * (mid - bass_state_r);
        let bass_l = bass_state_l;
        let bass_r = bass_state_r;

        // Vocals: mid minus bass
        let vocal_l = mid - bass_l;
        let vocal_r = mid - bass_r;

        // Other/Drums split from side: roughly even split
        let drum_l = side * 0.5;
        let drum_r = -side * 0.5;
        let other_l = side * 0.5;
        let other_r = -side * 0.5;

        vocals.push(vocal_l);
        vocals.push(vocal_r);
        drums.push(drum_l);
        drums.push(drum_r);
        bass.push(bass_l);
        bass.push(bass_r);
        other.push(other_l);
        other.push(other_r);
    }

    // Phase synchronisation: verify that stems sum to original
    // Adjust "other" stem to absorb any residual for perfect reconstruction.
    for i in 0..samples.len() {
        let sum = vocals[i] + drums[i] + bass[i] + other[i];
        let residual = samples[i] - sum;
        other[i] += residual;
    }

    Ok([vocals, drums, bass, other])
}

// ── Audio I/O helpers ──────────────────────────────────────────────────

/// Load an audio file as interleaved f32 samples using symphonia.
fn load_audio_f32(path: &str) -> Result<(Vec<f32>, u32, u16), String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open audio file {path}: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("Failed to probe audio: {e}"))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or("No default audio track found")?;
    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .unwrap_or(44100);
    let channels = track
        .codec_params
        .channels
        .map(|ch| ch.count() as u16)
        .unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {e}"))?;

    let mut all_samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(format!("Decode error: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("Packet decode error: {e}"))?;

        let spec = *decoded.spec();
        let duration = decoded.capacity();
        let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        all_samples.extend_from_slice(sample_buf.samples());
    }

    Ok((all_samples, sample_rate, channels))
}

/// Write interleaved f32 samples as a 32-bit float WAV file (minimal implementation).
fn write_wav_f32(
    path: &Path,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    use std::io::Write;

    let bits_per_sample: u16 = 32;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_size = (samples.len() * 4) as u32;
    // IEEE float format tag
    let format_tag: u16 = 3;

    let mut buf: Vec<u8> = Vec::with_capacity(44 + data_size as usize);
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16_u32.to_le_bytes());
    buf.extend_from_slice(&format_tag.to_le_bytes());
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    let mut file = std::fs::File::create(path)
        .map_err(|e| format!("Failed to create WAV file {}: {e}", path.display()))?;
    file.write_all(&buf)
        .map_err(|e| format!("Failed to write WAV file {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_cache_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pp-stems-test-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn stem_kind_names() {
        assert_eq!(StemKind::Vocals.as_str(), "vocals");
        assert_eq!(StemKind::Drums.as_str(), "drums");
        assert_eq!(StemKind::Bass.as_str(), "bass");
        assert_eq!(StemKind::Other.as_str(), "other");
    }

    #[test]
    fn not_cached_initially() {
        let sep = StemSeparator::new(temp_cache_dir());
        assert!(!sep.is_cached("/fake/track.flac"));
        assert!(sep.cached_paths("/fake/track.flac").is_none());
    }

    #[test]
    fn center_cancel_produces_four_stems() {
        // Create a simple stereo signal (L=1.0, R=0.5) × 100 frames
        let frames = 100;
        let mut samples = Vec::with_capacity(frames * 2);
        for _ in 0..frames {
            samples.push(1.0_f32);
            samples.push(0.5_f32);
        }
        let stems = center_cancel_fallback(&samples, 2).expect("fallback should succeed");
        assert_eq!(stems.len(), 4);
        for stem in &stems {
            assert_eq!(stem.len(), frames * 2);
        }
    }

    #[test]
    fn center_cancel_perfect_reconstruction() {
        let frames = 200;
        let mut samples = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let t = i as f32 / 48_000.0;
            samples.push((t * 440.0 * std::f32::consts::TAU).sin());
            samples.push((t * 880.0 * std::f32::consts::TAU).sin() * 0.8);
        }
        let stems = center_cancel_fallback(&samples, 2).expect("fallback should succeed");
        // Verify perfect reconstruction
        for i in 0..samples.len() {
            let sum = stems[0][i] + stems[1][i] + stems[2][i] + stems[3][i];
            assert!(
                (sum - samples[i]).abs() < 1e-6,
                "reconstruction error at sample {i}: expected {}, got {sum}",
                samples[i]
            );
        }
    }

    #[test]
    fn write_and_verify_wav() {
        let dir = temp_cache_dir();
        let path = dir.join("test.wav");
        let samples = vec![0.0_f32, 0.5, -0.5, 1.0];
        write_wav_f32(&path, &samples, 44100, 2).expect("write should succeed");
        assert!(path.exists());
        // Verify file starts with RIFF header
        let bytes = std::fs::read(&path).expect("read back");
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
    }
}
