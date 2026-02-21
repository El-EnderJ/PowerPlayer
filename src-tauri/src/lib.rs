use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use tauri::Emitter;

mod audio;
mod db;
mod library;
use audio::engine::{AudioState, AudioStats};
use db::manager::DbManager;
use db::search::SearchResults;
use db::spatial_store::SpatialSceneRow;
use library::queue::PlaybackQueue;
use library::stems::StemSeparator;

#[derive(Serialize)]
struct EqBandData {
    index: usize,
    frequency: f32,
    gain_db: f32,
    q_factor: f32,
}

#[derive(Serialize)]
struct FrequencyPoint {
    frequency: f32,
    magnitude_db: f32,
}

#[derive(Serialize)]
struct CoverArtData {
    media_type: String,
    data: Vec<u8>,
}

#[derive(Serialize)]
struct TrackData {
    artist: String,
    title: String,
    cover_art: Option<CoverArtData>,
    duration_seconds: f32,
}

#[derive(Serialize)]
struct VibeData {
    spectrum: Vec<f32>,
    amplitude: f32,
}

#[derive(Serialize)]
struct AudioStatsData {
    device: String,
    stream_latency_ms: f32,
    output_sample_rate_hz: u32,
    file_sample_rate_hz: u32,
    ring_buffer_capacity_bytes: u32,
    ring_buffer_used_bytes: u32,
}

#[derive(Serialize)]
struct LyricsLineData {
    timestamp: u32,
    text: String,
}

#[derive(Serialize)]
struct LibraryTrackData {
    path: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_seconds: Option<f32>,
    sample_rate: Option<u32>,
    art_url: Option<String>,
    corrupted: bool,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! PowerPlayer is ready.", name)
}

#[tauri::command]
fn update_eq_band(
    state: tauri::State<'_, AudioState>,
    index: usize,
    freq: f32,
    gain: f32,
    q: f32,
) -> Result<(), String> {
    state.update_eq_band(index, freq, gain, q)
}

#[tauri::command]
fn activate_autoeq_profile(
    state: tauri::State<'_, AudioState>,
    model: String,
) -> Result<Vec<EqBandData>, String> {
    let profile = audio::dsp::autoeq::profile_for_model(&model)
        .ok_or_else(|| format!("No AutoEQ profile found for model: {model}"))?;
    state.set_autoeq_profile(&profile)?;

    get_eq_bands(state)
}

#[tauri::command]
fn get_eq_bands(state: tauri::State<'_, AudioState>) -> Result<Vec<EqBandData>, String> {
    let bands = state.get_eq_bands()?;
    Ok(bands
        .into_iter()
        .enumerate()
        .map(|(i, (frequency, gain_db, q_factor))| EqBandData {
            index: i,
            frequency,
            gain_db,
            q_factor,
        })
        .collect())
}

#[tauri::command]
fn get_eq_frequency_response(
    state: tauri::State<'_, AudioState>,
    num_points: usize,
) -> Result<Vec<FrequencyPoint>, String> {
    let response = state.get_eq_frequency_response(num_points)?;
    Ok(response
        .into_iter()
        .map(|(frequency, magnitude_db)| FrequencyPoint {
            frequency,
            magnitude_db,
        })
        .collect())
}

#[tauri::command]
fn get_fft_data() -> Vec<f32> {
    vec![-100.0; 1024]
}

#[tauri::command]
fn load_track(
    app: tauri::AppHandle,
    state: tauri::State<'_, AudioState>,
    path: String,
) -> Result<TrackData, String> {
    let metadata = audio::decoder::read_track_metadata(Path::new(&path))?;
    state.load_lyrics_for_track(&path);
    if state.playback_supported() {
        state.load_track(&path)?;
        state.start_lyrics_monitor(app)?;
    }

    Ok(TrackData {
        artist: metadata
            .artist
            .unwrap_or_else(|| "Unknown Artist".to_string()),
        title: metadata
            .title
            .unwrap_or_else(|| "Unknown Title".to_string()),
        cover_art: metadata.cover_art.map(|cover| CoverArtData {
            media_type: cover.media_type,
            data: cover.data,
        }),
        duration_seconds: state
            .get_track_duration_seconds()
            .max(metadata.duration_seconds.unwrap_or(0.0)),
    })
}

#[tauri::command]
fn get_lyrics_lines(state: tauri::State<'_, AudioState>) -> Vec<LyricsLineData> {
    state
        .get_lyrics_lines()
        .into_iter()
        .map(|line| LyricsLineData {
            timestamp: line.timestamp,
            text: line.text,
        })
        .collect()
}

