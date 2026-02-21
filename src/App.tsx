import { useState, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import FluidBackground from "./components/FluidBackground";
import PlaybackControls from "./components/PlaybackControls";
import VisualEQ from "./components/VisualEQ";

function App() {
  const [isPlaying, setIsPlaying] = useState(false);
  const [volume] = useState(0.75);
  const [albumArt] = useState<string | undefined>(undefined);

  const handlePlay = useCallback(() => setIsPlaying(true), []);
  const handlePause = useCallback(() => setIsPlaying(false), []);
  const handleSkipForward = useCallback(() => {
    /* will invoke Rust skip command */
  }, []);
  const handleSkipBack = useCallback(() => {
    /* will invoke Rust skip command */
  }, []);

  return (
    <>
      <FluidBackground albumArt={albumArt} />

      <div className="flex min-h-screen flex-col items-center justify-center gap-6 p-6">
        {/* Album art + title area */}
        <AnimatePresence mode="wait">
          <motion.div
            key={albumArt ?? "default"}
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.5, ease: "easeOut" }}
            className="flex flex-col items-center gap-3"
          >
            <div className="h-48 w-48 overflow-hidden rounded-2xl border border-white/10 bg-white/5 shadow-xl backdrop-blur-md">
              {albumArt ? (
                <img
                  src={albumArt}
                  alt="Album art"
                  className="h-full w-full object-cover"
                />
              ) : (
                <div className="flex h-full w-full items-center justify-center">
                  <svg
                    className="h-16 w-16 text-white/20"
                    fill="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
                  </svg>
                </div>
              )}
            </div>
            <div className="text-center">
              <h1 className="text-xl font-semibold text-white">PowerPlayer</h1>
              <p className="text-sm text-white/50">Hi-Res Audio Player</p>
            </div>
          </motion.div>
        </AnimatePresence>

        {/* Playback Controls */}
        <PlaybackControls
          isPlaying={isPlaying}
          onPlay={handlePlay}
          onPause={handlePause}
          onSkipForward={handleSkipForward}
          onSkipBack={handleSkipBack}
          volume={volume}
        />

        {/* Visual EQ */}
        <div className="w-full max-w-2xl">
          <VisualEQ />
        </div>
      </div>
    </>
  );
}

export default App;

