use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;

#[derive(Clone)]
pub struct DbManager {
    pool: Pool<SqliteConnectionManager>,
}

#[derive(Clone, Debug)]
pub struct TrackInput {
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_seconds: Option<f32>,
    pub sample_rate: Option<u32>,
    pub art_url: Option<String>,
    pub corrupted: bool,
}

#[derive(Clone, Debug)]
pub struct TrackRecord {
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_seconds: Option<f32>,
    pub sample_rate: Option<u32>,
    pub art_url: Option<String>,
    pub corrupted: bool,
}

impl DbManager {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, String> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager).map_err(|e| format!("Failed to create DB pool: {e}"))?;
        let db = Self { pool };
        db.initialize_schema()?;
        Ok(db)
    }

    pub fn save_track(&self, track: &TrackInput) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO tracks (path, title, artist, album, duration_seconds, sample_rate, art_url, corrupted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(path) DO UPDATE SET
                  title = excluded.title,
                  artist = excluded.artist,
                  album = excluded.album,
                  duration_seconds = excluded.duration_seconds,
                  sample_rate = excluded.sample_rate,
                  art_url = excluded.art_url,
                  corrupted = excluded.corrupted,
                  updated_at = CURRENT_TIMESTAMP",
            params![
                track.path,
                track.title,
                track.artist,
                track.album,
                track.duration_seconds,
                track.sample_rate,
                track.art_url,
                track.corrupted as i32
            ],
        )
        .map_err(|e| format!("Failed to save track {}: {e}", track.path))?;

        // Empty/blank album names are intentionally skipped to keep the albums table normalized.
        if let Some(album) = track.album.as_ref().filter(|name| !name.trim().is_empty()) {
            conn.execute(
                "INSERT INTO albums (name, artist) VALUES (?1, ?2)
                 ON CONFLICT(name, artist) DO NOTHING",
                params![album, track.artist],
            )
            .map_err(|e| format!("Failed to save album {}: {e}", album))?;
        }

        Ok(())
    }

    pub fn get_tracks(&self) -> Result<Vec<TrackRecord>, String> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT path, title, artist, album, duration_seconds, sample_rate, art_url, corrupted
                 FROM tracks
                 ORDER BY artist COLLATE NOCASE, album COLLATE NOCASE, title COLLATE NOCASE, path",
            )
            .map_err(|e| format!("Failed to prepare track query: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(TrackRecord {
                    path: row.get(0)?,
                    title: row.get(1)?,
                    artist: row.get(2)?,
                    album: row.get(3)?,
                    duration_seconds: row.get(4)?,
                    sample_rate: row.get(5)?,
                    art_url: row.get(6)?,
                    corrupted: row.get::<_, i32>(7)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query tracks: {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read tracks: {e}"))
    }

    fn initialize_schema(&self) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                title TEXT,
                artist TEXT,
                album TEXT,
                duration_seconds REAL,
                sample_rate INTEGER,
                art_url TEXT,
                corrupted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS albums (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                artist TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(name, artist)
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT
            );",
        )
        .map_err(|e| format!("Failed to initialize DB schema: {e}"))?;
        self.ensure_track_column("art_url", "TEXT")?;
        self.ensure_track_column("corrupted", "INTEGER NOT NULL DEFAULT 0")?;
        Ok(())
    }

    pub fn delete_track(&self, path: &str) -> Result<(), String> {
        self.connection()?
            .execute("DELETE FROM tracks WHERE path = ?1", params![path])
            .map_err(|e| format!("Failed to delete track {path}: {e}"))?;
        Ok(())
    }

    fn ensure_track_column(&self, name: &str, definition: &str) -> Result<(), String> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare("PRAGMA table_info(tracks)")
            .map_err(|e| format!("Failed to inspect tracks schema: {e}"))?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| format!("Failed to read tracks schema rows: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect tracks schema: {e}"))?;

        if columns.iter().any(|column| column == name) {
            return Ok(());
        }

        conn.execute(
            &format!("ALTER TABLE tracks ADD COLUMN {name} {definition}"),
            [],
        )
        .map_err(|e| format!("Failed to add tracks.{name} column: {e}"))?;
        Ok(())
    }

    pub(crate) fn connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get DB connection from pool: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{DbManager, TrackInput};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_db_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("powerplayer-test-{nanos}.db"))
    }

    #[test]
    fn save_track_is_idempotent_by_path() {
        let path = unique_db_path();
        let db = DbManager::new(&path).expect("db should initialize");
        let first = TrackInput {
            path: "/music/song.flac".to_string(),
            title: Some("Song A".to_string()),
            artist: Some("Artist A".to_string()),
            album: Some("Album A".to_string()),
            duration_seconds: Some(120.0),
            sample_rate: Some(48_000),
            art_url: Some("asset:///tmp/art.jpg".to_string()),
            corrupted: false,
        };
        db.save_track(&first).expect("first save should work");

        let second = TrackInput {
            title: Some("Song B".to_string()),
            ..first.clone()
        };
        db.save_track(&second).expect("second save should upsert");

        let rows = db.get_tracks().expect("tracks should load");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title.as_deref(), Some("Song B"));
        assert!(!rows[0].corrupted);
    }

    #[test]
    fn delete_track_removes_row() {
        let path = unique_db_path();
        let db = DbManager::new(&path).expect("db should initialize");
        let track = TrackInput {
            path: "/music/delete-me.flac".to_string(),
            title: None,
            artist: None,
            album: None,
            duration_seconds: None,
            sample_rate: None,
            art_url: None,
            corrupted: true,
        };
        db.save_track(&track).expect("save should work");
        db.delete_track(&track.path).expect("delete should work");

        let rows = db.get_tracks().expect("tracks should load");
        assert!(rows.is_empty());
    }
}
