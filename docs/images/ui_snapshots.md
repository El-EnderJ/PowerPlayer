# PowerPlayer – UI Snapshots & Visual Reference

> Visual documentation for verifying the Liquid Glass Dark UI across all major views.

---

## 1. Library View – Liquid Glass Effect

**Description:**
- Full-screen dark background (`#0a0a0c`) with a subtle fractal noise overlay (`.noise-bg`).
- Sticky header bar at the top with rounded corners, Liquid Glass formula: `backdrop-blur(40px) saturate(180%)`, faint top/bottom borders.
- Filter tabs (Todas, Álbumes, Artistas, Géneros) styled as translucent chips.
- Track list below rendered as `TrackBubble` cards — each with rounded-2xl corners, `bg-black/30`, backdrop-blur.
- Album art thumbnails (64×64) with rounded corners and shadow.
- Alphabet scroll index pinned to the right edge.
- Ambient blur from the current album art faintly visible in the background.

**Key CSS classes:** `.liquid-glass`, `.noise-bg`, `.scrollbar-hide`

---

## 2. Equalizer View – Crystal Knobs & Bezier Curve

**Description:**
- Central canvas rendering the EQ frequency response as a smooth Bezier curve.
- Background uses the same Liquid Glass dark aesthetic with frosted panels.
- EQ band controls displayed as draggable points overlaid on the curve.
- Spectrum visualizer (`VisualEQ`) providing real-time animated bars below the curve.
- Frequency labels (Hz) on the X-axis and gain labels (dB) on the Y-axis.
- Controls panel with glass-pill buttons for AutoEQ profile selection and reverb presets.

**Key design tokens:** `border-t border-white/10`, `bg-white/5`, `shadow-[0_20px_50px_rgba(0,0,0,0.5)]`

---

## 3. Search View – Intelligent Grouped Search

**Description:**
- Large minimalist search input at the top with a glowing magnifying glass icon (cyan glow when active).
- Filter chips row below: "Todo", "Canciones", "Álbumes", "Artistas" — styled as rounded-full pills with `border-cyan-500/40` neon border when active.
- Results are grouped dynamically:
  - **Album groups:** A frosted glass header showing "Album Name – Artist" with tracks listed underneath.
  - **Artist groups:** A frosted glass header showing "Artist Name" with related tracks.
  - **Free results:** Tracks that don't match a specific group appear without a header.
- Each track rendered as a `TrackBubble` with:
  - HD cover art (left, 64×64).
  - Title (bold, large) with cyan highlight on matching text.
  - Subtitle: "Artist – Album" (gray) with cyan highlight on matching text.
  - Technical pill badge: `44.1kHz • FLAC` in monospace emerald-green on a subtle dark chip.
- Fade-in + slide-up animation for each result bubble (10px offset, staggered delay).
- Empty state: Large centered "📂 Seleccionar Carpeta de Música" button when no library is configured.
- No-results state: "🔍 Sin resultados" message with optional folder selector.

**Key interactions:**
- Debounced search (150ms) calls Rust `fast_search(query)` via Tauri IPC.
- Filter chips apply local filtering on the results returned from backend.
- Clicking a track starts playback and the DynamicPill reacts.

---

## 4. Dynamic Pill – Bottom Navigation

**Description:**
- Fixed at the bottom-center of the screen, floating above content.
- Rounded-full capsule shape with Liquid Glass effect.
- Contains: mini now-playing section (cover art + title + play/pause) and navigation icons (Library, EQ, Search, Settings).
- Active tab icon has `bg-white/15` highlight; hover shows a cyan glow ring.
- When a track is selected from search, the pill expands to show the now-playing section with a smooth spring animation.
- Library-empty state shows a cyan "📂 Seleccionar Biblioteca" button inside the pill.

---

## 5. Design Tokens Summary

| Token | Value |
|---|---|
| Background | `#0a0a0c` |
| Glass bg | `rgba(255,255,255,0.05)` |
| Blur | `40px` |
| Saturation | `180%` |
| Border top | `rgba(255,255,255,0.1)` |
| Border bottom | `rgba(0,0,0,0.4)` |
| Shadow | `0 20px 50px rgba(0,0,0,0.5)` |
| Accent (active) | `cyan-400` / `#22d3ee` |
| Tech pill text | `emerald-400` / monospace |
| Highlight color | `cyan-300` |
| Noise overlay | SVG fractal noise at 1.5% opacity |
