import type { Schema } from "@bugaboo/api-client";

import { renderSaid } from "../../shared/said.ts";
import { useNow } from "../../shared/use-now.ts";
import { useWording } from "../../shared/use-wording.ts";
import { elapsedMinutes } from "./timer-clock.ts";

/** How long the timer has run, in the two ways the screens say it; none until the shared wording is loaded. */
export interface RunClock {
  /** On a clock, as a grid does (`0:42`): hours and minutes, no seconds. */
  readonly clock: string | undefined;
  /** In words, for a screen reader (`42 minutes`). */
  readonly words: string | undefined;
}

/**
 * How long a running timer has run, from the moment the server says it started, written by the shared
 * wording of `bb-core` (the same `h:mm` as the timesheet) and moved on every half minute.
 */
export function useRunClock(
  running: Pick<Schema<"RunningItem">, "started_at">,
  serverOffsetMs: number,
): RunClock {
  const now = useNow();
  const loaded = useWording();
  if (loaded.state !== "ready") {
    return { clock: undefined, words: undefined };
  }
  const minutes = elapsedMinutes(running.started_at, serverOffsetMs, now);
  return {
    clock: loaded.wording.lengthOnClock(minutes),
    words: renderSaid(loaded.wording.lengthSentence(minutes)),
  };
}
