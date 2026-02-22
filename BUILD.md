# PowerPlayer — Build & Release Guide

## Prerequisites

| Tool        | Version  | Notes                                          |
|-------------|----------|------------------------------------------------|
| Node.js     | ≥ 18     | Frontend build                                 |
| Rust        | stable   | Backend compilation                            |
| Tauri CLI   | 2.x      | Installed via `npm` (dev dependency)           |

### Windows-specific

- Visual Studio Build Tools (C++ workload) for MSVC linker.
- WebView2 runtime (pre-installed on Windows 10 21H2+).

### Linux-specific (dev only)

```bash
sudo apt-get install -y \
  libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev \
  libappindicator3-dev librsvg2-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev
```

## Development

```bash
npm ci                # install frontend dependencies
npm run tauri dev     # launch dev mode with hot-reload
```

## Production Build

Generate an optimised installer (`.msi` and/or `.exe` on Windows):

```bash
npm run tauri build
```

This command:

1. Runs `tsc && vite build` to produce the optimised frontend bundle.
2. Compiles the Rust backend in **release** mode with:
   - **LTO** (Link-Time Optimisation) enabled for smaller, faster binaries.
   - **`panic = "abort"`** to eliminate unwind tables and reduce binary size.
3. Packages everything into platform installers under `src-tauri/target/release/bundle/`.

### Build Outputs

| Platform | Artefact                                         |
|----------|--------------------------------------------------|
| Windows  | `src-tauri/target/release/bundle/msi/*.msi`      |
| Windows  | `src-tauri/target/release/bundle/nsis/*.exe`      |
| macOS    | `src-tauri/target/release/bundle/dmg/*.dmg`       |
| Linux    | `src-tauri/target/release/bundle/deb/*.deb`       |

## Running Tests

```bash
# Frontend type-check + Vite build
npm run build

# Backend unit tests
cd src-tauri && cargo test
```
