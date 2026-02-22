import { memo, useState, useRef, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown } from "lucide-react";

interface GlassDropdownProps {
  label?: string;
  value: string;
  options: { id: string; name: string }[];
  onChange: (id: string) => void;
}

function GlassDropdown({ label, value, options, onChange }: GlassDropdownProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const selected = options.find((o) => o.id === value);

  return (
    <div ref={ref} className="relative">
      {label && (
        <span className="mb-1.5 block text-sm font-medium text-white">{label}</span>
      )}
      <button
        type="button"
        onClick={() => setOpen((p) => !p)}
        className="flex w-full items-center justify-between gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white backdrop-blur-md transition-colors hover:bg-white/10"
      >
        <span className="truncate font-mono text-cyan-300">
          {selected?.name ?? "—"}
        </span>
        <ChevronDown
          size={14}
          className={`flex-shrink-0 text-white/50 transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>
      <AnimatePresence>
        {open && (
          <motion.ul
            initial={{ opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.15 }}
            className="absolute z-50 mt-1 max-h-48 w-full overflow-auto rounded-lg border border-white/10 bg-black/80 backdrop-blur-2xl scrollbar-hide"
          >
            {options.map((opt) => (
              <li key={opt.id}>
                <button
                  type="button"
                  onClick={() => {
                    onChange(opt.id);
                    setOpen(false);
                  }}
                  className={`w-full px-3 py-2 text-left text-sm transition-colors hover:bg-white/10 ${
                    opt.id === value
                      ? "font-mono text-cyan-300"
                      : "text-white/70"
                  }`}
                >
                  {opt.name}
                </button>
              </li>
            ))}
          </motion.ul>
        )}
      </AnimatePresence>
    </div>
  );
}

export default memo(GlassDropdown);
