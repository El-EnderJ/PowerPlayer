# CONTEXT.md — PowerPlayer Project Memory

> This file acts as the short-term and long-term memory for the PowerPlayer project.
> It must be updated every time a task is completed or an important feature is implemented.

---

## Project Rules

### Rule 1 — Update Log
Every time a task is completed or an important feature is implemented, this file **must** be updated with a log entry containing:
- The date
- The task completed
- The next logical step

### Rule 2 — UI Performance First
All UI code must prioritize performance.
- Use **Tailwind CSS** for utility-first styling (no heavy CSS-in-JS).
- Use **Framer Motion** for animations targeting 60 fps minimum.
- Avoid unnecessary re-renders in React; use `React.memo`, `useMemo`, and `useCallback` where appropriate.

### Rule 3 — Rust Handles the Heavy Lifting
All heavy processing (audio decoding, DSP, file I/O, library scanning) must be done in **Rust**.
The React frontend is a "puppet" that:
- Draws the current state received from the backend.
- Sends commands to the backend via Tauri IPC (`invoke`).
- Never performs CPU-intensive work directly.

---

## Change Log

| Date | Task Completed | Next Step |
|------|---------------|-----------|
| 2026-02-21 | Project initialization: created Tauri + React + TS structure, README, docs, and CONTEXT.md | Build basic IPC bridge between React and Rust to load a .FLAC file |
| 2026-02-21 | DSP pipeline implemented in Rust audio engine: DF2T biquad module, 10-band parametric EQ, pre-amp stage, soft limiter, and Tauri command to update EQ bands | Expose remaining playback + DSP controls to frontend and bind them to UI |
| 2026-02-21 | Phase 3 UI: FFT bridge (rustfft), VisualEQ canvas component, Fluid Glass aesthetic (FluidBackground, PlaybackControls with neon glow), Framer Motion transitions, new Tauri commands (get_eq_bands, get_eq_frequency_response, get_fft_data) | Wire file-open dialog, real-time FFT from audio callback, seek bar, volume sliders |
| 2026-02-21 | Integrated native file dialog + `load_track` IPC with metadata payload (artist/title/cover), added `get_vibe_data` real-time feed, seek/progress and logarithmic volume sliders, and dev FPS counter | Add playlist/library management and persist playback state |
| 2026-02-21 | Phase 4 kickoff: Dynamic Lyrics Engine with `.lrc` parser in Rust, playback-synced `lyrics-line-changed` events, fullscreen LyricsView focus animation, and expanded spectrum fallback when no lyrics exist | Add karaoke-style word-level timing and playlist-aware lyrics preloading |
| 2026-02-21 | Phase B backend implemented: SQLite pool persistence (`tracks/albums/settings`), multithreaded library scan (`walkdir`+`rayon`) with metadata persistence, and AutoEQ 10-band profile activation path | Connect library + AutoEQ device suggestions to frontend interactions |
| 2026-02-21 | Backend hardening: added SHA-256 art thumbnail cache (`asset://` URLs), optional next-track look-ahead preloading at 95% progress, notify-based realtime library watcher, and corrupted-track persistence | Use cached art + corrupted flags in library UI and expose playlist queue wiring for automatic `set_next_track` |
| 2026-02-21 | Metadata Enrichment Layer: local-first art resolver (`cover/folder.jpg`) + iTunes/MusicBrainz fallback, LRCLIB synced lyrics downloader into `.lyrics_cache`, and async enrichment queue after DB save | Connect enrichment status to UI and expose retry controls for failed online lookups |
| 2026-02-21 | Bit-perfect backend polish: dynamic stream fade/restart scaffolding for sample-rate transitions, HQ rubato fallback resampler, memmap2 loading path for files >50MB, modular DSP node chain (PreAmp→AutoEQ→UserEQ→StereoWidener→Limiter), and `get_audio_stats` telemetry IPC | Expose new audio stats and widener controls in frontend diagnostics/audio settings UI |
| 2026-02-21 | **PowerAmp Level**: Advanced DSP nodes (Tone bass/treble shelving, Balance L/R, StereoExpansion crossfeed, algorithmic Reverb with Freeverb-style combs/allpasses + 4 presets), FTS5 ultra-fast full-text search engine, non-destructive Fisher-Yates shuffle queue, and 7 new Tauri IPC commands | Wire new DSP/search/queue controls to React frontend UI panels |

## DSP Topology (Engine)

- **Pre-Amp (global)**: applies gain in dB before EQ to create headroom.
- **Tone Node**: independent LowShelf (~100 Hz, bass) and HighShelf (~10 kHz, treble) biquad filters.
- **AutoEQ Node**: optional compensation profile applied before user shaping.
- **User EQ Node**: 10 configurable bands with atomic `frequency`, `gain_db`, and `Q_factor`.
  - Each band uses biquad filters in **Direct Form II Transposed**.
  - Coefficients are recalculated **only when parameters change**.
- **Balance Node**: stereo L/R panning from -1.0 (full left) to 1.0 (full right).
- **Stereo Expansion Node**: crossfeed algorithm with delay line + low-pass filter to simulate speaker listening.
- **Reverb Node**: Schroeder/Freeverb-inspired algorithmic reverb with 8 parallel comb filters + 4 series all-pass filters, predelay, damping, and wet/dry mix. Includes 4 presets: Estudio, Sala Grande, Club, Iglesia.
- **Soft Limiter**: final protection stage (threshold near **-0.1 dBFS**) to avoid digital clipping.
- **Order**: `Input sample -> Pre-Amp -> Tone -> AutoEQ -> UserEQ -> Balance -> StereoExpansion -> Reverb -> Soft Limiter -> Output`.

## UI-DSP Integration

