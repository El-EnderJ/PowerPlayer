import { memo, useCallback } from "react";

const LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");

interface AlphabetIndexProps {
  onLetterClick: (letter: string) => void;
  onScrollToTop: () => void;
}

function AlphabetIndex({ onLetterClick, onScrollToTop }: AlphabetIndexProps) {
  const handleTop = useCallback(() => {
    onScrollToTop();
  }, [onScrollToTop]);

  return (
    <div className="fixed right-2 top-1/4 bottom-1/4 z-40 flex flex-col items-center justify-between text-[10px] font-bold text-gray-500">
      <button
        type="button"
        onClick={handleTop}
        className="hover:text-white transition-colors"
        aria-label="Scroll to top"
      >
        ^^
      </button>
      {LETTERS.map((letter) => (
        <button
          key={letter}
          type="button"
          onClick={() => onLetterClick(letter)}
          className="hover:text-white transition-colors leading-none"
        >
          {letter}
        </button>
      ))}
      <button
        type="button"
        onClick={() => onLetterClick("#")}
        className="hover:text-white transition-colors"
      >
        #
      </button>
    </div>
  );
}

export default memo(AlphabetIndex);
