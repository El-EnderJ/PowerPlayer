import { memo, useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";

interface SplashScreenProps {
  /** Dismiss the splash once the backend is ready */
  visible: boolean;
}

/**
 * Minimalist splash screen — pure black background with a frosted-glass
 * logo that fades in while the Rust audio engine initialises.
 */
const SplashScreen = memo(function SplashScreen({ visible }: SplashScreenProps) {
  const [show, setShow] = useState(visible);

  useEffect(() => {
    if (!visible) {
      // small delay so the fade-out animation can play
      const id = setTimeout(() => setShow(false), 600);
      return () => clearTimeout(id);
    }
    setShow(true);
  }, [visible]);

  return (
    <AnimatePresence>
      {show && (
        <motion.div
          key="splash"
          initial={{ opacity: 1 }}
          animate={{ opacity: visible ? 1 : 0 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.5, ease: "easeOut" }}
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black"
        >
          {/* Frosted-glass logo pill */}
          <motion.div
            initial={{ opacity: 0, scale: 0.85, filter: "blur(16px)" }}
            animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
            transition={{ duration: 0.8, ease: "easeOut" }}
            className="flex flex-col items-center gap-4 rounded-3xl border border-white/10 bg-white/5 px-10 py-8 shadow-[0_30px_80px_rgba(0,0,0,0.7)] backdrop-blur-[40px]"
          >
            {/* Play-button icon */}
            <svg
              className="h-16 w-16 text-white/80"
              fill="currentColor"
              viewBox="0 0 24 24"
            >
              <path d="M8 5v14l11-7z" />
            </svg>
            <span className="text-lg font-semibold tracking-widest text-white/70">
              PowerPlayer
            </span>
          </motion.div>

          {/* Subtle loading indicator */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 0.4 }}
            transition={{ delay: 0.6, duration: 0.5 }}
            className="absolute bottom-12 text-xs tracking-wider text-white/30"
          >
            Initializing audio engine…
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
});

export default SplashScreen;
