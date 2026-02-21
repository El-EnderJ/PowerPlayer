use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const LRCLIB_GET_URL: &str = "https://lrclib.net/api/get";

pub fn download_lyrics_for_track(
    track_path: &Path,
    artist: &str,
    title: &str,
    duration_seconds: Option<f32>,
) -> Option<PathBuf> {
    if artist.trim().is_empty() || title.trim().is_empty() {
        return None;
    }
    let cache_path = cached_lyrics_path(track_path);
    if cache_path.is_file() {
        return Some(cache_path);
    }

    let duration = duration_seconds?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let duration = duration.round() as u32;
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("PowerPlayer/0.1")
        .build()
        .ok()?;
    let response = client
        .get(LRCLIB_GET_URL)
        .query(&[
            ("artist_name", artist.to_string()),
            ("track_name", title.to_string()),
            ("duration", duration.to_string()),
        ])
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload: LrcLibResponse = response.json().ok()?;
    let synced = payload
        .synced_lyrics
        .filter(|value| !value.trim().is_empty())?;
    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&cache_path, synced).ok()?;
    Some(cache_path)
}

pub fn cached_lyrics_path(track_path: &Path) -> PathBuf {
    let mut hash = Sha256::new();
    hash.update(track_path.to_string_lossy().as_bytes());
    let filename = format!("{:x}.lrc", hash.finalize());
    app_dir().join(".lyrics_cache").join(filename)
}

#[cfg(not(test))]
fn app_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
}

#[cfg(test)]
fn app_dir() -> PathBuf {
    std::env::temp_dir().join("powerplayer-test-cache")
}

#[derive(Deserialize)]
struct LrcLibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
}
