# PowerPlayer 🎵

**Hi-Res Audio Player for Windows** — A bit-perfect audio player built with Tauri, Rust, and React.

PowerPlayer is a desktop audio player designed for audiophiles. It combines a low-level, bit-perfect audio engine written in Rust with an ultra-minimalist "Fluid Glass" UI running at 60–120 fps.

## Key Technologies

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Backend | **Tauri** (Rust) | Native desktop shell, IPC bridge |
| Audio Engine | **cpal** (Rust) | Low-level audio output (WASAPI Exclusive) |
| Audio Decoding | **symphonia** (Rust) | FLAC / WAV / MP3 decoding |
| Frontend | **React + TypeScript** | UI rendering |
| Build Tool | **Vite** | Fast HMR & bundling |
| Styling | **Tailwind CSS** | Utility-first CSS |
| Animations | **Framer Motion** | 60 fps fluid animations |

## Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/tools/install) >= 1.70
- [Tauri CLI prerequisites](https://tauri.app/start/prerequisites/) (Windows: WebView2, Visual Studio Build Tools)

## Getting Started

```bash
# Install frontend dependencies
npm install

# Run in development mode (opens the Tauri window with hot-reload)
npm run tauri dev

# Build a production release
npm run tauri build
```

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
