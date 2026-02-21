import { useState, useCallback, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import FluidBackground from "./components/FluidBackground";
import LyricsView from "./components/LyricsView";
import PlaybackControls from "./components/PlaybackControls";
import VisualEQ from "./components/VisualEQ";

interface TrackData {
  artist: string;
  title: string;
  cover_art?: {
    media_type: string;
    data: number[];
  };
  duration_seconds: number;
}

interface VibeData {
  spectrum: number[];
  amplitude: number;
}

interface LyricsLine {
  timestamp: number;
  text: string;
}

interface LyricsEventPayload {
  index: number | null;
  timestamp: number | null;
  text: string | null;
}

const VOLUME_SLIDER_DB_RANGE = 60;
const VIBE_SKIP_THRESHOLD_MS = 8;
const VIBE_CHANGE_THRESHOLD = 0.75;

function App() {
  const [isPlaying, setIsPlaying] = useState(false);
  const [volume, setVolume] = useState(0.75);
  const [albumArt, setAlbumArt] = useState<string | undefined>(undefined);
  const [trackTitle, setTrackTitle] = useState("PowerPlayer");
  const [trackArtist, setTrackArtist] = useState("Hi-Res Audio Player");
  const [duration, setDuration] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [spectrum, setSpectrum] = useState<number[]>([]);
  const [amplitude, setAmplitude] = useState(0);
  const [fps, setFps] = useState(0);
  const [lyricsLines, setLyricsLines] = useState<LyricsLine[]>([]);
  const [activeLyricIndex, setActiveLyricIndex] = useState(0);
  const pendingVibeRef = useRef(false);
  const skipFrameRef = useRef(false);
  const amplitudeRef = useRef(0);
  const spectrumRef = useRef<number[]>([]);
  const activeArtUrlRef = useRef<string | null>(null);

  const handlePlay = useCallback(() => {
    setIsPlaying(true);
    void invoke("play");
  }, []);
  const handlePause = useCallback(() => {
    setIsPlaying(false);
    void invoke("pause");
  }, []);
  const handleSkipForward = useCallback(() => {
    const next = Math.min(duration, currentTime + 10);
    setCurrentTime(next);
    void invoke("seek", { seconds: next });
  }, [currentTime, duration]);
  const handleSkipBack = useCallback(() => {
    const prev = Math.max(0, currentTime - 10);
    setCurrentTime(prev);
    void invoke("seek", { seconds: prev });
  }, [currentTime]);

  const handleOpenTrack = useCallback(async () => {
    try {
      const selected = await open({
        filters: [{ name: "Audio", extensions: ["flac", "mp3", "wav"] }],
        multiple: false,
      });
      if (!selected || Array.isArray(selected)) return;

      const track = await invoke<TrackData>("load_track", { path: selected });
      const parsedLyrics = await invoke<LyricsLine[]>("get_lyrics_lines");
      setTrackTitle(track.title || "Unknown Title");
      setTrackArtist(track.artist || "Unknown Artist");
      setDuration(track.duration_seconds || 0);
      setCurrentTime(0);
      setLyricsLines(parsedLyrics);
      setActiveLyricIndex(0);
      if (track.cover_art) {
        if (activeArtUrlRef.current) {
          URL.revokeObjectURL(activeArtUrlRef.current);
        }
        const blob = new Blob([new Uint8Array(track.cover_art.data)], {
          type: track.cover_art.media_type || "image/jpeg",
        });
        const artUrl = URL.createObjectURL(blob);
        activeArtUrlRef.current = artUrl;
        setAlbumArt(artUrl);
      } else {
        setAlbumArt(undefined);
      }
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error("track load failed", error);
      }
    }
  }, []);

  const handleSeek = useCallback((seconds: number) => {
    setCurrentTime(seconds);
    void invoke("seek", { seconds });
  }, []);

  const handleVolume = useCallback((sliderVolume: number) => {
    setVolume(sliderVolume);
    const linearVolume =
      sliderVolume <= 0
        ? 0
        : Math.pow(
            10,
            (sliderVolume * VOLUME_SLIDER_DB_RANGE - VOLUME_SLIDER_DB_RANGE) / 20
          );
    void invoke("set_volume", { volume: linearVolume });
  }, []);

  useEffect(() => {
    let frameCounter = 0;
    let lastFpsSample = performance.now();
    let rafId = 0;

    const tick = () => {
      rafId = requestAnimationFrame(tick);
      frameCounter += 1;
      const now = performance.now();
      if (now - lastFpsSample >= 1000) {
        setFps(frameCounter);
        frameCounter = 0;
        lastFpsSample = now;
      }

      if (pendingVibeRef.current) return;
      if (skipFrameRef.current) {
        skipFrameRef.current = false;
        return;
      }

      pendingVibeRef.current = true;
      const start = performance.now();
      invoke<VibeData>("get_vibe_data")
        .then((vibe) => {
          const spectrumChanged = hasSignificantSpectrumChange(
            spectrumRef.current,
            vibe.spectrum
          );
          const amplitudeChanged =
            Math.abs(amplitudeRef.current - vibe.amplitude) > 0.01;
          if (spectrumChanged) {
            spectrumRef.current = vibe.spectrum;
            setSpectrum(vibe.spectrum);
          }
          if (amplitudeChanged) {
            amplitudeRef.current = vibe.amplitude;
            setAmplitude(vibe.amplitude);
          }
        })
        .catch(() => {})
        .finally(() => {
          if (performance.now() - start > VIBE_SKIP_THRESHOLD_MS) {
            skipFrameRef.current = true;
          }
          pendingVibeRef.current = false;
        });
    };

    rafId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafId);
  }, []);

  useEffect(
    () => () => {
      if (activeArtUrlRef.current) {
        URL.revokeObjectURL(activeArtUrlRef.current);
      }
    },
    []
  );

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<LyricsEventPayload>("lyrics-line-changed", (event) => {
      const index = event.payload.index;
      if (typeof index === "number" && index >= 0) {
        setActiveLyricIndex(index);
      } else {
        setActiveLyricIndex(0);
      }
    })
      .then((cleanup) => {
        unlisten = cleanup;
      })
      .catch(() => {});
    return () => {
      unlisten?.();
    };
  }, []);

  const showFps = import.meta.env.DEV;

  return (
    <>
      <FluidBackground albumArt={albumArt} />

      <div className="flex min-h-screen flex-col items-center justify-center gap-6 p-6">
        {/* Album art + title area */}
        <AnimatePresence mode="wait">
          <motion.div
            key={albumArt ?? "default"}
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.5, ease: "easeOut" }}
            className="flex flex-col items-center gap-3"
          >
            <div className="h-48 w-48 overflow-hidden rounded-2xl border border-white/10 bg-white/5 shadow-xl backdrop-blur-md">
              {albumArt ? (
                <img
                  src={albumArt}
                  alt="Album art"
                  className="h-full w-full object-cover"
                />
              ) : (
                <div className="flex h-full w-full items-center justify-center">
                  <svg
                    className="h-16 w-16 text-white/20"
                    fill="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
                  </svg>
                </div>
              )}
            </div>
            <div className="text-center">
              <h1 className="text-xl font-semibold text-white">{trackTitle}</h1>
              <p className="text-sm text-white/50">{trackArtist}</p>
            </div>
          </motion.div>
        </AnimatePresence>

        <button
          type="button"
          onClick={handleOpenTrack}
          className="rounded-lg border border-white/20 bg-white/10 px-4 py-2 text-xs text-white/80 transition hover:bg-white/20"
        >
          Open Track
        </button>

        <LyricsView
          lines={lyricsLines}
          activeIndex={activeLyricIndex}
          fallback={<VisualEQ spectrum={spectrum} />}
        />

        {/* Playback Controls */}
        <PlaybackControls
          isPlaying={isPlaying}
          onPlay={handlePlay}
          onPause={handlePause}
          onSkipForward={handleSkipForward}
          onSkipBack={handleSkipBack}
          volume={volume}
          currentTime={currentTime}
          duration={duration}
          amplitude={amplitude}
          onSeek={handleSeek}
          onVolumeChange={handleVolume}
        />

        {lyricsLines.length ? (
          <div className="w-full max-w-2xl">
            <VisualEQ spectrum={spectrum} />
          </div>
        ) : null}
      </div>
      {showFps ? (
        <div className="pointer-events-none fixed right-3 top-3 rounded bg-black/50 px-2 py-1 text-xs text-white/80">
          {fps} FPS
        </div>
      ) : null}
    </>
  );
}

function hasSignificantSpectrumChange(previous: number[], next: number[]): boolean {
  if (previous.length !== next.length) {
    return true;
  }
  if (!next.length) {
    return false;
  }
  const step = Math.max(1, Math.floor(next.length / 48));
  for (let i = 0; i < next.length; i += step) {
    if (Math.abs(previous[i] - next[i]) > VIBE_CHANGE_THRESHOLD) {
      return true;
    }
  }
  return false;
}

export default App;
