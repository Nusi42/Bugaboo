import { t } from "@bugaboo/messages";

import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import { keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import { useLinks } from "./dependencies.ts";
import type { BarDays } from "./lines.ts";
import type { Move } from "./moves.ts";
import type { TimelineRow } from "./timeline-data.ts";
import { useKeyRuns } from "./use-key-runs.ts";
import { stepFrom, useSaving, type Step } from "./use-saving.ts";

/** The rows of the page and of the tray, by the ID of their issue. */
export type RowOf = (issue: string) => TimelineRow | undefined;

/** Says why a bar cannot be moved. */
function sayLocked(row: TimelineRow): void {
  undoStore.offer({ message: t("timeline.locked", { key: known(row.issue.readable_id) ?? "" }) });
}

/** The change of *Set dates…*; none without any date, which would send the issue to the tray. */
function setTo(before: KeptDates, dates: KeptDates): Step | undefined {
  const [first, last] = [dates.start ?? dates.end, dates.end ?? dates.start];
  return first === null || last === null ? undefined : { before, dates, days: { first, last } };
}

/** The change of an issue dropped on a day: it starts there, and keeps a due date that comes after. */
function placeOn(before: KeptDates, day: string): Step {
  const end = before.end !== null && before.end >= day ? before.end : null;
  return { before, dates: { start: day, end }, days: { first: day, last: end ?? day } };
}

/**
 * What the person does to bars (PLAN-02, PLAN-03): new dates, worked out from where the issue stands
 * on this screen, shown at once and saved one change after the other with *Undo*; the presses of a
 * run of keys are one change; a dependency; and the sentence that says why a bar cannot be moved.
 */
export function useBarChanges(fields: DateFields | undefined, rowOf: RowOf) {
  const saving = useSaving(fields);
  const keys = useKeyRuns(saving);
  const keptOf = (row: TimelineRow) => (fields === undefined ? undefined : keptDates(row, fields));
  return {
    daysOf: (issue: string, bar: BarDays | undefined): BarDays | undefined =>
      saving.shown[issue] ?? bar,
    onMove: (row: TimelineRow, move: Move, days: number): void => {
      const now = keys.standing(row);
      if (now !== undefined) {
        const said = move === "both" ? { move, days } : ({ move: "set" } as const);
        saving.save(row, stepFrom(now, move, days), said);
      }
    },
    onKeyMove: keys.onKeyMove,
    onRunPause: keys.onRunPause,
    onRunEnd: keys.onRunEnd,
    onSet: (row: TimelineRow, dates: KeptDates): void => {
      const before = keptOf(row);
      const step = before === undefined ? undefined : setTo(before, dates);
      if (step !== undefined) {
        saving.save(row, step, { move: "set" });
      }
    },
    onPlace: (issue: string, day: string): void => {
      const row = rowOf(issue);
      const before = row === undefined ? undefined : keptOf(row);
      if (row !== undefined && before !== undefined) {
        saving.save(row, placeOn(before, day), { move: "set" });
      }
    },
    onLocked: sayLocked,
    ...useLinks(rowOf),
  };
}
