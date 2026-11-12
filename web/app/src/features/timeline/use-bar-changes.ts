import { t } from "@bugaboo/messages";
import { useState } from "react";

import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import { keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import { changeDates, moved, shownWhileMoving, type Move, type Said } from "./dates.ts";
import { addDependency, useDependencyKind, type Pair } from "./dependencies.ts";
import type { BarDays } from "./lines.ts";
import type { TimelineRow } from "./timeline-data.ts";

/** The rows of the page and of the tray, by the ID of their issue. */
export type RowOf = (issue: string) => TimelineRow | undefined;

/** A change of dates: what is kept after it, what the bar shows meanwhile, and what is said. */
interface DatesChange {
  readonly dates: KeptDates;
  readonly days: BarDays;
  readonly said: Said;
}

/** The change a drag or a key makes: the dates it keeps, the days it shows, the sentence. */
function moveOf(
  row: TimelineRow,
  fields: DateFields,
  bar: BarDays,
  move: Move,
  days: number,
): DatesChange {
  const after = shownWhileMoving(bar, move, days);
  return {
    dates: moved(keptDates(row, fields), bar, move, days),
    days: after,
    said: move === "both" ? { move, days } : { move: "set", days: after },
  };
}

/** The change of an issue dropped on a day: it starts there, and keeps a due date that comes after. */
function placeOn(row: TimelineRow, fields: DateFields, day: string): DatesChange {
  const kept = keptDates(row, fields).end;
  const end = kept !== null && kept >= day ? kept : null;
  const days = { first: day, last: end ?? day };
  return { dates: { start: day, end }, days, said: { move: "set", days } };
}

/** The change *Set dates…* makes; none without any date, which takes the bar to the tray. */
function setTo(dates: KeptDates): DatesChange | undefined {
  const [first, last] = [dates.start ?? dates.end, dates.end ?? dates.start];
  if (first === null || last === null) {
    return undefined;
  }
  const days = { first, last };
  return { dates, days, said: { move: "set", days } };
}

/** Says why a bar cannot be moved. */
function sayLocked(row: TimelineRow): void {
  undoStore.offer({ message: t("timeline.locked", { key: known(row.issue.readable_id) ?? "" }) });
}

/** Makes a dependency between two issues by their readable IDs, of the kind with *Must finish first*. */
function pairWith(kind: string | undefined) {
  return (pair: Pair): void => {
    if (kind === undefined) {
      undoStore.offer({ message: t("timeline.no-dependency-kind") });
    } else {
      void addDependency(kind, pair);
    }
  };
}

/** The same for two issues of the page by their IDs, as a drag of the dot gives them. */
function linkWith(pair: (pair: Pair) => void, rowOf: RowOf) {
  return (first: string, waiting: string): void => {
    const [from, to] = [
      known(rowOf(first)?.issue.readable_id),
      known(rowOf(waiting)?.issue.readable_id),
    ];
    if (from !== undefined && to !== undefined) {
      pair({ first: from, waiting: to });
    }
  };
}

/**
 * What the person does to bars (PLAN-02, PLAN-03): new dates, saved one change after the other
 * with *Undo*, shown on the bar at once until the server answers with its plan; a dependency; and
 * the sentence that says why a bar cannot be moved.
 */
export function useBarChanges(fields: DateFields | undefined, rowOf: RowOf) {
  const onPair = pairWith(useDependencyKind());
  const [shown, setShown] = useState<Readonly<Record<string, BarDays>>>({});
  const apply = (row: TimelineRow, change: DatesChange | undefined): void => {
    const issue = known(row.issue.id) ?? "";
    if (fields === undefined || change === undefined) {
      return;
    }
    setShown((before) => ({ ...before, [issue]: change.days }));
    void changeDates(row, fields, change.dates, change.said).finally(() => {
      setShown(({ [issue]: _done, ...others }) => others);
    });
  };
  return {
    daysOf: (issue: string, bar: BarDays | undefined): BarDays | undefined => shown[issue] ?? bar,
    onMove: (row: TimelineRow, move: Move, days: number): void => {
      const bar = shown[known(row.issue.id) ?? ""] ?? row.bar?.days;
      if (fields !== undefined && bar !== undefined) {
        apply(row, moveOf(row, fields, bar, move, days));
      }
    },
    onSet: (row: TimelineRow, dates: KeptDates): void => {
      apply(row, setTo(dates));
    },
    onPlace: (issue: string, day: string): void => {
      const row = rowOf(issue);
      if (row !== undefined && fields !== undefined) {
        apply(row, placeOn(row, fields, day));
      }
    },
    onLocked: sayLocked,
    onLink: linkWith(onPair, rowOf),
    onPair,
  };
}
