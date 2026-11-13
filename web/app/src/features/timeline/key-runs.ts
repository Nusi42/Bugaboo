import type { Schedule } from "@bugaboo/ui";
import type { Said } from "./dates.ts";
import type { Move } from "./moves.ts";
import { stepFrom, type SaveQueue, type Step } from "./save-queue.ts";

/** How long the keys of a bar rest before the run of presses is saved as one change (M8-F84). */
const keyRunPauseMs = 500;

/** A run of presses of the keys of one bar, not saved yet. */
interface Run<Row> extends Step {
  readonly row: Row;
  /** The days the whole bar moved while every press moved all of it; none once an end moved. */
  readonly moves: number | undefined;
  readonly stop?: () => void;
}

/** The runs of keys that wait, by the ID of their issue: read and written only through these. */
export function runBook<Row>() {
  const runs = new Map<string, Run<Row>>();
  return {
    get: (id: string): Run<Row> | undefined => runs.get(id),
    set: (id: string, run: Run<Row>): void => {
      runs.set(id, run);
    },
    drop: (id: string): void => {
      runs.delete(id);
    },
    ids: (): readonly string[] => [...runs.keys()],
  };
}

/** The runs, as the keys read them. */
export type RunBook<Row> = Readonly<ReturnType<typeof runBook<Row>>>;

/** What a run says when it is saved: by how many days the bar moved, or only its new dates. */
function saidOf<Row>(run: Run<Row>): Said {
  return run.moves === undefined || run.moves === 0
    ? { move: "set" }
    : { move: "both", days: run.moves };
}

/** The run after one more press: it starts where the first press found the bar. */
function runAfter<Row>(
  run: Run<Row> | undefined,
  step: Step,
  row: Row,
  press: { readonly move: Move; readonly days: number },
): Run<Row> {
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
function saveRun<Row>(saving: Readonly<SaveQueue<Row>>, id: string, run: Run<Row>): void {
  saving.release(id);
  if (run.dates.start === run.before.start && run.dates.end === run.before.end) {
    saving.settle(id);
  } else {
    saving.save(run.row, run, saidOf(run));
  }
}

/** Ends the run of an issue, if one waits, and saves it. */
function endRun<Row>(runs: RunBook<Row>, saving: Readonly<SaveQueue<Row>>, id: string): void {
  const run = runs.get(id);
  if (run !== undefined) {
    run.stop?.();
    runs.drop(id);
    saveRun(saving, id, run);
  }
}

/**
 * The presses of the keys of a bar as one change (the maintainer, M8-F84): each press moves the bar
 * on screen at once; the change is saved when the keys rest for a moment or the bar loses the
 * focus, so it has one *Undo* and one line of history. A run is a newer change of its issue: its
 * first press withdraws the *Undo* of the change before.
 */
export function keyRuns<Row>(
  runs: RunBook<Row>,
  saving: Readonly<SaveQueue<Row>>,
  schedule: Schedule,
) {
  const standing = (row: Row) => runs.get(saving.idOf(row)) ?? saving.standing(row);
  const end = (id: string): void => {
    endRun(runs, saving, id);
  };
  return {
    standing,
    onKeyMove: (row: Row, move: Move, days: number): void => {
      const id = saving.idOf(row);
      const now = standing(row);
      if (now === undefined) {
        return;
      }
      const run = runs.get(id);
      run?.stop?.();
      if (run === undefined) {
        saving.withdraw(id);
      }
      saving.hold(id);
      const step = stepFrom(now, move, days);
      runs.set(id, runAfter(run, step, row, { move, days }));
      saving.show(id, step.days);
    },
    onRunPause: (id: string): void => {
      const run = runs.get(id);
      if (run !== undefined) {
        run.stop?.();
        const stop = schedule(() => {
          end(id);
        }, keyRunPauseMs);
        runs.set(id, { ...run, stop });
      }
    },
    onRunEnd: end,
    /** Saves every run that waits: the page is left before its keys rested. */
    endAll: (): void => {
      for (const id of runs.ids()) {
        end(id);
      }
    },
  };
}
