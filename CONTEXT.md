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

## DSP Topology (Engine)

- **Pre-Amp (global)**: applies gain in dB before EQ to create headroom.
- **Parametric EQ**: 10 configurable bands with atomic `frequency`, `gain_db`, and `Q_factor`.
  - Each band uses biquad filters in **Direct Form II Transposed**.
  - Coefficients are recalculated **only when parameters change**.
- **Soft Limiter**: final protection stage (threshold near **-0.1 dBFS**) to avoid digital clipping.
- **Order**: `Input sample -> Pre-Amp -> ParametricEQ (L/R independent, shared params) -> Soft Limiter -> Output`.

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

### Lyrics Synchronization Flow
- Backend resolves `<track_name>.lrc` next to the loaded audio file and parses `[mm:ss.xx]` tags into `LyricsLine { timestamp, text }`.
- `AudioState` keeps a playback frame counter updated by the output callback and converts it to milliseconds.
- A monitor thread compares current time against lyric timestamps and emits `lyrics-line-changed` only when the active line index actually changes.
- Frontend subscribes to the event and animates the centered active line in `LyricsView`; when no lyrics are available, it automatically switches to an expanded spectrum visual mode.

### Frontend Components
- **VisualEQ**: Canvas-based parametric EQ editor. Drag points for freq/gain; scroll for Q. Uses `requestAnimationFrame` for 60fps+ rendering.
- **FluidBackground**: Album art with `blur(80px)` and rotation/pulsation animation. Falls back to animated gradient.
- **PlaybackControls**: Glass-effect buttons with `backdrop-blur`, semi-transparent borders, and neon glow that reacts to volume level.

### Performance Notes
- Canvas rendering uses `requestAnimationFrame` to avoid blocking the main thread.
- EQ curve and FFT data updates are debounced through React state batching.
- All heavy computation (FFT, coefficient calculation) runs in Rust; the frontend only draws.
