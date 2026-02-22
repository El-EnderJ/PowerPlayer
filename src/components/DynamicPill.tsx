import { memo, useState, useRef, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  LayoutGrid,
  SlidersHorizontal,
  Search,
  Settings,
  Play,
  Pause,
  SlidersHorizontal as AudioIcon,
  FolderSearch,
  Info,
  Mic2,
  ChevronDown,
} from "lucide-react";

export type PillTab = "library" | "eq" | "search" | "settings";

interface DynamicPillProps {
  activeTab: PillTab;
  onTabChange: (tab: PillTab) => void;
  isPlaying: boolean;
  onPlayPause: () => void;
  currentTrack?: {
    title: string;
    artUrl?: string;
  };
  onScanLibrary?: () => void;
  onTrackClick?: () => void;
  /** When true, the pill becomes ultra-minimalist (full player is open) */
  fullPlayerOpen?: boolean;
  /** Remaining time string to show in minimalist mode */
  remainingTime?: string;
  /** Callback when user clicks the Lyrics quick-toggle in minimalist mode */
  onToggleLyrics?: () => void;
  /** Callback to close the full player from the minimalist pill */
  onCloseFullPlayer?: () => void;
}

const TABS: { id: PillTab; icon: typeof LayoutGrid; label: string }[] = [
  { id: "library", icon: LayoutGrid, label: "Library" },
  { id: "eq", icon: SlidersHorizontal, label: "EQ" },
  { id: "search", icon: Search, label: "Search" },
  { id: "settings", icon: Settings, label: "Settings" },
];

