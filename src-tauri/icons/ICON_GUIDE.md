# PowerPlayer Icon Guide

## Required Icon Set

Tauri requires the following icon files in this directory (`src-tauri/icons/`):

| File               | Size      | Purpose                           |
|--------------------|-----------|-----------------------------------|
| `32x32.png`        | 32×32 px  | Taskbar / small icon (Windows)    |
| `128x128.png`      | 128×128 px| Application icon                  |
| `128x128@2x.png`   | 256×256 px| HiDPI / Retina application icon   |
| `icon.ico`         | multi-res | Windows `.exe` embedded icon      |
| `icon.png`         | 512×512 px| Linux / fallback high-res icon    |

## Design Guidelines

1. **Base canvas**: Start with a **1024×1024 px** master icon in SVG or PNG.
2. **Style**: Use the PowerPlayer branding — a frosted-glass play-button silhouette on a pure black (`#000000`) background, matching the splash screen aesthetic.
3. **Safe area**: Keep essential artwork within the central 80 % of the canvas to avoid clipping on rounded platforms.
4. **Export**: Export each size from the master. Use **PNG-8/24** with transparency for all `.png` files.

## Generating the Icon Set

### Option A — Tauri CLI (recommended)

Place a single `icon.png` (1024×1024+) in this directory and run:

```bash
npm run tauri icon src-tauri/icons/icon.png
```

The CLI generates every required size and the `.ico` file automatically.

### Option B — Manual export

Use any image editor (Figma, GIMP, Inkscape) to export the sizes listed above, then place them in this folder. Ensure the filenames match exactly.

## Verification

After generating icons, confirm they are referenced in `tauri.conf.json`:

```json
"bundle": {
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.ico"
  ]
}
```
