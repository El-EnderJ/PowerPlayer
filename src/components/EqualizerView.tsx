import { useState, useCallback, useEffect, useRef, memo } from "react";
import { motion } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";

/* ─── Types ─── */
interface EqBand {
  index: number;
  frequency: number;
  gain_db: number;
  q_factor: number;
}

interface EqualizerViewProps {
  spectrum?: number[];
}

/* ─── Constants ─── */
const MIN_GAIN = -24;
const MAX_GAIN = 24;
const GAIN_RANGE = MAX_GAIN - MIN_GAIN;
const KNOB_MIN = -1;
const KNOB_MAX = 1;

const DEFAULT_FREQS = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

function defaultBands(): EqBand[] {
  return DEFAULT_FREQS.map((f, i) => ({
    index: i,
    frequency: f,
    gain_db: 0,
    q_factor: 1.0,
  }));
}

function formatFreq(f: number): string {
  return f >= 1000 ? `${f / 1000}k` : `${f}`;
}

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

/* ─── SVG Bezier curve connecting band nodes ─── */
function BezierCurve({
  bands,
  width,
  height,
}: {
  bands: EqBand[];
  width: number;
  height: number;
}) {
  if (bands.length < 2 || width === 0) return null;

  const spacing = width / (bands.length - 1);
  const points = bands.map((b, i) => ({
    x: i * spacing,
    y: ((MAX_GAIN - b.gain_db) / GAIN_RANGE) * height,
  }));

  // Build smooth cubic bezier through all points
  let d = `M ${points[0].x},${points[0].y}`;
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[Math.max(0, i - 1)];
    const p1 = points[i];
    const p2 = points[i + 1];
    const p3 = points[Math.min(points.length - 1, i + 2)];

    const cp1x = p1.x + (p2.x - p0.x) / 6;
    const cp1y = p1.y + (p2.y - p0.y) / 6;
    const cp2x = p2.x - (p3.x - p1.x) / 6;
    const cp2y = p2.y - (p3.y - p1.y) / 6;

    d += ` C ${cp1x},${cp1y} ${cp2x},${cp2y} ${p2.x},${p2.y}`;
  }

  // Build fill path (close at bottom)
  const fillD =
    d +
    ` L ${points[points.length - 1].x},${height} L ${points[0].x},${height} Z`;

  return (
    <svg
      className="pointer-events-none absolute inset-0"
      viewBox={`0 0 ${width} ${height}`}
      preserveAspectRatio="none"
    >
      <defs>
        <linearGradient id="curveGrad" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="rgba(34,211,238,0.25)" />
          <stop offset="100%" stopColor="rgba(34,211,238,0)" />
        </linearGradient>
      </defs>
      <path d={fillD} fill="url(#curveGrad)" />
      <path
        d={d}
        fill="none"
        stroke="rgba(34,211,238,0.8)"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

/* ─── Vertical Slider (glass rail + glowing pill knob) ─── */
function BandSlider({
  band,
  onChange,
}: {
  band: EqBand;
  onChange: (gain: number) => void;
}) {
  const railRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  const pct = ((band.gain_db - MIN_GAIN) / GAIN_RANGE) * 100;

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      const rail = railRef.current;
      if (!rail) return;
      const rect = rail.getBoundingClientRect();
      const y = clamp(e.clientY - rect.top, 0, rect.height);
      const gain = MAX_GAIN - (y / rect.height) * GAIN_RANGE;
      onChange(clamp(gain, MIN_GAIN, MAX_GAIN));
    },
    [onChange]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const rail = railRef.current;
      if (!rail) return;
      const rect = rail.getBoundingClientRect();
      const y = clamp(e.clientY - rect.top, 0, rect.height);
      const gain = MAX_GAIN - (y / rect.height) * GAIN_RANGE;
      onChange(clamp(gain, MIN_GAIN, MAX_GAIN));
    },
    [onChange]
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  return (
    <div className="flex flex-col items-center gap-1">
      {/* Gain label */}
      <span className="text-[9px] tabular-nums text-cyan-300/70">
        {band.gain_db > 0 ? "+" : ""}
        {band.gain_db.toFixed(1)}
      </span>

      {/* Glass rail */}
      <div
        ref={railRef}
        className="relative h-36 w-3 cursor-pointer rounded-full border border-white/10 bg-white/5 backdrop-blur-sm"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
      >
        {/* Fill */}
        <div
          className="absolute bottom-0 left-0 w-full rounded-full bg-gradient-to-t from-cyan-500/40 to-cyan-300/10"
          style={{ height: `${pct}%` }}
        />
        {/* Pill knob */}
        <motion.div
          className="absolute left-1/2 h-4 w-5 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white/30 bg-white/20 shadow-[0_0_10px_rgba(34,211,238,0.5)] backdrop-blur-md"
          style={{ bottom: `${pct}%` }}
          layout
          transition={{ type: "spring", stiffness: 500, damping: 35 }}
        />
      </div>

      {/* Frequency label */}
      <span className="text-[9px] text-white/40">{formatFreq(band.frequency)}</span>
    </div>
  );
}

