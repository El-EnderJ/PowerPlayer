use crate::audio::decoder::read_track_metadata;
use crate::db::manager::{DbManager, TrackInput};
use crate::library::art_cache;
use crate::library::enrichment_queue;
use crate::library::metadata::art_fetcher;
use id3::TagLike;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, MetadataRevision, StandardTagKey},
    probe::Hint,
};
use walkdir::WalkDir;

pub fn scan_library_path(root: &Path, db: &DbManager) -> Result<usize, String> {
    let files = collect_audio_files(root);
    let saved_count = AtomicUsize::new(0);

    files.par_iter().for_each(|path| {
        let track = extract_track(path);
        if track.corrupted {
            eprintln!("Persisting track marked as corrupted: {}", track.path);
        }
        match db.save_track(&track) {
            Ok(_) => {
                saved_count.fetch_add(1, Ordering::Relaxed);
                enrichment_queue::enqueue(track.clone(), db.clone());
            }
            Err(err) => {
                eprintln!("Failed to persist track {}: {err}", track.path);
            }
        }
    });

    Ok(saved_count.load(Ordering::Relaxed))
}

fn collect_audio_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "flac" | "mp3" | "m4a" | "ogg" | "wav"
                    )
                })
                .unwrap_or(false)
        })
        .collect()
}

pub fn register_library_watch(path: &Path, db: &DbManager) -> Result<(), String> {
    watcher_manager()
        .lock()
        .map_err(|_| "Library watcher lock poisoned".to_string())?
        .register(path, db)
}

fn watcher_manager() -> &'static Mutex<LibraryWatcherManager> {
    static MANAGER: OnceLock<Mutex<LibraryWatcherManager>> = OnceLock::new();
    MANAGER.get_or_init(|| Mutex::new(LibraryWatcherManager::default()))
}

#[derive(Default)]
struct LibraryWatcherManager {
    watchers: Vec<RecommendedWatcher>,
    watched_paths: HashSet<PathBuf>,
}

impl LibraryWatcherManager {
    fn register(&mut self, path: &Path, db: &DbManager) -> Result<(), String> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if self.watched_paths.contains(&canonical) {
            return Ok(());
        }

        let db = db.clone();
        let mut watcher = RecommendedWatcher::new(
            move |event: notify::Result<Event>| {
                if let Ok(event) = event {
                    handle_library_event(event, &db);
                }
            },
            Config::default(),
        )
        .map_err(|e| format!("Failed to create library watcher: {e}"))?;
        watcher
            .watch(&canonical, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch {}: {e}", canonical.display()))?;
        self.watched_paths.insert(canonical);
        self.watchers.push(watcher);
        Ok(())
    }
}

fn handle_library_event(event: Event, db: &DbManager) {
    for path in event.paths {
        if !is_supported_audio_path(&path) {
            continue;
        }
        if path.exists() {
            let track = extract_track(&path);
            if let Err(err) = db.save_track(&track) {
                eprintln!("Failed to persist watched track {}: {err}", track.path);
            } else {
                enrichment_queue::enqueue(track, db.clone());
            }
        } else if let Err(err) = db.delete_track(path.to_string_lossy().as_ref()) {
            eprintln!("Failed to delete removed track {}: {err}", path.display());
        }
    }
}

fn extract_track(path: &Path) -> TrackInput {
    let (mut title, mut artist, mut album, duration_seconds, sample_rate) =
        read_symphonia_metadata(path);
    let mut corrupted = false;
    let mut art_url = None;

    match read_track_metadata(path) {
        Ok(metadata) => {
            if title.is_none() {
                title = metadata.title;
            }
            if artist.is_none() {
                artist = metadata.artist;
            }
            if let Some(cover_art) = metadata.cover_art {
                art_url = art_cache::cache_cover_art(path, &cover_art).ok().flatten();
            }
        }
        Err(err) => {
            corrupted = true;
            eprintln!("Corrupted track detected {}: {err}", path.display());
        }
    }

    if let Ok(tag) = id3::Tag::read_from_path(path) {
        if title.is_none() {
            title = tag.title().map(ToOwned::to_owned);
        }
        if artist.is_none() {
            artist = tag.artist().map(ToOwned::to_owned);
        }
        if album.is_none() {
            album = tag.album().map(ToOwned::to_owned);
        }
    }

    if art_url.is_none() {
        art_url = art_fetcher::find_local_cover(path)
            .and_then(|cover| art_cache::cache_cover_file(path, &cover).ok().flatten());
    }

    apply_filename_repair(path, &mut title, &mut artist, &mut corrupted);

    TrackInput {
        path: path.to_string_lossy().to_string(),
        title: title.or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(std::string::ToString::to_string)
        }),
        artist,
        album,
        duration_seconds,
        sample_rate,
        art_url,
        corrupted,
    }
}

