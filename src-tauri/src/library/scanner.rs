use crate::db::manager::{DbManager, TrackInput};
use id3::TagLike;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
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
        if let Some(track) = extract_track(path) {
            match db.save_track(&track) {
                Ok(_) => {
                    saved_count.fetch_add(1, Ordering::Relaxed);
                }
                Err(err) => {
                    eprintln!("Failed to persist track {}: {err}", track.path);
                }
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

fn extract_track(path: &Path) -> Option<TrackInput> {
    let (mut title, mut artist, mut album, duration_seconds, sample_rate) =
        read_symphonia_metadata(path);

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

    Some(TrackInput {
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
    })
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
