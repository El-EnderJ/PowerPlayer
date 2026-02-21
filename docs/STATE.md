# Current Project State

## Status: PowerAmp Level+ — Backend avanzado estable, frontend parcial

**Last updated**: 2026-02-21

### Completed
- Project structure initialized (Tauri + React + TypeScript + Vite)
- Documentation created (README.md, CONTEXT.md, docs/)
- Development tooling configured (Tailwind CSS, Framer Motion, PostCSS)
- DSP Engine in Rust (parametric EQ, pre-amp and limiter)
- Audio playback engine with WASAPI integration (cpal + ringbuf)
- Symphonia-based FLAC decoder with resampling
- FFT analysis module (rustfft) for frequency spectrum visualization
- Dynamic sample-rate transition scaffolding with fade-out stream handoff and HQ `rubato` fallback resampling
- Large-file decode source optimization: `memmap2` path for audio files bigger than 50MB
- Modular DSP backend chain: `PreAmp -> Tone -> AutoEQ -> UserEQ -> Balance -> StereoExpansion -> Spatial -> Reverb -> SoftLimiter`
- Audio telemetry IPC command `get_audio_stats` (device, latency estimate, output/file sample-rates, ring-buffer memory)
- Tauri IPC commands: `update_eq_band`, `get_eq_bands`, `get_eq_frequency_response`, `get_fft_data`
- VisualEQ component: interactive canvas-based parametric EQ with drag points and scroll Q adjustment
- Fluid Glass UI: FluidBackground (blur album art), PlaybackControls (glass effects + neon glow), Framer Motion transitions
- Native file loading with `tauri-plugin-dialog` and backend `load_track` metadata payload (artist/title/cover/duration)
- Real-time vibe feed (`get_vibe_data`) connected to requestAnimationFrame visual updates and neon glow intensity
- Browser-safe frontend invocation wrapper for Tauri commands (prevents runtime errors when testing in pure Vite/web mode)
- Seek `ProgressBar` with debounced `seek(seconds)` calls and logarithmic volume slider mapping
- Optional dev FPS counter for canvas/render profiling
- Phase B backend foundations:
  - SQLite persistence layer (`DbManager`) with pooled `r2d2` connections
  - Auto-created `tracks`, `albums`, and `settings` tables
  - Idempotent `save_track` upsert by track path
  - Multithreaded library scanner (`walkdir` + `rayon`) with metadata extraction (title, artist, album, duration, sample rate)
  - AutoEQ profile resolver for 10-band EQ and instant DSP application through existing band update path
  - New Tauri commands: `scan_library(path)`, `get_library_tracks()`, `activate_autoeq_profile(model)`
  - Art Cache Manager: embedded cover extraction, local JPEG thumbnail cache, `asset://` art URLs in DB
  - Gapless backend look-ahead: optional `next_track`, 95% preload trigger, producer-side buffer handoff
  - Real-time library watcher (`notify`) to upsert/remove tracks in background when files change
  - Corrupted file robustness: scanner marks unreadable/corrupt tracks with `corrupted` flag instead of aborting
  - Enrichment Layer complete:
    - Intelligent art provider with local-first (`cover/folder.jpg`) and online fallback (iTunes + MusicBrainz/Cover Art Archive)
    - LRCLIB synced lyrics downloader with app-level `.lyrics_cache` used by the existing lyrics sync engine
    - Background worker queue for metadata enrichment (network downloads run after DB insert, without blocking scans)
    - Lightweight metadata repair fallback (`Artist - Title` filename fingerprinting) for missing/corrupt tags
  - **PowerAmp Level** DSP & backend features:
    - Tone Node: independent LowShelf (~100 Hz) and HighShelf (~10 kHz) biquad filters for bass/treble control
    - Balance Node: stereo L/R panning (-1.0 to 1.0) with min-gain formula
    - Stereo Expansion Node: crossfeed algorithm with delay line + low-pass filter for speaker-like imaging
    - Algorithmic Reverb Node: Freeverb/Schroeder-inspired with 8 parallel comb filters + 4 series all-pass, predelay, damping, LP filter, decay, wet/dry mix
    - Reverb presets: Estudio, Sala Grande, Club, Iglesia
    - FTS5 full-text search engine: SQLite virtual table with auto-sync triggers, prefix matching, grouped results (tracks/albums/artists)
    - Non-destructive playback queue: Fisher-Yates shuffle preserving original order and current-track position
    - DSP chain updated: `PreAmp → Tone → AutoEQ → UserEQ → Balance → StereoExpansion → Reverb → Limiter`
    - New Tauri IPC commands: `set_tone`, `set_balance`, `set_expansion`, `set_reverb_params`, `load_reverb_preset`, `fast_search`, `toggle_shuffle`

### In Progress
- UI specialization phase: consume remaining backend DSP/spatial/stems/search/queue controls in React frontend panels

### Next Steps
1. Build React UI panels for Tone (bass/treble sliders), Balance, Expansion, Reverb and Spatial controls
2. Build search bar UI connected to `fast_search` with grouped result display
3. Build queue management UI with shuffle toggle
4. Consume `art_url` and `corrupted` in library browser cards/list rows
5. Wire playlist queue to `set_next_track(path)` for end-to-end gapless UX

### Manual Validation Snapshot (2026-02-21)
- ✅ `npm ci && npm run build` (frontend build OK)
- ✅ `cd src-tauri && cargo test` (69 tests passing on Linux with GTK/WebKit deps installed)
- ✅ Manual UI smoke test in dev mode (home, playback controls, volume, EQ panel) with screenshots in `docs/images/`
