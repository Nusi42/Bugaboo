// Helpers for the tests of the optimistic helpers: a cache holding a list of names, a server that
// answers when the test says so, and a clock that only moves when the test says so.
import type { OptimisticChange, Schedule } from "./optimistic.ts";

export interface Names {
  readonly names: readonly string[];
}

export const key = ["names"] as const;

export function isNames(value: unknown): value is Names {
  return typeof value === "object" && value !== null && Array.isArray(Reflect.get(value, "names"));
}

/** A change that adds a name to the list in the cache; `send` is whatever the test needs. */
export function addName(send: (variables: { readonly name: string }) => Promise<string>) {
  const change: OptimisticChange<{ readonly name: string }, Names, string> = {
    queryKey: key,
    isCached: isNames,
    apply: (before, { name }) => ({ names: [...(before?.names ?? []), name] }),
    send,
  };
  return change;
}

/** A promise that the test settles when it chooses: a slow server. */
export function slowServer() {
  const { promise, resolve, reject } = Promise.withResolvers<string>();
  return { promise, answer: resolve, refuse: reject };
}

/** Lets what was started run as far as it can without an answer from the server. */
export function nextTick(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

/** What an undo does when the test does not look at it. */
export const doNothing = (): void => {
  // Nothing to take back in this test.
};

/** Timers that only move when the test says so. */
export function manualClock() {
  let now = 0;
  const waiting: { at: number; run: () => void; stopped: boolean }[] = [];
  const schedule: Schedule = (run, ms) => {
    const entry = { at: now + ms, run, stopped: false };
    waiting.push(entry);
    return () => {
      entry.stopped = true;
    };
  };
  return {
    schedule,
    now: () => now,
    advance: (ms: number) => {
      now += ms;
      for (const entry of waiting) {
        if (!entry.stopped && entry.at <= now) {
          entry.stopped = true;
          entry.run();
        }
      }
    },
  };
}
