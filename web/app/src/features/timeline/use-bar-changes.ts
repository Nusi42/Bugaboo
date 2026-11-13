import { keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import { sayLocked } from "./dates.ts";
import { useLinks } from "./dependencies.ts";
import type { BarDays } from "./lines.ts";
import type { Move } from "./moves.ts";
import { idOf, type TimelineRow } from "./timeline-data.ts";
import { useKeyRuns, useSaveOnLeave } from "./use-key-runs.ts";
import { placeOn, setTo, stepFrom } from "./save-queue.ts";
import { useSaving } from "./use-saving.ts";

/** The rows of the page and of the tray, by the ID of their issue. */
export type RowOf = (issue: string) => TimelineRow | undefined;

/**
 * What the person does to bars (PLAN-02, PLAN-03): new dates, worked out from where the issue stands
 * on this screen, shown at once and saved one change after the other with *Undo*; the presses of a
 * run of keys are one change, which a drag or a typed change of the same bar ends first; a
 * dependency; and the sentence that says why a bar cannot be moved.
 */
export function useBarChanges(fields: DateFields | undefined, rowOf: RowOf) {
  const saving = useSaving(fields);
  const keys = useKeyRuns(saving);
  useSaveOnLeave(keys.endAll);
  // A waiting run of keys is saved first, so the next change starts where it left the bar.
  const beforeOf = (row: TimelineRow) => {
    keys.onRunEnd(idOf(row));
    const now = keys.standing(row);
    const kept = fields === undefined ? undefined : keptDates(row, fields);
    return now ?? (kept === undefined ? undefined : { dates: kept, days: undefined });
  };
  return {
    daysOf: (issue: string, bar: BarDays | undefined): BarDays | undefined =>
      saving.shown[issue] ?? bar,
    onMove: (row: TimelineRow, move: Move, days: number): void => {
      const now = beforeOf(row);
      if (now?.days !== undefined) {
        const said = move === "both" ? { move, days } : ({ move: "set" } as const);
        saving.save(row, stepFrom({ dates: now.dates, days: now.days }, move, days), said);
      }
    },
    onKeyMove: keys.onKeyMove,
    onRunPause: keys.onRunPause,
    onRunEnd: keys.onRunEnd,
    onSet: (row: TimelineRow, dates: KeptDates): void => {
      const before = beforeOf(row);
      const step = before === undefined ? undefined : setTo(before, dates);
      if (step !== undefined) {
        saving.save(row, step, { move: "set" });
      }
    },
    onPlace: (issue: string, day: string): void => {
      const row = rowOf(issue);
      const before = row === undefined ? undefined : beforeOf(row);
      if (row !== undefined && before !== undefined) {
        saving.save(row, placeOn(before, day), { move: "set" });
      }
    },
    onLocked: sayLocked,
    ...useLinks(rowOf),
  };
}
