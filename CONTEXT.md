# CONTEXT

## Motor de Audio (Backend Rust/Tauri)

- Se añadió el módulo `src-tauri/src/audio/engine.rs` con `AudioState` thread-safe basado en `Arc` + `Atomic` para control sin bloqueos en el callback (`play`, `pause`, `seek`, `set_volume`).
- Se añadió `src-tauri/src/audio/decoder.rs` con decodificación usando `symphonia::core::io::MediaSourceStream` y foco en FLAC/multiformato.
- El motor intenta seleccionar el dispositivo de salida predeterminado y configurar sample-rate exacto al del archivo; si no hay coincidencia, registra advertencia y aplica resampling lineal previo.
- Se usa un ring buffer (`ringbuf::HeapRb`) para desacoplar el productor de PCM (hilo de decodificación/alimentación) del callback de reproducción.
- En Windows, el flujo prioriza la ruta WASAPI disponible vía host por defecto y registra explícitamente cuando se ejecuta en modo best-effort compartido por limitaciones de API de `cpal`.
- Se expusieron comandos Tauri en `src-tauri/src/lib.rs` bajo feature `tauri-state`: `load_track(path)`, `play()`, `pause()`, `seek(seconds)`, `set_volume(0.0..1.0)`.
