import { memo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  LayoutGrid,
  SlidersHorizontal,
  Search,
  Settings,
  Play,
  Pause,
  FolderOpen,
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
  libraryEmpty?: boolean;
  onSelectLibrary?: () => void;
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
  libraryEmpty = false,
  onSelectLibrary,
}: DynamicPillProps) {
  const hasTrack = !!currentTrack;
  const [hoveredTab, setHoveredTab] = useState<PillTab | null>(null);

  return (
    <motion.div
      layout
      className="fixed bottom-8 left-1/2 z-50 flex -translate-x-1/2 items-center gap-1.5 rounded-full border-t border-white/10 border-b-black/40 bg-white/5 px-4 py-2.5 shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%] md:gap-2 md:px-6 md:py-3.5"
      transition={{ layout: { type: "spring", stiffness: 400, damping: 30 } }}
      animate={hoveredTab ? { scale: 1.03 } : { scale: 1 }}
    >
      {/* Library empty state button */}
      <AnimatePresence>
        {libraryEmpty && onSelectLibrary && (
          <motion.button
            type="button"
            initial={{ opacity: 0, width: 0 }}
            animate={{ opacity: 1, width: "auto" }}
            exit={{ opacity: 0, width: 0 }}
            onClick={onSelectLibrary}
            className="mr-1 flex items-center gap-2 overflow-hidden rounded-full border border-cyan-500/30 bg-cyan-500/10 px-3 py-1.5 text-xs font-medium text-cyan-300 transition-colors hover:bg-cyan-500/20 md:px-4 md:py-2 md:text-sm"
          >
            <FolderOpen size={14} className="md:h-4 md:w-4" />
            <span className="whitespace-nowrap">📂 Seleccionar Biblioteca</span>
          </motion.button>
        )}
      </AnimatePresence>

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
            {/* Mini cover art */}
            <div className="h-8 w-8 flex-shrink-0 overflow-hidden rounded-lg bg-white/10 md:h-10 md:w-10">
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
            </div>

            {/* Scrolling title */}
            <motion.span
              className="max-w-[120px] truncate text-xs font-medium text-white md:max-w-[160px] md:text-sm"
              title={currentTrack.title}
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
        return (
          <motion.button
            key={tab.id}
            type="button"
            onClick={() => onTabChange(tab.id)}
            onMouseEnter={() => setHoveredTab(tab.id)}
            onMouseLeave={() => setHoveredTab(null)}
            className={`relative flex h-9 w-9 items-center justify-center rounded-full transition-colors md:h-12 md:w-12 ${
              isActive
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
        );
      })}
    </motion.div>
  );
}

export default memo(DynamicPill);
