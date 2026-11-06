import { useEffect, useSyncExternalStore } from "react";

/** How many notes float over the page at its bottom edge now (the warning of a timer that runs long). */
let showing = 0;
const listeners = new Set<() => void>();

function change(by: number): void {
  showing += by;
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Tells the page that a note floats over it at the bottom of the start side while `shows` is true, so
 * what else stands at the bottom (the offers to undo) stands beside it and does not cover it.
 */
export function useFloatingNote(shows: boolean): void {
  useEffect(() => {
    if (shows) {
      change(1);
    }
    return () => {
      if (shows) {
        change(-1);
      }
    };
  }, [shows]);
}

/** Whether a note floats over the page at the bottom of the start side. */
export function useFloatingNoteShows(): boolean {
  return useSyncExternalStore(subscribe, () => showing > 0);
}