#[tauri::command]
fn scan_library(state: tauri::State<'_, DbManager>, path: String) -> Result<usize, String> {
    let root = Path::new(&path);
    let scanned = library::scanner::scan_library_path(root, &state)?;
    library::scanner::register_library_watch(root, &state)?;
    Ok(scanned)
}

#[tauri::command]
fn get_library_tracks(state: tauri::State<'_, DbManager>) -> Result<Vec<LibraryTrackData>, String> {
    Ok(state
        .get_tracks()?
        .into_iter()
        .map(|track| LibraryTrackData {
            path: track.path,
            title: track.title,
            artist: track.artist,
            album: track.album,
            duration_seconds: track.duration_seconds,
            sample_rate: track.sample_rate,
            art_url: track.art_url,
            corrupted: track.corrupted,
        })
        .collect())
}

#[tauri::command]
fn play(state: tauri::State<'_, AudioState>) {
    state.play();
}

#[tauri::command]
fn pause(state: tauri::State<'_, AudioState>) {
    state.pause();
}

#[tauri::command]
fn set_next_track(state: tauri::State<'_, AudioState>, path: Option<String>) {
    state.set_next_track(path.as_deref());
}

#[tauri::command]
fn seek(state: tauri::State<'_, AudioState>, seconds: f64) {
    state.seek(seconds);
}

#[tauri::command]
fn set_volume(state: tauri::State<'_, AudioState>, volume: f32) {
    state.set_volume(volume);
}

#[tauri::command]
fn get_vibe_data(state: tauri::State<'_, AudioState>) -> VibeData {
    let (spectrum, amplitude) = state.get_vibe_data();
    VibeData {
        spectrum,
        amplitude,
    }
}

#[tauri::command]
fn get_audio_stats(state: tauri::State<'_, AudioState>) -> AudioStatsData {
    let AudioStats {
        device,
        stream_latency_ms,
        output_sample_rate_hz,
        file_sample_rate_hz,
        ring_buffer_capacity_bytes,
        ring_buffer_used_bytes,
    } = state.get_audio_stats();
    AudioStatsData {
        device,
        stream_latency_ms,
        output_sample_rate_hz,
        file_sample_rate_hz,
        ring_buffer_capacity_bytes,
        ring_buffer_used_bytes,
    }
}

#[tauri::command]
fn set_tone(
    state: tauri::State<'_, AudioState>,
    bass: f32,
    treble: f32,
) -> Result<(), String> {
    state.set_tone(bass, treble)
}

#[tauri::command]
fn set_balance(state: tauri::State<'_, AudioState>, val: f32) -> Result<(), String> {
    state.set_balance(val)
}

#[tauri::command]
fn set_expansion(state: tauri::State<'_, AudioState>, val: f32) -> Result<(), String> {
    state.set_expansion(val)
}

#[tauri::command]
fn set_reverb_params(
    state: tauri::State<'_, AudioState>,
    room_size: f32,
    damping: f32,
    predelay_ms: f32,
    lowpass_filter: f32,
    decay: f32,
    wet_mix: f32,
) -> Result<(), String> {
    state.set_reverb_params(room_size, damping, predelay_ms, lowpass_filter, decay, wet_mix)
}

#[tauri::command]
fn load_reverb_preset(
    state: tauri::State<'_, AudioState>,
    name: String,
) -> Result<(), String> {
    state.load_reverb_preset(&name)
}

#[tauri::command]
fn fast_search(
    state: tauri::State<'_, DbManager>,
    query: String,
) -> Result<SearchResults, String> {
    state.fast_search(&query)
}

#[tauri::command]
fn toggle_shuffle(
    state: tauri::State<'_, Mutex<PlaybackQueue>>,
    enabled: bool,
) -> Result<(), String> {
    let mut queue = state.lock().map_err(|e| format!("Queue lock error: {e}"))?;
    queue.toggle_shuffle(enabled);
    Ok(())
}

// ── Spatial Audio IPC commands ─────────────────────────────────────────

#[derive(Serialize)]
struct SpatialSourceData {
    index: usize,
    name: String,
    x: f32,
    y: f32,
    z: f32,
    is_active: bool,
}

#[tauri::command]
fn toggle_spatial_mode(
    state: tauri::State<'_, AudioState>,
    enabled: bool,
) -> Result<(), String> {
    state.set_spatial_enabled(enabled)
}

#[tauri::command]
fn update_source_position(
    state: tauri::State<'_, AudioState>,
    source_id: usize,
    x: f32,
    y: f32,
    z: f32,
) -> Result<(), String> {
    state.set_spatial_source_position(source_id, x, y, z)
}

