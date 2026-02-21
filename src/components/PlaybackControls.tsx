import { motion } from "framer-motion";
import ProgressBar from "./ProgressBar";

interface PlaybackControlsProps {
  isPlaying: boolean;
  onPlay: () => void;
  onPause: () => void;
  onSkipForward: () => void;
  onSkipBack: () => void;
  volume: number;
  currentTime: number;
  duration: number;
  amplitude: number;
  onSeek: (seconds: number) => void;
  onVolumeChange: (volume: number) => void;
}

function GlassButton({
  onClick,
  children,
  size = "md",
  glow = false,
}: {
  onClick: () => void;
  children: React.ReactNode;
  size?: "sm" | "md" | "lg";
  glow?: boolean;
}) {
  const sizeClasses = {
    sm: "h-10 w-10",
    md: "h-12 w-12",
    lg: "h-14 w-14",
  };

  return (
    <motion.button
      onClick={onClick}
      whileHover={{ scale: 1.1 }}
      whileTap={{ scale: 0.95 }}
      className={`${sizeClasses[size]} flex items-center justify-center rounded-full border border-white/15 bg-white/10 backdrop-blur-xl transition-colors hover:bg-white/15 ${
        glow ? "shadow-[0_0_20px_rgba(139,92,246,0.4)]" : ""
      }`}
    >
      {children}
    </motion.button>
  );
}

export default function PlaybackControls({
  isPlaying,
  onPlay,
  onPause,
  onSkipForward,
  onSkipBack,
  volume,
  currentTime,
  duration,
  amplitude,
  onSeek,
  onVolumeChange,
}: PlaybackControlsProps) {
  // Scale glow intensity by volume (0-1)
  const glowIntensity = Math.min(1, Math.max(volume, amplitude));
  const glowStyle = {
    boxShadow: `0 0 ${20 + glowIntensity * 20}px rgba(139,92,246,${0.2 + glowIntensity * 0.3})`,
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, ease: "easeOut" }}
      className="flex w-full max-w-2xl flex-col gap-4 rounded-2xl border border-white/10 bg-white/5 px-6 py-4 backdrop-blur-xl"
      style={glowStyle}
    >
      <ProgressBar currentTime={currentTime} duration={duration} onSeek={onSeek} />

      <div className="flex items-center justify-center gap-4">
      {/* Skip Back */}
      <GlassButton onClick={onSkipBack} size="sm">
        <svg
          className="h-4 w-4 text-white/80"
          fill="currentColor"
          viewBox="0 0 24 24"
        >
          <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" />
        </svg>
      </GlassButton>

      {/* Play / Pause */}
      <GlassButton
        onClick={isPlaying ? onPause : onPlay}
        size="lg"
        glow={isPlaying}
      >
        {isPlaying ? (
          <svg
            className="h-6 w-6 text-white"
            fill="currentColor"
            viewBox="0 0 24 24"
          >
            <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
          </svg>
        ) : (
          <svg
            className="ml-0.5 h-6 w-6 text-white"
            fill="currentColor"
            viewBox="0 0 24 24"
          >
            <path d="M8 5v14l11-7z" />
          </svg>
        )}
      </GlassButton>

      {/* Skip Forward */}
      <GlassButton onClick={onSkipForward} size="sm">
        <svg
          className="h-4 w-4 text-white/80"
          fill="currentColor"
          viewBox="0 0 24 24"
        >
          <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" />
        </svg>
      </GlassButton>
      </div>

      <div className="flex items-center gap-3">
        <span className="w-14 text-xs text-white/60">Volume</span>
        <input
          type="range"
          min={0}
          max={1}
          step={0.01}
          value={volume}
          onChange={(event) => onVolumeChange(Number(event.target.value))}
          className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-white/20 accent-violet-400"
        />
      </div>
    </motion.div>
  );
}