fn apply_filename_repair(
    path: &Path,
    title: &mut Option<String>,
    artist: &mut Option<String>,
    corrupted: &mut bool,
) {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let Some((file_artist, file_title)) = parse_artist_title_from_stem(stem) else {
        return;
    };
    if artist.is_none() {
        *artist = Some(file_artist);
    }
    if title.is_none() {
        *title = Some(file_title);
    }
    if artist.is_some() && title.is_some() {
        *corrupted = false;
    }
}

fn parse_artist_title_from_stem(stem: &str) -> Option<(String, String)> {
    let mut parts = stem.splitn(2, " - ");
    let artist = parts.next()?.trim();
    let title = parts.next()?.trim();
    if artist.is_empty() || title.is_empty() {
        return None;
    }
    Some((artist.to_string(), title.to_string()))
}

fn read_symphonia_metadata(
    path: &Path,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<f32>,
    Option<u32>,
) {
    let Ok(file) = std::fs::File::open(path) else {
        return (None, None, None, None, None);
    };
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }
    let Ok(mut probed) = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) else {
        return (None, None, None, None, None);
    };

    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;
    let mut album: Option<String> = None;
    if let Some(pre_metadata) = probed.metadata.get() {
        if let Some(revision) = pre_metadata.current() {
            apply_revision_metadata(revision, &mut title, &mut artist, &mut album);
        }
    }

    let format = &mut probed.format;
    if let Some(revision) = format.metadata().current() {
        apply_revision_metadata(revision, &mut title, &mut artist, &mut album);
    }

    let mut duration_seconds = None;
    let sample_rate = format.default_track().and_then(|track| {
        if let (Some(sample_rate), Some(n_frames)) =
            (track.codec_params.sample_rate, track.codec_params.n_frames)
        {
            if sample_rate > 0 {
                duration_seconds = Some(n_frames as f32 / sample_rate as f32);
            }
        }
        track.codec_params.sample_rate
    });

    (title, artist, album, duration_seconds, sample_rate)
}

fn is_supported_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "flac" | "mp3" | "m4a" | "ogg" | "wav"
            )
        })
        .unwrap_or(false)
}

fn apply_revision_metadata(
    revision: &MetadataRevision,
    title: &mut Option<String>,
    artist: &mut Option<String>,
    album: &mut Option<String>,
) {
    for tag in revision.tags() {
        if title.is_none() && matches!(tag.std_key, Some(StandardTagKey::TrackTitle)) {
            *title = Some(tag.value.to_string());
        }
        if artist.is_none()
            && matches!(
                tag.std_key,
                Some(
                    StandardTagKey::Artist
                        | StandardTagKey::AlbumArtist
                        | StandardTagKey::Performer
                )
            )
        {
            *artist = Some(tag.value.to_string());
        }
        if album.is_none() && matches!(tag.std_key, Some(StandardTagKey::Album)) {
            *album = Some(tag.value.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_track, parse_artist_title_from_stem};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_audio_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("powerplayer-corrupted-{nanos}.flac"))
    }

    #[test]
    fn corrupted_file_is_marked_without_panicking_scan() {
        let path = unique_audio_path();
        std::fs::write(&path, b"not-a-real-flac").expect("test file should be created");

        let track = extract_track(&path);
        assert!(track.corrupted);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn filename_repair_extracts_artist_and_title() {
        let parsed = parse_artist_title_from_stem("Daft Punk - One More Time");
        assert_eq!(
            parsed,
            Some(("Daft Punk".to_string(), "One More Time".to_string()))
        );
    }
}
