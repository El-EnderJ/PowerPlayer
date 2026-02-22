import { useState, useEffect, useRef, useCallback, memo, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Search, FolderOpen } from "lucide-react";
import TrackBubble from "./TrackBubble";

/* ── Types ─────────────────────────────────────────────────────────── */

interface SearchResultTrack {
  id: number;
  path: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  duration_seconds: number | null;
  sample_rate: number | null;
  art_url: string | null;
}

interface SearchResults {
  tracks: SearchResultTrack[];
  albums: string[];
  artists: string[];
}

type FilterChip = "all" | "songs" | "albums" | "artists";

const FILTER_CHIPS: { id: FilterChip; label: string }[] = [
  { id: "all", label: "Todo" },
  { id: "songs", label: "Canciones" },
  { id: "albums", label: "Álbumes" },
  { id: "artists", label: "Artistas" },
];

/* ── Highlighting helper ───────────────────────────────────────────── */

function HighlightedText({
  text,
  query,
  className,
}: {
  text: string;
  query: string;
  className?: string;
}) {
  if (!query.trim()) {
    return <span className={className}>{text}</span>;
  }

  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const regex = new RegExp(`(${escaped})`, "gi");
  const parts = text.split(regex);

  return (
    <span className={className}>
      {parts.map((part, i) =>
        regex.test(part) ? (
          <span key={i} className="text-cyan-300">
            {part}
          </span>
        ) : (
          <span key={i}>{part}</span>
        )
      )}
    </span>
  );
}

/* ── Grouping logic ────────────────────────────────────────────────── */

interface AlbumGroup {
  type: "album";
  album: string;
  artist: string;
  tracks: SearchResultTrack[];
}

interface ArtistGroup {
  type: "artist";
  artist: string;
  tracks: SearchResultTrack[];
}

interface FreeGroup {
  type: "free";
  tracks: SearchResultTrack[];
}

type ResultGroup = AlbumGroup | ArtistGroup | FreeGroup;

function groupResults(
  tracks: SearchResultTrack[],
  matchedAlbums: string[],
  matchedArtists: string[]
): ResultGroup[] {
  const groups: ResultGroup[] = [];
  const albumSet = new Set(matchedAlbums);
  const artistSet = new Set(matchedArtists);
  const used = new Set<number>();

  // 1. Group by matching album
  const albumMap = new Map<string, SearchResultTrack[]>();
  for (const track of tracks) {
    if (track.album && albumSet.has(track.album)) {
      const key = `${track.album}|||${track.artist ?? ""}`;
      if (!albumMap.has(key)) albumMap.set(key, []);
      albumMap.get(key)!.push(track);
      used.add(track.id);
    }
  }
  for (const [key, albumTracks] of albumMap) {
    const [album, artist] = key.split("|||");
    groups.push({ type: "album", album, artist, tracks: albumTracks });
  }

  // 2. Group remaining by matching artist
  const artistMap = new Map<string, SearchResultTrack[]>();
  for (const track of tracks) {
    if (!used.has(track.id) && track.artist && artistSet.has(track.artist)) {
      if (!artistMap.has(track.artist)) artistMap.set(track.artist, []);
      artistMap.get(track.artist)!.push(track);
      used.add(track.id);
    }
  }
  for (const [artist, artistTracks] of artistMap) {
    groups.push({ type: "artist", artist, tracks: artistTracks });
  }

  // 3. Free results
  const free = tracks.filter((t) => !used.has(t.id));
  if (free.length > 0) {
    groups.push({ type: "free", tracks: free });
  }

  return groups;
}

/* ── Props ─────────────────────────────────────────────────────────── */

interface SearchViewProps {
  onTrackSelect: (path: string) => void;
  onSelectLibrary?: () => void;
  libraryEmpty?: boolean;
}

/* ── Component ─────────────────────────────────────────────────────── */

