use rusqlite::params;
use serde::Serialize;

use super::manager::DbManager;

#[derive(Clone, Debug, Serialize)]
pub struct SearchResultTrack {
    pub id: i64,
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_seconds: Option<f32>,
    pub sample_rate: Option<u32>,
    pub art_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchResults {
    pub tracks: Vec<SearchResultTrack>,
    pub albums: Vec<String>,
    pub artists: Vec<String>,
}

impl DbManager {
    /// Creates the FTS5 virtual table for full-text search.
    /// Called once during schema initialization.
    pub fn initialize_fts(&self) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
                title, artist, album, content='tracks', content_rowid='id'
            );",
        )
        .map_err(|e| format!("Failed to create FTS5 virtual table: {e}"))?;

        // Triggers to keep FTS in sync with the tracks table
        conn.execute_batch(
            "CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
                INSERT INTO tracks_fts(rowid, title, artist, album)
                VALUES (new.id, new.title, new.artist, new.album);
            END;
            CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks BEGIN
                INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album)
                VALUES ('delete', old.id, old.title, old.artist, old.album);
            END;
            CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks BEGIN
                INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album)
                VALUES ('delete', old.id, old.title, old.artist, old.album);
                INSERT INTO tracks_fts(rowid, title, artist, album)
                VALUES (new.id, new.title, new.artist, new.album);
            END;",
        )
        .map_err(|e| format!("Failed to create FTS5 triggers: {e}"))?;

        // Populate FTS from existing tracks (idempotent rebuild)
        conn.execute_batch(
            "INSERT OR IGNORE INTO tracks_fts(rowid, title, artist, album)
             SELECT id, title, artist, album FROM tracks;",
        )
        .map_err(|e| format!("Failed to populate FTS5 table: {e}"))?;

        Ok(())
    }

    /// Ultra-fast full-text search using FTS5. Accepts a user query and returns
    /// results grouped by tracks, albums, and artists.
    pub fn fast_search(&self, query: &str) -> Result<SearchResults, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(SearchResults {
                tracks: Vec::new(),
                albums: Vec::new(),
                artists: Vec::new(),
            });
        }

        // FTS5 prefix search: append * to each token for partial matching
        let fts_query = trimmed
            .split_whitespace()
            .map(|word| format!("\"{}\"*", word.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" ");

        let conn = self.connection()?;

        // Matching tracks
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.path, t.title, t.artist, t.album,
                        t.duration_seconds, t.sample_rate, t.art_url
                 FROM tracks_fts f
                 JOIN tracks t ON t.id = f.rowid
                 WHERE tracks_fts MATCH ?1
                 ORDER BY rank
                 LIMIT 100",
            )
            .map_err(|e| format!("FTS query prepare failed: {e}"))?;

        let tracks: Vec<SearchResultTrack> = stmt
            .query_map(params![fts_query], |row| {
                Ok(SearchResultTrack {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    album: row.get(4)?,
                    duration_seconds: row.get(5)?,
                    sample_rate: row.get(6)?,
                    art_url: row.get(7)?,
                })
            })
            .map_err(|e| format!("FTS track query failed: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("FTS track row read failed: {e}"))?;

        // Distinct matching albums
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT t.album
                 FROM tracks_fts f
                 JOIN tracks t ON t.id = f.rowid
                 WHERE tracks_fts MATCH ?1 AND t.album IS NOT NULL AND t.album != ''
                 ORDER BY rank
                 LIMIT 50",
            )
            .map_err(|e| format!("FTS album query prepare failed: {e}"))?;

        let albums: Vec<String> = stmt
            .query_map(params![fts_query], |row| row.get(0))
            .map_err(|e| format!("FTS album query failed: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("FTS album row read failed: {e}"))?;

        // Distinct matching artists
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT t.artist
                 FROM tracks_fts f
                 JOIN tracks t ON t.id = f.rowid
                 WHERE tracks_fts MATCH ?1 AND t.artist IS NOT NULL AND t.artist != ''
                 ORDER BY rank
                 LIMIT 50",
            )
            .map_err(|e| format!("FTS artist query prepare failed: {e}"))?;

        let artists: Vec<String> = stmt
            .query_map(params![fts_query], |row| row.get(0))
            .map_err(|e| format!("FTS artist query failed: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("FTS artist row read failed: {e}"))?;

        Ok(SearchResults {
            tracks,
            albums,
            artists,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::manager::{DbManager, TrackInput};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_db_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("powerplayer-search-test-{nanos}.db"))
    }

    #[test]
    fn fts_search_finds_tracks_by_artist() {
        let path = unique_db_path();
        let db = DbManager::new(&path).expect("db should initialize");
        db.initialize_fts().expect("FTS should initialize");

        db.save_track(&TrackInput {
            path: "/music/michael1.flac".to_string(),
            title: Some("Billie Jean".to_string()),
            artist: Some("Michael Jackson".to_string()),
            album: Some("Thriller".to_string()),
            duration_seconds: Some(295.0),
            sample_rate: Some(44100),
            art_url: None,
            corrupted: false,
        })
        .expect("save should work");

        db.save_track(&TrackInput {
            path: "/music/other.flac".to_string(),
            title: Some("Someone Else".to_string()),
            artist: Some("Other Artist".to_string()),
            album: Some("Other Album".to_string()),
            duration_seconds: Some(180.0),
            sample_rate: Some(44100),
            art_url: None,
            corrupted: false,
        })
        .expect("save should work");

        let results = db.fast_search("Michael").expect("search should work");
        assert_eq!(results.tracks.len(), 1);
        assert_eq!(results.tracks[0].artist.as_deref(), Some("Michael Jackson"));
        assert!(results.artists.contains(&"Michael Jackson".to_string()));
    }

    #[test]
    fn fts_search_empty_query_returns_empty() {
        let path = unique_db_path();
        let db = DbManager::new(&path).expect("db should initialize");
        db.initialize_fts().expect("FTS should initialize");

        let results = db.fast_search("").expect("search should work");
        assert!(results.tracks.is_empty());
        assert!(results.albums.is_empty());
        assert!(results.artists.is_empty());
    }

    #[test]
    fn fts_search_finds_by_album() {
        let path = unique_db_path();
        let db = DbManager::new(&path).expect("db should initialize");
        db.initialize_fts().expect("FTS should initialize");

        db.save_track(&TrackInput {
            path: "/music/track1.flac".to_string(),
            title: Some("Track One".to_string()),
            artist: Some("Some Artist".to_string()),
            album: Some("Michael".to_string()),
            duration_seconds: Some(200.0),
            sample_rate: Some(48000),
            art_url: None,
            corrupted: false,
        })
        .expect("save should work");

        let results = db.fast_search("Michael").expect("search should work");
        assert_eq!(results.tracks.len(), 1);
        assert!(results.albums.contains(&"Michael".to_string()));
    }
}
