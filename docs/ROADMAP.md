# Development Roadmap

PowerPlayer development is divided into four phases.

---

## Phase 1 — Core Audio Engine (Rust) + UI Base

**Goal**: Establish the foundation — a working Tauri app that can load and play a FLAC file via Rust, controlled from a minimal React UI.

- [x] Initialize Tauri + React + TypeScript project structure
- [x] Create project documentation (README, CONTEXT, docs/)
- [ ] Build basic IPC bridge (React ↔ Rust) for commands and state
- [ ] Implement FLAC decoding in Rust using `symphonia`
- [ ] Implement audio output in Rust using `cpal` (WASAPI Exclusive)
- [ ] Create minimal playback UI: play, pause, stop, file open dialog
- [ ] Display basic track metadata (title, artist, duration)

---

## Phase 2 — DSP & Parametric EQ (Rust Biquad Filters)

**Goal**: Add a real-time DSP pipeline with a parametric equalizer, all processed in Rust.

- [ ] Design DSP pipeline architecture (chain of processors)
- [ ] Implement biquad filter engine in Rust (low-pass, high-pass, peaking, shelving)
- [ ] Build 10-band parametric EQ with adjustable frequency, gain, and Q
- [ ] Create EQ visualization UI (frequency response curve)
- [ ] Add preset management (save/load EQ profiles)
- [ ] Implement gapless playback

---

## Phase 3 — Dynamic Lyrics Engine

**Goal**: Synchronized lyrics display with smooth animations.

- [ ] Parse LRC (time-synced lyrics) files
- [ ] Implement real-time lyrics synchronization with playback position
- [ ] Build lyrics UI with smooth scroll animations (Framer Motion)
- [ ] Support embedded lyrics from audio file metadata
- [ ] Add karaoke-style word-by-word highlighting

---

## Phase 4 — Library Management & Performance Tuning

**Goal**: Full music library with scanning, search, and optimized performance.

- [ ] Implement folder/library scanning in Rust (recursive, async)
- [ ] Build metadata indexing and caching (SQLite via Rust)
- [ ] Create library browser UI (albums, artists, genres, playlists)
- [ ] Add search functionality with fuzzy matching
- [ ] Performance profiling and optimization (memory, CPU, GPU)
- [ ] Implement album art caching and lazy loading
- [ ] Final UI polish and animation tuning
