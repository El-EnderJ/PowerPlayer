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

## DSP Topology (Engine)

- **Pre-Amp (global)**: applies gain in dB before EQ to create headroom.
- **Parametric EQ**: 10 configurable bands with atomic `frequency`, `gain_db`, and `Q_factor`.
  - Each band uses biquad filters in **Direct Form II Transposed**.
  - Coefficients are recalculated **only when parameters change**.
- **Soft Limiter**: final protection stage (threshold near **-0.1 dBFS**) to avoid digital clipping.
- **Order**: `Input sample -> Pre-Amp -> ParametricEQ (L/R independent, shared params) -> Soft Limiter -> Output`.
