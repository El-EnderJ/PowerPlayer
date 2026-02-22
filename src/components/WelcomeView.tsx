import { memo } from "react";
import { motion } from "framer-motion";
import { FolderOpen, Music } from "lucide-react";

interface WelcomeViewProps {
  onSelectLibrary: () => void;
}

function WelcomeView({ onSelectLibrary }: WelcomeViewProps) {
  return (
    <div className="flex h-full w-full items-center justify-center p-6">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.5, ease: "easeOut" }}
        className="liquid-glass flex max-w-md flex-col items-center gap-6 rounded-3xl px-10 py-12 text-center"
      >
        <div className="flex h-20 w-20 items-center justify-center rounded-full bg-white/5">
          <Music size={36} className="text-cyan-400" />
        </div>

        <div>
          <h2 className="text-2xl font-bold text-white">Bienvenido a PowerPlayer</h2>
          <p className="mt-2 text-sm leading-relaxed text-white/50">
            Selecciona una carpeta de música para empezar a escuchar en alta resolución.
          </p>
        </div>

        <motion.button
          type="button"
          onClick={onSelectLibrary}
          whileHover={{ scale: 1.04 }}
          whileTap={{ scale: 0.97 }}
          className="flex items-center gap-3 rounded-2xl border border-cyan-500/20 bg-cyan-500/10 px-6 py-3 text-sm font-medium text-cyan-300 transition-colors hover:bg-cyan-500/20"
        >
          <FolderOpen size={18} />
          Seleccionar Biblioteca
        </motion.button>
      </motion.div>
    </div>
  );
}

export default memo(WelcomeView);
