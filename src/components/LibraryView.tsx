import { useState, useEffect, useRef, useCallback, memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Shuffle, Play, Pause, Search, CheckSquare } from "lucide-react";
import TrackBubble from "./TrackBubble";
import AlphabetIndex from "./AlphabetIndex";

interface LibraryTrack {
  path: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  duration_seconds: number | null;
  sample_rate: number | null;
  art_url: string | null;
  corrupted: boolean;
}

type Tab = "all" | "albums" | "artists" | "genres";

const TABS: { id: Tab; label: string }[] = [
  { id: "all", label: "Todas" },
  { id: "albums", label: "Álbumes" },
  { id: "artists", label: "Artistas" },
  { id: "genres", label: "Géneros" },
];

interface LibraryViewProps {
  isPlaying: boolean;
  onPlayPause: () => void;
  onTrackSelect: (path: string) => void;
}

function LibraryView({ isPlaying, onPlayPause, onTrackSelect }: LibraryViewProps) {
  const [tracks, setTracks] = useState<LibraryTrack[]>([]);
  const [activeTab, setActiveTab] = useState<Tab>("all");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<LibraryTrack[]>("get_library_tracks")
      .then((data) => setTracks(data))
      .catch(() => {});
  }, []);

  const scrollToTop = useCallback(() => {
    scrollRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  }, []);

  const scrollToLetter = useCallback(
    (letter: string) => {
      const target = tracks.findIndex((t) => {
        const first = (t.title ?? t.path).trim().charAt(0).toUpperCase();
        if (letter === "#") return !/[A-Z]/.test(first);
        return first === letter;
      });
      if (target >= 0) {
        const el = scrollRef.current?.querySelector(`[data-index="${target}"]`);
        el?.scrollIntoView({ behavior: "smooth", block: "start" });
      }
    },
    [tracks]
  );

  return (
    <div className="relative flex h-full w-full flex-col overflow-hidden">
      {/* Sticky Header */}
      <div className="sticky top-0 z-30 mx-4 mt-4 mb-2 flex items-center gap-2 rounded-2xl border border-white/5 bg-black/30 px-4 py-3 shadow-2xl backdrop-blur-2xl">
        <button
          type="button"
          className="flex items-center gap-1 rounded-xl px-3 py-1.5 text-xs text-gray-300 transition-colors hover:bg-white/10"
          aria-label="Shuffle"
        >
          <Shuffle size={16} />
          <span className="hidden sm:inline">Mezclar</span>
        </button>
        <button
          type="button"
          onClick={onPlayPause}
          className="flex items-center gap-1 rounded-xl px-3 py-1.5 text-xs text-gray-300 transition-colors hover:bg-white/10"
          aria-label={isPlaying ? "Pause" : "Play"}
        >
          {isPlaying ? <Pause size={16} /> : <Play size={16} />}
        </button>
        <button
          type="button"
          className="flex items-center gap-1 rounded-xl px-3 py-1.5 text-xs text-gray-300 transition-colors hover:bg-white/10"
          aria-label="Search"
        >
          <Search size={16} />
        </button>
        <button
          type="button"
          className="flex items-center gap-1 rounded-xl px-3 py-1.5 text-xs text-gray-300 transition-colors hover:bg-white/10"
          aria-label="Select"
        >
          <CheckSquare size={16} />
        </button>

        {/* Spacer */}
        <div className="flex-1" />

        {/* Tabs */}
        <div className="flex items-center gap-1">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id)}
              className={`rounded-lg px-3 py-1 text-xs font-medium transition-colors ${
                activeTab === tab.id
                  ? "bg-white/15 text-white"
                  : "text-gray-500 hover:text-gray-300"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Track list */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-4 pb-32 scrollbar-hide"
      >
        {tracks.length === 0 ? (
          <div className="flex h-64 items-center justify-center text-gray-500 text-sm">
            No tracks in library. Use the backend to scan a folder.
          </div>
        ) : (
          tracks.map((track, index) => (
            <div key={track.path} data-index={index}>
              <TrackBubble
                title={track.title ?? "Unknown Title"}
                artist={track.artist ?? "Unknown Artist"}
                album={track.album ?? "Unknown Album"}
                durationSeconds={track.duration_seconds ?? 0}
                sampleRate={track.sample_rate ?? undefined}
                artUrl={track.art_url ?? undefined}
                path={track.path}
                onClick={() => onTrackSelect(track.path)}
              />
            </div>
          ))
        )}
      </div>

      {/* Alphabet Index */}
      <AlphabetIndex onLetterClick={scrollToLetter} onScrollToTop={scrollToTop} />
    </div>
  );
}

export default memo(LibraryView);
