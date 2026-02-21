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
  onSelectLibrary?: () => void;
}

function LibraryView({ isPlaying, onPlayPause, onTrackSelect, onSelectLibrary }: LibraryViewProps) {
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
        const name = t.title ?? t.path.split(/[\\/]/).pop() ?? "";
        const first = name.trim().charAt(0).toUpperCase();
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
      <div className="sticky top-0 z-30 mx-4 mt-4 mb-2 flex items-center gap-2 rounded-2xl border-t border-white/10 border-b-black/40 bg-white/5 px-4 py-3 shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%]">
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
          <div className="flex h-64 flex-col items-center justify-center gap-4">
            <div className="rounded-2xl border-t border-white/10 border-b-black/40 bg-white/5 px-8 py-6 text-center shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%]">
              <div className="mb-3 text-4xl">🎵</div>
              <p className="mb-4 text-sm text-white/50">
                Tu biblioteca está vacía
              </p>
              {onSelectLibrary ? (
                <button
                  type="button"
                  onClick={onSelectLibrary}
                  className="rounded-xl border border-cyan-500/30 bg-cyan-500/10 px-6 py-2.5 text-sm font-medium text-cyan-300 transition-colors hover:bg-cyan-500/20"
                >
                  📂 Configurar Biblioteca
                </button>
              ) : (
                <p className="text-xs text-gray-500">
                  Use the backend to scan a folder.
                </p>
              )}
            </div>
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
