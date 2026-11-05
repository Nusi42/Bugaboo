import { msPerSecond, secondsPerHour, secondsPerMinute } from "@bugaboo/api-client";

/** How many digits a part of a clock face has: `0:42`, `1:05:09`. */
const TWO_DIGITS = 2;

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

/** Whole minutes a timer has run now, which is what the server saves (rounded down); 0 without a start. */
export function minutesRun(startedAt: string | undefined, serverOffsetMs: number): number {
  return startedAt === undefined
    ? 0
    : Math.floor(elapsedSeconds(startedAt, serverOffsetMs, Date.now()) / secondsPerMinute);
}

/** The face of a clock for a number of seconds: `0:42` under an hour, `1:05:09` from an hour. */
export function clockText(seconds: number): string {
  const hours = Math.floor(seconds / secondsPerHour);
  const minutes = Math.floor((seconds % secondsPerHour) / secondsPerMinute);
  const rest = String(seconds % secondsPerMinute).padStart(TWO_DIGITS, "0");
  return hours === 0
    ? `${String(minutes)}:${rest}`
    : `${String(hours)}:${String(minutes).padStart(TWO_DIGITS, "0")}:${rest}`;
}
