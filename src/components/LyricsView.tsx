import { motion } from "framer-motion";
import type { ReactNode } from "react";

interface LyricsLine {
  timestamp: number;
  text: string;
}

interface LyricsViewProps {
  lines: LyricsLine[];
  activeIndex: number;
  fallback?: ReactNode;
}

const LINE_HEIGHT = 52;

export default function LyricsView({ lines, activeIndex, fallback }: LyricsViewProps) {
  if (!lines.length) {
    return (
      <div className="w-full max-w-4xl rounded-2xl border border-white/10 bg-black/20 p-6 backdrop-blur-xl">
        <div className="mb-3 text-center text-sm tracking-wide text-white/60">
          No lyrics available · Expanded Spectrum Mode
        </div>
        {fallback}
      </div>
    );
  }

  const offset = -(activeIndex * LINE_HEIGHT) + LINE_HEIGHT * 2;

  return (
    <div className="relative h-[52vh] w-full max-w-4xl overflow-hidden rounded-2xl border border-white/10 bg-black/30 p-6 backdrop-blur-xl">
      <motion.div
        animate={{ y: offset }}
        transition={{ type: "spring", stiffness: 130, damping: 24 }}
        className="space-y-2"
      >
        {lines.map((line, index) => {
          const distance = Math.abs(activeIndex - index);
          const isActive = index === activeIndex;
          return (
            <motion.p
              key={`${line.timestamp}-${index}`}
              layout
              animate={{
                opacity: isActive ? 1 : distance > 2 ? 0.2 : 0.45,
                scale: isActive ? 1.08 : 1,
                filter: isActive ? "blur(0px)" : distance > 2 ? "blur(2px)" : "blur(1px)",
              }}
              transition={{ duration: 0.28 }}
              className="h-[52px] text-center text-xl font-medium text-white/90"
              style={
                isActive
                  ? { color: "#fff", textShadow: "0 0 20px rgba(255,255,255,0.35)" }
                  : undefined
              }
            >
              {line.text || "♪"}
            </motion.p>
          );
        })}
      </motion.div>
    </div>
  );
}
