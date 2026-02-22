use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;
use thiserror::Error;

mod audio;
mod db;
mod library;
use audio::engine::{AudioState, AudioStats};
use db::manager::DbManager;
use db::search::SearchResults;
use db::spatial_store::SpatialSceneRow;
use library::queue::PlaybackQueue;
use library::stems::StemSeparator;

type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
enum AppError {
    #[error("{error}")]
    Dsp { error: String, code: &'static str },
    #[error("{error}")]
    Db { error: String, code: &'static str },
    #[error("{error}")]
    Fs { error: String, code: &'static str },
}

impl AppError {
    fn dsp(error: impl Into<String>) -> Self {
        Self::Dsp {
            error: error.into(),
            code: "DSP_ERROR",
        }
    }

    fn db(error: impl Into<String>) -> Self {
        Self::Db {
            error: error.into(),
            code: "DB_ERROR",
        }
    }

    fn fs(error: impl Into<String>) -> Self {
        Self::Fs {
            error: error.into(),
            code: "FS_ERROR",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct ErrorPayload<'a> {
            error: &'a str,
            code: &'a str,
        }
        let payload = match self {
            AppError::Dsp { error, code } | AppError::Db { error, code } | AppError::Fs { error, code } => {
                ErrorPayload {
                    error: error.as_str(),
                    code,
                }
            }
        };
        payload.serialize(serializer)
    }
}

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
fn greet(name: &str) -> AppResult<String> {
    Ok(format!("Hello, {}! PowerPlayer is ready.", name))
}

#[tauri::command]
fn update_eq_band(
    state: tauri::State<'_, AudioState>,
    index: usize,
    freq: f32,
    gain: f32,
    q: f32,
) -> AppResult<()> {
    state
        .update_eq_band(index, freq, gain, q)
        .map_err(AppError::dsp)
}

#[tauri::command]
fn activate_autoeq_profile(
    state: tauri::State<'_, AudioState>,
    model: String,
) -> AppResult<Vec<EqBandData>> {
    let profile = audio::dsp::autoeq::profile_for_model(&model)
        .ok_or_else(|| AppError::dsp(format!("No AutoEQ profile found for model: {model}")))?;
    state.set_autoeq_profile(&profile).map_err(AppError::dsp)?;

    get_eq_bands(state)
}

#[tauri::command]
fn get_eq_bands(state: tauri::State<'_, AudioState>) -> AppResult<Vec<EqBandData>> {
    let bands = state.get_eq_bands().map_err(AppError::dsp)?;
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
) -> AppResult<Vec<FrequencyPoint>> {
    let response = state
        .get_eq_frequency_response(num_points)
        .map_err(AppError::dsp)?;
    Ok(response
        .into_iter()
        .map(|(frequency, magnitude_db)| FrequencyPoint {
            frequency,
            magnitude_db,
        })
        .collect())
}

#[tauri::command]
fn get_fft_data() -> AppResult<Vec<f32>> {
    Ok(vec![-100.0; 1024])
}

#[tauri::command]
async fn load_track(
    app: tauri::AppHandle,
    path: String,
) -> AppResult<TrackData> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AudioState>();
        let metadata = audio::decoder::read_track_metadata(Path::new(&path)).map_err(AppError::fs)?;
        state.load_lyrics_for_track(&path);
        if state.playback_supported() {
            state.load_track(&path).map_err(AppError::dsp)?;
            state
                .start_lyrics_monitor(app.clone())
                .map_err(AppError::dsp)?;
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
    })
    .await
    .map_err(|err| AppError::dsp(format!("Blocking load track task failed: {err}")))?
}

#[tauri::command]
fn get_lyrics_lines(state: tauri::State<'_, AudioState>) -> AppResult<Vec<LyricsLineData>> {
    Ok(state
        .get_lyrics_lines()
        .into_iter()
        .map(|line| LyricsLineData {
            timestamp: line.timestamp,
            text: line.text,
        })
        .collect())
}

#[tauri::command]
async fn scan_library(app: tauri::AppHandle, path: String) -> AppResult<usize> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<DbManager>();
        let root = PathBuf::from(path);
        let scanned = library::scanner::scan_library_path(&root, &db).map_err(AppError::fs)?;
        library::scanner::register_library_watch(&root, &db).map_err(AppError::fs)?;
        Ok(scanned)
    })
    .await
    .map_err(|err| AppError::fs(format!("Blocking library scan task failed: {err}")))?
}

