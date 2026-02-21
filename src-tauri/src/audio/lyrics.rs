use crate::audio::lyrics_downloader;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricsLine {
    pub timestamp: u32,
    pub text: String,
}

pub fn find_lrc_for_track(track_path: &Path) -> Option<PathBuf> {
    let stem = track_path.file_stem()?;
    let parent = track_path.parent()?;
    let candidate = parent.join(stem).with_extension("lrc");
    if candidate.is_file() {
        return Some(candidate);
    }
    let cached = lyrics_downloader::cached_lyrics_path(track_path);
    cached.is_file().then_some(cached)
}

pub fn load_lyrics_for_track(track_path: &Path) -> Vec<LyricsLine> {
    let Some(lrc_path) = find_lrc_for_track(track_path) else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(lrc_path) else {
        return Vec::new();
    };
    parse_lrc(&content)
}

pub fn parse_lrc(content: &str) -> Vec<LyricsLine> {
    let mut lines = Vec::new();
    for raw in content.lines() {
        let parsed = parse_line(raw);
        lines.extend(parsed);
    }
    lines.sort_by_key(|line| line.timestamp);
    lines
}

fn parse_line(line: &str) -> Vec<LyricsLine> {
    let mut rest = line.trim();
    let mut timestamps = Vec::new();
    while let Some(stripped) = rest.strip_prefix('[') {
        let Some(close) = stripped.find(']') else {
            break;
        };
        let time_token = &stripped[..close];
        let Some(timestamp) = parse_timestamp(time_token) else {
            break;
        };
        timestamps.push(timestamp);
        rest = stripped[close + 1..].trim_start();
    }
    if timestamps.is_empty() {
        return Vec::new();
    }
    let text = rest.trim().to_string();
    timestamps
        .into_iter()
        .map(|timestamp| LyricsLine {
            timestamp,
            text: text.clone(),
        })
        .collect()
}

fn parse_timestamp(value: &str) -> Option<u32> {
    let mut parts = value.split(':');
    let minutes = parts.next()?.trim().parse::<u32>().ok()?;
    if minutes > 6_000 {
        return None;
    }
    let sec_fraction = parts.next()?.trim();
    if parts.next().is_some() {
        return None;
    }
    let mut sec_parts = sec_fraction.split('.');
    let seconds = sec_parts.next()?.trim().parse::<u32>().ok()?;
    if seconds >= 60 {
        return None;
    }
    let fraction = sec_parts.next().unwrap_or("0").trim();
    if sec_parts.next().is_some() {
        return None;
    }
    let millis = parse_fraction_to_millis(fraction)?;
    minutes
        .checked_mul(60_000)?
        .checked_add(seconds.checked_mul(1_000)?)?
        .checked_add(millis)
}

fn parse_fraction_to_millis(fraction: &str) -> Option<u32> {
    if fraction.is_empty() {
        return Some(0);
    }
    if !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let mut digits = fraction.chars();
    let d1 = digits.next().unwrap_or('0').to_digit(10).unwrap_or(0);
    let d2 = digits.next().unwrap_or('0').to_digit(10).unwrap_or(0);
    let d3 = digits.next().unwrap_or('0').to_digit(10).unwrap_or(0);
    Some(d1 * 100 + d2 * 10 + d3)
}

#[cfg(test)]
mod tests {
    use super::{find_lrc_for_track, parse_lrc, LyricsLine};
    use crate::audio::lyrics_downloader::cached_lyrics_path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_single_timestamp_line() {
        let parsed = parse_lrc("[01:02.34] Hello world");
        assert_eq!(
            parsed,
            vec![LyricsLine {
                timestamp: 62_340,
                text: "Hello world".to_string()
            }]
        );
    }

    #[test]
    fn parses_multiple_timestamps_in_one_line() {
        let parsed = parse_lrc("[00:10.00][00:12.50] Chorus");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].timestamp, 10_000);
        assert_eq!(parsed[1].timestamp, 12_500);
        assert_eq!(parsed[0].text, "Chorus");
    }

    #[test]
    fn ignores_invalid_lines_and_sorts() {
        let parsed = parse_lrc("[00:20.xx]bad\n[00:15.00]A\n[00:10.00]B");
        assert_eq!(
            parsed
                .iter()
                .map(|line| (line.timestamp, line.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(10_000, "B"), (15_000, "A")]
        );
    }

    #[test]
    fn falls_back_to_cached_lrc_file() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let track = std::env::temp_dir().join(format!("powerplayer-lyrics-{nanos}.flac"));
        let cached = cached_lyrics_path(&track);
        if let Some(parent) = cached.parent() {
            std::fs::create_dir_all(parent).expect("cache directory should exist");
        }
        std::fs::write(&cached, "[00:01.00] cached").expect("cached lyrics should be written");

        let found = find_lrc_for_track(&track);
        assert_eq!(found.as_deref(), Some(cached.as_path()));

        let _ = std::fs::remove_file(cached);
    }
}
