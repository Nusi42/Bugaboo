import type { Schema } from "@bugaboo/api-client";

import { useNow } from "../../shared/use-now.ts";
import { useWording } from "../../shared/use-wording.ts";
import { elapsedMinutes } from "./timer-clock.ts";
import { useTimer } from "./use-timer.ts";

/** What the timer that runs shows of itself. */
export interface RunClock {
  /** On a clock, as a grid does (`0:42`): hours and minutes, no seconds; none until the shared wording is loaded. */
  readonly clock: string | undefined;
}

/** A start that is none, for the clock of a timer that does not run. */
const notRunning = { started_at: "" };

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
    return { clock: undefined };
  }
  return {
    clock: loaded.wording.lengthOnClock(elapsedMinutes(running.started_at, serverOffsetMs, now)),
  };
}

/** The person's timer that runs, if one does, and its clock: for a screen that shows the timer wherever it runs. */
export function useRunningClock(): {
  readonly running: Schema<"RunningItem"> | undefined;
  readonly clock: string | undefined;
} {
  const reading = useTimer().data;
  const running = reading?.timer.running ?? undefined;
  const { clock } = useRunClock(running ?? notRunning, reading?.serverOffsetMs ?? 0);
  return { running, clock };
}
