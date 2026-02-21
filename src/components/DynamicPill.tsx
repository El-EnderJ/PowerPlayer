import { memo } from "react";
import { motion } from "framer-motion";
import {
  LayoutGrid,
  SlidersHorizontal,
  Search,
  Settings,
  Play,
  Pause,
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
}: DynamicPillProps) {
  const hasTrack = !!currentTrack;

  return (
    <motion.div
      layout
      className="fixed bottom-8 left-1/2 z-50 flex -translate-x-1/2 items-center gap-1 rounded-full border border-white/5 bg-black/30 px-3 py-2 shadow-2xl backdrop-blur-2xl"
      transition={{ layout: { type: "spring", stiffness: 400, damping: 30 } }}
    >
      {/* Now-playing mini section */}
      {hasTrack && (
        <motion.div
          layout
          initial={{ opacity: 0, width: 0 }}
          animate={{ opacity: 1, width: "auto" }}
          exit={{ opacity: 0, width: 0 }}
          className="flex items-center gap-2 overflow-hidden pr-2 mr-1 border-r border-white/10"
        >
          {/* Mini cover art */}
          <div className="h-8 w-8 flex-shrink-0 overflow-hidden rounded-lg bg-white/10">
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
            className="max-w-[120px] truncate text-xs font-medium text-white"
            title={currentTrack.title}
          >
            {currentTrack.title}
          </motion.span>

          {/* Play/Pause */}
          <button
            type="button"
            onClick={onPlayPause}
            className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20"
            aria-label={isPlaying ? "Pause" : "Play"}
          >
            {isPlaying ? <Pause size={14} /> : <Play size={14} />}
          </button>
        </motion.div>
      )}

      {/* Navigation icons */}
      {TABS.map((tab) => {
        const Icon = tab.icon;
        const isActive = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            type="button"
            onClick={() => onTabChange(tab.id)}
            className={`relative flex h-9 w-9 items-center justify-center rounded-full transition-colors ${
              isActive
                ? "bg-white/15 text-white"
                : "text-gray-400 hover:text-white"
            }`}
            aria-label={tab.label}
          >
            <Icon size={18} />
          </button>
        );
      })}
    </motion.div>
  );
}

export default memo(DynamicPill);
