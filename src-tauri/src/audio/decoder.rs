use std::{fs::File, path::Path};

use symphonia::core::{
    audio::SampleBuffer,
    codecs::DecoderOptions,
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, MetadataRevision, StandardTagKey},
    probe::Hint,
};

#[derive(Clone, Debug)]
pub struct DecodedTrack {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct CoverArt {
    pub media_type: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct TrackMetadata {
    pub artist: Option<String>,
    pub title: Option<String>,
    pub cover_art: Option<CoverArt>,
    pub duration_seconds: Option<f32>,
}

pub fn read_track_metadata(path: &Path) -> Result<TrackMetadata, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open file {}: {e}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Format probe failed: {e}"))?;

    let mut metadata = TrackMetadata {
        artist: None,
        title: path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(std::string::ToString::to_string),
        cover_art: None,
        duration_seconds: None,
    };

    if let Some(mut pre_metadata) = probed.metadata.get() {
        if let Some(revision) = pre_metadata.current() {
            apply_metadata_revision(revision, &mut metadata);
        }
    }

    let format = &mut probed.format;
    if let Some(revision) = format.metadata().current() {
        apply_metadata_revision(revision, &mut metadata);
    }

    if let Some(track) = format.default_track() {
        if let (Some(sample_rate), Some(n_frames)) =
            (track.codec_params.sample_rate, track.codec_params.n_frames)
        {
            if sample_rate > 0 {
                metadata.duration_seconds = Some(n_frames as f32 / sample_rate as f32);
            }
        }
    }

    Ok(metadata)
}

fn apply_metadata_revision(revision: &MetadataRevision, metadata: &mut TrackMetadata) {
    for tag in revision.tags() {
        if metadata.artist.is_none() {
            if matches!(
                tag.std_key,
                Some(
                    StandardTagKey::Artist
                        | StandardTagKey::AlbumArtist
                        | StandardTagKey::Performer
                )
            ) {
                metadata.artist = Some(tag.value.to_string());
            }
        }

        if metadata.title.is_none() && matches!(tag.std_key, Some(StandardTagKey::TrackTitle)) {
            metadata.title = Some(tag.value.to_string());
        }
    }

    if metadata.cover_art.is_none() {
        if let Some(visual) = revision.visuals().first() {
            metadata.cover_art = Some(CoverArt {
                media_type: visual.media_type.clone(),
                data: visual.data.to_vec(),
            });
        }
    }
}

pub fn decode_file(path: &Path) -> Result<DecodedTrack, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open file {}: {e}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Format probe failed: {e}"))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "No default audio track found".to_string())?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Decoder creation failed: {e}"))?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| "Track has no sample-rate metadata".to_string())?;
    let channels = track
        .codec_params
        .channels
        .ok_or_else(|| "Track has no channel metadata".to_string())?
        .count() as u16;

    let mut samples = Vec::<f32>::new();
    let mut sample_buffer: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => {
                return Err("Decoder reset required; unsupported stream transition".to_string())
            }
            Err(Error::IoError(_)) => break,
            Err(err) => return Err(format!("Error reading packet: {err}")),
        };

        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("Decode failure: {e}"))?;

        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;
        let buffer = sample_buffer.get_or_insert_with(|| SampleBuffer::<f32>::new(duration, spec));
        buffer.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buffer.samples());
    }

    Ok(DecodedTrack {
        sample_rate,
        channels,
        samples,
    })
}

/// Minimal-cost linear interpolation resampler used only when device and track sample-rates differ.
/// It is intentionally simple for low-latency startup and predictable memory behavior, but quality is
/// lower than dedicated sinc-based resamplers; this is acceptable here as a fallback path.
pub fn resample_linear(
    interleaved: &[f32],
    in_rate: u32,
    out_rate: u32,
    channels: usize,
) -> Vec<f32> {
    if in_rate == out_rate || channels == 0 || interleaved.is_empty() {
        return interleaved.to_vec();
    }

    let in_frames = interleaved.len() / channels;
    if in_frames < 2 {
        return interleaved.to_vec();
    }

    let ratio = out_rate as f64 / in_rate as f64;
    let out_frames = ((in_frames as f64) * ratio).round() as usize;
    let mut out = vec![0.0_f32; out_frames * channels];

    for out_frame in 0..out_frames {
        let src_pos = (out_frame as f64) / ratio;
        let src_base = src_pos.floor() as usize;
        let src_next = (src_base + 1).min(in_frames - 1);
        let frac = (src_pos - src_base as f64) as f32;

        for ch in 0..channels {
            let a = interleaved[src_base * channels + ch];
            let b = interleaved[src_next * channels + ch];
            out[out_frame * channels + ch] = a + (b - a) * frac;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::resample_linear;

    #[test]
    fn resample_changes_frame_count() {
        let stereo = vec![0.0_f32, 0.0, 1.0, 1.0, 0.5, 0.5, -0.5, -0.5];
        let out = resample_linear(&stereo, 48_000, 96_000, 2);
        assert!(out.len() > stereo.len());
    }
}
