import { useState, useCallback, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { open } from "@tauri-apps/plugin-dialog";
import FluidBackground from "./components/FluidBackground";
import FullPlayerView from "./components/FullPlayerView";
import LyricsView from "./components/LyricsView";
import PlaybackControls from "./components/PlaybackControls";
import VisualEQ from "./components/VisualEQ";
import EqualizerView from "./components/EqualizerView";
import DynamicPill, { type PillTab } from "./components/DynamicPill";
import LibraryView from "./components/LibraryView";
import SearchView from "./components/SearchView";
import SettingsView from "./components/SettingsView";
import WelcomeView from "./components/WelcomeView";
import { useAudioIPC } from "./hooks/useAudioIPC";
import { useTrackState } from "./hooks/useTrackState";

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

function App() {
  const { invokeSafe, listenSafe } = useAudioIPC();
  const {
    isPlaying,
    setIsPlaying,
    volume,
    setVolume,
    albumArt,
    setAlbumArt,
    trackTitle,
    setTrackTitle,
    trackArtist,
    setTrackArtist,
    duration,
    setDuration,
    currentTime,
    setCurrentTime,
    lyricsLines,
    setLyricsLines,
    activeLyricIndex,
    setActiveLyricIndex,
  } = useTrackState();
  const [spectrum, setSpectrum] = useState<number[]>([]);
  const [amplitude, setAmplitude] = useState(0);
  const [fps, setFps] = useState(0);
  const [activeView, setActiveView] = useState<PillTab>("library");
  const [libraryEmpty, setLibraryEmpty] = useState(false);
  const [showFullPlayer, setShowFullPlayer] = useState(false);
  const pendingVibeRef = useRef(false);
  const skipFrameRef = useRef(false);
  const amplitudeRef = useRef(0);
  const spectrumRef = useRef<number[]>([]);
  const activeArtUrlRef = useRef<string | null>(null);

  const updateAlbumArt = useCallback(
    (coverArt?: TrackData["cover_art"]) => {
      if (activeArtUrlRef.current) {
        URL.revokeObjectURL(activeArtUrlRef.current);
        activeArtUrlRef.current = null;
      }
      if (coverArt) {
        const blob = new Blob([new Uint8Array(coverArt.data)], {
          type: coverArt.media_type || "image/jpeg",
        });
        const artUrl = URL.createObjectURL(blob);
        activeArtUrlRef.current = artUrl;
        setAlbumArt(artUrl);
      } else {
        setAlbumArt(undefined);
      }
    },
    [setAlbumArt]
  );

  const handlePlay = useCallback(() => {
    setIsPlaying(true);
    void invokeSafe("play").catch((error) => {
      console.error("Failed to play track", error);
    });
  }, [invokeSafe, setIsPlaying]);
  const handlePause = useCallback(() => {
    setIsPlaying(false);
    void invokeSafe("pause").catch((error) => {
      console.error("Failed to pause track", error);
    });
  }, [invokeSafe, setIsPlaying]);
  const handleSkipForward = useCallback(() => {
    const next = Math.min(duration, currentTime + 10);
    setCurrentTime(next);
    void invokeSafe("seek", { seconds: next }).catch((error) => {
      console.error("Failed to seek forward", error);
    });
  }, [currentTime, duration, invokeSafe]);
  const handleSkipBack = useCallback(() => {
    const prev = Math.max(0, currentTime - 10);
    setCurrentTime(prev);
    void invokeSafe("seek", { seconds: prev }).catch((error) => {
      console.error("Failed to seek backward", error);
    });
  }, [currentTime, invokeSafe]);

  const handleOpenTrack = useCallback(async () => {
    try {
      const selected = await open({
        filters: [{ name: "Audio", extensions: ["flac", "mp3", "wav"] }],
        multiple: false,
      });
      if (!selected || Array.isArray(selected)) return;

      const track = await invokeSafe<TrackData>("load_track", { path: selected });
      const parsedLyrics = await invokeSafe<LyricsLine[]>("get_lyrics_lines");
      setTrackTitle(track.title || "Unknown Title");
      setTrackArtist(track.artist || "Unknown Artist");
      setDuration(track.duration_seconds || 0);
      setCurrentTime(0);
      setLyricsLines(parsedLyrics);
      setActiveLyricIndex(0);
      updateAlbumArt(track.cover_art);
    } catch (error) {
      console.error("Track load failed while opening file picker selection", error);
    }
  }, [invokeSafe, updateAlbumArt]);

  const handleSeek = useCallback((seconds: number) => {
    setCurrentTime(seconds);
    void invokeSafe("seek", { seconds }).catch((error) => {
      console.error("Failed to seek to selected time", error);
    });
  }, [invokeSafe]);

  const handleVolume = useCallback((sliderVolume: number) => {
    setVolume(sliderVolume);
    const linearVolume =
      sliderVolume <= 0
        ? 0
        : Math.pow(
            10,
            (sliderVolume * VOLUME_SLIDER_DB_RANGE - VOLUME_SLIDER_DB_RANGE) / 20
          );
    void invokeSafe("set_volume", { volume: linearVolume }).catch((error) => {
      console.error("Failed to update playback volume", error);
    });
  }, [invokeSafe]);

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
        const track = await invokeSafe<TrackData>("load_track", { path });
        const parsedLyrics = await invokeSafe<LyricsLine[]>("get_lyrics_lines");
        setTrackTitle(track.title || "Unknown Title");
        setTrackArtist(track.artist || "Unknown Artist");
        setDuration(track.duration_seconds || 0);
        setCurrentTime(0);
        setLyricsLines(parsedLyrics);
        setActiveLyricIndex(0);
        updateAlbumArt(track.cover_art);
        handlePlay();
      } catch (error) {
        console.error(`Track selection failed for path "${path}"`, error);
      }
    },
    [handlePlay, invokeSafe, updateAlbumArt]
  );

  // Check if library is empty on mount
  useEffect(() => {
    invokeSafe<{ path: string }[]>("get_library_tracks")
      .then((tracks) => setLibraryEmpty(!tracks || tracks.length === 0))
      .catch((error) => {
        console.error("Failed to read library tracks on app mount", error);
        setLibraryEmpty(true);
      });
  }, [invokeSafe]);

  // Select folder and scan library (Tauri integration)
  const handleSelectLibrary = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return;
      await invokeSafe("scan_library", { path: selected });
      setLibraryEmpty(false);
      setActiveView("library");
    } catch (error) {
      console.error("Library scan failed for selected folder", error);
    }
  }, [invokeSafe]);

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
        .catch((error) => {
          console.error("Failed to read vibe data from backend", error);
        })
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
        activeArtUrlRef.current = null;
      }
    },
    []
  );

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void (async () => {
      try {
        unlisten = await listenSafe<LyricsEventPayload>("lyrics-line-changed", (event) => {
          const index = event.payload.index;
          if (typeof index === "number" && index >= 0) {
            setActiveLyricIndex(index);
          } else {
            setActiveLyricIndex(0);
          }
        });
      } catch (error) {
        console.error("Failed to register lyrics-line-changed listener", error);
      }
    })();
    return () => {
      unlisten?.();
    };
  }, [listenSafe, setActiveLyricIndex]);

  const showFps = import.meta.env.DEV;

  return (
    <div className="noise-bg h-screen w-screen overflow-hidden bg-[#0a0a0c] text-white">
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
            {libraryEmpty ? (
              <WelcomeView onSelectLibrary={handleSelectLibrary} />
            ) : (
              <LibraryView
                isPlaying={isPlaying}
                onPlayPause={handlePlayPause}
                onTrackSelect={handleTrackSelect}
                onSelectLibrary={handleSelectLibrary}
              />
            )}
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
        ) : activeView === "settings" ? (
          <motion.div
            key="settings"
            initial={{ opacity: 0, x: 50 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -50 }}
            transition={{
              enter: { type: "spring", stiffness: 300, damping: 25 },
              exit: { duration: 0.25 },
            }}
            className="h-full w-full"
          >
            <SettingsView onBack={() => setActiveView("library")} />
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

      {/* Full Player Overlay */}
      <AnimatePresence>
        {showFullPlayer && (
          <FullPlayerView
            albumArt={albumArt}
            trackTitle={trackTitle}
            trackArtist={trackArtist}
            isPlaying={isPlaying}
            currentTime={currentTime}
            duration={duration}
            onPlayPause={handlePlayPause}
            onSkipBack={handleSkipBack}
            onSkipForward={handleSkipForward}
            onSeek={handleSeek}
            onClose={() => setShowFullPlayer(false)}
          />
        )}
      </AnimatePresence>

      {/* Dynamic Pill Navigation */}
      <DynamicPill
        activeTab={activeView}
        onTabChange={setActiveView}
        isPlaying={isPlaying}
        onPlayPause={handlePlayPause}
        currentTrack={currentTrackForPill}
        onScanLibrary={handleSelectLibrary}
        onTrackClick={() => currentTrackForPill && setShowFullPlayer(true)}
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
