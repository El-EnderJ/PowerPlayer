# Development Roadmap

PowerPlayer development is currently tracked in five phases.

---

## Phase 1 — Core Audio Engine (Rust) + UI Base

**Goal**: Establish the foundation — a working Tauri app that can load and play a FLAC file via Rust, controlled from a minimal React UI.

- [x] Initialize Tauri + React + TypeScript project structure
- [x] Create project documentation (README, CONTEXT, docs/)
- [x] Build IPC bridge (React ↔ Rust) for commands and state
- [x] Implement FLAC decoding in Rust using `symphonia`
- [x] Implement audio output in Rust using `cpal`
- [x] Create minimal playback UI: play, pause, file open dialog
- [x] Display track metadata (title, artist, duration)

---

## Phase 2 — DSP & Parametric EQ (Rust Biquad Filters)

**Goal**: Add a real-time DSP pipeline with a parametric equalizer, all processed in Rust.

- [x] Design DSP pipeline architecture (chain of processors)
- [x] Implement biquad filter engine in Rust (low-pass, high-pass, peaking, shelving)
- [x] Build 10-band parametric EQ with adjustable frequency, gain, and Q
- [x] Create EQ visualization UI (frequency response curve)
- [x] Add preset management (AutoEQ profile activation + reverb presets)
- [x] Implement backend gapless-preload foundations

---

## Phase 3 — Dynamic Lyrics Engine

**Goal**: Synchronized lyrics display with smooth animations.

- [x] Parse LRC (time-synced lyrics) files
- [x] Implement real-time lyrics synchronization with playback position
- [x] Build lyrics UI with smooth scroll animations (Framer Motion)
- [x] Support cached lyrics retrieval pipeline
- [ ] Add karaoke-style word-by-word highlighting

---

## Phase 4 — Library Management & Performance Tuning

**Goal**: Full music library with scanning, search, and optimized performance.

- [x] Implement folder/library scanning in Rust (recursive, async)
- [x] Build metadata indexing and caching (SQLite via Rust)
- [ ] Create library browser UI (albums, artists, genres, playlists)
- [x] Add search functionality with FTS5 grouping (tracks/albums/artists)
- [ ] Performance profiling and optimization (memory, CPU, GPU)
- [x] Implement album art caching and lazy loading foundations
- [ ] Final UI polish and animation tuning

---

## Phase 5 — Spatial Audio, Stems, and Frontend Parity

**Goal**: Expose advanced backend features in the frontend UI and stabilize desktop UX.

- [x] Spatial audio backend (HRTF/room/source placement)
- [x] Spatial scene persistence in SQLite
- [x] Stem separation pipeline with cache/fallback
- [ ] Frontend controls for tone/balance/expansion/reverb/spatial/stems/search/queue
- [ ] End-to-end manual QA loops in real Tauri runtime across supported platforms
