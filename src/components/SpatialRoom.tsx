import { useState, useCallback, useRef, useEffect, useMemo } from "react";
import { motion } from "framer-motion";
import { useAudioIPC } from "../hooks/useAudioIPC";

interface StemSource {
  id: number;
  name: string;
  x: number;
  y: number;
  z: number;
}

const STEM_COLORS: Record<string, string> = {
  vocals: "rgba(139,92,246,0.9)",
  drums: "rgba(234,88,12,0.9)",
  bass: "rgba(34,211,238,0.9)",
  other: "rgba(16,185,129,0.9)",
};

const STEM_LABELS: Record<string, string> = {
  vocals: "Vocales",
  drums: "Batería",
  bass: "Bajo",
  other: "Otros",
};

const ROOM_SIZE = 280;
const SPHERE_SIZE = 52;

/** Map 3D coordinates (-1..1) to 2D canvas position. */
function to2D(x: number, z: number): { left: number; top: number } {
  return {
    left: ((x + 1) / 2) * (ROOM_SIZE - SPHERE_SIZE),
    top: ((z + 1) / 2) * (ROOM_SIZE - SPHERE_SIZE),
  };
}

/** Map 2D canvas position back to 3D coordinates (-1..1). */
function to3D(left: number, top: number): { x: number; z: number } {
  return {
    x: (left / (ROOM_SIZE - SPHERE_SIZE)) * 2 - 1,
    z: (top / (ROOM_SIZE - SPHERE_SIZE)) * 2 - 1,
  };
}

export default function SpatialRoom() {
  const { invokeSafe } = useAudioIPC();
  const [sources, setSources] = useState<StemSource[]>([]);
  const [dragging, setDragging] = useState<number | null>(null);
  const roomRef = useRef<HTMLDivElement>(null);

  // Load initial source positions
  useEffect(() => {
    invokeSafe<StemSource[]>("get_spatial_sources")
      .then((data) => {
        if (Array.isArray(data) && data.length) {
          setSources(data);
        } else {
          // Default positions if none from backend
          setSources([
            { id: 0, name: "vocals", x: 0, y: 0, z: -0.5 },
            { id: 1, name: "drums", x: -0.5, y: 0, z: 0.3 },
            { id: 2, name: "bass", x: 0.5, y: 0, z: 0.3 },
            { id: 3, name: "other", x: 0, y: 0, z: 0.6 },
          ]);
        }
      })
      .catch(() => {
        setSources([
          { id: 0, name: "vocals", x: 0, y: 0, z: -0.5 },
          { id: 1, name: "drums", x: -0.5, y: 0, z: 0.3 },
          { id: 2, name: "bass", x: 0.5, y: 0, z: 0.3 },
          { id: 3, name: "other", x: 0, y: 0, z: 0.6 },
        ]);
      });
  }, [invokeSafe]);

  const handleDragEnd = useCallback(
    (sourceId: number, newX: number, newZ: number) => {
      const y = sources.find((s) => s.id === sourceId)?.y ?? 0;
      setSources((prev) =>
        prev.map((s) => (s.id === sourceId ? { ...s, x: newX, z: newZ } : s))
      );
      setDragging(null);
      void invokeSafe("update_source_position", {
        sourceId,
        x: newX,
        y,
        z: newZ,
      }).catch((err) => {
        console.error("Failed to update source position", err);
      });
    },
    [invokeSafe, sources]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (dragging === null || !roomRef.current) return;
      const rect = roomRef.current.getBoundingClientRect();
      const left = Math.max(0, Math.min(ROOM_SIZE - SPHERE_SIZE, e.clientX - rect.left - SPHERE_SIZE / 2));
      const top = Math.max(0, Math.min(ROOM_SIZE - SPHERE_SIZE, e.clientY - rect.top - SPHERE_SIZE / 2));
      const { x, z } = to3D(left, top);
      setSources((prev) =>
        prev.map((s) => (s.id === dragging ? { ...s, x, z } : s))
      );
    },
    [dragging]
  );

  const handlePointerUp = useCallback(() => {
    if (dragging === null) return;
    const src = sources.find((s) => s.id === dragging);
    if (src) {
      handleDragEnd(src.id, src.x, src.z);
    }
  }, [dragging, sources, handleDragEnd]);

  // Grid lines for spatial feel
  const gridLines = useMemo(() => {
    const lines: JSX.Element[] = [];
    for (let i = 1; i < 4; i++) {
      const pos = (i / 4) * 100;
      lines.push(
        <div key={`h-${i}`} className="absolute left-0 right-0 border-t border-white/5" style={{ top: `${pos}%` }} />,
        <div key={`v-${i}`} className="absolute bottom-0 top-0 border-l border-white/5" style={{ left: `${pos}%` }} />
      );
    }
    return lines;
  }, []);

  return (
    <div className="flex flex-col items-center gap-3">
      <div
        ref={roomRef}
        className="relative rounded-2xl border border-white/10 bg-black/30 backdrop-blur-md"
        style={{ width: ROOM_SIZE, height: ROOM_SIZE, perspective: 600 }}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
      >
        {/* Grid overlay */}
        {gridLines}

        {/* Listener position indicator (center) */}
        <div
          className="absolute rounded-full border border-white/20 bg-white/5"
          style={{
            width: 12,
            height: 12,
            left: ROOM_SIZE / 2 - 6,
            top: ROOM_SIZE / 2 - 6,
          }}
        />

        {/* Stem spheres */}
        {sources.map((src) => {
          const pos = to2D(src.x, src.z);
          const color = STEM_COLORS[src.name] ?? "rgba(255,255,255,0.6)";
          const label = STEM_LABELS[src.name] ?? src.name;
          const isDragging = dragging === src.id;
          return (
            <motion.div
              key={src.id}
              animate={{
                left: pos.left,
                top: pos.top,
                scale: isDragging ? 1.15 : 1,
              }}
              transition={isDragging ? { duration: 0 } : { type: "spring", stiffness: 300, damping: 25 }}
              className="absolute flex cursor-grab flex-col items-center justify-center rounded-full active:cursor-grabbing"
              style={{
                width: SPHERE_SIZE,
                height: SPHERE_SIZE,
                background: `radial-gradient(circle at 35% 35%, rgba(255,255,255,0.3), ${color} 60%, transparent)`,
                boxShadow: `0 0 20px ${color}, 0 0 40px ${color}`,
                touchAction: "none",
              }}
              onPointerDown={(e) => {
                e.preventDefault();
                const el = e.target as HTMLElement;
                if (el.setPointerCapture) {
                  el.setPointerCapture(e.pointerId);
                }
                setDragging(src.id);
              }}
            >
              <span className="select-none text-[10px] font-bold text-white drop-shadow-md">
                {label}
              </span>
            </motion.div>
          );
        })}
      </div>
      <p className="text-center text-xs text-white/40">
        Arrastra las esferas para posicionar los stems en el espacio 3D
      </p>
    </div>
  );
}
