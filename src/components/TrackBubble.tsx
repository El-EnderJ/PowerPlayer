import { memo } from "react";

export interface TrackBubbleProps {
  title: string;
  artist: string;
  album: string;
  durationSeconds: number;
  sampleRate?: number;
  artUrl?: string;
  path: string;
  highlightQuery?: string;
  onClick?: () => void;
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatFromPath(path: string): string {
  const ext = path.split(".").pop()?.toUpperCase() ?? "";
  return ext === "FLAC" || ext === "WAV" || ext === "MP3" ? ext : "AUDIO";
}

function formatSampleRate(rate?: number): string {
  if (!rate) return "";
  return rate >= 1000 ? `${(rate / 1000).toFixed(1)}kHz` : `${rate}Hz`;
}

/* ── Highlight helper ───────────────────────────────────────────────── */

function Highlighted({ text, query }: { text: string; query?: string }) {
  if (!query?.trim()) return <>{text}</>;

  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const regex = new RegExp(`(${escaped})`, "gi");
  const parts = text.split(regex);
  const lowerQuery = query.toLowerCase();

  return (
    <>
      {parts.map((part, i) =>
        part.toLowerCase() === lowerQuery ? (
          <span key={i} className="text-cyan-300">
            {part}
          </span>
        ) : (
          <span key={i}>{part}</span>
        )
      )}
    </>
  );
}

function TrackBubble({
  title,
  artist,
  album,
  durationSeconds,
  sampleRate,
  artUrl,
  path,
  highlightQuery,
  onClick,
}: TrackBubbleProps) {
  const format = formatFromPath(path);
  const sr = formatSampleRate(sampleRate);
  const dur = formatDuration(durationSeconds);
  const techParts = [sr, format].filter(Boolean).join(" • ");

  return (
    <button
      type="button"
      onClick={onClick}
      className="flex w-full flex-row items-center gap-4 rounded-2xl border border-white/5 bg-black/30 p-3 mb-2 shadow-2xl backdrop-blur-2xl text-left transition-colors hover:bg-white/5"
    >
      {/* Cover art */}
      <div className="h-16 w-16 flex-shrink-0 overflow-hidden rounded-xl bg-white/5 shadow-lg">
        {artUrl ? (
          <img
            src={artUrl}
            alt={`${album} cover`}
            className="h-full w-full object-cover"
            loading="lazy"
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center">
            <svg
              className="h-6 w-6 text-white/20"
              fill="currentColor"
              viewBox="0 0 24 24"
            >
              <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
            </svg>
          </div>
        )}
      </div>

      {/* Track info */}
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-lg font-bold text-white">
          <Highlighted text={title} query={highlightQuery} />
        </span>
        <span className="truncate text-sm text-gray-400">
          <Highlighted text={`${artist} – ${album}`} query={highlightQuery} />
        </span>
        <div className="mt-1 flex items-center gap-2">
          <span className="text-[11px] text-white/40">{dur}</span>
          {techParts && (
            <span className="rounded-md bg-white/5 px-1.5 py-0.5 text-[10px] font-mono text-emerald-400/80">
              {techParts}
            </span>
          )}
        </div>
      </div>
    </button>
  );
}

export default memo(TrackBubble);
