import { useRef, useEffect, useMemo } from "react";
import { motion } from "framer-motion";

interface LyricsLine {
  timestamp: number;
  text: string;
}

interface LyricsEngineProps {
  lines: LyricsLine[];
  currentTime: number;
  /** Neon color extracted from album art (CSS color string) */
  neonColor?: string;
}

/** Find the active lyric line index based on current playback time. */
function findActiveLine(lines: LyricsLine[], time: number): number {
  let activeLineIndex = 0;
  for (let i = lines.length - 1; i >= 0; i--) {
    if (time >= lines[i].timestamp) {
      activeLineIndex = i;
      break;
    }
  }
  return activeLineIndex;
}

const LINE_HEIGHT_PX = 80;

export default function LyricsEngine({ lines, currentTime, neonColor = "rgba(139,92,246,0.7)" }: LyricsEngineProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const activeIndex = useMemo(() => findActiveLine(lines, currentTime), [lines, currentTime]);

  const neonShadow = useMemo(
    () => `0 0 24px ${neonColor}, 0 0 48px ${neonColor}`,
    [neonColor]
  );

  // Keep active line centered via scrollIntoView as a fallback
  const activeRef = useRef<HTMLParagraphElement>(null);
  useEffect(() => {
    activeRef.current?.scrollIntoView({ behavior: "smooth", block: "center" });
  }, [activeIndex]);

  if (!lines.length) {
    return (
      <div className="flex h-full w-full items-center justify-center text-white/30 text-lg">
        No lyrics available
      </div>
    );
  }

  // Calculate offset to center the active line vertically
  const containerHeight = containerRef.current?.clientHeight ?? 400;
  const offset = -(activeIndex * LINE_HEIGHT_PX) + containerHeight / 2 - LINE_HEIGHT_PX / 2;

  return (
    <div
      ref={containerRef}
      className="relative flex h-full w-full items-start justify-center overflow-hidden"
    >
      <motion.div
        animate={{ y: offset }}
        transition={{ type: "spring", stiffness: 80, damping: 20, mass: 0.8 }}
        className="w-full px-6"
      >
        {lines.map((line, index) => {
          const isActive = index === activeIndex;
          const distance = Math.abs(activeIndex - index);
          return (
            <motion.p
              key={`${line.timestamp}-${index}`}
              ref={isActive ? activeRef : undefined}
              animate={{
                opacity: isActive ? 1 : distance <= 1 ? 0.35 : 0.25,
                scale: isActive ? 1.05 : 0.95,
                filter: isActive ? "blur(0px)" : "blur(1px)",
              }}
              transition={{ duration: 0.3, ease: "easeOut" }}
              className="text-center text-4xl font-black leading-tight text-white md:text-6xl"
              style={{
                height: LINE_HEIGHT_PX,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                textShadow: isActive ? neonShadow : "none",
              }}
            >
              {line.text || "♪"}
            </motion.p>
          );
        })}
      </motion.div>
    </div>
  );
}
