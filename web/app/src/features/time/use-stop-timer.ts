import { useState } from "react";

import { entryDetails, startingValues, type EntryValues } from "./entry-values.ts";
import { closeTimeAsk } from "./time-asks.ts";
import { minutesRun } from "./timer-clock.ts";
import { stopTimer, switchTimer } from "./timer-requests.ts";
import { tellSaved } from "./timer-saved.ts";
import { problemOf, type EntryProblem } from "./use-add-time.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";
import type { TimerReading } from "./use-timer.ts";

export interface StopTimerOptions {
  /** The issue the timer runs on. */
  readonly issue: string;
  /** Another issue to go on with after saving: the time so far needed this window to fill in what is missing. */
  readonly switchingTo: string | undefined;
  readonly defaults: EntryDefaults;
}

/**
 * What the window that stops the timer holds: the values of the entry (the defaults of the issue at
 * first), the whole minutes run when it opened, and the save, which stops the timer with them (or
 * switches it to another issue), tells what came of it and closes the window. The problem is what
 * the server refused.
 */
export function useStopTimer(reading: TimerReading, options: StopTimerOptions) {
  const [values, setValues] = useState<EntryValues>(() => startingValues(options.defaults));
  const [minutes] = useState(() =>
    minutesRun(reading.timer.running?.started_at, reading.serverOffsetMs),
  );
  const [problem, setProblem] = useState<EntryProblem | undefined>();
  const [isPending, setPending] = useState(false);
  const save = async (): Promise<void> => {
    setPending(true);
    setProblem(undefined);
    try {
      const details = entryDetails(values, options.defaults);
      const { version } = reading.timer;
      const saved =
        options.switchingTo === undefined
          ? await stopTimer(version, details)
          : await switchTimer(version, options.switchingTo, details);
      tellSaved(saved, options.issue);
      closeTimeAsk();
    } catch (error) {
      setProblem(problemOf(error));
    } finally {
      setPending(false);
    }
  };
  return { values, setValues, minutes, save, problem, isPending };
}