#[tauri::command]
fn set_room_properties(
    state: tauri::State<'_, AudioState>,
    width: f32,
    length: f32,
    height: f32,
    damping: f32,
) -> Result<(), String> {
    state.set_spatial_room_size(width, length, height)?;
    state.set_spatial_damping(damping)
}

#[tauri::command]
fn get_spatial_sources(
    state: tauri::State<'_, AudioState>,
) -> Result<Vec<SpatialSourceData>, String> {
    let positions = state.get_spatial_source_positions()?;
    let names = audio::dsp::spatial::SOURCE_NAMES;
    Ok(positions
        .into_iter()
        .enumerate()
        .map(|(i, (x, y, z, active))| SpatialSourceData {
            index: i,
            name: names.get(i).unwrap_or(&"unknown").to_string(),
            x,
            y,
            z,
            is_active: active,
        })
        .collect())
}

#[tauri::command]
fn auto_orchestra(state: tauri::State<'_, AudioState>) -> Result<(), String> {
    state.spatial_auto_orchestra()
}

// ── Spatial Scene Persistence IPC ──────────────────────────────────────

#[tauri::command]
fn save_spatial_scene(
    audio: tauri::State<'_, AudioState>,
    db: tauri::State<'_, DbManager>,
    track_id: String,
) -> Result<(), String> {
    let positions = audio.get_spatial_source_positions()?;
    let names = audio::dsp::spatial::SOURCE_NAMES;
    for (i, (x, y, z, active)) in positions.iter().enumerate() {
        let name = names.get(i).unwrap_or(&"unknown");
        db.save_spatial_scene(&track_id, name, *x, *y, *z, *active)?;
    }
    Ok(())
}

#[tauri::command]
fn load_spatial_scene(
    audio: tauri::State<'_, AudioState>,
    db: tauri::State<'_, DbManager>,
    track_id: String,
) -> Result<Vec<SpatialSceneRow>, String> {
    let rows = db.load_spatial_scene(&track_id)?;
    let names = audio::dsp::spatial::SOURCE_NAMES;
    for row in &rows {
        if let Some(idx) = names.iter().position(|&n| n == row.source_name) {
            audio.set_spatial_source_position(idx, row.x, row.y, row.z)?;
            audio.set_spatial_source_active(idx, row.is_active)?;
        }
    }
    Ok(rows)
}

// ── Stem Separation IPC ────────────────────────────────────────────────

#[derive(Serialize)]
struct StemPathsData {
    vocals: String,
    drums: String,
    bass: String,
    other: String,
}

#[tauri::command]
fn analyze_spatial_stems(
    app: tauri::AppHandle,
    stem_sep: tauri::State<'_, Mutex<StemSeparator>>,
    track_id: String,
) -> Result<StemPathsData, String> {
    let separator = stem_sep
        .lock()
        .map_err(|e| format!("Stem separator lock error: {e}"))?;

    let paths = separator.analyze_spatial_stems(&track_id, |progress| {
        let _ = app.emit("stems-progress", &progress);
    })?;

    Ok(StemPathsData {
        vocals: paths.vocals.to_string_lossy().to_string(),
        drums: paths.drums.to_string_lossy().to_string(),
        bass: paths.bass.to_string_lossy().to_string(),
        other: paths.other.to_string_lossy().to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = DbManager::new("powerplayer.db").expect("failed to initialize SQLite manager");
    db.initialize_fts().expect("failed to initialize FTS5 search");
    db.initialize_spatial_schema().expect("failed to initialize spatial schema");

    let stems_cache = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
        .join("powerplayer")
        .join("stems");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState::new())
        .manage(db)
        .manage(Mutex::new(PlaybackQueue::new()))
        .manage(Mutex::new(StemSeparator::new(stems_cache)))
        .invoke_handler(tauri::generate_handler![
            greet,
            update_eq_band,
            activate_autoeq_profile,
            get_eq_bands,
            get_eq_frequency_response,
            get_fft_data,
            load_track,
            play,
            pause,
            set_next_track,
            seek,
            set_volume,
            get_vibe_data,
            get_audio_stats,
            get_lyrics_lines,
            scan_library,
            get_library_tracks,
            set_tone,
            set_balance,
            set_expansion,
            set_reverb_params,
            load_reverb_preset,
            fast_search,
            toggle_shuffle,
            toggle_spatial_mode,
            update_source_position,
            set_room_properties,
            get_spatial_sources,
            auto_orchestra,
            save_spatial_scene,
            load_spatial_scene,
            analyze_spatial_stems,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PowerPlayer");
}
