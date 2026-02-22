import { memo, useState, useEffect, useCallback } from "react";
import { motion } from "framer-motion";
import {
  Speaker,
  Cpu,
  Headphones,
  Library,
  FolderPlus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import GlassSwitch from "./GlassSwitch";
import GlassDropdown from "./GlassDropdown";

type SettingsCategory = "output" | "engine" | "spatial" | "library";

interface CategoryItem {
  id: SettingsCategory;
  label: string;
  icon: typeof Speaker;
}

const CATEGORIES: CategoryItem[] = [
  { id: "output", label: "Salida", icon: Speaker },
  { id: "engine", label: "Motor DSP", icon: Cpu },
  { id: "spatial", label: "Audio 3D & IA", icon: Headphones },
  { id: "library", label: "Biblioteca", icon: Library },
];

interface AudioDevice {
  id: string;
  name: string;
}

interface AudioStats {
  device_name: string;
  sample_rate: number;
  buffer_size: number;
  bit_depth: number;
  latency_ms: number;
  channels: number;
}

interface LibraryStats {
  total_tracks: number;
  scan_paths: string[];
  cache_size_mb: number;
}

const invokeSafe = <T,>(command: string, args?: Record<string, unknown>) =>
  Promise.resolve().then(() => invoke<T>(command, args));

/* ------------------------------------------------------------------ */
/*  Output Panel                                                       */
/* ------------------------------------------------------------------ */
function OutputPanel() {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState("");
  const [exclusiveMode, setExclusiveMode] = useState(false);
  const [stats, setStats] = useState<AudioStats | null>(null);

  useEffect(() => {
    invokeSafe<AudioStats>("get_audio_stats")
      .then((s) => {
        setStats(s);
        setSelectedDevice(s.device_name);
      })
      .catch(() => {});

    invokeSafe<AudioDevice[]>("get_audio_devices")
      .then((d) => setDevices(d))
      .catch(() =>
        setDevices([{ id: "default", name: "Dispositivo por defecto" }])
      );
  }, []);

  const handleDeviceChange = useCallback((id: string) => {
    setSelectedDevice(id);
    invokeSafe("set_output_device", { id, exclusive: exclusiveMode }).catch(() => {});
  }, [exclusiveMode]);

  const handleExclusiveToggle = useCallback(
    (val: boolean) => {
      setExclusiveMode(val);
      invokeSafe("set_output_device", {
        id: selectedDevice,
        exclusive: val,
      }).catch(() => {});
    },
    [selectedDevice]
  );

  return (
    <div className="space-y-6">
      <h2 className="text-lg font-semibold text-white">Salida de Audio</h2>

      <GlassDropdown
        label="Dispositivo (WASAPI)"
        value={selectedDevice}
        options={
          devices.length
            ? devices
            : [{ id: selectedDevice || "default", name: stats?.device_name ?? "Dispositivo por defecto" }]
        }
        onChange={handleDeviceChange}
      />

      {stats && (
        <div className="rounded-lg border border-white/10 bg-white/[0.03] p-3 space-y-1">
          <p className="text-xs text-white/50">Formato actual</p>
          <p className="font-mono text-sm text-cyan-300">
            {stats.bit_depth}-bit / {stats.sample_rate / 1000}kHz — {stats.channels}ch
          </p>
          <p className="font-mono text-xs text-emerald-400/70">
            Buffer {stats.buffer_size} samples · {stats.latency_ms.toFixed(1)}ms
          </p>
        </div>
      )}

      <GlassSwitch
        enabled={exclusiveMode}
        onChange={handleExclusiveToggle}
        label="Modo Exclusivo (Bit-Perfect)"
        description="Toma control exclusivo del DAC. Evita el mezclador de Windows para una salida sin alteraciones."
      />
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Engine Panel                                                       */
/* ------------------------------------------------------------------ */
function EnginePanel() {
  const [bufferMs, setBufferMs] = useState(10);
  const [hqResampler, setHqResampler] = useState(false);

  const handleBufferChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const val = Number(e.target.value);
    setBufferMs(val);
    invokeSafe("set_buffer_size", { ms: val }).catch(() => {});
  }, []);

  return (
    <div className="space-y-6">
      <h2 className="text-lg font-semibold text-white">Motor DSP</h2>

      <div>
        <label className="mb-1.5 block text-sm font-medium text-white">
          Tamaño de Buffer (Latencia)
        </label>
        <div className="flex items-center gap-3">
          <input
            type="range"
            min={2}
            max={50}
            step={1}
            value={bufferMs}
            onChange={handleBufferChange}
            className="glass-slider flex-1"
          />
          <span className="w-14 text-right font-mono text-sm text-cyan-300">
            {bufferMs}ms
          </span>
        </div>
        <p className="mt-1 text-xs text-white/40">
          Valores bajos reducen la latencia pero aumentan el uso de CPU.
        </p>
      </div>

      <GlassSwitch
        enabled={hqResampler}
        onChange={(val) => {
          setHqResampler(val);
          invokeSafe("toggle_hq_resampler", { enabled: val }).catch(() => {});
        }}
        label="Resampler de Alta Calidad"
        description="Usa algoritmos sinc de alta precisión para conversión de frecuencia de muestreo."
      />
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Spatial Panel                                                      */
/* ------------------------------------------------------------------ */
function SpatialPanel() {
  const [gpuEnabled, setGpuEnabled] = useState(false);
  const [roomSize, setRoomSize] = useState(5);

  return (
    <div className="space-y-6">
      <h2 className="text-lg font-semibold text-white">Audio 3D & IA</h2>

      <GlassSwitch
        enabled={gpuEnabled}
        onChange={(val) => {
          setGpuEnabled(val);
          invokeSafe("set_ai_provider", {
            provider: val ? "gpu" : "cpu",
          }).catch(() => {});
        }}
        label="Cargar Modelo ONNX en GPU"
        description="Acelera la separación de pistas usando la GPU. Requiere CUDA o DirectML."
      />

      <div>
        <label className="mb-1.5 block text-sm font-medium text-white">
          Tamaño de Sala Virtual
        </label>
        <div className="flex items-center gap-3">
          <input
            type="range"
            min={1}
            max={20}
            step={0.5}
            value={roomSize}
            onChange={(e) => {
              const val = Number(e.target.value);
              setRoomSize(val);
              invokeSafe("set_room_properties", {
                width: val,
                length: val,
                height: val * 0.5,
                damping: 0.5,
              }).catch(() => {});
            }}
            className="glass-slider flex-1"
          />
          <span className="w-14 text-right font-mono text-sm text-cyan-300">
            {roomSize}m
          </span>
        </div>
        <p className="mt-1 text-xs text-white/40">
          Simula un espacio acústico virtual con HRTF e ITD/ILD.
        </p>
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Library Panel                                                      */
/* ------------------------------------------------------------------ */
function LibraryPanel() {
  const [stats, setStats] = useState<LibraryStats>({
    total_tracks: 0,
    scan_paths: [],
    cache_size_mb: 0,
  });
  const [scanning, setScanning] = useState(false);

  useEffect(() => {
    invokeSafe<{ path: string }[]>("get_library_tracks")
      .then((tracks) =>
        setStats((prev) => ({ ...prev, total_tracks: tracks?.length ?? 0 }))
      )
      .catch(() => {});
  }, []);

  const handleAddFolder = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return;
      setScanning(true);
      await invokeSafe("scan_library", { path: selected });
      const tracks = await invokeSafe<{ path: string }[]>("get_library_tracks");
      setStats((prev) => ({
        ...prev,
        total_tracks: tracks?.length ?? 0,
        scan_paths: [...prev.scan_paths, selected],
      }));
    } catch {
      /* ignore */
    } finally {
      setScanning(false);
    }
  }, []);

  const handleRebuildIndex = useCallback(async () => {
    setScanning(true);
    try {
      await invokeSafe("rebuild_fts_index");
    } catch {
      /* ignore */
    } finally {
      setScanning(false);
    }
  }, []);

  return (
    <div className="space-y-6">
      <h2 className="text-lg font-semibold text-white">Biblioteca</h2>

      {/* Stats card */}
      <div className="rounded-lg border border-white/10 bg-white/[0.03] p-4 space-y-3">
        <div className="flex items-center justify-between">
          <span className="text-sm text-white/60">Tracks Totales</span>
          <span className="font-mono text-lg text-cyan-300">
            {stats.total_tracks}
          </span>
        </div>
        <div className="flex items-center justify-between">
          <span className="text-sm text-white/60">Rutas escaneadas</span>
          <span className="font-mono text-sm text-emerald-400">
            {stats.scan_paths.length}
          </span>
        </div>
        <div className="flex items-center justify-between">
          <span className="text-sm text-white/60">Tamaño de caché de carátulas</span>
          <span className="font-mono text-sm text-white/70">
            {stats.cache_size_mb.toFixed(1)} MB
          </span>
        </div>
      </div>

      {/* Actions */}
      <div className="flex flex-wrap gap-3">
        <button
          type="button"
          onClick={handleAddFolder}
          disabled={scanning}
          className="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-sm text-white transition-colors hover:bg-white/10 disabled:opacity-50"
        >
          <FolderPlus size={16} />
          Añadir Carpeta
        </button>
        <button
          type="button"
          onClick={handleRebuildIndex}
          disabled={scanning}
          className="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-sm text-white transition-colors hover:bg-white/10 disabled:opacity-50"
        >
          <RefreshCw size={16} className={scanning ? "animate-spin" : ""} />
          Reconstruir Índice FTS5
        </button>
      </div>

      {/* Clear cache */}
      <button
        type="button"
        onClick={() => {
          invokeSafe("clear_art_cache").catch(() => {});
          setStats((prev) => ({ ...prev, cache_size_mb: 0 }));
        }}
        className="flex items-center gap-2 rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-2 text-sm text-red-300 transition-colors hover:bg-red-500/20"
      >
        <Trash2 size={16} />
        Limpiar Caché
      </button>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  SettingsView (Dual Pane)                                           */
/* ------------------------------------------------------------------ */
interface SettingsViewProps {
  onBack?: () => void;
}

function SettingsView({ onBack }: SettingsViewProps) {
  const [activeCategory, setActiveCategory] =
    useState<SettingsCategory>("output");

  return (
    <div className="flex h-full w-full flex-col p-4 md:p-6">
      {/* Header */}
      <div className="mb-4 flex items-center gap-3">
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            className="rounded-lg bg-white/5 px-3 py-1.5 text-sm text-white/70 transition-colors hover:bg-white/10"
          >
            ← Volver
          </button>
        )}
        <h1 className="text-xl font-bold text-white">Ajustes</h1>
      </div>

      {/* Dual-pane layout */}
      <div className="flex flex-1 gap-4 overflow-hidden">
        {/* Left sidebar – categories */}
        <div className="liquid-glass w-1/3 max-w-[280px] rounded-2xl p-3 overflow-y-auto scrollbar-hide">
          <nav className="space-y-1">
            {CATEGORIES.map((cat) => {
              const Icon = cat.icon;
              const isActive = activeCategory === cat.id;
              return (
                <button
                  key={cat.id}
                  type="button"
                  onClick={() => setActiveCategory(cat.id)}
                  className={`flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-colors ${
                    isActive
                      ? "bg-white/10 text-white shadow-[0_0_12px_rgba(34,211,238,0.15)]"
                      : "text-white/60 hover:bg-white/5 hover:text-white/80"
                  }`}
                >
                  <Icon size={18} />
                  {cat.label}
                </button>
              );
            })}
          </nav>
        </div>

        {/* Right panel – content */}
        <motion.div
          key={activeCategory}
          initial={{ opacity: 0, x: 12 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.2 }}
          className="liquid-glass flex-1 rounded-2xl p-5 overflow-y-auto scrollbar-hide"
        >
          {activeCategory === "output" && <OutputPanel />}
          {activeCategory === "engine" && <EnginePanel />}
          {activeCategory === "spatial" && <SpatialPanel />}
          {activeCategory === "library" && <LibraryPanel />}
        </motion.div>
      </div>
    </div>
  );
}

export default memo(SettingsView);
