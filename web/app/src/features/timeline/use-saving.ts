import { useRef, useState } from "react";

import { known } from "../../shared/present.ts";
import { keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import { changeDates, type Said } from "./dates.ts";
import type { BarDays } from "./lines.ts";
import { moved, shownWhileMoving, type Move } from "./moves.ts";
import type { TimelineRow } from "./timeline-data.ts";

/** Where the dates of an issue stand on this screen: what it keeps and what its bar shows. */
export interface Standing {
  readonly dates: KeptDates;
  readonly days: BarDays;
}

/** A step of a bar from where it stands: the dates before, the dates and days after. */
export interface Step extends Standing {
  readonly before: KeptDates;
}

/** The ID of the issue of a row. */
export function idOf(row: TimelineRow): string {
  return known(row.issue.id) ?? "";
}

/** The step a move makes from where a bar stands. */
export function stepFrom(now: Standing, move: Move, days: number): Step {
  return {
    before: now.dates,
    dates: moved(now.dates, now.days, move, days),
    days: shownWhileMoving(now.days, move, days),
  };
}

/**
 * The changes of dates on their way (PLAN-03): each is shown on its bar at once and saved one after
 * the other. A new change starts from the dates the last one saved or is saving, never from the
 * page read before, and a bar shows the server's days again only when its last change has ended.
 */
export function useSaving(fields: DateFields | undefined) {
  const [shown, setShown] = useState<Readonly<Record<string, BarDays>>>({});
  const waiting = useRef(new Map<string, Standing & { readonly count: number }>());
  const show = (id: string, days?: BarDays): void => {
    setShown(({ [id]: _before, ...others }) =>
      days === undefined ? others : { ...others, [id]: days },
    );
  };
  const ended = (id: string): void => {
    const now = waiting.current.get(id);
    if (now === undefined || now.count <= 1) {
      waiting.current.delete(id);
      show(id);
    } else {
      waiting.current.set(id, { ...now, count: now.count - 1 });
    }
  };
  return {
    shown,
    show,
    /** Where a bar stands: its last change on the way, or the page as read. */
    standing: (row: TimelineRow): Standing | undefined =>
      waiting.current.get(idOf(row)) ??
      (fields === undefined || row.bar === null
        ? undefined
        : { dates: keptDates(row, fields), days: row.bar.days }),
    save: (row: TimelineRow, step: Step, said: Said): void => {
      if (fields === undefined) {
        return;
      }
      const id = idOf(row);
      const count = (waiting.current.get(id)?.count ?? 0) + 1;
      waiting.current.set(id, { dates: step.dates, days: step.days, count });
      show(id, step.days);
      void changeDates(row, fields, { before: step.before, dates: step.dates, said }).finally(
        () => {
          ended(id);
        },
      );
    },
  };
}

/** What the screen saves through. */
export type Saving = ReturnType<typeof useSaving>;
