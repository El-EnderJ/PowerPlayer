use crate::audio::lyrics_downloader;
use crate::db::manager::{DbManager, TrackInput};
use crate::library::metadata::art_fetcher;
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;

#[derive(Clone)]
struct EnrichmentTask {
    track: TrackInput,
    db: DbManager,
}

pub fn enqueue(track: TrackInput, db: DbManager) {
    if track.path.is_empty() {
        return;
    }
    let sender = queue_sender();
    let _ = sender.send(EnrichmentTask { track, db });
}

fn queue_sender() -> &'static Sender<EnrichmentTask> {
    static QUEUE: OnceLock<Sender<EnrichmentTask>> = OnceLock::new();
    QUEUE.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<EnrichmentTask>();
        std::thread::spawn(move || {
            while let Ok(task) = receiver.recv() {
                process_task(task);
            }
        });
        sender
    })
}

fn process_task(task: EnrichmentTask) {
    let mut updated_track = task.track.clone();
    let mut should_save = false;
    let track_path = Path::new(&updated_track.path);

    if updated_track.art_url.is_none() {
        if let Ok(art_url) = art_fetcher::fetch_and_cache_art(
            track_path,
            updated_track.artist.as_deref(),
            updated_track.title.as_deref(),
        ) {
            if art_url.is_some() {
                updated_track.art_url = art_url;
                should_save = true;
            }
        }
    }

    if let (Some(artist), Some(title)) = (
        updated_track.artist.as_deref(),
        updated_track.title.as_deref(),
    ) {
        let _ = lyrics_downloader::download_lyrics_for_track(
            track_path,
            artist,
            title,
            updated_track.duration_seconds,
        );
    }

    if should_save {
        let _ = task.db.save_track(&updated_track);
    }
}
