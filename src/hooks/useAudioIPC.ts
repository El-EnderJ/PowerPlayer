import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function useAudioIPC() {
  const invokeSafe = useCallback(
    async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
      try {
        return await invoke<T>(command, args);
      } catch (error) {
        console.error(`Tauri invoke failed for command "${command}"`, error);
        throw error;
      }
    },
    []
  );

  const listenSafe = useCallback(
    async <T,>(
      event: string,
      handler: Parameters<typeof listen<T>>[1]
    ): Promise<Awaited<ReturnType<typeof listen<T>>>> => {
      try {
        return await listen<T>(event, handler);
      } catch (error) {
        console.error(`Tauri listen setup failed for event "${event}"`, error);
        throw error;
      }
    },
    []
  );

  return { invokeSafe, listenSafe };
}