/* ─── Rotary Knob (concentric glass circles) ─── */
function GlassKnob({
  label,
  value,
  min,
  max,
  onChange,
  color = "cyan",
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (v: number) => void;
  color?: "cyan" | "violet" | "emerald";
}) {
  const knobRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);
  const startY = useRef(0);
  const startVal = useRef(0);

  const range = max - min;
  const normalized = (value - min) / range; // 0-1
  const angle = -135 + normalized * 270; // -135° to +135°

  const colorMap = {
    cyan: {
      ring: "border-cyan-400/30",
      glow: "shadow-[0_0_20px_rgba(34,211,238,0.3)]",
      indicator: "bg-cyan-400",
      arc: "rgba(34,211,238,0.6)",
    },
    violet: {
      ring: "border-violet-400/30",
      glow: "shadow-[0_0_20px_rgba(139,92,246,0.3)]",
      indicator: "bg-violet-400",
      arc: "rgba(139,92,246,0.6)",
    },
    emerald: {
      ring: "border-emerald-400/30",
      glow: "shadow-[0_0_20px_rgba(52,211,153,0.3)]",
      indicator: "bg-emerald-400",
      arc: "rgba(52,211,153,0.6)",
    },
  };
  const c = colorMap[color];

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      startY.current = e.clientY;
      startVal.current = value;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [value]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const delta = (startY.current - e.clientY) / 150;
      const newVal = clamp(startVal.current + delta * range, min, max);
      onChange(newVal);
    },
    [min, max, range, onChange]
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  // SVG arc for the value indicator
  const arcRadius = 38;
  const startAngle = -225;
  const endAngle = startAngle + normalized * 270;
  const arcPath = describeArc(44, 44, arcRadius, startAngle, endAngle);

  return (
    <div className="flex flex-col items-center gap-2">
      <div
        ref={knobRef}
        className={`relative flex h-[88px] w-[88px] cursor-grab items-center justify-center rounded-full border ${c.ring} bg-white/5 ${c.glow} backdrop-blur-xl active:cursor-grabbing`}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
      >
        {/* Arc indicator */}
        <svg
          className="pointer-events-none absolute inset-0"
          viewBox="0 0 88 88"
        >
          <circle
            cx="44"
            cy="44"
            r={arcRadius}
            fill="none"
            stroke="rgba(255,255,255,0.06)"
            strokeWidth="3"
          />
          {arcPath && (
            <path
              d={arcPath}
              fill="none"
              stroke={c.arc}
              strokeWidth="3"
              strokeLinecap="round"
            />
          )}
        </svg>

        {/* Inner glass circle */}
        <div className="flex h-14 w-14 items-center justify-center rounded-full border border-white/10 bg-white/5 backdrop-blur-sm">
          {/* Rotation indicator line */}
          <div
            className="absolute h-5 w-0.5 origin-bottom rounded-full"
            style={{ transform: `rotate(${angle}deg)` }}
          >
            <div className={`h-2 w-full rounded-full ${c.indicator}`} />
          </div>
        </div>
      </div>

      <span className="text-[11px] font-medium tracking-wide text-white/60">
        {label}
      </span>
      <span className="text-[10px] tabular-nums text-white/30">
        {value.toFixed(value === Math.round(value) ? 0 : 1)}
      </span>
    </div>
  );
}

