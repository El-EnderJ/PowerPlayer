import { motion } from "framer-motion";

interface FluidBackgroundProps {
  albumArt?: string;
}

export default function FluidBackground({ albumArt }: FluidBackgroundProps) {
  return (
    <div className="pointer-events-none fixed inset-0 -z-10 overflow-hidden bg-black">
      {albumArt ? (
        <motion.div
          key={albumArt}
          initial={{ opacity: 0, scale: 1.1 }}
          animate={{
            opacity: 0.6,
            scale: 1.0,
            rotate: [0, 1, -1, 0],
          }}
          transition={{
            opacity: { duration: 1.5, ease: "easeOut" },
            scale: { duration: 2, ease: "easeOut" },
            rotate: {
              duration: 30,
              repeat: Infinity,
              ease: "linear",
            },
          }}
          className="absolute inset-0"
          style={{
            backgroundImage: `url(${albumArt})`,
            backgroundSize: "cover",
            backgroundPosition: "center",
            filter: "blur(80px) saturate(1.5)",
            transform: "scale(1.3)",
          }}
        />
      ) : (
        /* Default gradient when no album art */
        <motion.div
          animate={{
            background: [
              "radial-gradient(ellipse at 30% 50%, rgba(88,28,135,0.3) 0%, transparent 70%)",
              "radial-gradient(ellipse at 70% 50%, rgba(88,28,135,0.3) 0%, transparent 70%)",
              "radial-gradient(ellipse at 50% 30%, rgba(88,28,135,0.3) 0%, transparent 70%)",
              "radial-gradient(ellipse at 30% 50%, rgba(88,28,135,0.3) 0%, transparent 70%)",
            ],
          }}
          transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
          className="absolute inset-0"
        />
      )}

      {/* Overlay noise/vignette */}
      <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-black/40" />
    </div>
  );
}
