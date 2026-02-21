use tauri::Manager;

mod audio;
use audio::engine::AudioState;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState::new())
        .invoke_handler(tauri::generate_handler![greet, update_eq_band])
        .run(tauri::generate_context!())
        .expect("error while running PowerPlayer");
}
