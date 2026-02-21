use crate::library::art_cache;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ITUNES_SEARCH_URL: &str = "https://itunes.apple.com/search";
const MUSICBRAINZ_SEARCH_URL: &str = "https://musicbrainz.org/ws/2/recording";

pub fn find_local_cover(track_path: &Path) -> Option<PathBuf> {
    let parent = track_path.parent()?;
    ["cover.jpg", "cover.jpeg", "folder.jpg", "folder.jpeg"]
        .iter()
        .map(|name| parent.join(name))
        .find(|path| path.is_file())
}

pub fn fetch_and_cache_art(
    track_path: &Path,
    artist: Option<&str>,
    title: Option<&str>,
) -> Result<Option<String>, String> {
    if let Some(local_cover) = find_local_cover(track_path) {
        return art_cache::cache_cover_file(track_path, &local_cover);
    }

    let Some(title) = title.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("PowerPlayer/0.1")
        .build()
        .map_err(|e| format!("Failed to build art HTTP client: {e}"))?;

    if let Some(bytes) = fetch_from_itunes(&client, artist, title) {
        return art_cache::cache_cover_bytes(track_path, &bytes);
    }
    if let Some(bytes) = fetch_from_musicbrainz(&client, artist, title) {
        return art_cache::cache_cover_bytes(track_path, &bytes);
    }

    Ok(None)
}

fn fetch_from_itunes(client: &Client, artist: Option<&str>, title: &str) -> Option<Vec<u8>> {
    let term = artist
        .filter(|value| !value.trim().is_empty())
        .map(|name| format!("{name} {title}"))
        .unwrap_or_else(|| title.to_string());
    let response = client
        .get(ITUNES_SEARCH_URL)
        .query(&[
            ("term", term),
            ("entity", "song".to_string()),
            ("limit", "1".to_string()),
        ])
        .send()
        .ok()?;
    let data: ItunesSearchResponse = response.json().ok()?;
    let artwork_url = data
        .results
        .into_iter()
        .find_map(|item| item.artwork_url100.or(item.artwork_url60))?;
    client
        .get(artwork_url)
        .send()
        .ok()?
        .bytes()
        .ok()
        .map(|b| b.to_vec())
}

fn fetch_from_musicbrainz(client: &Client, artist: Option<&str>, title: &str) -> Option<Vec<u8>> {
    let mut query = format!("recording:\"{title}\"");
    if let Some(artist) = artist.filter(|value| !value.trim().is_empty()) {
        query.push_str(&format!(" AND artist:\"{artist}\""));
    }
    let search = client
        .get(MUSICBRAINZ_SEARCH_URL)
        .query(&[
            ("query", query),
            ("fmt", "json".to_string()),
            ("limit", "1".to_string()),
        ])
        .send()
        .ok()?;
    let data: MusicBrainzSearchResponse = search.json().ok()?;
    let release_id = data.recordings.into_iter().find_map(|recording| {
        recording
            .releases
            .and_then(|releases| releases.into_iter().next())
            .map(|release| release.id)
    })?;
    let cover_url = format!("https://coverartarchive.org/release/{release_id}/front-500");
    client
        .get(cover_url)
        .send()
        .ok()?
        .bytes()
        .ok()
        .map(|b| b.to_vec())
}

#[derive(Deserialize)]
struct ItunesSearchResponse {
    #[serde(default)]
    results: Vec<ItunesResult>,
}

#[derive(Deserialize)]
struct ItunesResult {
    #[serde(rename = "artworkUrl100")]
    artwork_url100: Option<String>,
    #[serde(rename = "artworkUrl60")]
    artwork_url60: Option<String>,
}

#[derive(Deserialize)]
struct MusicBrainzSearchResponse {
    #[serde(default)]
    recordings: Vec<MusicBrainzRecording>,
}

#[derive(Deserialize)]
struct MusicBrainzRecording {
    releases: Option<Vec<MusicBrainzRelease>>,
}

#[derive(Deserialize)]
struct MusicBrainzRelease {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::find_local_cover;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_folder_cover_without_network() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("powerplayer-art-{nanos}"));
        std::fs::create_dir_all(&dir).expect("test folder should exist");
        let track_path = dir.join("track.flac");
        std::fs::write(&track_path, b"audio").expect("dummy track should be created");
        let cover_path = dir.join("folder.jpg");
        std::fs::write(&cover_path, b"jpg").expect("dummy cover should be created");

        let found = find_local_cover(&track_path);
        assert_eq!(found.as_deref(), Some(cover_path.as_path()));

        let _ = std::fs::remove_file(cover_path);
        let _ = std::fs::remove_file(track_path);
        let _ = std::fs::remove_dir(dir);
    }
}
