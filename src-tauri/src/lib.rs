use serde::Serialize;

mod audio;
use audio::engine::AudioState;

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
    // Returns empty spectrum when no audio is playing.
    // On Windows with active playback, this would be fed from the audio callback buffer.
    vec![-100.0; 1024]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState::new())
        .invoke_handler(tauri::generate_handler![
            greet,
            update_eq_band,
            get_eq_bands,
            get_eq_frequency_response,
            get_fft_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PowerPlayer");
}
