# PowerPlayer 🎵

**Hi-Res Audio Player (Tauri + Rust + React)** with a Rust audio engine and Fluid Glass UI.

PowerPlayer is a desktop audio player focused on high-quality playback. It combines a low-level audio engine written in Rust with an ultra-minimalist "Fluid Glass" UI running at 60–120 fps.

## Key Technologies

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Backend | **Tauri** (Rust) | Native desktop shell, IPC bridge |
| Audio Engine | **cpal** (Rust) | Low-level audio output (platform dependent backend) |
| Audio Decoding | **symphonia** (Rust) | FLAC / WAV / MP3 decoding |
| Frontend | **React + TypeScript** | UI rendering |
| Build Tool | **Vite** | Fast HMR & bundling |
| Styling | **Tailwind CSS** | Utility-first CSS |
| Animations | **Framer Motion** | 60 fps fluid animations |

## Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/tools/install) >= 1.70
- [Tauri CLI prerequisites](https://tauri.app/start/prerequisites/)
- Linux (for `cargo test` / Tauri builds): `libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev`

## Getting Started

```bash
# Install frontend dependencies
npm ci

# Run in development mode (opens the Tauri window with hot-reload)
npm run tauri dev

# Build frontend bundle
npm run build

# Run backend tests
cd src-tauri && cargo test

# Build a production release (Tauri app)
cd .. && npm run tauri build
```

## Current Status (2026-02-21)

- Rust backend includes DSP chain (preamp, tone, AutoEQ, user EQ, balance, stereo expansion, spatial, reverb, limiter).
- SQLite library persistence, FTS5 search, queue shuffle, and metadata enrichment are integrated.
- Frontend includes playback controls, visual parametric EQ, and synced lyrics panel.
- Browser/dev preview now fails gracefully when Tauri runtime APIs are unavailable.

## UI Captures

Screenshots from manual app usage are available in `docs/images`:

- `docs/images/app-home.png`
- `docs/images/app-playing.png`
- `docs/images/app-volume-adjusted.png`
- `docs/images/app-smoke-round2.png`
## Project Structure

```
PowerPlayer/
├── src/                  # React frontend (TypeScript)
│   ├── components/       # UI components
│   ├── styles/           # Global styles
│   ├── App.tsx           # Root component
│   └── main.tsx          # Entry point
├── src-tauri/            # Rust backend (Tauri)
│   ├── src/
│   │   ├── main.rs       # Tauri entry point
│   │   └── lib.rs        # Core library & commands
│   ├── Cargo.toml        # Rust dependencies
│   └── tauri.conf.json   # Tauri configuration
├── docs/                 # Project documentation
│   ├── OBJECTIVE.md      # Product vision
│   ├── ROADMAP.md        # Development phases
│   └── STATE.md          # Current project state
├── CONTEXT.md            # Project memory & rules
└── README.md             # This file
```

## Documentation

- [Product Vision & Objectives](docs/OBJECTIVE.md)
- [Development Roadmap](docs/ROADMAP.md)
- [Current State](docs/STATE.md)
- [Project Context & Rules](CONTEXT.md)

## License

MIT
