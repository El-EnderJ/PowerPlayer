import { useState, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";

interface FluidBackgroundProps {
  albumArt?: string;
}

/** Extract 3 dominant colours from an image using a small canvas sample. */
function extractColors(src: string): Promise<string[]> {
  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      const size = 64;
      const canvas = document.createElement("canvas");
      canvas.width = size;
      canvas.height = size;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        resolve(["#581c87", "#0e7490", "#1e1b4b"]);
        return;
      }
      ctx.drawImage(img, 0, 0, size, size);
      const data = ctx.getImageData(0, 0, size, size).data;

      // Simple k-bucket quantisation into 3 colour clusters
      const buckets: { r: number; g: number; b: number; count: number }[] = [
        { r: 0, g: 0, b: 0, count: 0 },
        { r: 0, g: 0, b: 0, count: 0 },
        { r: 0, g: 0, b: 0, count: 0 },
      ];

      for (let i = 0; i < data.length; i += 4) {
        const r = data[i];
        const g = data[i + 1];
        const b = data[i + 2];
        const lum = r * 0.299 + g * 0.587 + b * 0.114;
        // Assign to bucket by luminance
        const idx = lum < 85 ? 0 : lum < 170 ? 1 : 2;
        buckets[idx].r += r;
        buckets[idx].g += g;
        buckets[idx].b += b;
        buckets[idx].count += 1;
      }

      const colors = buckets.map((b) => {
        if (b.count === 0) return "rgba(30,20,60,0.3)";
        const r = Math.round(b.r / b.count);
        const g = Math.round(b.g / b.count);
        const bl = Math.round(b.b / b.count);
        return `rgba(${r},${g},${bl},0.3)`;
      });
      resolve(colors);
    };
    img.onerror = () => resolve(["rgba(88,28,135,0.3)", "rgba(14,116,144,0.3)", "rgba(30,27,75,0.3)"]);
    img.src = src;
  });
}

export default function FluidBackground({ albumArt }: FluidBackgroundProps) {
  const [colors, setColors] = useState<string[]>([
    "rgba(88,28,135,0.3)",
    "rgba(14,116,144,0.3)",
    "rgba(30,27,75,0.3)",
  ]);
  const prevArtRef = useRef<string | undefined>(undefined);

  useEffect(() => {
    if (albumArt && albumArt !== prevArtRef.current) {
      prevArtRef.current = albumArt;
      extractColors(albumArt).then(setColors);
    } else if (!albumArt) {
      prevArtRef.current = undefined;
      setColors(["rgba(88,28,135,0.3)", "rgba(14,116,144,0.3)", "rgba(30,27,75,0.3)"]);
    }
  }, [albumArt]);

  return (
    <div className="pointer-events-none fixed inset-0 -z-10 overflow-hidden bg-black">
      <AnimatePresence mode="wait">
        <motion.div
          key={albumArt ?? "default"}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 1.2, ease: "easeOut" }}
          className="absolute inset-0"
          style={{ filter: "blur(120px)" }}
        >
          {/* Colour blob 1 */}
          <motion.div
            animate={{
              x: ["0%", "15%", "-10%", "0%"],
              y: ["0%", "-20%", "10%", "0%"],
              scale: [1, 1.2, 0.9, 1],
            }}
            transition={{ duration: 18, repeat: Infinity, ease: "linear" }}
            className="absolute left-[10%] top-[15%] h-[60%] w-[60%] rounded-full"
            style={{ background: `radial-gradient(circle, ${colors[0]} 0%, transparent 70%)` }}
          />
          {/* Colour blob 2 */}
          <motion.div
            animate={{
              x: ["0%", "-20%", "10%", "0%"],
              y: ["0%", "15%", "-15%", "0%"],
              scale: [1, 0.95, 1.15, 1],
            }}
            transition={{ duration: 22, repeat: Infinity, ease: "linear" }}
            className="absolute right-[5%] top-[20%] h-[55%] w-[55%] rounded-full"
            style={{ background: `radial-gradient(circle, ${colors[1]} 0%, transparent 70%)` }}
          />
          {/* Colour blob 3 */}
          <motion.div
            animate={{
              x: ["0%", "12%", "-8%", "0%"],
              y: ["0%", "-10%", "20%", "0%"],
              scale: [1, 1.1, 0.95, 1],
            }}
            transition={{ duration: 25, repeat: Infinity, ease: "linear" }}
            className="absolute bottom-[10%] left-[25%] h-[50%] w-[50%] rounded-full"
            style={{ background: `radial-gradient(circle, ${colors[2]} 0%, transparent 70%)` }}
          />
        </motion.div>
      </AnimatePresence>

      {/* Overlay vignette */}
      <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-black/40" />
    </div>
  );
}
