import { useState, useCallback, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import FluidBackground from "./components/FluidBackground";
import LyricsView from "./components/LyricsView";
import PlaybackControls from "./components/PlaybackControls";
import VisualEQ from "./components/VisualEQ";
import EqualizerView from "./components/EqualizerView";
import DynamicPill, { type PillTab } from "./components/DynamicPill";
import LibraryView from "./components/LibraryView";
import SearchView from "./components/SearchView";

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
const MAX_SPECTRUM_SAMPLE_POINTS = 48;
const invokeSafe = <T,>(command: string, args?: Record<string, unknown>) =>
  Promise.resolve().then(() => invoke<T>(command, args));
const listenSafe = <T,>(
  event: string,
  handler: (event: { payload: T }) => void
) => Promise.resolve().then(() => listen<T>(event, handler));

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
  const [activeView, setActiveView] = useState<PillTab>("library");
  const [libraryEmpty, setLibraryEmpty] = useState(false);
  const pendingVibeRef = useRef(false);
  const skipFrameRef = useRef(false);
  const amplitudeRef = useRef(0);
  const spectrumRef = useRef<number[]>([]);
  const activeArtUrlRef = useRef<string | null>(null);
  const lyricsUnlistenRef = useRef<(() => void) | null>(null);

  const handlePlay = useCallback(() => {
    setIsPlaying(true);
    void invokeSafe("play").catch(() => {});
  }, []);
  const handlePause = useCallback(() => {
    setIsPlaying(false);
    void invokeSafe("pause").catch(() => {});
  }, []);
  const handleSkipForward = useCallback(() => {
    const next = Math.min(duration, currentTime + 10);
    setCurrentTime(next);
    void invokeSafe("seek", { seconds: next }).catch(() => {});
  }, [currentTime, duration]);
  const handleSkipBack = useCallback(() => {
    const prev = Math.max(0, currentTime - 10);
    setCurrentTime(prev);
    void invokeSafe("seek", { seconds: prev }).catch(() => {});
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
    void invokeSafe("seek", { seconds }).catch(() => {});
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
    void invokeSafe("set_volume", { volume: linearVolume }).catch(() => {});
  }, []);

  const handlePlayPause = useCallback(() => {
    if (isPlaying) {
      handlePause();
    } else {
      handlePlay();
    }
  }, [isPlaying, handlePlay, handlePause]);

  const handleTrackSelect = useCallback(
    async (path: string) => {
      try {
        const track = await invoke<TrackData>("load_track", { path });
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
        handlePlay();
      } catch (error) {
        if (import.meta.env.DEV) {
          console.error("track select failed", error);
        }
      }
    },
    [handlePlay]
  );

  // Check if library is empty on mount
  useEffect(() => {
    invokeSafe<{ path: string }[]>("get_library_tracks")
      .then((tracks) => setLibraryEmpty(!tracks || tracks.length === 0))
      .catch(() => setLibraryEmpty(true));
  }, []);

  // Select folder and scan library (Tauri integration)
  const handleSelectLibrary = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return;
      await invoke("scan_library", { path: selected });
      setLibraryEmpty(false);
      setActiveView("library");
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error("library scan failed", error);
      }
    }
  }, []);

  const currentTrackForPill =
    trackTitle !== "PowerPlayer"
      ? { title: trackTitle, artUrl: albumArt }
      : undefined;

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
      invokeSafe<VibeData>("get_vibe_data")
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
    let disposed = false;
    listenSafe<LyricsEventPayload>("lyrics-line-changed", (event) => {
      const index = event.payload.index;
      if (typeof index === "number" && index >= 0) {
        setActiveLyricIndex(index);
      } else {
        setActiveLyricIndex(0);
      }
    })
      .then((cleanup) => {
        lyricsUnlistenRef.current = cleanup;
        if (disposed) {
          cleanup();
          lyricsUnlistenRef.current = null;
        }
      })
      .catch(() => {});
    return () => {
      disposed = true;
      lyricsUnlistenRef.current?.();
      lyricsUnlistenRef.current = null;
    };
  }, []);

  const showFps = import.meta.env.DEV;

  return (
    <div className="noise-bg h-screen w-screen overflow-hidden bg-[#0a0a0c] text-white">
      {/* Ambient background from album art */}
      {albumArt && (
        <div
          className="pointer-events-none fixed inset-0 -z-10 opacity-20 blur-3xl"
          style={{
            backgroundImage: `url(${albumArt})`,
            backgroundSize: "cover",
            backgroundPosition: "center",
          }}
        />
      )}

      <FluidBackground albumArt={albumArt} />

      {/* Main content area with animated transitions */}
      <AnimatePresence mode="wait">
        {activeView === "library" ? (
          <motion.div
            key="library"
            initial={{ opacity: 0, y: 40, filter: "blur(0px)" }}
            animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
            exit={{ opacity: 0, filter: "blur(8px)" }}
            transition={{
              enter: { type: "spring", stiffness: 300, damping: 25 },
              exit: { duration: 0.25 },
            }}
            className="h-full w-full"
          >
            <LibraryView
              isPlaying={isPlaying}
              onPlayPause={handlePlayPause}
              onTrackSelect={handleTrackSelect}
              onSelectLibrary={handleSelectLibrary}
            />
          </motion.div>
        ) : activeView === "eq" ? (
          <motion.div
            key="eq"
            initial={{ opacity: 0, y: 40, filter: "blur(0px)" }}
            animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
            exit={{ opacity: 0, filter: "blur(8px)" }}
            transition={{
              enter: { type: "spring", stiffness: 300, damping: 25 },
              exit: { duration: 0.25 },
            }}
            className="h-full w-full"
          >
            <EqualizerView spectrum={spectrum} />
          </motion.div>
        ) : activeView === "search" ? (
          <motion.div
            key="search"
            initial={{ opacity: 0, y: 40, filter: "blur(0px)" }}
            animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
            exit={{ opacity: 0, filter: "blur(8px)" }}
            transition={{
              enter: { type: "spring", stiffness: 300, damping: 25 },
              exit: { duration: 0.25 },
            }}
            className="h-full w-full"
          >
            <SearchView
              onTrackSelect={handleTrackSelect}
              onSelectLibrary={handleSelectLibrary}
              libraryEmpty={libraryEmpty}
            />
          </motion.div>
        ) : (
          <motion.div
            key="player"
            initial={{ opacity: 0, y: 40, filter: "blur(0px)" }}
            animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
            exit={{ opacity: 0, filter: "blur(8px)" }}
            transition={{
              enter: { type: "spring", stiffness: 300, damping: 25 },
              exit: { duration: 0.25 },
            }}
            className="flex h-full flex-col items-center justify-center gap-6 p-6"
          >
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
                <div className="h-48 w-48 overflow-hidden rounded-2xl border-t border-white/10 border-b-black/40 bg-white/5 shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%]">
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
              className="rounded-lg border-t border-white/10 border-b-black/40 bg-white/5 px-4 py-2 text-xs text-white/80 shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%] transition hover:bg-white/10"
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
          </motion.div>
        )}
      </AnimatePresence>

      {/* Dynamic Pill Navigation */}
      <DynamicPill
        activeTab={activeView}
        onTabChange={setActiveView}
        isPlaying={isPlaying}
        onPlayPause={handlePlayPause}
        currentTrack={currentTrackForPill}
        libraryEmpty={libraryEmpty}
        onSelectLibrary={handleSelectLibrary}
      />

      {showFps ? (
        <div className="pointer-events-none fixed right-3 top-3 rounded bg-black/50 px-2 py-1 text-xs text-white/80">
          {fps} FPS
        </div>
      ) : null}
    </div>
  );
}

function hasSignificantSpectrumChange(previous: number[], next: number[]): boolean {
  if (previous.length !== next.length) {
    return true;
  }
  if (!next.length) {
    return false;
  }
  const step = Math.max(1, Math.floor(next.length / MAX_SPECTRUM_SAMPLE_POINTS));
  for (let i = 0; i < next.length; i += step) {
    if (Math.abs(previous[i] - next[i]) > VIBE_CHANGE_THRESHOLD) {
      return true;
    }
  }
  return false;
}

export default App;
