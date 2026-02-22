import { memo, useMemo, useCallback, useRef, useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Shuffle, Repeat, Timer, SkipBack, SkipForward, Mic2, Box } from "lucide-react";
import { useAudioIPC } from "../hooks/useAudioIPC";
import LyricsEngine from "./LyricsEngine";
import SpatialRoom from "./SpatialRoom";

type FullPlayerMode = "art" | "lyrics" | "spatial";

interface LyricsLine {
  timestamp: number;
  text: string;
}

interface FullPlayerViewProps {
  albumArt?: string;
  trackTitle: string;
  trackArtist: string;
  activeTrackPath?: string;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  lyricsLines?: LyricsLine[];
  neonColor?: string;
  onPlayPause: () => void;
  onSkipBack: () => void;
  onSkipForward: () => void;
  onSeek: (seconds: number) => void;
  onClose: () => void;
}

const WAVEFORM_BARS = 80;
const SEEK_DEBOUNCE_MS = 75;

/** Generate a stable pseudo-random waveform from the track title. */
function generateWaveform(seed: string, count: number): number[] {
  let hash = 0;
  for (let i = 0; i < seed.length; i++) {
    hash = (hash << 5) - hash + seed.charCodeAt(i);
    hash |= 0;
  }
  const bars: number[] = [];
  for (let i = 0; i < count; i++) {
    hash = ((hash * 1103515245 + 12345) & 0x7fffffff) >>> 0;
    const base = (hash % 100) / 100;
    // Shape: taller in the middle, shorter at the edges
    const pos = i / count;
    const envelope = 0.4 + 0.6 * Math.sin(pos * Math.PI);
    bars.push(0.15 + base * 0.85 * envelope);
  }
  return bars;
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

function FullPlayerView({
  albumArt,
  trackTitle,
  trackArtist,
  activeTrackPath,
  isPlaying,
  currentTime,
  duration,
  lyricsLines = [],
  neonColor,
  onPlayPause,
  onSkipBack,
  onSkipForward,
  onSeek,
  onClose,
}: FullPlayerViewProps) {
  const { invokeSafe } = useAudioIPC();
  const [mode, setMode] = useState<FullPlayerMode>("art");
  const fallbackWaveform = useMemo(
    () => generateWaveform(trackTitle + trackArtist, WAVEFORM_BARS),
    [trackTitle, trackArtist]
  );
  const [waveformData, setWaveformData] = useState<number[] | null>(null);
  const [isWaveformLoading, setIsWaveformLoading] = useState(false);
  const waveform = waveformData && waveformData.length ? waveformData : fallbackWaveform;
  const progress = duration > 0 ? currentTime / duration : 0;

  // Local seek state for smooth interaction
  const [localProgress, setLocalProgress] = useState<number | null>(null);
  const seekDebounceRef = useRef<number | null>(null);
  const waveContainerRef = useRef<HTMLDivElement>(null);
  const dragListenersRef = useRef<{ move: (e: MouseEvent) => void; up: (e: MouseEvent) => void } | null>(null);

  useEffect(
    () => () => {
      if (seekDebounceRef.current !== null) {
        window.clearTimeout(seekDebounceRef.current);
      }
      if (dragListenersRef.current) {
        document.removeEventListener("mousemove", dragListenersRef.current.move);
        document.removeEventListener("mouseup", dragListenersRef.current.up);
        dragListenersRef.current = null;
      }
    },
    []
  );

  useEffect(() => {
    let cancelled = false;
    if (!activeTrackPath) {
      setWaveformData(null);
      setIsWaveformLoading(false);
      return;
    }

    setWaveformData(null);
    setIsWaveformLoading(true);
    void invokeSafe<number[]>("extract_waveform", { path: activeTrackPath, points: WAVEFORM_BARS })
      .then((data) => {
        if (!cancelled && Array.isArray(data) && data.length) {
          // Defensive clamp in case stale/corrupt cache data falls outside normalized range.
          setWaveformData(data.map((value) => Math.max(0, Math.min(1, Number(value) || 0))));
        }
      })
      .catch((error) => {
        if (!cancelled) {
          console.error(`Failed to extract waveform for "${activeTrackPath}"`, error);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsWaveformLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [activeTrackPath, invokeSafe]);

  const displayProgress = localProgress ?? progress;

  const handleWaveSeek = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const rect = e.currentTarget.getBoundingClientRect();
      const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      setLocalProgress(x);
      if (seekDebounceRef.current !== null) {
        window.clearTimeout(seekDebounceRef.current);
      }
      seekDebounceRef.current = window.setTimeout(() => {
        onSeek(x * duration);
        setLocalProgress(null);
      }, SEEK_DEBOUNCE_MS);
    },
    [duration, onSeek]
  );

  const handleWaveMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      handleWaveSeek(e);
      const onMove = (me: MouseEvent) => {
        if (!waveContainerRef.current) return;
        const rect = waveContainerRef.current.getBoundingClientRect();
        const x = Math.max(0, Math.min(1, (me.clientX - rect.left) / rect.width));
        setLocalProgress(x);
      };
      const onUp = (me: MouseEvent) => {
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
        dragListenersRef.current = null;
        if (!waveContainerRef.current) return;
        const rect = waveContainerRef.current.getBoundingClientRect();
        const x = Math.max(0, Math.min(1, (me.clientX - rect.left) / rect.width));
        if (seekDebounceRef.current !== null) {
          window.clearTimeout(seekDebounceRef.current);
          seekDebounceRef.current = null;
        }
        onSeek(x * duration);
        setLocalProgress(null);
      };
      // Clean up any previous listeners before adding new ones
      if (dragListenersRef.current) {
        document.removeEventListener("mousemove", dragListenersRef.current.move);
        document.removeEventListener("mouseup", dragListenersRef.current.up);
      }
      dragListenersRef.current = { move: onMove, up: onUp };
      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [duration, onSeek, handleWaveSeek]
  );

  const handleWaveKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const step = duration * 0.02;
      if (e.key === "ArrowRight") {
        e.preventDefault();
        onSeek(Math.min(duration, currentTime + step));
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        onSeek(Math.max(0, currentTime - step));
      }
    },
    [currentTime, duration, onSeek]
  );

  return (
    <motion.div
      initial={{ opacity: 0, y: 60 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 60 }}
      transition={{ type: "spring", stiffness: 260, damping: 28 }}
      className="fixed inset-0 z-40 flex flex-col items-center overflow-hidden"
    >
      {/* Click-to-close overlay (top area) */}
      <button
        type="button"
        aria-label="Close full player"
        onClick={onClose}
        className="absolute right-6 top-5 z-50 flex h-9 w-9 items-center justify-center rounded-full bg-white/5 text-white/50 backdrop-blur-md transition-colors hover:bg-white/10 hover:text-white"
      >
        <svg className="h-5 w-5" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {/* Content container – scrollable if needed, centered */}
      <div className="flex w-full max-w-xl flex-1 flex-col items-center justify-center gap-5 px-6 pb-32 pt-12">
        {/* ── Mode selector tabs ── */}
        <div className="flex items-center gap-2 rounded-full bg-white/5 p-1 backdrop-blur-md">
          {([
            { id: "art" as FullPlayerMode, label: "Arte" },
            { id: "lyrics" as FullPlayerMode, label: "Letras", icon: Mic2 },
            { id: "spatial" as FullPlayerMode, label: "Escenario 3D", icon: Box },
          ]).map((tab) => {
            const isActive = mode === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => setMode(tab.id)}
                className={`flex items-center gap-1.5 rounded-full px-4 py-1.5 text-xs font-medium transition-all ${
                  isActive ? "bg-white/15 text-white shadow-sm" : "text-white/50 hover:text-white/80"
                }`}
              >
                {tab.icon && <tab.icon size={13} />}
                {tab.label}
              </button>
            );
          })}
        </div>

        {/* ── 2. Central content area with mode transitions ── */}
        <AnimatePresence mode="wait">
          {mode === "art" && (
            <motion.div
              key="art"
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              transition={{ duration: 0.35, ease: "easeOut" }}
              className="flex w-full flex-col items-center gap-5"
            >
              <motion.div
                layoutId="track-art"
                className="relative aspect-square w-full max-w-[22rem] overflow-hidden rounded-[2rem] shadow-2xl"
                style={{
                  boxShadow: "0 30px 80px -10px rgba(0,0,0,0.6), 0 0 60px -15px rgba(139,92,246,0.35)",
                }}
              >
                {albumArt ? (
                  <img src={albumArt} alt="Album art" className="h-full w-full object-cover" />
                ) : (
                  <div className="flex h-full w-full items-center justify-center bg-white/5">
                    <svg className="h-20 w-20 text-white/20" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
                    </svg>
                  </div>
                )}
              </motion.div>
            </motion.div>
          )}

          {mode === "lyrics" && (
            <motion.div
              key="lyrics"
              initial={{ opacity: 0, y: 30 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
              transition={{ type: "spring", stiffness: 200, damping: 24 }}
              className="w-full"
              style={{ height: "22rem" }}
            >
              <LyricsEngine
                lines={lyricsLines}
                currentTime={currentTime}
                neonColor={neonColor}
              />
            </motion.div>
          )}

          {mode === "spatial" && (
            <motion.div
              key="spatial"
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
              transition={{ type: "spring", stiffness: 200, damping: 22 }}
            >
              <SpatialRoom />
            </motion.div>
          )}
        </AnimatePresence>

        {/* Track info */}
        <div className="w-full text-center">
          <h2 className="truncate text-2xl font-bold text-white">{trackTitle}</h2>
          <p className="mt-0.5 truncate text-sm text-white/50">{trackArtist}</p>
        </div>

        {/* ── 3. Secondary Controls ── */}
        <div className="flex items-center gap-4">
          <motion.button
            type="button"
            whileHover={{ scale: 1.12 }}
            whileTap={{ scale: 0.92 }}
            className="liquid-glass flex h-10 w-10 items-center justify-center rounded-full text-white/60 transition-colors hover:text-white"
            aria-label="Shuffle"
          >
            <Shuffle size={16} />
          </motion.button>
          <motion.button
            type="button"
            whileHover={{ scale: 1.12 }}
            whileTap={{ scale: 0.92 }}
            className="liquid-glass flex h-10 w-10 items-center justify-center rounded-full text-white/60 transition-colors hover:text-white"
            aria-label="Loop"
          >
            <Repeat size={16} />
          </motion.button>
          <motion.button
            type="button"
            whileHover={{ scale: 1.12 }}
            whileTap={{ scale: 0.92 }}
            className="liquid-glass flex h-10 w-10 items-center justify-center rounded-full text-white/60 transition-colors hover:text-white"
            aria-label="Sleep timer"
          >
            <Timer size={16} />
          </motion.button>
        </div>

        {/* ── 4 & 5. Waveform + Main Controls ── */}
        <div className="flex w-full items-center gap-3">
          {/* Previous */}
          <motion.button
            type="button"
            whileHover={{ scale: 1.15 }}
            whileTap={{ scale: 0.9 }}
            onClick={onSkipBack}
            className="liquid-glass flex h-11 w-11 flex-shrink-0 items-center justify-center rounded-full text-white/80"
            aria-label="Previous"
          >
            <SkipBack size={18} fill="currentColor" />
          </motion.button>

          {/* Waveform bar (left half) + Play/Pause + (right half) */}
          <div className="relative flex flex-1 items-center">
            {/* Waveform container */}
            <div
              ref={waveContainerRef}
              className={`flex h-12 flex-1 cursor-pointer items-end gap-[2px] rounded-xl ${isWaveformLoading ? "animate-pulse" : ""}`}
              onMouseDown={handleWaveMouseDown}
              onKeyDown={handleWaveKeyDown}
              role="slider"
              aria-label="Seek"
              aria-valuemin={0}
              aria-valuemax={duration}
              aria-valuenow={currentTime}
              tabIndex={0}
            >
              {waveform.map((height, i) => {
                const barProgress = i / waveform.length;
                const played = barProgress <= displayProgress;
                return (
                  <div
                    key={i}
                    className="flex-1 rounded-sm transition-colors duration-150"
                    style={{
                      height: `${height * 100}%`,
                      background: played
                        ? "linear-gradient(to top, rgba(139,92,246,0.9), rgba(34,211,238,0.8))"
                        : "rgba(255,255,255,0.08)",
                      boxShadow: played ? "0 0 6px rgba(139,92,246,0.4)" : "none",
                    }}
                  />
                );
              })}
            </div>

            {/* Play/Pause button – centered over waveform */}
            <motion.button
              type="button"
              whileHover={{ scale: 1.1 }}
              whileTap={{ scale: 0.92 }}
              onClick={onPlayPause}
              className="liquid-glass absolute left-1/2 top-1/2 z-10 flex h-14 w-14 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full text-white shadow-[0_0_24px_rgba(139,92,246,0.4)]"
              aria-label={isPlaying ? "Pause" : "Play"}
            >
              {isPlaying ? (
                <svg className="h-6 w-6" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                </svg>
              ) : (
                <svg className="ml-0.5 h-6 w-6" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M8 5v14l11-7z" />
                </svg>
              )}
            </motion.button>
          </div>

          {/* Next */}
          <motion.button
            type="button"
            whileHover={{ scale: 1.15 }}
            whileTap={{ scale: 0.9 }}
            onClick={onSkipForward}
            className="liquid-glass flex h-11 w-11 flex-shrink-0 items-center justify-center rounded-full text-white/80"
            aria-label="Next"
          >
            <SkipForward size={18} fill="currentColor" />
          </motion.button>
        </div>

        {/* ── 6. Metadata Footer ── */}
        <div className="flex w-full items-center justify-between text-xs">
          <span className="text-white/50">{formatTime(currentTime)}</span>
          <span className="font-mono text-emerald-400/70">
            24-bit / 192kHz&nbsp;•&nbsp;FLAC
          </span>
          <span className="text-white/50">{formatTime(duration)}</span>
        </div>
      </div>
    </motion.div>
  );
}

export default memo(FullPlayerView);