function DynamicPill({
  activeTab,
  onTabChange,
  isPlaying,
  onPlayPause,
  currentTrack,
  onScanLibrary,
  onTrackClick,
  fullPlayerOpen = false,
  remainingTime,
  onToggleLyrics,
  onCloseFullPlayer,
}: DynamicPillProps) {
  const hasTrack = !!currentTrack;
  const [hoveredTab, setHoveredTab] = useState<PillTab | null>(null);
  const [dropUpOpen, setDropUpOpen] = useState(false);
  const dropUpRef = useRef<HTMLDivElement>(null);

  // Close drop-up on outside click
  useEffect(() => {
    if (!dropUpOpen) return;
    const handler = (e: MouseEvent) => {
      if (dropUpRef.current && !dropUpRef.current.contains(e.target as Node)) {
        setDropUpOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [dropUpOpen]);

  // ── Ultra-minimalist mode when full player is open ──
  if (fullPlayerOpen) {
    return (
      <motion.div
        layout
        className="liquid-glass fixed bottom-8 left-1/2 z-50 flex -translate-x-1/2 items-center gap-3 rounded-full px-5 py-2.5"
        transition={{ layout: { type: "spring", stiffness: 400, damping: 30 } }}
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
      >
        {remainingTime && (
          <span className="font-mono text-xs text-white/60">{remainingTime}</span>
        )}
        {onToggleLyrics && (
          <motion.button
            type="button"
            whileHover={{ scale: 1.15 }}
            whileTap={{ scale: 0.92 }}
            onClick={onToggleLyrics}
            className="flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-white/70 transition-colors hover:text-white"
            aria-label="Toggle Lyrics"
          >
            <Mic2 size={14} />
          </motion.button>
        )}
        {onCloseFullPlayer && (
          <motion.button
            type="button"
            whileHover={{ scale: 1.15 }}
            whileTap={{ scale: 0.92 }}
            onClick={onCloseFullPlayer}
            className="flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-white/70 transition-colors hover:text-white"
            aria-label="Close player"
          >
            <ChevronDown size={14} />
          </motion.button>
        )}
      </motion.div>
    );
  }

  return (
    <motion.div
      layout
      className="liquid-glass fixed bottom-8 left-1/2 z-50 flex -translate-x-1/2 items-center gap-1.5 rounded-full px-4 py-2.5 md:gap-2 md:px-6 md:py-3.5"
      transition={{ layout: { type: "spring", stiffness: 400, damping: 30 } }}
      animate={hoveredTab ? { scale: 1.03 } : { scale: 1 }}
    >
      {/* Now-playing mini section */}
      <AnimatePresence>
        {hasTrack && (
          <motion.div
            layout
            initial={{ opacity: 0, width: 0 }}
            animate={{ opacity: 1, width: "auto" }}
            exit={{ opacity: 0, width: 0 }}
            className="flex items-center gap-2 overflow-hidden pr-2 mr-1 border-r border-white/10"
          >
            {/* Mini cover art – clickable to open full player */}
            <motion.div
              layoutId="track-art"
              className="h-8 w-8 flex-shrink-0 cursor-pointer overflow-hidden rounded-lg bg-white/10 md:h-10 md:w-10"
              onClick={onTrackClick}
              onKeyDown={(e) => e.key === "Enter" && onTrackClick?.()}
              role="button"
              tabIndex={0}
              aria-label="Open full player"
            >
              {currentTrack.artUrl ? (
                <img
                  src={currentTrack.artUrl}
                  alt="Now playing"
                  className="h-full w-full object-cover"
                />
              ) : (
                <div className="flex h-full w-full items-center justify-center">
                  <svg
                    className="h-4 w-4 text-white/30"
                    fill="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
                  </svg>
                </div>
              )}
            </motion.div>

            {/* Scrolling title – clickable to open full player */}
            <motion.span
              className="max-w-[120px] cursor-pointer truncate text-xs font-medium text-white md:max-w-[160px] md:text-sm"
              title={currentTrack.title}
              onClick={onTrackClick}
              onKeyDown={(e) => e.key === "Enter" && onTrackClick?.()}
              role="button"
              tabIndex={0}
            >
              {currentTrack.title}
            </motion.span>

            {/* Play/Pause */}
            <button
              type="button"
              onClick={onPlayPause}
              className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20 md:h-9 md:w-9"
              aria-label={isPlaying ? "Pause" : "Play"}
            >
              {isPlaying ? (
                <Pause size={14} className="md:h-4 md:w-4" />
              ) : (
                <Play size={14} className="md:h-4 md:w-4" />
              )}
            </button>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Navigation icons */}
      {TABS.map((tab) => {
        const Icon = tab.icon;
        const isActive = activeTab === tab.id;
        const isHovered = hoveredTab === tab.id;
        const isSettings = tab.id === "settings";
        return (
          <div key={tab.id} className="relative" ref={isSettings ? dropUpRef : undefined}>
            <motion.button
              type="button"
              onClick={() => {
                if (isSettings) {
                  setDropUpOpen((p) => !p);
                } else {
                  onTabChange(tab.id);
                  setDropUpOpen(false);
                }
              }}
              onMouseEnter={() => setHoveredTab(tab.id)}
              onMouseLeave={() => setHoveredTab(null)}
              className={`relative flex h-9 w-9 items-center justify-center rounded-full transition-colors md:h-12 md:w-12 ${
                isActive || (isSettings && dropUpOpen)
                  ? "bg-white/15 text-white"
                  : "text-gray-400 hover:text-white"
              }`}
              aria-label={tab.label}
              whileHover={{ scale: 1.15 }}
              whileTap={{ scale: 0.95 }}
            >
              <Icon size={18} className="md:h-5 md:w-5" />
              {/* Hover glow */}
              {isHovered && (
                <motion.div
                  layoutId="pill-glow"
                  className="pointer-events-none absolute inset-0 rounded-full bg-white/10 shadow-[0_0_12px_rgba(34,211,238,0.3)]"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.2 }}
                />
              )}
            </motion.button>

            {/* Settings Drop-Up Menu */}
            {isSettings && (
              <AnimatePresence>
                {dropUpOpen && (
                  <motion.div
                    initial={{ opacity: 0, scaleY: 0 }}
                    animate={{ opacity: 1, scaleY: 1 }}
                    exit={{ opacity: 0, scaleY: 0 }}
                    transition={{ type: "spring", stiffness: 400, damping: 25 }}
                    style={{ originY: 1, transformOrigin: "bottom" }}
                    className="liquid-glass absolute bottom-[120%] right-0 w-48 rounded-xl overflow-hidden"
                  >
                    <button
                      type="button"
                      onClick={() => {
                        onTabChange("settings");
                        setDropUpOpen(false);
                      }}
                      className="flex w-full items-center gap-2.5 px-3 py-2.5 text-sm text-white/80 transition-colors hover:bg-white/10"
                    >
                      <AudioIcon size={15} className="text-cyan-400" />
                      Ajustes de Audio
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        setDropUpOpen(false);
                        onScanLibrary?.();
                      }}
                      className="flex w-full items-center gap-2.5 px-3 py-2.5 text-sm text-white/80 transition-colors hover:bg-white/10"
                    >
                      <FolderSearch size={15} className="text-emerald-400" />
                      Escanear Biblioteca
                    </button>
                    <button
                      type="button"
                      onClick={() => setDropUpOpen(false)}
                      className="flex w-full items-center gap-2.5 px-3 py-2.5 text-sm text-white/80 transition-colors hover:bg-white/10"
                    >
                      <Info size={15} className="text-white/50" />
                      Acerca de
                    </button>
                  </motion.div>
                )}
              </AnimatePresence>
            )}
          </div>
        );
      })}
    </motion.div>
  );
}

export default memo(DynamicPill);
