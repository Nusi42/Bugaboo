import { Listeners } from "@bugaboo/ui";
import { useSyncExternalStore } from "react";

/** How often the clock of the screens moves, in milliseconds: words like "2 minutes ago" are good for that long. */
const TICK_MS = 30_000;

/** A clock that moves every `periodMs` while some screen listens, and stops when nobody does. */
function clockEvery(periodMs: number) {
  let current = Date.now();
  let timer: ReturnType<typeof setInterval> | null = null;
  const listeners = new Listeners();
  const tick = (): void => {
    current = Date.now();
    listeners.notify();
  };
  return {
    snapshot: (): number => current,
    subscribe: (listener: () => void): (() => void) => {
      const stop = listeners.subscribe(listener);
      if (timer === null) {
        current = Date.now();
        timer = setInterval(tick, periodMs);
      }
      return () => {
        stop();
        if (listeners.size === 0 && timer !== null) {
          clearInterval(timer);
          timer = null;
        }
      };
    },
  };
}

const halfMinute = clockEvery(TICK_MS);

/**
 * The time now in milliseconds, moving every half minute: what a screen needs for words about how
 * long ago something was, without reading the clock while it draws (a screen that reads the clock
 * differently in two draws shows two different texts).
 */
export function useNow(): number {
  return useSyncExternalStore(halfMinute.subscribe, halfMinute.snapshot);
}
