import type { RecordRef } from "@bugaboo/api-client";

import { isNoAnswer } from "../errors.ts";

/** A record on the screen and the version the screen has of it. */
export interface Watched {
  readonly record: RecordRef;
  readonly version: number;
}

/** The version the server has now of a record asked about; `null` when it is gone or not for the person. */
export interface Found {
  readonly record: RecordRef;
  readonly version?: number | null | undefined;
}

/**
 * The records whose version on the server is newer than the one on the screen (ADR-0019): somebody
 * else changed them. A record that is gone or that the person may not know about is not a change
 * the screen can show, so it is left out.
 */
export function newerThan(watched: readonly Watched[], found: readonly Found[]): Watched[] {
  return watched.flatMap((one, at) => {
    const now = found[at]?.version;
    return now !== undefined && now !== null && now > one.version
      ? [{ record: one.record, version: now }]
      : [];
  });
}

/** Coming back to a tab fires several events together; one check answers all of them. */
const ONE_CHECK_WITHIN_MS = 1000;

/** Whether the last check was so near `now` (both in milliseconds) that another one is not needed. */
export function checkedJustNow(lastCheck: number, now: number): boolean {
  return now - lastCheck < ONE_CHECK_WITHIN_MS;
}

/** The events of the window after which the screen asks at once: the person comes back. */
const comingBack = ["focus", "online"] as const;

/** What the check of versions needs from outside, so it can be tested without a browser. */
export interface WatchEnv {
  /** Calls `run` for each event of the window with this name; returns the way to stop. */
  readonly onWindow: (name: string, run: () => void) => () => void;
  /** Calls `run` for each event of the document with this name; returns the way to stop. */
  readonly onDocument: (name: string, run: () => void) => () => void;
  /** Whether the tab is hidden. */
  readonly hidden: () => boolean;
  /** Calls `run` every `seconds`; returns the way to stop. */
  readonly repeat: (seconds: number, run: () => void) => () => void;
  readonly now: () => number;
  /** The versions the server has now of the records. */
  readonly ask: (records: readonly RecordRef[]) => Promise<readonly Found[]>;
}

/**
 * Checks the versions of the watched records every `seconds` and when the person comes back to
 * the tab (ADR-0019): one small request asks for all of them, and one return sends one request
 * though several events come with it. `onNewer` is told which records somebody else changed.
 * Returns the way to stop.
 */
export function startWatching(
  env: WatchEnv,
  watched: readonly Watched[],
  seconds: number,
  onNewer: (newer: readonly Watched[]) => void,
): () => void {
  let lastCheck = Number.NEGATIVE_INFINITY;
  const check = (): void => {
    if (watched.length === 0 || env.hidden() || checkedJustNow(lastCheck, env.now())) {
      return;
    }
    lastCheck = env.now();
    void env.ask(watched.map((one) => one.record)).then(
      (found) => {
        const newer = newerThan(watched, found);
        if (newer.length > 0) {
          onNewer(newer);
        }
      },
      (error: unknown) => {
        // No answer now (offline, signed out): the next check tries again. A fault of the program
        // is not "no answer" and is not hidden.
        if (!isNoAnswer(error)) {
          throw error;
        }
      },
    );
  };
  const stops = [
    ...(watched.length === 0 ? [] : [env.repeat(seconds, check)]),
    ...comingBack.map((name) => env.onWindow(name, check)),
    env.onDocument("visibilitychange", check),
  ];
  return () => {
    for (const stop of stops) {
      stop();
    }
  };
}
