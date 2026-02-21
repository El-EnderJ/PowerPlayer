use rusqlite::params;

use crate::db::manager::DbManager;

/// Row from the `spatial_scenes` table.
#[derive(Clone, Debug, serde::Serialize)]
pub struct SpatialSceneRow {
    pub track_id: String,
    pub source_name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub is_active: bool,
}

impl DbManager {
    /// Create the spatial_scenes table if it doesn't exist.
    pub fn initialize_spatial_schema(&self) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS spatial_scenes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id TEXT NOT NULL,
                source_name TEXT NOT NULL,
                x REAL NOT NULL DEFAULT 0.0,
                y REAL NOT NULL DEFAULT 0.0,
                z REAL NOT NULL DEFAULT 0.0,
                is_active INTEGER NOT NULL DEFAULT 1,
                UNIQUE(track_id, source_name)
            );",
        )
        .map_err(|e| format!("Failed to create spatial_scenes table: {e}"))?;
        Ok(())
    }

    /// Save or update a single source position for a track.
    pub fn save_spatial_scene(
        &self,
        track_id: &str,
        source_name: &str,
        x: f32,
        y: f32,
        z: f32,
        is_active: bool,
    ) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO spatial_scenes (track_id, source_name, x, y, z, is_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(track_id, source_name) DO UPDATE SET
                  x = excluded.x,
                  y = excluded.y,
                  z = excluded.z,
                  is_active = excluded.is_active",
            params![track_id, source_name, x, y, z, is_active as i32],
        )
        .map_err(|e| format!("Failed to save spatial scene: {e}"))?;
        Ok(())
    }

    /// Load all source positions for a given track.
    pub fn load_spatial_scene(&self, track_id: &str) -> Result<Vec<SpatialSceneRow>, String> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT track_id, source_name, x, y, z, is_active
                 FROM spatial_scenes
                 WHERE track_id = ?1
                 ORDER BY source_name",
            )
            .map_err(|e| format!("Failed to prepare spatial scene query: {e}"))?;

        let rows = stmt
            .query_map(params![track_id], |row| {
                Ok(SpatialSceneRow {
                    track_id: row.get(0)?,
                    source_name: row.get(1)?,
                    x: row.get(2)?,
                    y: row.get(3)?,
                    z: row.get(4)?,
                    is_active: row.get::<_, i32>(5)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query spatial scenes: {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read spatial scene rows: {e}"))
    }

    /// Delete all spatial scene data for a track.
    pub fn delete_spatial_scene(&self, track_id: &str) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute(
            "DELETE FROM spatial_scenes WHERE track_id = ?1",
            params![track_id],
        )
        .map_err(|e| format!("Failed to delete spatial scene: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::manager::DbManager;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_db_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("powerplayer-spatial-test-{nanos}.db"))
    }

    #[test]
    fn spatial_schema_creates_table() {
        let db = DbManager::new(unique_db_path()).expect("db init");
        db.initialize_spatial_schema()
            .expect("spatial schema should initialize");
        // Calling again should be idempotent
        db.initialize_spatial_schema()
            .expect("second call should be fine");
    }

    #[test]
    fn save_and_load_spatial_scene() {
        let db = DbManager::new(unique_db_path()).expect("db init");
        db.initialize_spatial_schema().expect("schema");

        db.save_spatial_scene("/music/song.flac", "vocals", 1.0, 2.0, 3.0, true)
            .expect("save vocals");
        db.save_spatial_scene("/music/song.flac", "drums", 4.0, 5.0, 6.0, false)
            .expect("save drums");

        let rows = db
            .load_spatial_scene("/music/song.flac")
            .expect("load scene");
        assert_eq!(rows.len(), 2);

        let drums = rows.iter().find(|r| r.source_name == "drums").unwrap();
        assert!((drums.x - 4.0).abs() < f32::EPSILON);
        assert!((drums.y - 5.0).abs() < f32::EPSILON);
        assert!(!drums.is_active);
    }

    #[test]
    fn save_spatial_scene_upserts() {
        let db = DbManager::new(unique_db_path()).expect("db init");
        db.initialize_spatial_schema().expect("schema");

        db.save_spatial_scene("/music/song.flac", "vocals", 1.0, 2.0, 3.0, true)
            .expect("first save");
        db.save_spatial_scene("/music/song.flac", "vocals", 7.0, 8.0, 9.0, false)
            .expect("upsert");

        let rows = db
            .load_spatial_scene("/music/song.flac")
            .expect("load scene");
        assert_eq!(rows.len(), 1);
        assert!((rows[0].x - 7.0).abs() < f32::EPSILON);
        assert!(!rows[0].is_active);
    }

    #[test]
    fn delete_spatial_scene_removes_all() {
        let db = DbManager::new(unique_db_path()).expect("db init");
        db.initialize_spatial_schema().expect("schema");

        db.save_spatial_scene("/music/song.flac", "vocals", 1.0, 2.0, 3.0, true)
            .expect("save");
        db.delete_spatial_scene("/music/song.flac")
            .expect("delete");

        let rows = db
            .load_spatial_scene("/music/song.flac")
            .expect("load scene");
        assert!(rows.is_empty());
    }

    #[test]
    fn load_empty_scene_returns_empty() {
        let db = DbManager::new(unique_db_path()).expect("db init");
        db.initialize_spatial_schema().expect("schema");

        let rows = db
            .load_spatial_scene("/nonexistent/track")
            .expect("load scene");
        assert!(rows.is_empty());
    }
}
