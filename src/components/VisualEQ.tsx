import { useRef, useEffect, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface EqBand {
  index: number;
  frequency: number;
  gain_db: number;
  q_factor: number;
}

interface FrequencyPoint {
  frequency: number;
  magnitude_db: number;
}

const MIN_FREQ = 20;
const MAX_FREQ = 20000;
const MIN_GAIN = -24;
const MAX_GAIN = 24;
const MIN_Q = 0.1;
const MAX_Q = 18.0;
const RESPONSE_POINTS = 256;

function freqToX(freq: number, width: number): number {
  return (Math.log10(freq / MIN_FREQ) / Math.log10(MAX_FREQ / MIN_FREQ)) * width;
}

function xToFreq(x: number, width: number): number {
  const ratio = x / width;
  return MIN_FREQ * Math.pow(MAX_FREQ / MIN_FREQ, ratio);
}

function gainToY(gain: number, height: number): number {
  return ((MAX_GAIN - gain) / (MAX_GAIN - MIN_GAIN)) * height;
}

function yToGain(y: number, height: number): number {
  return MAX_GAIN - (y / height) * (MAX_GAIN - MIN_GAIN);
}

interface VisualEQProps {
  spectrum?: number[];
}

export default function VisualEQ({ spectrum = [] }: VisualEQProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [bands, setBands] = useState<EqBand[]>([]);
  const [response, setResponse] = useState<FrequencyPoint[]>([]);
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const animFrameRef = useRef<number>(0);
  const spectrumRef = useRef<number[]>(spectrum);

  useEffect(() => {
    spectrumRef.current = spectrum;
  }, [spectrum]);

  const fetchBands = useCallback(async () => {
    try {
      const data = await invoke<EqBand[]>("get_eq_bands");
      setBands(data);
    } catch (error) {
      console.error("Failed to fetch EQ bands from backend, using defaults", error);
      // Generate default 10-band EQ data for standalone preview
      const defaultBands: EqBand[] = Array.from({ length: 10 }, (_, i) => ({
        index: i,
        frequency: 32 * Math.pow(16000 / 32, i / 9),
        gain_db: 0,
        q_factor: 1.0,
      }));
      setBands(defaultBands);
    }
  }, []);

  const fetchResponse = useCallback(async () => {
    try {
      const data = await invoke<FrequencyPoint[]>("get_eq_frequency_response", {
        numPoints: RESPONSE_POINTS,
      });
      setResponse(data);
    } catch (error) {
      console.error("Failed to fetch EQ response from backend, using flat response", error);
      // Generate flat response for standalone preview
      const flat: FrequencyPoint[] = Array.from({ length: RESPONSE_POINTS }, (_, i) => ({
        frequency: MIN_FREQ * Math.pow(MAX_FREQ / MIN_FREQ, i / (RESPONSE_POINTS - 1)),
        magnitude_db: 0,
      }));
      setResponse(flat);
    }
  }, []);

  useEffect(() => {
    fetchBands();
    fetchResponse();
  }, [fetchBands, fetchResponse]);

  const updateBand = useCallback(
    async (index: number, freq: number, gain: number, q: number) => {
      const clampedFreq = Math.max(MIN_FREQ, Math.min(MAX_FREQ, freq));
      const clampedGain = Math.max(MIN_GAIN, Math.min(MAX_GAIN, gain));
      const clampedQ = Math.max(MIN_Q, Math.min(MAX_Q, q));

      setBands((prev) =>
        prev.map((b) =>
          b.index === index
            ? { ...b, frequency: clampedFreq, gain_db: clampedGain, q_factor: clampedQ }
            : b
        )
      );

      try {
        await invoke("update_eq_band", {
          index,
          freq: clampedFreq,
          gain: clampedGain,
          q: clampedQ,
        });
        fetchResponse();
      } catch (error) {
        console.error("Failed to update EQ band, refreshing response with fallback", error);
        // Compute local response when backend is unavailable
        fetchResponse();
      }
    },
    [fetchResponse]
  );

  // Draw EQ curve on canvas using requestAnimationFrame
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const draw = () => {
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const dpr = window.devicePixelRatio || 1;
      const rect = canvas.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      ctx.scale(dpr, dpr);

      const w = rect.width;
      const h = rect.height;

      // Clear
      ctx.clearRect(0, 0, w, h);

      // Grid lines
      ctx.strokeStyle = "rgba(255,255,255,0.06)";
      ctx.lineWidth = 1;
      const gridFreqs = [50, 100, 200, 500, 1000, 2000, 5000, 10000];
      for (const f of gridFreqs) {
        const x = freqToX(f, w);
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, h);
        ctx.stroke();
      }
      const gridGains = [-18, -12, -6, 0, 6, 12, 18];
      for (const g of gridGains) {
        const y = gainToY(g, h);
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
        ctx.stroke();
      }

      // 0 dB line
      ctx.strokeStyle = "rgba(255,255,255,0.15)";
      ctx.lineWidth = 1;
      const zeroY = gainToY(0, h);
      ctx.beginPath();
      ctx.moveTo(0, zeroY);
      ctx.lineTo(w, zeroY);
      ctx.stroke();

      // Frequency labels
      ctx.fillStyle = "rgba(255,255,255,0.3)";
      ctx.font = "10px Inter, system-ui, sans-serif";
      ctx.textAlign = "center";
      for (const f of gridFreqs) {
        const x = freqToX(f, w);
        const label = f >= 1000 ? `${f / 1000}k` : `${f}`;
        ctx.fillText(label, x, h - 4);
      }

      // Gain labels
      ctx.textAlign = "right";
      for (const g of gridGains) {
        if (g === 0) continue;
        const y = gainToY(g, h);
        ctx.fillText(`${g > 0 ? "+" : ""}${g}`, w - 4, y + 3);
      }

      // Draw filled response area
      if (response.length > 1) {
        ctx.beginPath();
        ctx.moveTo(freqToX(response[0].frequency, w), gainToY(response[0].magnitude_db, h));
        for (let i = 1; i < response.length; i++) {
          ctx.lineTo(freqToX(response[i].frequency, w), gainToY(response[i].magnitude_db, h));
        }
        // Close path at bottom (0 dB line) for fill
        ctx.lineTo(freqToX(response[response.length - 1].frequency, w), zeroY);
        ctx.lineTo(freqToX(response[0].frequency, w), zeroY);
        ctx.closePath();

        const gradient = ctx.createLinearGradient(0, 0, 0, h);
        gradient.addColorStop(0, "rgba(139,92,246,0.25)");
        gradient.addColorStop(0.5, "rgba(139,92,246,0.05)");
        gradient.addColorStop(1, "rgba(139,92,246,0.0)");
        ctx.fillStyle = gradient;
        ctx.fill();
      }

      // Draw response curve line
      if (response.length > 1) {
        ctx.beginPath();
        ctx.moveTo(freqToX(response[0].frequency, w), gainToY(response[0].magnitude_db, h));
        for (let i = 1; i < response.length; i++) {
          ctx.lineTo(freqToX(response[i].frequency, w), gainToY(response[i].magnitude_db, h));
        }
        ctx.strokeStyle = "rgba(139,92,246,0.9)";
        ctx.lineWidth = 2;
        ctx.shadowColor = "rgba(139,92,246,0.6)";
        ctx.shadowBlur = 8;
        ctx.stroke();
        ctx.shadowBlur = 0;
      }

      if (spectrumRef.current.length > 0) {
        const bins = Math.min(64, spectrumRef.current.length);
        const barWidth = w / bins;
        for (let i = 0; i < bins; i++) {
          const db = spectrumRef.current[i];
          const normalized = Math.max(0, Math.min(1, (db + 100) / 100));
          const barHeight = normalized * h * 0.25;
          const x = i * barWidth;
          const y = h - barHeight;
          ctx.fillStyle = "rgba(56,189,248,0.35)";
          ctx.fillRect(x, y, barWidth * 0.8, barHeight);
        }
      }

      // Draw band control points
      for (const band of bands) {
        const bx = freqToX(band.frequency, w);
        const by = gainToY(band.gain_db, h);
        const isActive = dragIndex === band.index;

        // Outer glow
        ctx.beginPath();
        ctx.arc(bx, by, isActive ? 12 : 8, 0, Math.PI * 2);
        ctx.fillStyle = isActive
          ? "rgba(139,92,246,0.3)"
          : "rgba(139,92,246,0.15)";
        ctx.fill();

        // Inner point
        ctx.beginPath();
        ctx.arc(bx, by, isActive ? 6 : 4, 0, Math.PI * 2);
        ctx.fillStyle = isActive ? "#a78bfa" : "#8b5cf6";
        ctx.fill();
        ctx.strokeStyle = "rgba(255,255,255,0.5)";
        ctx.lineWidth = 1;
        ctx.stroke();
      }

      animFrameRef.current = requestAnimationFrame(draw);
    };

    animFrameRef.current = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(animFrameRef.current);
  }, [bands, response, dragIndex]);

  // Mouse interaction handlers
  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;

      // Find closest band point
      let closestIdx = -1;
      let closestDist = 20; // max pixel distance to select
      for (const band of bands) {
        const bx = freqToX(band.frequency, rect.width);
        const by = gainToY(band.gain_db, rect.height);
        const dist = Math.hypot(mx - bx, my - by);
        if (dist < closestDist) {
          closestDist = dist;
          closestIdx = band.index;
        }
      }

      if (closestIdx >= 0) {
        setDragIndex(closestIdx);
      }
    },
    [bands]
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (dragIndex === null) return;
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;

      const freq = xToFreq(mx, rect.width);
      const gain = yToGain(my, rect.height);
      const band = bands.find((b) => b.index === dragIndex);
      if (band) {
        updateBand(dragIndex, freq, gain, band.q_factor);
      }
    },
    [dragIndex, bands, updateBand]
  );

  const handleMouseUp = useCallback(() => {
    setDragIndex(null);
  }, []);

  // Scroll for Q factor adjustment
  const handleWheel = useCallback(
    (e: React.WheelEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;

      // Find closest band
      let closestIdx = -1;
      let closestDist = 25;
      for (const band of bands) {
        const bx = freqToX(band.frequency, rect.width);
        const by = gainToY(band.gain_db, rect.height);
        const dist = Math.hypot(mx - bx, my - by);
        if (dist < closestDist) {
          closestDist = dist;
          closestIdx = band.index;
        }
      }

      if (closestIdx >= 0) {
        e.preventDefault();
        const band = bands.find((b) => b.index === closestIdx);
        if (band) {
          const delta = e.deltaY > 0 ? -0.3 : 0.3;
          const newQ = Math.max(MIN_Q, Math.min(MAX_Q, band.q_factor + delta));
          updateBand(closestIdx, band.frequency, band.gain_db, newQ);
        }
      }
    },
    [bands, updateBand]
  );

  return (
    <div className="relative w-full rounded-xl border border-white/10 bg-white/5 p-1 backdrop-blur-md">
      <div className="mb-1 flex items-center justify-between px-3 pt-2">
        <span className="text-xs font-medium tracking-wider text-white/50">PARAMETRIC EQ</span>
        <span className="text-[10px] text-white/30">
          Drag points · Scroll to adjust Q
        </span>
      </div>
      <canvas
        ref={canvasRef}
        className="h-48 w-full cursor-crosshair rounded-lg"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onWheel={handleWheel}
      />
    </div>
  );
}