#[tauri::command]
fn get_library_tracks(state: tauri::State<'_, DbManager>) -> AppResult<Vec<LibraryTrackData>> {
    Ok(state
        .get_tracks()
        .map_err(AppError::db)?
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
fn play(state: tauri::State<'_, AudioState>) -> AppResult<()> {
    state.play();
    Ok(())
}

#[tauri::command]
fn pause(state: tauri::State<'_, AudioState>) -> AppResult<()> {
    state.pause();
    Ok(())
}

#[tauri::command]
fn set_next_track(state: tauri::State<'_, AudioState>, path: Option<String>) -> AppResult<()> {
    state.set_next_track(path.as_deref());
    Ok(())
}

#[tauri::command]
fn seek(state: tauri::State<'_, AudioState>, seconds: f64) -> AppResult<()> {
    state.seek(seconds);
    Ok(())
}

#[tauri::command]
fn set_volume(state: tauri::State<'_, AudioState>, volume: f32) -> AppResult<()> {
    state.set_volume(volume);
    Ok(())
}

#[tauri::command]
fn get_vibe_data(state: tauri::State<'_, AudioState>) -> AppResult<VibeData> {
    let (spectrum, amplitude) = state.get_vibe_data();
    Ok(VibeData {
        spectrum,
        amplitude,
    })
}

#[tauri::command]
fn get_audio_stats(state: tauri::State<'_, AudioState>) -> AppResult<AudioStatsData> {
    let AudioStats {
        device,
        stream_latency_ms,
        output_sample_rate_hz,
        file_sample_rate_hz,
        ring_buffer_capacity_bytes,
        ring_buffer_used_bytes,
    } = state.get_audio_stats();
    Ok(AudioStatsData {
        device,
        stream_latency_ms,
        output_sample_rate_hz,
        file_sample_rate_hz,
        ring_buffer_capacity_bytes,
        ring_buffer_used_bytes,
    })
}

#[tauri::command]
fn set_tone(
    state: tauri::State<'_, AudioState>,
    bass: f32,
    treble: f32,
) -> AppResult<()> {
    state.set_tone(bass, treble).map_err(AppError::dsp)
}

#[tauri::command]
fn set_balance(state: tauri::State<'_, AudioState>, val: f32) -> AppResult<()> {
    state.set_balance(val).map_err(AppError::dsp)
}

#[tauri::command]
fn set_expansion(state: tauri::State<'_, AudioState>, val: f32) -> AppResult<()> {
    state.set_expansion(val).map_err(AppError::dsp)
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
) -> AppResult<()> {
    state.set_reverb_params(room_size, damping, predelay_ms, lowpass_filter, decay, wet_mix)
        .map_err(AppError::dsp)
}

#[tauri::command]
fn load_reverb_preset(
    state: tauri::State<'_, AudioState>,
    name: String,
) -> AppResult<()> {
    state.load_reverb_preset(&name).map_err(AppError::dsp)
}

#[tauri::command]
fn fast_search(
    state: tauri::State<'_, DbManager>,
    query: String,
) -> AppResult<SearchResults> {
    state.fast_search(&query).map_err(AppError::db)
}

#[tauri::command]
fn toggle_shuffle(
    state: tauri::State<'_, Mutex<PlaybackQueue>>,
    enabled: bool,
) -> AppResult<()> {
    let mut queue = state
        .lock()
        .map_err(|e| AppError::dsp(format!("Queue lock error: {e}")))?;
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
) -> AppResult<()> {
    state.set_spatial_enabled(enabled).map_err(AppError::dsp)
}

#[tauri::command]
fn update_source_position(
    state: tauri::State<'_, AudioState>,
    source_id: usize,
    x: f32,
    y: f32,
    z: f32,
) -> AppResult<()> {
    state
        .set_spatial_source_position(source_id, x, y, z)
        .map_err(AppError::dsp)
}

#[tauri::command]
fn set_room_properties(
    state: tauri::State<'_, AudioState>,
    width: f32,
    length: f32,
    height: f32,
    damping: f32,
) -> AppResult<()> {
    state
        .set_spatial_room_size(width, length, height)
        .map_err(AppError::dsp)?;
    state.set_spatial_damping(damping).map_err(AppError::dsp)
}

#[tauri::command]
fn get_spatial_sources(
    state: tauri::State<'_, AudioState>,
) -> AppResult<Vec<SpatialSourceData>> {
    let positions = state.get_spatial_source_positions().map_err(AppError::dsp)?;
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
fn auto_orchestra(state: tauri::State<'_, AudioState>) -> AppResult<()> {
    state.spatial_auto_orchestra().map_err(AppError::dsp)
}

// ── Spatial Scene Persistence IPC ──────────────────────────────────────

#[tauri::command]
fn save_spatial_scene(
    audio: tauri::State<'_, AudioState>,
    db: tauri::State<'_, DbManager>,
    track_id: String,
) -> AppResult<()> {
    let positions = audio.get_spatial_source_positions().map_err(AppError::dsp)?;
    let names = audio::dsp::spatial::SOURCE_NAMES;
    for (i, (x, y, z, active)) in positions.iter().enumerate() {
        let name = names.get(i).unwrap_or(&"unknown");
        db.save_spatial_scene(&track_id, name, *x, *y, *z, *active)
            .map_err(AppError::db)?;
    }
    Ok(())
}

#[tauri::command]
fn load_spatial_scene(
    audio: tauri::State<'_, AudioState>,
    db: tauri::State<'_, DbManager>,
    track_id: String,
) -> AppResult<Vec<SpatialSceneRow>> {
    let rows = db.load_spatial_scene(&track_id).map_err(AppError::db)?;
    let names = audio::dsp::spatial::SOURCE_NAMES;
    for row in &rows {
        if let Some(idx) = names.iter().position(|&n| n == row.source_name) {
            audio
                .set_spatial_source_position(idx, row.x, row.y, row.z)
                .map_err(AppError::dsp)?;
            audio
                .set_spatial_source_active(idx, row.is_active)
                .map_err(AppError::dsp)?;
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
async fn analyze_spatial_stems(
    app: tauri::AppHandle,
    track_id: String,
) -> AppResult<StemPathsData> {
    tauri::async_runtime::spawn_blocking(move || {
        let stem_sep = app.state::<Mutex<StemSeparator>>();
        let separator = stem_sep
            .lock()
            .map_err(|e| AppError::dsp(format!("Stem separator lock error: {e}")))?;

        let paths = separator
            .analyze_spatial_stems(&track_id, |progress| {
                let _ = app.emit("stems-progress", &progress);
            })
            .map_err(AppError::dsp)?;

        Ok(StemPathsData {
            vocals: paths.vocals.to_string_lossy().to_string(),
            drums: paths.drums.to_string_lossy().to_string(),
            bass: paths.bass.to_string_lossy().to_string(),
            other: paths.other.to_string_lossy().to_string(),
        })
    })
    .await
    .map_err(|err| AppError::dsp(format!("Blocking stem analysis task failed: {err}")))?
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

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn app_error_serializes_with_error_and_code() {
        let payload =
            serde_json::to_value(AppError::db("database unavailable")).expect("serialize AppError");
        assert_eq!(payload["error"], "database unavailable");
        assert_eq!(payload["code"], "DB_ERROR");
    }
}
