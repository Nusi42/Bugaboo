import { Listeners } from "@bugaboo/ui";
import { useSyncExternalStore } from "react";

/** How often the clock of the screens moves, in milliseconds: words like "2 minutes ago" are good for that long. */
const TICK_MS = 30_000;

let current = Date.now();
let timer: ReturnType<typeof setInterval> | null = null;
const listeners = new Listeners();

function tick(): void {
  current = Date.now();
  listeners.notify();
}

function subscribe(listener: () => void): () => void {
  const stop = listeners.subscribe(listener);
  if (timer === null) {
    current = Date.now();
    timer = setInterval(tick, TICK_MS);
  }
  return () => {
    stop();
    if (listeners.size === 0 && timer !== null) {
      clearInterval(timer);
      timer = null;
    }
  };
}

/**
 * The time now in milliseconds, moving every half minute: what a screen needs for words about how
 * long ago something was, without reading the clock while it draws (a screen that reads the clock
 * differently in two draws shows two different texts).
 */
export function useNow(): number {
  return useSyncExternalStore(subscribe, () => current);
}
