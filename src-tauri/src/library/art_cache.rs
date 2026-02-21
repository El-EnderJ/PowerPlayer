use crate::audio::decoder::CoverArt;
use image::{codecs::jpeg::JpegEncoder, ColorType};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

const THUMBNAIL_SIZE: u32 = 256;
const ART_CACHE_MAX_FILES: usize = 512;

pub fn cache_cover_art(track_path: &Path, cover_art: &CoverArt) -> Result<Option<String>, String> {
    cache_cover_bytes(track_path, &cover_art.data)
}

pub fn cache_cover_file(track_path: &Path, art_path: &Path) -> Result<Option<String>, String> {
    let bytes = fs::read(art_path)
        .map_err(|e| format!("Failed to read cover art file {}: {e}", art_path.display()))?;
    cache_cover_bytes(track_path, &bytes)
}

pub fn cache_cover_bytes(track_path: &Path, bytes: &[u8]) -> Result<Option<String>, String> {
    let cache_file = cache_file_path(track_path);
    if !cache_file.exists() {
        if let Some(cache_dir) = cache_file.parent() {
            prune_flat_cache_dir(cache_dir, ART_CACHE_MAX_FILES);
        }
        let image = image::load_from_memory(bytes)
            .map_err(|e| format!("Failed to decode embedded cover art: {e}"))?;
        let thumbnail = image.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE).to_rgb8();
        let mut encoded = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut encoded, 80);
        encoder
            .encode(
                &thumbnail,
                thumbnail.width(),
                thumbnail.height(),
                ColorType::Rgb8.into(),
            )
            .map_err(|e| format!("Failed to encode cover thumbnail: {e}"))?;
        fs::write(&cache_file, encoded)
            .map_err(|e| format!("Failed to write cached art {}: {e}", cache_file.display()))?;
    }

    Ok(Some(to_asset_url(&cache_file)))
}

fn prune_flat_cache_dir(dir: &Path, max_files: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            Some((path, modified))
        })
        .collect::<Vec<_>>();

    if files.len() <= max_files {
        return;
    }
    files.sort_by_key(|(_, modified)| *modified);
    for (path, _) in files.iter().take(files.len() - max_files) {
        let _ = fs::remove_file(path);
    }
}

fn cache_file_path(track_path: &Path) -> PathBuf {
    let mut hash = Sha256::new();
    hash.update(track_path.to_string_lossy().as_bytes());
    let filename = format!("{:x}.jpg", hash.finalize());

    let cache_dir = std::env::temp_dir().join("powerplayer").join("art_cache");
    let _ = fs::create_dir_all(&cache_dir);
    cache_dir.join(filename)
}

fn to_asset_url(path: &Path) -> String {
    format!("asset://{}", path.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::cache_cover_art;
    use crate::audio::decoder::CoverArt;
    use image::{codecs::jpeg::JpegEncoder, ColorType, RgbImage};
    use std::path::Path;

    #[test]
    fn caches_cover_art_as_asset_url() {
        let image = RgbImage::from_pixel(16, 16, image::Rgb([255, 0, 0]));
        let mut bytes = Vec::new();
        JpegEncoder::new(&mut bytes)
            .encode(
                &image,
                image.width(),
                image.height(),
                ColorType::Rgb8.into(),
            )
            .expect("test jpeg should encode");

        let art = CoverArt {
            media_type: "image/jpeg".to_string(),
            data: bytes,
        };
        let url = cache_cover_art(Path::new("/tmp/test-track.flac"), &art)
            .expect("cache operation should work")
            .expect("url should exist");

        assert!(url.starts_with("asset://"));
    }
}
