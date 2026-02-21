# Product Vision & Objectives

## Vision

PowerPlayer is a **Hi-Res Audio Player for Windows** designed to be the desktop evolution of PowerAmp. It combines a bit-perfect audio engine with an ultra-minimalist "Fluid Glass" aesthetic, delivering studio-quality playback in a beautiful, performant interface.

## Target Audience

Audiophiles and music enthusiasts who demand:
- **Bit-perfect playback** — no resampling, no mixing; audio data reaches the DAC untouched.
- **WASAPI Exclusive Mode** — bypasses the Windows audio mixer for direct hardware access.
- **Hi-Res format support** — FLAC, WAV, AIFF, DSD (up to 32-bit / 384 kHz).
- **Low-latency DSP** — parametric EQ and effects processed in Rust with zero-copy buffers.

## Audio Philosophy

1. **Bit-Perfect Path**: Audio is decoded in Rust (via `symphonia`), processed through an optional DSP chain (Rust biquad filters), and output via `cpal` in WASAPI Exclusive mode. The frontend never touches raw audio data.
2. **Format Priority**: FLAC is the primary target format. Lossless and high-resolution formats are first-class citizens.
3. **Zero Compromise**: No hidden resampling, no unnecessary conversions. The signal path is transparent and auditable.

## UI / UX — "Fluid Glass" Aesthetic

The interface follows a **Fluid Glass** design language:
- **Glassmorphism** — translucent panels with backdrop blur and subtle borders.
- **Fluid animations** — all transitions run at 60–120 fps using Framer Motion with GPU-accelerated transforms.
- **Minimalism** — the UI shows only what matters: album art, track info, playback controls, and a waveform/spectrum visualizer.
- **Dark-first** — designed for dark environments (listening sessions), with an optional light mode.

## Technical Stack

| Component | Technology |
|-----------|-----------|
| Desktop Shell | Tauri v2 |
| Audio Engine | Rust + cpal (WASAPI) |
| Audio Decoding | Rust + symphonia |
| DSP / EQ | Rust biquad filters |
| Frontend | React + TypeScript + Vite |
| Styling | Tailwind CSS |
| Animations | Framer Motion |
