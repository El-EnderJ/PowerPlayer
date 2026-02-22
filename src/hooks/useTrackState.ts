import { useState } from "react";

export function useTrackState() {
  const [isPlaying, setIsPlaying] = useState(false);
  const [volume, setVolume] = useState(0.75);
  const [albumArt, setAlbumArt] = useState<string | undefined>(undefined);
  const [trackTitle, setTrackTitle] = useState("PowerPlayer");
  const [trackArtist, setTrackArtist] = useState("Hi-Res Audio Player");
  const [duration, setDuration] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [lyricsLines, setLyricsLines] = useState<{ timestamp: number; text: string }[]>([]);
  const [activeLyricIndex, setActiveLyricIndex] = useState(0);

  return {
    isPlaying,
    setIsPlaying,
    volume,
    setVolume,
    albumArt,
    setAlbumArt,
    trackTitle,
    setTrackTitle,
    trackArtist,
    setTrackArtist,
    duration,
    setDuration,
    currentTime,
    setCurrentTime,
    lyricsLines,
    setLyricsLines,
    activeLyricIndex,
    setActiveLyricIndex,
  };
}
