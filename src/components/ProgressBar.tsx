import { useEffect, useRef, useState } from "react";

interface ProgressBarProps {
  currentTime: number;
  duration: number;
  onSeek: (seconds: number) => void;
}

const SEEK_DEBOUNCE_MS = 75;

export default function ProgressBar({ currentTime, duration, onSeek }: ProgressBarProps) {
  const [localTime, setLocalTime] = useState(currentTime);
  const seekDebounceRef = useRef<number | null>(null);

  useEffect(() => {
    setLocalTime(currentTime);
  }, [currentTime]);

  useEffect(
    () => () => {
      if (seekDebounceRef.current !== null) {
        window.clearTimeout(seekDebounceRef.current);
      }
    },
    []
  );

  const debouncedSeek = (seconds: number) => {
    if (seekDebounceRef.current !== null) {
      window.clearTimeout(seekDebounceRef.current);
    }
    seekDebounceRef.current = window.setTimeout(() => onSeek(seconds), SEEK_DEBOUNCE_MS);
  };

  return (
    <div className="w-full">
      <input
        type="range"
        min={0}
        max={Math.max(duration, 1)}
        step={0.01}
        value={Math.min(localTime, Math.max(duration, 1))}
        onChange={(event) => {
          const value = Number(event.target.value);
          setLocalTime(value);
          debouncedSeek(value);
        }}
        onMouseUp={() => {
          if (seekDebounceRef.current !== null) {
            window.clearTimeout(seekDebounceRef.current);
            seekDebounceRef.current = null;
          }
          onSeek(localTime);
        }}
        onTouchEnd={() => {
          if (seekDebounceRef.current !== null) {
            window.clearTimeout(seekDebounceRef.current);
            seekDebounceRef.current = null;
          }
          onSeek(localTime);
        }}
        className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-white/20 accent-violet-400"
      />
      <div className="mt-1 flex justify-between text-[10px] text-white/40">
        <span>{localTime.toFixed(1)}s</span>
        <span>{duration.toFixed(1)}s</span>
      </div>
    </div>
  );
}