/* ─── iOS-style Toggle Switch ─── */
function GlassSwitch({
  label,
  enabled,
  onToggle,
}: {
  label: string;
  enabled: boolean;
  onToggle: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onToggle(!enabled)}
      className="flex items-center gap-3 rounded-xl border border-white/5 bg-white/[0.03] px-4 py-2.5 transition-colors hover:bg-white/[0.06]"
    >
      <span className="text-xs text-white/60">{label}</span>
      <div
        className={`relative h-[22px] w-[40px] rounded-full transition-colors duration-300 ${
          enabled ? "bg-cyan-500/60" : "bg-white/10"
        }`}
      >
        <motion.div
          className="absolute top-[2px] h-[18px] w-[18px] rounded-full border border-white/20 bg-white shadow-sm"
          animate={{ left: enabled ? 20 : 2 }}
          transition={{ type: "spring", stiffness: 500, damping: 35 }}
        />
      </div>
    </button>
  );
}

/* ─── Balance Slider (horizontal) ─── */
function BalanceSlider({
  value,
  onChange,
}: {
  value: number;
  onChange: (v: number) => void;
}) {
  const pct = ((value + 1) / 2) * 100; // -1..1 → 0..100

  return (
    <div className="flex w-full max-w-xs flex-col items-center gap-1">
      <span className="text-[11px] font-medium tracking-wide text-white/60">
        Balance
      </span>
      <div className="relative h-3 w-full rounded-full border border-white/10 bg-white/5 backdrop-blur-sm">
        {/* Center line */}
        <div className="absolute left-1/2 top-0.5 h-2 w-px bg-white/20" />
        {/* Thumb */}
        <motion.div
          className="absolute top-1/2 h-5 w-5 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white/30 bg-white/20 shadow-[0_0_10px_rgba(52,211,153,0.4)] backdrop-blur-md"
          style={{ left: `${pct}%` }}
          layout
          transition={{ type: "spring", stiffness: 500, damping: 35 }}
        />
        <input
          type="range"
          min={-1}
          max={1}
          step={0.01}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          className="absolute inset-0 w-full cursor-pointer opacity-0"
        />
      </div>
      <div className="flex w-full justify-between text-[9px] text-white/30">
        <span>L</span>
        <span>C</span>
        <span>R</span>
      </div>
    </div>
  );
}

/* ─── Spectrum Background ─── */
function SpectrumBackground({ spectrum }: { spectrum: number[] }) {
  if (!spectrum.length) return null;
  const bins = Math.min(64, spectrum.length);

  return (
    <div className="pointer-events-none absolute inset-0 opacity-20">
      <div className="flex h-full w-full items-end">
        {Array.from({ length: bins }, (_, i) => {
          const db = spectrum[i] ?? -100;
          const normalized = Math.max(0, Math.min(1, (db + 100) / 100));
          return (
            <div
              key={i}
              className="flex-1 bg-gradient-to-t from-cyan-500/60 to-cyan-300/10"
              style={{ height: `${normalized * 100}%` }}
            />
          );
        })}
      </div>
    </div>
  );
}

