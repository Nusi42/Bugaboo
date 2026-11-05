import { msPerSecond, secondsPerMinute } from "@bugaboo/api-client";

/**
 * How long a timer has run, in whole seconds, counted from the moment the server says it started
 * (never from the clock of this browser alone: `serverOffsetMs` is how far the server's clock is
 * from it), and never below zero.
 */
export function elapsedSeconds(startedAt: string, serverOffsetMs: number, nowMs: number): number {
  const started = Date.parse(startedAt);
  return Number.isNaN(started)
    ? 0
    : Math.max(0, Math.floor((nowMs + serverOffsetMs - started) / msPerSecond));
}

/** Whole minutes a timer has run at a moment, which is what the server saves (rounded down). */
export function elapsedMinutes(startedAt: string, serverOffsetMs: number, nowMs: number): number {
  return Math.floor(elapsedSeconds(startedAt, serverOffsetMs, nowMs) / secondsPerMinute);
}

/** Whole minutes a timer has run now; 0 without a start. */
export function minutesRun(startedAt: string | undefined, serverOffsetMs: number): number {
  return startedAt === undefined ? 0 : elapsedMinutes(startedAt, serverOffsetMs, Date.now());
}
