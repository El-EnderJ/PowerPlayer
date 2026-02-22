import { memo } from "react";
import { motion } from "framer-motion";

interface GlassSwitchProps {
  enabled: boolean;
  onChange: (value: boolean) => void;
  label?: string;
  description?: string;
}

function GlassSwitch({ enabled, onChange, label, description }: GlassSwitchProps) {
  return (
    <div className="flex items-start justify-between gap-4">
      {(label || description) && (
        <div className="flex-1 min-w-0">
          {label && <span className="text-sm font-medium text-white">{label}</span>}
          {description && (
            <p className="mt-0.5 text-xs text-white/40 leading-relaxed">{description}</p>
          )}
        </div>
      )}
      <button
        type="button"
        role="switch"
        aria-checked={enabled}
        onClick={() => onChange(!enabled)}
        className={`relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full transition-colors duration-200 ${
          enabled
            ? "bg-cyan-500/20 shadow-[inset_0_2px_4px_rgba(0,0,0,0.4),0_1px_0_rgba(255,255,255,0.05)]"
            : "bg-white/5 shadow-[inset_0_2px_4px_rgba(0,0,0,0.4),0_1px_0_rgba(255,255,255,0.05)]"
        }`}
      >
        <motion.span
          layout
          transition={{ type: "spring", stiffness: 500, damping: 30 }}
          className={`pointer-events-none inline-block h-5 w-5 rounded-full ${
            enabled
              ? "bg-cyan-400 shadow-[0_2px_8px_rgba(34,211,238,0.5),0_0_12px_rgba(34,211,238,0.25)]"
              : "bg-white/20 shadow-[0_1px_3px_rgba(0,0,0,0.3)]"
          }`}
          style={{ marginTop: "1px" }}
          animate={{ x: enabled ? 20 : 2 }}
        />
      </button>
    </div>
  );
}

export default memo(GlassSwitch);
