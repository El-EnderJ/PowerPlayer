/// Non-destructive playback queue with true shuffle (Fisher-Yates).
/// Maintains `original_order` and `shuffled_order` so the user can toggle
/// shuffle on/off without losing their position.
pub struct PlaybackQueue {
    original_order: Vec<String>,
    shuffled_order: Vec<String>,
    current_index: usize,
    shuffle_enabled: bool,
}

impl PlaybackQueue {
    pub fn new() -> Self {
        Self {
            original_order: Vec::new(),
            shuffled_order: Vec::new(),
            current_index: 0,
            shuffle_enabled: false,
        }
    }

    /// Replaces the queue contents with a new list of track paths.
    pub fn set_tracks(&mut self, tracks: Vec<String>) {
        self.original_order = tracks;
        self.shuffled_order.clear();
        self.current_index = 0;
        self.shuffle_enabled = false;
    }

    /// Toggles shuffle mode. When enabling, applies Fisher-Yates shuffle to build
    /// `shuffled_order`. When disabling, resolves the current track back to its
    /// position in `original_order`.
    pub fn toggle_shuffle(&mut self, enable: bool) {
        if enable == self.shuffle_enabled {
            return;
        }

        if enable {
            // Remember which track we're on
            let current_track = self.current_track().map(|s| s.to_string());
            self.shuffled_order = self.original_order.clone();
            fisher_yates_shuffle(&mut self.shuffled_order);

            // Move the current track to the front of the shuffled list so playback
            // continues seamlessly from the current song.
            if let Some(track) = current_track {
                if let Some(pos) = self.shuffled_order.iter().position(|t| *t == track) {
                    self.shuffled_order.swap(0, pos);
                }
                self.current_index = 0;
            }
        } else {
            // Switching back to original order: find where the current track is
            // in the original list and continue from there.
            if let Some(track) = self.current_track().map(|s| s.to_string()) {
                self.current_index = self
                    .original_order
                    .iter()
                    .position(|t| *t == track)
                    .unwrap_or(0);
            }
            self.shuffled_order.clear();
        }

        self.shuffle_enabled = enable;
    }

    /// Returns the active track list (shuffled when shuffle is on).
    pub fn active_order(&self) -> &[String] {
        if self.shuffle_enabled && !self.shuffled_order.is_empty() {
            &self.shuffled_order
        } else {
            &self.original_order
        }
    }

    /// Returns the current track path, if any.
    pub fn current_track(&self) -> Option<&str> {
        self.active_order().get(self.current_index).map(|s| s.as_str())
    }

    /// Advances to the next track. Returns the new current track, or None if at end.
    pub fn next(&mut self) -> Option<&str> {
        let order = self.active_order();
        if self.current_index + 1 < order.len() {
            self.current_index += 1;
        }
        self.current_track()
    }

    /// Goes back to the previous track. Returns the new current track.
    pub fn previous(&mut self) -> Option<&str> {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
        self.current_track()
    }

    /// Jumps to a specific index in the active order.
    pub fn jump_to(&mut self, index: usize) {
        if index < self.active_order().len() {
            self.current_index = index;
        }
    }

    pub fn is_shuffle_enabled(&self) -> bool {
        self.shuffle_enabled
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn len(&self) -> usize {
        self.active_order().len()
    }

    pub fn is_empty(&self) -> bool {
        self.active_order().is_empty()
    }
}

impl Default for PlaybackQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Fisher-Yates (Knuth) in-place shuffle using a simple LCG PRNG seeded from
/// system time to avoid pulling in the `rand` crate.
fn fisher_yates_shuffle(items: &mut [String]) {
    let len = items.len();
    if len <= 1 {
        return;
    }

    // Seed from system time nanoseconds
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);

    let mut rng_state = seed;
    for i in (1..len).rev() {
        // Simple LCG: state = state * 6364136223846793005 + 1442695040888963407
        rng_state = rng_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = (rng_state >> 33) as usize % (i + 1);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tracks() -> Vec<String> {
        (0..10).map(|i| format!("/music/track{i}.flac")).collect()
    }

    #[test]
    fn queue_starts_at_first_track() {
        let mut q = PlaybackQueue::new();
        q.set_tracks(sample_tracks());
        assert_eq!(q.current_track(), Some("/music/track0.flac"));
        assert_eq!(q.current_index(), 0);
    }

    #[test]
    fn next_and_previous_navigate() {
        let mut q = PlaybackQueue::new();
        q.set_tracks(sample_tracks());
        q.next();
        assert_eq!(q.current_track(), Some("/music/track1.flac"));
        q.previous();
        assert_eq!(q.current_track(), Some("/music/track0.flac"));
    }

    #[test]
    fn shuffle_preserves_current_track() {
        let mut q = PlaybackQueue::new();
        q.set_tracks(sample_tracks());
        q.next();
        q.next(); // now on track2
        let track_before = q.current_track().unwrap().to_string();
        q.toggle_shuffle(true);
        assert_eq!(q.current_track().unwrap(), track_before);
    }

    #[test]
    fn unshuffle_finds_original_position() {
        let mut q = PlaybackQueue::new();
        q.set_tracks(sample_tracks());
        q.next();
        q.next(); // track2
        q.toggle_shuffle(true);
        // Navigate forward in shuffled
        q.next();
        q.next();
        let shuffled_track = q.current_track().unwrap().to_string();
        q.toggle_shuffle(false);
        // After disabling shuffle, current track should be the same
        assert_eq!(q.current_track().unwrap(), shuffled_track);
        // And the index should match original_order
        let expected_idx = q
            .original_order
            .iter()
            .position(|t| *t == shuffled_track)
            .unwrap();
        assert_eq!(q.current_index(), expected_idx);
    }

    #[test]
    fn shuffle_changes_order() {
        let mut q = PlaybackQueue::new();
        let tracks = sample_tracks();
        q.set_tracks(tracks.clone());
        q.toggle_shuffle(true);
        // Shuffled order should differ from original (with very high probability for 10 items)
        // The first item is the current track (track0), so compare the rest
        let shuffled = q.active_order().to_vec();
        assert_eq!(shuffled.len(), tracks.len());
        // At least one element in positions 1..N should differ
        let differs = shuffled[1..]
            .iter()
            .zip(tracks[1..].iter())
            .any(|(a, b)| a != b);
        assert!(
            differs,
            "shuffled order should differ from original (statistically)"
        );
    }

    #[test]
    fn empty_queue_handles_gracefully() {
        let mut q = PlaybackQueue::new();
        assert!(q.current_track().is_none());
        assert!(q.next().is_none());
        assert!(q.previous().is_none());
        assert!(q.is_empty());
        q.toggle_shuffle(true);
        assert!(q.current_track().is_none());
    }

    #[test]
    fn fisher_yates_does_not_panic_on_empty() {
        let mut items: Vec<String> = Vec::new();
        fisher_yates_shuffle(&mut items);
        assert!(items.is_empty());
    }

    #[test]
    fn fisher_yates_single_element() {
        let mut items = vec!["only".to_string()];
        fisher_yates_shuffle(&mut items);
        assert_eq!(items, vec!["only"]);
    }
}