### Tauri IPC Commands
| Command | Direction | Description |
|---------|-----------|-------------|
| `update_eq_band(index, freq, gain, q)` | Frontend → Rust | Updates a single EQ band in real-time |
| `get_eq_bands()` | Frontend ← Rust | Returns all EQ band parameters (frequency, gain_db, q_factor) |
| `get_eq_frequency_response(num_points)` | Frontend ← Rust | Returns the combined EQ magnitude response curve |
| `get_fft_data()` | Frontend ← Rust | Returns FFT frequency magnitude data for spectrum visualization |
| `load_track(path)` | Frontend → Rust | Loads selected audio file and returns artist/title/cover/duration metadata |
| `play()` / `pause()` | Frontend → Rust | Toggles playback state in audio engine |
| `seek(seconds)` | Frontend → Rust | Requests playback repositioning in seconds |
| `set_volume(volume)` | Frontend → Rust | Applies final output gain (0..1, UI uses logarithmic mapping) |
| `get_vibe_data()` | Frontend ← Rust | Returns current FFT spectrum + instantaneous amplitude from callback buffer |
| `get_lyrics_lines()` | Frontend ← Rust | Returns parsed `.lrc` lines (`timestamp` in ms + lyric text) for the loaded track |
| `scan_library(path)` | Frontend → Rust | Recursively scans audio files in a folder and persists metadata in SQLite (`tracks` upsert by path) |
| `get_library_tracks()` | Frontend ← Rust | Returns persisted library tracks from SQLite for browser/queue UIs |
| `activate_autoeq_profile(model)` | Frontend → Rust | Resolves a 10-band AutoEQ profile for headphone model and applies bands via existing EQ update path |
| `get_audio_stats()` | Frontend ← Rust | Returns device name, stream latency estimate, output/file sample-rates, and ring-buffer memory usage |
| `set_tone(bass, treble)` | Frontend → Rust | Sets independent bass (LowShelf ~100 Hz) and treble (HighShelf ~10 kHz) gain in dB (±12) |
| `set_balance(val)` | Frontend → Rust | Sets stereo balance from -1.0 (full left) to 1.0 (full right) |
| `set_expansion(val)` | Frontend → Rust | Sets crossfeed stereo expansion amount (0.0–1.0) |
| `set_reverb_params(room_size, damping, predelay_ms, lowpass_filter, decay, wet_mix)` | Frontend → Rust | Sets all reverb parameters atomically |
| `load_reverb_preset(name)` | Frontend → Rust | Loads a named reverb preset ("Estudio", "Sala Grande", "Club", "Iglesia") |
| `fast_search(query)` | Frontend ← Rust | FTS5 full-text search returning grouped results (tracks, albums, artists) in milliseconds |
| `toggle_shuffle(enabled)` | Frontend → Rust | Enables/disables Fisher-Yates shuffle on the playback queue, preserving current track position |

### Lyrics Synchronization Flow
- Backend resolves `<track_name>.lrc` next to the loaded audio file and parses `[mm:ss.xx]` tags into `LyricsLine { timestamp, text }`.
- `AudioState` keeps a playback frame counter updated by the output callback and converts it to milliseconds.
- A monitor thread compares current time against lyric timestamps and emits `lyrics-line-changed` only when the active line index actually changes.
- Frontend subscribes to the event and animates the centered active line in `LyricsView`; when no lyrics are available, it automatically switches to an expanded spectrum visual mode.

### Library Art Cache Flow
- During library scan (and watcher updates), backend attempts to read embedded cover art.
- If cover art exists, a SHA-256 hash of the track path is used as cache key and a `256x256` JPEG thumbnail is written to local cache (`$TMP/powerplayer/art_cache`).
- The track row stores an `art_url` in `asset://...` form so the library UI can render instantly without storing blob bytes in SQLite.

### Metadata Enrichment Flow (Local + Web)
- Scanner persists the track first, then pushes a background enrichment task into a worker queue (non-blocking for the main scan).
- Art resolver priority: embedded art → local `cover.jpg`/`folder.jpg` (same folder) → iTunes Search API (`https://itunes.apple.com/search`) → MusicBrainz recording search (`https://musicbrainz.org/ws/2/recording`) + Cover Art Archive (`https://coverartarchive.org`).
- Downloaded images are reprocessed by `art_cache.rs` into the same `asset://` thumbnail format used by embedded covers.
- Lyrics resolver uses LRCLIB (`https://lrclib.net/api/get`) with artist/title/duration and stores synced `.lrc` text in app-local `.lyrics_cache` so the existing sync engine can load it transparently.
- Track metadata repair tries filename fingerprint fallback (`Artist - Title`) when tags are missing/corrupted.

### Gapless Look-ahead Flow
- `AudioState` now stores an optional `next_track` path (`set_next_track(path)` IPC).
- Output callback computes playback progress; once current track reaches `>=95%`, it arms look-ahead.
- Producer thread pre-decodes the optional next track and swaps buffers immediately when current PCM ends, reusing the same stream for click-free, zero-restart transition behavior.

### Frontend Components
- **VisualEQ**: Canvas-based parametric EQ editor. Drag points for freq/gain; scroll for Q. Uses `requestAnimationFrame` for 60fps+ rendering.
- **FluidBackground**: Album art with `blur(80px)` and rotation/pulsation animation. Falls back to animated gradient.
- **PlaybackControls**: Glass-effect buttons with `backdrop-blur`, semi-transparent borders, and neon glow that reacts to volume level.

### Performance Notes
- Canvas rendering uses `requestAnimationFrame` to avoid blocking the main thread.
- EQ curve and FFT data updates are debounced through React state batching.
- All heavy computation (FFT, coefficient calculation) runs in Rust; the frontend only draws.
