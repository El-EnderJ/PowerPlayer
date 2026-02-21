use serde::Serialize;
use std::path::Path;

mod audio;
mod db;
mod library;
use audio::engine::AudioState;
use db::manager::DbManager;

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

    for (index, band) in profile.iter().enumerate() {
        state.update_eq_band(index, band.frequency, band.gain_db, band.q_factor)?;
    }

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = DbManager::new("powerplayer.db").expect("failed to initialize SQLite manager");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState::new())
        .manage(db)
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
            get_lyrics_lines,
            scan_library,
            get_library_tracks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PowerPlayer");
}