function SearchView({
  onTrackSelect,
  onSelectLibrary,
  libraryEmpty = false,
}: SearchViewProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResults | null>(null);
  const [activeFilter, setActiveFilter] = useState<FilterChip>("all");
  const [isSearching, setIsSearching] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Debounced search
  const doSearch = useCallback((q: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);

    const trimmed = q.trim();
    if (!trimmed) {
      setResults(null);
      setIsSearching(false);
      return;
    }

    setIsSearching(true);
    debounceRef.current = setTimeout(() => {
      invoke<SearchResults>("fast_search", { query: trimmed })
        .then((res) => setResults(res))
        .catch(() => setResults({ tracks: [], albums: [], artists: [] }))
        .finally(() => setIsSearching(false));
    }, 150);
  }, []);

  const handleInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const val = e.target.value;
      setQuery(val);
      doSearch(val);
    },
    [doSearch]
  );

  // Cleanup debounce on unmount
  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  // Filter results based on active chip
  const filteredResults = useMemo<SearchResults | null>(() => {
    if (!results) return null;
    if (activeFilter === "all") return results;

    if (activeFilter === "songs") {
      return { ...results, albums: [], artists: [] };
    }
    if (activeFilter === "albums") {
      // Only show tracks that belong to matched albums
      const albumSet = new Set(results.albums);
      return {
        tracks: results.tracks.filter((t) => t.album && albumSet.has(t.album)),
        albums: results.albums,
        artists: [],
      };
    }
    if (activeFilter === "artists") {
      const artistSet = new Set(results.artists);
      return {
        tracks: results.tracks.filter(
          (t) => t.artist && artistSet.has(t.artist)
        ),
        albums: [],
        artists: results.artists,
      };
    }
    return results;
  }, [results, activeFilter]);

  // Grouped results
  const groups = useMemo<ResultGroup[]>(() => {
    if (!filteredResults) return [];
    return groupResults(
      filteredResults.tracks,
      filteredResults.albums,
      filteredResults.artists
    );
  }, [filteredResults]);

  const hasResults =
    filteredResults &&
    (filteredResults.tracks.length > 0 ||
      filteredResults.albums.length > 0 ||
      filteredResults.artists.length > 0);

  const showEmptyLibrary =
    libraryEmpty && !query.trim() && !hasResults;

  const showNoResults =
    query.trim().length > 0 && !isSearching && !hasResults;

  return (
    <div className="relative flex h-full w-full flex-col overflow-hidden">
      {/* Search bar */}
      <div className="mx-4 mt-4 mb-3">
        <div className="flex items-center gap-3 rounded-2xl border-t border-white/10 border-b-black/40 bg-white/5 px-5 py-3.5 shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%]">
          <Search
            size={20}
            className={`flex-shrink-0 transition-colors ${
              query ? "text-cyan-400 drop-shadow-[0_0_6px_rgba(34,211,238,0.5)]" : "text-white/30"
            }`}
          />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={handleInput}
            placeholder="Buscar canciones, álbumes, artistas..."
            className="flex-1 bg-transparent text-lg text-white placeholder-white/20 outline-none"
          />
          {query && (
            <button
              type="button"
              onClick={() => {
                setQuery("");
                setResults(null);
                inputRef.current?.focus();
              }}
              className="text-sm text-white/30 transition-colors hover:text-white/60"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* Filter chips */}
      <div className="mx-4 mb-3 flex items-center gap-2">
        {FILTER_CHIPS.map((chip) => {
          const isActive = activeFilter === chip.id;
          return (
            <button
              key={chip.id}
              type="button"
              onClick={() => setActiveFilter(chip.id)}
              className={`rounded-full px-4 py-1.5 text-xs font-medium transition-all ${
                isActive
                  ? "border border-cyan-500/40 bg-cyan-500/15 text-cyan-300 shadow-[0_0_12px_rgba(34,211,238,0.15)]"
                  : "border border-white/5 bg-white/5 text-gray-400 backdrop-blur-xl hover:bg-white/10 hover:text-gray-200"
              }`}
            >
              {chip.label}
            </button>
          );
        })}
      </div>

      {/* Results area */}
      <div className="flex-1 overflow-y-auto px-4 pb-32 scrollbar-hide">
        {/* Empty library – folder selector */}
        {showEmptyLibrary && (
          <div className="flex h-64 flex-col items-center justify-center gap-4">
            <div className="rounded-2xl border-t border-white/10 border-b-black/40 bg-white/5 px-8 py-8 text-center shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%]">
              <div className="mb-4 text-5xl">🎵</div>
              <p className="mb-5 text-sm text-white/50">
                No hay biblioteca configurada
              </p>
              {onSelectLibrary && (
                <button
                  type="button"
                  onClick={onSelectLibrary}
                  className="flex items-center gap-2 rounded-2xl border border-cyan-500/30 bg-cyan-500/10 px-8 py-4 text-base font-medium text-cyan-300 transition-colors hover:bg-cyan-500/20"
                >
                  <FolderOpen size={22} />
                  📂 Seleccionar Carpeta de Música
                </button>
              )}
            </div>
          </div>
        )}

        {/* No results found */}
        {showNoResults && (
          <div className="flex h-48 items-center justify-center">
            <div className="text-center">
              <div className="mb-2 text-3xl">🔍</div>
              <p className="text-sm text-white/40">
                Sin resultados para &quot;{query}&quot;
              </p>
              {libraryEmpty && onSelectLibrary && (
                <button
                  type="button"
                  onClick={onSelectLibrary}
                  className="mt-4 flex items-center gap-2 rounded-2xl border border-cyan-500/30 bg-cyan-500/10 px-6 py-3 text-sm font-medium text-cyan-300 transition-colors hover:bg-cyan-500/20"
                >
                  <FolderOpen size={18} />
                  📂 Seleccionar Carpeta de Música
                </button>
              )}
            </div>
          </div>
        )}

        {/* Searching indicator */}
        {isSearching && !hasResults && (
          <div className="flex h-32 items-center justify-center">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-white/10 border-t-cyan-400" />
          </div>
        )}

        {/* Grouped results */}
        <AnimatePresence mode="sync">
          {groups.map((group, gi) => (
            <div key={`group-${gi}`}>
              {/* Group header */}
              {group.type === "album" && (
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: gi * 0.05 }}
                  className="sticky top-0 z-10 mb-2 mt-4 rounded-xl border-t border-white/10 border-b-black/40 bg-white/5 px-4 py-2.5 backdrop-blur-[40px] saturate-[180%]"
                >
                  <HighlightedText
                    text={`${group.album} – ${group.artist}`}
                    query={query}
                    className="text-sm font-semibold text-white/80"
                  />
                </motion.div>
              )}
              {group.type === "artist" && (
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: gi * 0.05 }}
                  className="sticky top-0 z-10 mb-2 mt-4 rounded-xl border-t border-white/10 border-b-black/40 bg-white/5 px-4 py-2.5 backdrop-blur-[40px] saturate-[180%]"
                >
                  <HighlightedText
                    text={group.artist}
                    query={query}
                    className="text-sm font-semibold text-white/80"
                  />
                </motion.div>
              )}

              {/* Tracks */}
              {group.tracks.map((track, ti) => (
                <motion.div
                  key={track.id}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: gi * 0.05 + ti * 0.03 }}
                >
                  <TrackBubble
                    title={track.title ?? "Unknown Title"}
                    artist={track.artist ?? "Unknown Artist"}
                    album={track.album ?? "Unknown Album"}
                    durationSeconds={track.duration_seconds ?? 0}
                    sampleRate={track.sample_rate ?? undefined}
                    artUrl={track.art_url ?? undefined}
                    path={track.path}
                    highlightQuery={query}
                    onClick={() => onTrackSelect(track.path)}
                  />
                </motion.div>
              ))}
            </div>
          ))}
        </AnimatePresence>
      </div>
    </div>
  );
}

export default memo(SearchView);
