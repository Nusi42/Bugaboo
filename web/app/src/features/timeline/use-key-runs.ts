import { useEffect, useRef } from "react";

import { browserSchedule } from "../../shared/optimistic.ts";
import type { Said } from "./dates.ts";
import type { Move } from "./moves.ts";
import type { TimelineRow } from "./timeline-data.ts";
import { idOf, stepFrom, type Saving, type Standing, type Step } from "./use-saving.ts";

/** How long the keys of a bar rest before the run of presses is saved as one change (M8-F84). */
const keyRunPauseMs = 500;

/** A run of presses of the keys of one bar, not saved yet. */
interface Run extends Step {
  readonly row: TimelineRow;
  /** The days the whole bar moved while every press moved all of it; none once an end moved. */
  readonly moves: number | undefined;
  readonly stop?: () => void;
}

/** What a run says when it is saved: by how many days the bar moved, or only its new dates. */
function saidOf(run: Run): Said {
  return run.moves === undefined || run.moves === 0
    ? { move: "set" }
    : { move: "both", days: run.moves };
}

/** The run after one more press: it starts where the first press found the bar. */
function runAfter(
  run: Run | undefined,
  step: Step,
  row: TimelineRow,
  press: { readonly move: Move; readonly days: number },
): Run {
  const moves = run === undefined ? 0 : run.moves;
  return {
    ...step,
    row,
    before: run?.before ?? step.before,
    from: run === undefined ? step.from : run.from,
    moves: press.move === "both" && moves !== undefined ? moves + press.days : undefined,
  };
}

/** Saves a run that ended; one that came back to where it began saves nothing. */
function saveRun(saving: Readonly<Saving>, id: string, run: Run): void {
  saving.release(id);
  if (run.dates.start === run.before.start && run.dates.end === run.before.end) {
    saving.settle(id);
  } else {
    saving.save(run.row, run, saidOf(run));
  }
}

/**
 * The presses of the keys of a bar as one change (the maintainer, M8-F84): each press moves the bar
 * on screen at once; the change is saved when the keys rest for a moment or the bar loses the
 * focus, so it has one *Undo* and one line of history.
 */
export function useKeyRuns(saving: Readonly<Saving>) {
  const runs = useRef(new Map<string, Run>());
  const standing = (row: TimelineRow): Standing | undefined =>
    runs.current.get(idOf(row)) ?? saving.standing(row);
  const end = (id: string): void => {
    const run = runs.current.get(id);
    if (run !== undefined) {
      run.stop?.();
      runs.current.delete(id);
      saveRun(saving, id, run);
    }
  };
  return {
    standing,
    onKeyMove: (row: TimelineRow, move: Move, days: number): void => {
      const id = idOf(row);
      const now = standing(row);
      if (now === undefined) {
        return;
      }
      const run = runs.current.get(id);
      run?.stop?.();
      saving.hold(id);
      const step = stepFrom(now, move, days);
      runs.current.set(id, runAfter(run, step, row, { move, days }));
      saving.show(id, step.days);
    },
    onRunPause: (id: string): void => {
      const run = runs.current.get(id);
      if (run !== undefined) {
        run.stop?.();
        const stop = browserSchedule(() => {
          end(id);
        }, keyRunPauseMs);
        runs.current.set(id, { ...run, stop });
      }
    },
    onRunEnd: end,
    /** Saves every run that waits: the page is left before its keys rested. */
    endAll: (): void => {
      for (const id of runs.current.keys()) {
        end(id);
      }
    },
  };
}

/** Saves the runs of keys that wait when the page is hidden or left (`pagehide`). */
export function useSaveOnLeave(endAll: () => void): void {
  useEffect(() => {
    window.addEventListener("pagehide", endAll);
    return () => {
      window.removeEventListener("pagehide", endAll);
    };
  });
}
