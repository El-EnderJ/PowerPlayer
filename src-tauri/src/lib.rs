pub mod audio;

#[cfg(feature = "tauri-state")]
use audio::engine::AudioState;

#[cfg(feature = "tauri-state")]
#[tauri::command]
pub fn load_track(state: tauri::State<'_, AudioState>, path: String) -> Result<(), String> {
    state.load_track(path)
}

#[cfg(feature = "tauri-state")]
#[tauri::command]
pub fn play(state: tauri::State<'_, AudioState>) -> Result<(), String> {
    state.play();
    Ok(())
}

#[cfg(feature = "tauri-state")]
#[tauri::command]
pub fn pause(state: tauri::State<'_, AudioState>) -> Result<(), String> {
    state.pause();
    Ok(())
}

#[cfg(feature = "tauri-state")]
#[tauri::command]
pub fn seek(state: tauri::State<'_, AudioState>, seconds: f64) -> Result<(), String> {
    state.seek(seconds);
    Ok(())
}

#[cfg(feature = "tauri-state")]
#[tauri::command]
pub fn set_volume(state: tauri::State<'_, AudioState>, volume: f32) -> Result<(), String> {
    state.set_volume(volume);
    Ok(())
}