/* ─── Main Equalizer View ─── */
function EqualizerView({ spectrum = [] }: EqualizerViewProps) {
  const [bands, setBands] = useState<EqBand[]>(defaultBands);
  const [bass, setBass] = useState(0);
  const [treble, setTreble] = useState(0);
  const [balance, setBalance] = useState(0);
  const [spatialEnabled, setSpatialEnabled] = useState(false);
  const [expansionEnabled, setExpansionEnabled] = useState(false);
  const sliderContainerRef = useRef<HTMLDivElement>(null);
  const [sliderWidth, setSliderWidth] = useState(0);

  // Fetch bands from backend
  useEffect(() => {
    invoke<EqBand[]>("get_eq_bands")
      .then((data) => setBands(data))
      .catch(() => {});
  }, []);

  // Measure slider container width for SVG curve
  useEffect(() => {
    const el = sliderContainerRef.current;
    if (!el) return;
    const obs = new ResizeObserver(([entry]) => {
      setSliderWidth(entry.contentRect.width);
    });
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  const updateBand = useCallback((index: number, gain: number) => {
    setBands((prev) => {
      const band = prev.find((b) => b.index === index);
      if (band) {
        invoke("update_eq_band", {
          index,
          freq: band.frequency,
          gain,
          q: band.q_factor,
        }).catch(() => {});
      }
      return prev.map((b) =>
        b.index === index ? { ...b, gain_db: gain } : b
      );
    });
  }, []);

  const handleBassChange = useCallback((v: number) => {
    setBass(v);
    setTreble((prev) => {
      invoke("set_tone", { bass: v, treble: prev }).catch(() => {});
      return prev;
    });
  }, []);

  const handleTrebleChange = useCallback((v: number) => {
    setTreble(v);
    setBass((prev) => {
      invoke("set_tone", { bass: prev, treble: v }).catch(() => {});
      return prev;
    });
  }, []);

  const handleBalanceChange = useCallback((v: number) => {
    setBalance(v);
    invoke("set_balance", { val: v }).catch(() => {});
  }, []);

  const handleSpatialToggle = useCallback((enabled: boolean) => {
    setSpatialEnabled(enabled);
    invoke("toggle_spatial_mode", { enabled }).catch(() => {});
  }, []);

  const handleExpansionToggle = useCallback((enabled: boolean) => {
    setExpansionEnabled(enabled);
    invoke("set_expansion", { val: enabled ? 0.5 : 0 }).catch(() => {});
  }, []);

  return (
    <div className="relative flex h-full w-full flex-col overflow-hidden">
      {/* Spectrum background behind everything */}
      <SpectrumBackground spectrum={spectrum} />

      <div className="relative z-10 flex flex-1 flex-col gap-4 px-4 pb-28 pt-4 overflow-y-auto scrollbar-hide">
        {/* ── Band Sliders Panel ── */}
        <div className="rounded-2xl border-t border-white/10 border-b-black/40 bg-white/5 p-4 shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%]">
          <div className="mb-3 flex items-center justify-between">
            <span className="text-xs font-semibold tracking-wider text-white/50">
              PARAMETRIC EQ
            </span>
            <span className="text-[10px] text-white/30">10-Band</span>
          </div>

          <div className="relative" ref={sliderContainerRef}>
            {/* SVG Bezier curve overlay */}
            <div className="pointer-events-none absolute inset-0 z-0 px-3">
              <BezierCurve bands={bands} width={sliderWidth - 24} height={144} />
            </div>

            <div className="relative z-10 flex items-end justify-between px-1">
              {bands.map((band) => (
                <BandSlider
                  key={band.index}
                  band={band}
                  onChange={(gain) => updateBand(band.index, gain)}
                />
              ))}
            </div>
          </div>
        </div>

        {/* ── Knobs Panel ── */}
        <div className="rounded-2xl border-t border-white/10 border-b-black/40 bg-white/5 p-5 shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%]">
          <span className="mb-4 block text-xs font-semibold tracking-wider text-white/50">
            TONE CONTROL
          </span>

          <div className="flex flex-wrap items-start justify-center gap-8">
            <GlassKnob
              label="Bass"
              value={bass}
              min={KNOB_MIN}
              max={KNOB_MAX}
              onChange={handleBassChange}
              color="cyan"
            />
            <GlassKnob
              label="Treble"
              value={treble}
              min={KNOB_MIN}
              max={KNOB_MAX}
              onChange={handleTrebleChange}
              color="violet"
            />
          </div>

          <div className="mx-auto mt-5 max-w-xs">
            <BalanceSlider value={balance} onChange={handleBalanceChange} />
          </div>
        </div>

        {/* ── Special Effects Panel ── */}
        <div className="rounded-2xl border-t border-white/10 border-b-black/40 bg-white/5 p-4 shadow-[0_20px_50px_rgba(0,0,0,0.5)] backdrop-blur-[40px] saturate-[180%]">
          <span className="mb-3 block text-xs font-semibold tracking-wider text-white/50">
            EFFECTS
          </span>
          <div className="flex flex-wrap gap-3">
            <GlassSwitch
              label="3D Immersive"
              enabled={spatialEnabled}
              onToggle={handleSpatialToggle}
            />
            <GlassSwitch
              label="Stereo Expansion"
              enabled={expansionEnabled}
              onToggle={handleExpansionToggle}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Utility: SVG arc path ─── */
function describeArc(
  cx: number,
  cy: number,
  r: number,
  startAngle: number,
  endAngle: number
): string | null {
  if (Math.abs(endAngle - startAngle) < 0.5) return null;
  const toRad = (d: number) => (d * Math.PI) / 180;
  const start = {
    x: cx + r * Math.cos(toRad(endAngle)),
    y: cy + r * Math.sin(toRad(endAngle)),
  };
  const end = {
    x: cx + r * Math.cos(toRad(startAngle)),
    y: cy + r * Math.sin(toRad(startAngle)),
  };
  const largeArc = endAngle - startAngle > 180 ? 1 : 0;
  return `M ${start.x} ${start.y} A ${r} ${r} 0 ${largeArc} 0 ${end.x} ${end.y}`;
}

export default memo(EqualizerView);
