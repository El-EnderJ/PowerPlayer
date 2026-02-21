use super::dsp::fft::compute_spectrum_mono;
use super::dsp::filters::ParametricEQ;
#[cfg(target_os = "windows")]
use super::dsp::filters::SoftLimiter;
use super::lyrics::{load_lyrics_for_track, LyricsLine};
use serde::Serialize;
use std::collections::VecDeque;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "windows")]
use log::{info, warn};

#[cfg(target_os = "windows")]
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SampleRate, Stream, StreamConfig,
};
#[cfg(target_os = "windows")]
use ringbuf::{
    traits::{Consumer as _, Producer as _, Split},
    HeapRb,
};

#[cfg(target_os = "windows")]
use super::decoder::{decode_file, resample_linear, DecodedTrack};

const STATE_PAUSED: u8 = 0;
const STATE_PLAYING: u8 = 1;
const NO_ACTIVE_LYRIC: u32 = u32::MAX;
const LYRICS_POLL_INTERVAL_MS: u64 = 40;
/// Sample history used by the visualizer FFT.
/// 4096 mono samples balance frequency detail while keeping visual updates responsive.
const VIBE_WINDOW_SAMPLES: usize = 4096;

/// 4096 frames is a low-latency compromise: enough headroom against occasional decode jitter
/// while keeping callback fill chunks small to reduce interaction latency for pause/seek.
/// On underrun the callback injects silence, so this size also caps audible dropouts to short gaps.
#[cfg(target_os = "windows")]
const RING_BUFFER_FRAMES: usize = 4096;
#[cfg(target_os = "windows")]
const PRODUCER_CHUNK_FRAMES: usize = 256;

pub struct AudioState {
    inner: Arc<AudioEngine>,
}

#[derive(Clone, Serialize)]
pub struct LyricsEventPayload {
    pub index: Option<usize>,
    pub timestamp: Option<u32>,
    pub text: Option<String>,
}

