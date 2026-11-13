import { useRef } from "react";

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
    if (run === undefined) {
      return;
    }
    run.stop?.();
    runs.current.delete(id);
    if (run.dates.start === run.before.start && run.dates.end === run.before.end) {
      saving.show(id);
    } else {
      saving.save(run.row, run, saidOf(run));
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
      const step = stepFrom(now, move, days);
      const moves = run === undefined ? 0 : run.moves;
      runs.current.set(id, {
        ...step,
        row,
        before: run?.before ?? step.before,
        moves: move === "both" && moves !== undefined ? moves + days : undefined,
      });
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
  };
}