struct AudioEngine {
    is_playing: AtomicU8,
    should_stop: AtomicBool,
    volume_bits: AtomicU32,
    preamp_db_bits: AtomicU32,
    output_rate_hz: AtomicU32,
    seek_frame: AtomicU32,
    current_frame: AtomicU32,
    track_duration_bits: AtomicU32,
    vibe_amplitude_bits: AtomicU32,
    vibe_samples: Mutex<VecDeque<f32>>,
    lyrics: Mutex<Vec<LyricsLine>>,
    active_lyric_index: AtomicU32,
    lookahead_started: AtomicBool,
    lookahead_completed: AtomicBool,
    eq: Mutex<ParametricEQ>,
    next_track: Mutex<Option<PathBuf>>,
    #[cfg(target_os = "windows")]
    preloaded_next_track: Mutex<Option<DecodedTrack>>,
    #[cfg(target_os = "windows")]
    limiter: SoftLimiter,
    #[cfg(target_os = "windows")]
    stream: Mutex<Option<Stream>>,
    decoder_thread: Mutex<Option<thread::JoinHandle<()>>>,
    lyric_monitor_thread: Mutex<Option<thread::JoinHandle<()>>>,
    #[cfg(target_os = "windows")]
    loaded_path: Mutex<Option<PathBuf>>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AudioEngine {
                is_playing: AtomicU8::new(STATE_PAUSED),
                should_stop: AtomicBool::new(false),
                volume_bits: AtomicU32::new(1.0_f32.to_bits()),
                preamp_db_bits: AtomicU32::new(0.0_f32.to_bits()),
                output_rate_hz: AtomicU32::new(48_000),
                seek_frame: AtomicU32::new(0),
                current_frame: AtomicU32::new(0),
                track_duration_bits: AtomicU32::new(0.0_f32.to_bits()),
                vibe_amplitude_bits: AtomicU32::new(0.0_f32.to_bits()),
                vibe_samples: Mutex::new(VecDeque::with_capacity(VIBE_WINDOW_SAMPLES)),
                lyrics: Mutex::new(Vec::new()),
                active_lyric_index: AtomicU32::new(NO_ACTIVE_LYRIC),
                lookahead_started: AtomicBool::new(false),
                lookahead_completed: AtomicBool::new(false),
                eq: Mutex::new(ParametricEQ::new(10, 48_000.0)),
                next_track: Mutex::new(None),
                #[cfg(target_os = "windows")]
                preloaded_next_track: Mutex::new(None),
                #[cfg(target_os = "windows")]
                limiter: SoftLimiter::new(),
                #[cfg(target_os = "windows")]
                stream: Mutex::new(None),
                decoder_thread: Mutex::new(None),
                lyric_monitor_thread: Mutex::new(None),
                #[cfg(target_os = "windows")]
                loaded_path: Mutex::new(None),
            }),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn load_track(&self, path: impl AsRef<Path>) -> Result<(), String> {
        self.inner.should_stop.store(true, Ordering::SeqCst);
        self.inner.is_playing.store(STATE_PAUSED, Ordering::SeqCst);
        self.inner.seek_frame.store(0, Ordering::SeqCst);
        self.inner.current_frame.store(0, Ordering::SeqCst);
        self.inner.lookahead_started.store(false, Ordering::SeqCst);
        self.inner.lookahead_completed.store(false, Ordering::SeqCst);
        self.inner
            .active_lyric_index
            .store(NO_ACTIVE_LYRIC, Ordering::SeqCst);

        if let Some(handle) = self.inner.decoder_thread.lock().map_err(lock_err)?.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self
            .inner
            .lyric_monitor_thread
            .lock()
            .map_err(lock_err)?
            .take()
        {
            let _ = handle.join();
        }
        self.inner.stream.lock().map_err(lock_err)?.take();
        if let Ok(mut preloaded) = self.inner.preloaded_next_track.lock() {
            preloaded.take();
        }

        let path = path.as_ref().to_path_buf();
        let decoded = decode_file(&path)?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No default output device available".to_string())?;

        let (stream_config, sample_format, exact_rate) = select_stream_config(&device, &decoded)?;
        #[cfg(target_os = "windows")]
        {
            info!(
                "WASAPI path selected through default host. Exact rate match: {}. cpal exclusive-mode APIs are limited, so stream starts in best available mode.",
                exact_rate
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            info!("Default host output configured. Exact rate match: {exact_rate}");
        }

        let source_channels = decoded.channels as usize;
        let output_channels = stream_config.channels as usize;
        let output_rate = stream_config.sample_rate.0;
        self.inner
            .output_rate_hz
            .store(output_rate, Ordering::SeqCst);
        if let Ok(mut eq) = self.inner.eq.lock() {
            eq.set_sample_rate(output_rate as f32);
        }

        let mut pcm = decoded.samples;
        if decoded.sample_rate != output_rate {
            warn!(
                "Device sample-rate {} Hz differs from track {} Hz; applying linear resampling before playback.",
                output_rate, decoded.sample_rate
            );
            pcm = resample_linear(&pcm, decoded.sample_rate, output_rate, source_channels);
        }

        if source_channels != output_channels {
            warn!(
                "Channel adaptation required: source {} -> output {}. Using simple channel copy/fold strategy.",
                source_channels, output_channels
            );
            pcm = adapt_channels(&pcm, source_channels, output_channels);
        }
        self.inner.track_duration_bits.store(
            (pcm.len() as f32 / output_channels as f32 / output_rate as f32).to_bits(),
            Ordering::SeqCst,
        );

        let ring = HeapRb::<f32>::new(RING_BUFFER_FRAMES * output_channels);
        let (mut producer, mut consumer) = ring.split();

        self.inner.should_stop.store(false, Ordering::SeqCst);
        let producer_engine = Arc::clone(&self.inner);
        let producer_handle = thread::spawn(move || {
            let mut read_frame: usize = 0;
            let mut total_frames = pcm.len() / output_channels;

            loop {
                if producer_engine.should_stop.load(Ordering::SeqCst) {
                    break;
                }

                if producer_engine.lookahead_started.load(Ordering::SeqCst) {
                    if !producer_engine.lookahead_completed.load(Ordering::SeqCst) {
                        let next_path =
                            producer_engine.next_track.lock().ok().and_then(|path| path.clone());
                        if let Some(next_path) = next_path {
                            if let Ok(decoded_next) = decode_file(&next_path) {
                                if let Ok(mut preloaded) = producer_engine.preloaded_next_track.lock()
                                {
                                    if preloaded.is_none() {
                                        *preloaded = Some(decoded_next);
                                        producer_engine
                                            .lookahead_started
                                            .store(false, Ordering::SeqCst);
                                        producer_engine
                                            .lookahead_completed
                                            .store(true, Ordering::SeqCst);
                                    }
                                }
                            }
                        }
                    }
                }

                let requested_seek = producer_engine.seek_frame.swap(u32::MAX, Ordering::SeqCst);
                if requested_seek != u32::MAX {
                    read_frame = (requested_seek as usize).min(total_frames);
                    producer.clear();
                }

                if read_frame >= total_frames {
                    if let Ok(mut preloaded) = producer_engine.preloaded_next_track.lock() {
                        if let Some(next) = preloaded.take() {
                            let mut next_pcm = next.samples;
                            if next.sample_rate != output_rate {
                                next_pcm = resample_linear(
                                    &next_pcm,
                                    next.sample_rate,
                                    output_rate,
                                    next.channels as usize,
                                );
                            }
                            if next.channels as usize != output_channels {
                                next_pcm = adapt_channels(
                                    &next_pcm,
                                    next.channels as usize,
                                    output_channels,
                                );
                            }
                            pcm = next_pcm;
                            total_frames = pcm.len() / output_channels;
                            read_frame = 0;
                            producer_engine.current_frame.store(0, Ordering::SeqCst);
                            producer_engine.track_duration_bits.store(
                                (total_frames as f32 / output_rate as f32).to_bits(),
                                Ordering::SeqCst,
                            );
                            producer_engine
                                .lookahead_started
                                .store(false, Ordering::SeqCst);
                            producer_engine
                                .lookahead_completed
                                .store(false, Ordering::SeqCst);
                            if let Ok(mut next_track) = producer_engine.next_track.lock() {
                                next_track.take();
                            }
                            continue;
                        }
                    }
                    thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                let free_slots = producer.vacant_len();
                if free_slots < output_channels {
                    thread::sleep(std::time::Duration::from_millis(2));
                    continue;
                }

                // 256-frame batches reduce producer wakeups without building long queueing latency.
                let writable_frames = (free_slots / output_channels).min(PRODUCER_CHUNK_FRAMES);
                let end = ((read_frame + writable_frames) * output_channels).min(pcm.len());
                for sample in &pcm[read_frame * output_channels..end] {
                    if producer.try_push(*sample).is_err() {
                        break;
                    }
                }
                read_frame = end / output_channels;
            }
        });

        let callback_engine = Arc::clone(&self.inner);
        let err_fn = |err| warn!("Audio stream error: {err}");
        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_output_stream(
                    &stream_config,
                    move |output: &mut [f32], _| {
                        write_samples(output, output_channels, &mut consumer, &callback_engine);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to build f32 output stream: {e}"))?,
            SampleFormat::I16 => device
                .build_output_stream(
                    &stream_config,
                    move |output: &mut [i16], _| {
                        write_samples_i16(output, output_channels, &mut consumer, &callback_engine);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to build i16 output stream: {e}"))?,
            SampleFormat::U16 => device
                .build_output_stream(
                    &stream_config,
                    move |output: &mut [u16], _| {
                        write_samples_u16(output, output_channels, &mut consumer, &callback_engine);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to build u16 output stream: {e}"))?,
            other => {
                return Err(format!(
                    "Unsupported output sample format {other:?}; expected f32/i16/u16"
                ))
            }
        };

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {e}"))?;

        *self.inner.loaded_path.lock().map_err(lock_err)? = Some(path);
        *self.inner.stream.lock().map_err(lock_err)? = Some(stream);
        *self.inner.decoder_thread.lock().map_err(lock_err)? = Some(producer_handle);

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn load_track(&self, _path: impl AsRef<Path>) -> Result<(), String> {
        Err("Audio engine WASAPI implementation is only available on Windows targets".to_string())
    }

    #[cfg(target_os = "windows")]
    pub fn playback_supported(&self) -> bool {
        true
    }

    #[cfg(not(target_os = "windows"))]
    pub fn playback_supported(&self) -> bool {
        false
    }

    pub fn play(&self) {
        self.inner.is_playing.store(STATE_PLAYING, Ordering::SeqCst);
    }

    pub fn set_next_track(&self, path: Option<impl AsRef<Path>>) {
        if let Ok(mut next_track) = self.inner.next_track.lock() {
            *next_track = path.map(|path| path.as_ref().to_path_buf());
        }
        self.inner.lookahead_started.store(false, Ordering::SeqCst);
        self.inner.lookahead_completed.store(false, Ordering::SeqCst);
        #[cfg(target_os = "windows")]
        if let Ok(mut preloaded) = self.inner.preloaded_next_track.lock() {
            preloaded.take();
        }
    }

    pub fn pause(&self) {
        self.inner.is_playing.store(STATE_PAUSED, Ordering::SeqCst);
    }

    pub fn seek(&self, seconds: f64) {
        let clamped = seconds.max(0.0);
        let sample_rate = self.inner.output_rate_hz.load(Ordering::SeqCst) as f64;
        let frame = (clamped * sample_rate) as u32;
        self.inner.seek_frame.store(frame, Ordering::SeqCst);
        self.inner.current_frame.store(frame, Ordering::SeqCst);
        self.inner
            .active_lyric_index
            .store(NO_ACTIVE_LYRIC, Ordering::SeqCst);
    }

    pub fn set_volume(&self, volume: f32) {
        let clamped = volume.clamp(0.0, 1.0);
        self.inner
            .volume_bits
            .store(clamped.to_bits(), Ordering::SeqCst);
    }

    pub fn set_preamp_db(&self, preamp_db: f32) {
        let clamped = preamp_db.clamp(-24.0, 24.0);
        self.inner
            .preamp_db_bits
            .store(clamped.to_bits(), Ordering::SeqCst);
    }

    pub fn update_eq_band(
        &self,
        index: usize,
        frequency: f32,
        gain_db: f32,
        q_factor: f32,
    ) -> Result<(), String> {
        let eq = self.inner.eq.lock().map_err(lock_err)?;
        eq.update_band(index, frequency, gain_db, q_factor)
    }

    /// Returns current EQ band parameters as Vec of (frequency, gain_db, q_factor).
    pub fn get_eq_bands(&self) -> Result<Vec<(f32, f32, f32)>, String> {
        let eq = self.inner.eq.lock().map_err(lock_err)?;
        Ok(eq.get_bands())
    }

    /// Computes the combined EQ frequency response curve.
    /// Returns Vec of (frequency_hz, magnitude_db) pairs.
    pub fn get_eq_frequency_response(&self, num_points: usize) -> Result<Vec<(f32, f32)>, String> {
        let eq = self.inner.eq.lock().map_err(lock_err)?;
        Ok(eq.compute_frequency_response(num_points))
    }

    pub fn get_vibe_data(&self) -> (Vec<f32>, f32) {
        let mono = self
            .inner
            .vibe_samples
            .lock()
            .map(|samples| samples.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        let amplitude = f32::from_bits(self.inner.vibe_amplitude_bits.load(Ordering::Relaxed));
        if mono.is_empty() {
            return (vec![-100.0; 1024], amplitude);
        }

        (compute_spectrum_mono(&mono), amplitude)
    }

    pub fn get_track_duration_seconds(&self) -> f32 {
        f32::from_bits(self.inner.track_duration_bits.load(Ordering::Relaxed))
    }

    pub fn load_lyrics_for_track(&self, path: impl AsRef<Path>) {
        let lyrics = load_lyrics_for_track(path.as_ref());
        if let Ok(mut shared) = self.inner.lyrics.lock() {
            *shared = lyrics;
        }
        self.inner
            .active_lyric_index
            .store(NO_ACTIVE_LYRIC, Ordering::SeqCst);
    }

    pub fn get_lyrics_lines(&self) -> Vec<LyricsLine> {
        self.inner
            .lyrics
            .lock()
            .map(|lines| lines.clone())
            .unwrap_or_default()
    }

    pub fn start_lyrics_monitor(&self, app: AppHandle) -> Result<(), String> {
        if let Some(handle) = self
            .inner
            .lyric_monitor_thread
            .lock()
            .map_err(lock_err)?
            .take()
        {
            let _ = handle.join();
        }
        let engine = Arc::clone(&self.inner);
        let handle = thread::spawn(move || loop {
            if engine.should_stop.load(Ordering::SeqCst) {
                break;
            }
            let lyrics = match engine.lyrics.lock() {
                Ok(lines) => lines.clone(),
                Err(_) => Vec::new(),
            };
            let rate = engine.output_rate_hz.load(Ordering::Relaxed).max(1);
            let frame = engine.current_frame.load(Ordering::Relaxed);
            let now_ms = ((frame as u64) * 1000 / (rate as u64)) as u32;
            // `Err(next)` means insertion point for `now_ms`, so the active lyric is `next - 1`.
            let index = match lyrics.binary_search_by(|line| line.timestamp.cmp(&now_ms)) {
                Ok(found) => Some(found),
                Err(0) => None,
                Err(next) => Some(next - 1),
            };
            let current_idx = index.map(|i| i as u32).unwrap_or(NO_ACTIVE_LYRIC);
            if engine
                .active_lyric_index
                .swap(current_idx, Ordering::SeqCst)
                != current_idx
            {
                let payload = index
                    .and_then(|i| lyrics.get(i).map(|line| (i, line)))
                    .map(|(i, line)| LyricsEventPayload {
                        index: Some(i),
                        timestamp: Some(line.timestamp),
                        text: Some(line.text.clone()),
                    })
                    .unwrap_or(LyricsEventPayload {
                        index: None,
                        timestamp: None,
                        text: None,
                    });
                let _ = app.emit("lyrics-line-changed", payload);
            }
            thread::sleep(std::time::Duration::from_millis(LYRICS_POLL_INTERVAL_MS));
        });
        *self.inner.lyric_monitor_thread.lock().map_err(lock_err)? = Some(handle);
        Ok(())
    }

    #[cfg(test)]
    fn playing_state(&self) -> u8 {
        self.inner.is_playing.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    fn volume(&self) -> f32 {
        f32::from_bits(self.inner.volume_bits.load(Ordering::SeqCst))
    }

    #[cfg(test)]
    fn preamp_db(&self) -> f32 {
        f32::from_bits(self.inner.preamp_db_bits.load(Ordering::SeqCst))
    }

    #[cfg(test)]
    fn has_next_track(&self) -> bool {
        self.inner
            .next_track
            .lock()
            .map(|next| next.is_some())
            .unwrap_or(false)
    }
}

impl Drop for AudioState {
    fn drop(&mut self) {
        self.inner.should_stop.store(true, Ordering::SeqCst);
        if let Ok(mut handle) = self.inner.decoder_thread.lock() {
            if let Some(join_handle) = handle.take() {
                let _ = join_handle.join();
            }
        }
        if let Ok(mut handle) = self.inner.lyric_monitor_thread.lock() {
            if let Some(join_handle) = handle.take() {
                let _ = join_handle.join();
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn select_stream_config(
    device: &cpal::Device,
    track: &DecodedTrack,
) -> Result<(StreamConfig, SampleFormat, bool), String> {
    let mut preferred: Option<(StreamConfig, SampleFormat, bool)> = None;
    let mut fallback: Option<(StreamConfig, SampleFormat, bool)> = None;

    let ranges = device
        .supported_output_configs()
        .map_err(|e| format!("Cannot query output configs: {e}"))?;

    for cfg in ranges {
        let channels = cfg.channels();
        let sample_format = cfg.sample_format();
        let min = cfg.min_sample_rate().0;
        let max = cfg.max_sample_rate().0;

        let exact_rate = track.sample_rate >= min && track.sample_rate <= max;
        let f32_preferred = matches!(sample_format, SampleFormat::F32);

        if channels == track.channels && exact_rate {
            let chosen = (
                StreamConfig {
                    channels,
                    sample_rate: SampleRate(track.sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                },
                sample_format,
                true,
            );
            if f32_preferred {
                return Ok(chosen);
            }
            preferred = Some(chosen);
        }

        if fallback.is_none() || f32_preferred {
            fallback = Some((
                StreamConfig {
                    channels,
                    sample_rate: cfg.max_sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                },
                sample_format,
                false,
            ));
        }
    }

    preferred
        .or(fallback)
        .ok_or_else(|| "No output stream configuration available".to_string())
}

#[cfg(target_os = "windows")]
fn adapt_channels(input: &[f32], in_channels: usize, out_channels: usize) -> Vec<f32> {
    if in_channels == out_channels || in_channels == 0 || out_channels == 0 {
        return input.to_vec();
    }

    let frames = input.len() / in_channels;
    let mut out = vec![0.0_f32; frames * out_channels];
    for frame in 0..frames {
        for ch in 0..out_channels {
            out[frame * out_channels + ch] = input[frame * in_channels + (ch % in_channels)];
        }
    }
    out
}

#[cfg(target_os = "windows")]
fn write_samples(
    output: &mut [f32],
    channels: usize,
    consumer: &mut impl ringbuf::traits::Consumer<Item = f32>,
    engine: &AudioEngine,
) {
    if engine.is_playing.load(Ordering::SeqCst) != STATE_PLAYING {
        output.fill(0.0);
        return;
    }

    let preamp = db_to_gain(f32::from_bits(
        engine.preamp_db_bits.load(Ordering::Relaxed),
    ));
    let volume = f32::from_bits(engine.volume_bits.load(Ordering::Relaxed));
    let mut eq = engine.eq.lock().ok();
    let frame_channels = channels.max(1);
    for frame in output.chunks_mut(frame_channels) {
        let mut left = consumer.try_pop().unwrap_or(0.0) * preamp;
        let mut right = if frame.len() > 1 {
            consumer.try_pop().unwrap_or(0.0) * preamp
        } else {
            left
        };
        if let Some(eq) = eq.as_mut() {
            (left, right) = eq.process_stereo_frame(left, right);
        }
        frame[0] = engine.limiter.process_sample(left) * volume;
        if frame.len() > 1 {
            frame[1] = engine.limiter.process_sample(right) * volume;
        }
        for out_sample in frame.iter_mut().skip(2) {
            let sample = consumer.try_pop().unwrap_or(0.0) * preamp;
            *out_sample = engine.limiter.process_sample(sample) * volume;
        }
    }
    update_vibe_from_f32(engine, output, frame_channels);
    let frame = engine
        .current_frame
        .fetch_add((output.len() / frame_channels) as u32, Ordering::Relaxed)
        + (output.len() / frame_channels) as u32;
    trigger_next_track_lookahead(engine, frame);
}

#[cfg(target_os = "windows")]
fn write_samples_i16(
    output: &mut [i16],
    channels: usize,
    consumer: &mut impl ringbuf::traits::Consumer<Item = f32>,
    engine: &AudioEngine,
) {
    if engine.is_playing.load(Ordering::SeqCst) != STATE_PLAYING {
        output.fill(0);
        return;
    }

    let preamp = db_to_gain(f32::from_bits(
        engine.preamp_db_bits.load(Ordering::Relaxed),
    ));
    let volume = f32::from_bits(engine.volume_bits.load(Ordering::Relaxed));
    let mut eq = engine.eq.lock().ok();
    let frame_channels = channels.max(1);
    for frame in output.chunks_mut(frame_channels) {
        let mut left = consumer.try_pop().unwrap_or(0.0) * preamp;
        let mut right = if frame.len() > 1 {
            consumer.try_pop().unwrap_or(0.0) * preamp
        } else {
            left
        };
        if let Some(eq) = eq.as_mut() {
            (left, right) = eq.process_stereo_frame(left, right);
        }
        let left = engine.limiter.process_sample(left) * volume;
        frame[0] = (left.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        if frame.len() > 1 {
            let right = engine.limiter.process_sample(right) * volume;
            frame[1] = (right.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        }
        for out_sample in frame.iter_mut().skip(2) {
            let sample = consumer.try_pop().unwrap_or(0.0) * preamp;
            let limited = engine.limiter.process_sample(sample) * volume;
            *out_sample = (limited.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        }
    }
    update_vibe_from_i16(engine, output, frame_channels);
    let frame = engine
        .current_frame
        .fetch_add((output.len() / frame_channels) as u32, Ordering::Relaxed)
        + (output.len() / frame_channels) as u32;
    trigger_next_track_lookahead(engine, frame);
}

#[cfg(target_os = "windows")]
fn write_samples_u16(
    output: &mut [u16],
    channels: usize,
    consumer: &mut impl ringbuf::traits::Consumer<Item = f32>,
    engine: &AudioEngine,
) {
    if engine.is_playing.load(Ordering::SeqCst) != STATE_PLAYING {
        output.fill(u16::MAX / 2);
        return;
    }

    let preamp = db_to_gain(f32::from_bits(
        engine.preamp_db_bits.load(Ordering::Relaxed),
    ));
    let volume = f32::from_bits(engine.volume_bits.load(Ordering::Relaxed));
    let mut eq = engine.eq.lock().ok();
    let frame_channels = channels.max(1);
    for frame in output.chunks_mut(frame_channels) {
        let mut left = consumer.try_pop().unwrap_or(0.0) * preamp;
        let mut right = if frame.len() > 1 {
            consumer.try_pop().unwrap_or(0.0) * preamp
        } else {
            left
        };
        if let Some(eq) = eq.as_mut() {
            (left, right) = eq.process_stereo_frame(left, right);
        }
        let left = engine.limiter.process_sample(left) * volume;
        frame[0] = (((left.clamp(-1.0, 1.0) + 1.0) * 0.5) * u16::MAX as f32) as u16;
        if frame.len() > 1 {
            let right = engine.limiter.process_sample(right) * volume;
            frame[1] = (((right.clamp(-1.0, 1.0) + 1.0) * 0.5) * u16::MAX as f32) as u16;
        }
        for out_sample in frame.iter_mut().skip(2) {
            let sample = consumer.try_pop().unwrap_or(0.0) * preamp;
            let limited = engine.limiter.process_sample(sample) * volume;
            *out_sample = (((limited.clamp(-1.0, 1.0) + 1.0) * 0.5) * u16::MAX as f32) as u16;
        }
    }
    update_vibe_from_u16(engine, output, frame_channels);
    let frame = engine
        .current_frame
        .fetch_add((output.len() / frame_channels) as u32, Ordering::Relaxed)
        + (output.len() / frame_channels) as u32;
    trigger_next_track_lookahead(engine, frame);
}

#[cfg(target_os = "windows")]
fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[cfg(target_os = "windows")]
fn update_vibe_from_f32(engine: &AudioEngine, output: &[f32], channels: usize) {
    let mut peak = 0.0_f32;
    let mut mono = Vec::with_capacity(output.len() / channels.max(1));
    for frame in output.chunks(channels.max(1)) {
        let mut sum = 0.0_f32;
        for sample in frame {
            peak = peak.max(sample.abs());
            sum += *sample;
        }
        mono.push(sum / frame.len() as f32);
    }
    update_vibe_state(engine, mono, peak);
}

#[cfg(target_os = "windows")]
fn update_vibe_from_i16(engine: &AudioEngine, output: &[i16], channels: usize) {
    let mut peak = 0.0_f32;
    let mut mono = Vec::with_capacity(output.len() / channels.max(1));
    for frame in output.chunks(channels.max(1)) {
        let mut sum = 0.0_f32;
        for sample in frame {
            let normalized = *sample as f32 / i16::MAX as f32;
            peak = peak.max(normalized.abs());
            sum += normalized;
        }
        mono.push(sum / frame.len() as f32);
    }
    update_vibe_state(engine, mono, peak);
}

#[cfg(target_os = "windows")]
fn update_vibe_from_u16(engine: &AudioEngine, output: &[u16], channels: usize) {
    let mut peak = 0.0_f32;
    let mut mono = Vec::with_capacity(output.len() / channels.max(1));
    for frame in output.chunks(channels.max(1)) {
        let mut sum = 0.0_f32;
        for sample in frame {
            let normalized = (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0;
            peak = peak.max(normalized.abs());
            sum += normalized;
        }
        mono.push(sum / frame.len() as f32);
    }
    update_vibe_state(engine, mono, peak);
}

#[cfg(target_os = "windows")]
fn update_vibe_state(engine: &AudioEngine, mono_samples: Vec<f32>, peak: f32) {
    engine
        .vibe_amplitude_bits
        .store(peak.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    if let Ok(mut shared) = engine.vibe_samples.lock() {
        for sample in mono_samples {
            shared.push_back(sample);
        }
        if shared.len() > VIBE_WINDOW_SAMPLES {
            while shared.len() > VIBE_WINDOW_SAMPLES {
                let _ = shared.pop_front();
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn trigger_next_track_lookahead(engine: &AudioEngine, current_frame: u32) {
    let duration = f32::from_bits(engine.track_duration_bits.load(Ordering::Relaxed));
    let rate = engine.output_rate_hz.load(Ordering::Relaxed).max(1);
    if duration <= 0.0 {
        return;
    }
    if engine.lookahead_completed.load(Ordering::Relaxed) {
        return;
    }
    if engine
        .next_track
        .lock()
        .ok()
        .and_then(|path| path.clone())
        .is_none()
    {
        return;
    }
    let progress = current_frame as f32 / (duration * rate as f32);
    // Short-circuit keeps swap() from running before 95%. Once >=95%, swap(true) returns the
    // previous armed flag; if it was already true, we skip to avoid duplicate preload attempts.
    if progress < 0.95 || engine.lookahead_started.swap(true, Ordering::SeqCst) {
        return;
    }
}

fn lock_err<T>(_: T) -> String {
    "Audio state lock poisoned".to_string()
}

#[cfg(test)]
mod tests {
    use super::{AudioState, STATE_PAUSED, STATE_PLAYING};

    #[test]
    fn volume_is_clamped() {
        let state = AudioState::new();
        state.set_volume(2.0);
        assert_eq!(state.volume(), 1.0);
        state.set_volume(-1.0);
        assert_eq!(state.volume(), 0.0);
    }

    #[test]
    fn play_pause_updates_atomic_state() {
        let state = AudioState::new();
        assert_eq!(state.playing_state(), STATE_PAUSED);
        state.play();
        assert_eq!(state.playing_state(), STATE_PLAYING);
        state.pause();
        assert_eq!(state.playing_state(), STATE_PAUSED);
    }

    #[test]
    fn preamp_is_clamped() {
        let state = AudioState::new();
        state.set_preamp_db(30.0);
        assert_eq!(state.preamp_db(), 24.0);
        state.set_preamp_db(-30.0);
        assert_eq!(state.preamp_db(), -24.0);
    }

    #[test]
    fn next_track_can_be_set_and_cleared() {
        let state = AudioState::new();
        state.set_next_track(Some("/music/next.flac"));
        assert!(state.has_next_track());
        state.set_next_track(None::<&str>);
        assert!(!state.has_next_track());
    }
}
