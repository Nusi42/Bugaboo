import { useState } from "react";

import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import { useChangeBook, type ChangeBook } from "./change-book.ts";
import { keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import { changeDates, changedText, type Said } from "./dates.ts";
import type { BarDays } from "./lines.ts";
import { moved, shownWhileMoving, type Move } from "./moves.ts";
import type { TimelineRow } from "./timeline-data.ts";

/** Where the dates of an issue stand on this screen: what it keeps and what its bar shows. */
export interface Standing {
  readonly dates: KeptDates;
  readonly days: BarDays;
}

/** A step of a bar from where it stands: the dates and days before, the dates and days after. */
export interface Step extends Standing {
  readonly before: KeptDates;
  /** The days the bar showed before; none for an issue that had no bar. */
  readonly from: BarDays | undefined;
}

/** The ID of the issue of a row. */
export function idOf(row: TimelineRow): string {
  return known(row.issue.id) ?? "";
}

/** The step a move makes from where a bar stands. */
export function stepFrom(now: Standing, move: Move, days: number): Step {
  return {
    before: now.dates,
    from: now.days,
    dates: moved(now.dates, now.days, move, days),
    days: shownWhileMoving(now.days, move, days),
  };
}

/** The step back of a change: from where the issue stands now to the dates before the change. */
function stepBack(now: Standing, change: Step): Step {
  return {
    before: now.dates,
    from: now.days,
    dates: change.before,
    days: change.from ?? change.days,
  };
}

/** A change has ended: the bar shows the server's days when it was the last and no run holds it. */
function ended(book: ChangeBook, id: string, show: (id: string) => void): void {
  const now = book.get(id);
  if (now !== undefined && now.count > 1) {
    book.put(id, { ...now, count: now.count - 1 });
    return;
  }
  book.forget(id);
  if (!book.holds(id)) {
    show(id);
  }
}

/** A change that was saved, and its number among the changes of its issue. */
interface Saved {
  readonly row: TimelineRow;
  readonly step: Step;
  readonly said: Said;
  readonly newest: number;
}

/** Offers *Undo* of a saved change while it is the newest change of its issue. */
function offerUndo(book: ChangeBook, saved: Saved, undo: () => void): void {
  const id = idOf(saved.row);
  const now = book.get(id);
  if (now?.newest === saved.newest) {
    book.setOffer(
      id,
      undoStore.offer({ message: changedText(saved.row, saved.said, saved.step.dates), undo }),
    );
  }
}

/**
 * The changes of dates on their way (PLAN-03): each is shown on its bar at once and saved one after
 * the other. A new change starts from the dates the last one saved or is saving, never from the
 * page read before; *Undo* is a change like the others, in the same queue; an *Undo* on offer is
 * withdrawn when a newer change of the same issue starts, so it never puts back old dates over new
 * ones; and a bar shows the server's days again only when its last change has ended and no run of
 * keys holds it.
 */
export function useSaving(fields: DateFields | undefined) {
  const [shown, setShown] = useState<Readonly<Record<string, BarDays>>>({});
  const book = useChangeBook();
  const show = (id: string, days?: BarDays): void => {
    setShown(({ [id]: _before, ...others }) =>
      days === undefined ? others : { ...others, [id]: days },
    );
  };
  const standing = (row: TimelineRow): Standing | undefined =>
    book.get(idOf(row)) ??
    (fields === undefined || row.bar === null
      ? undefined
      : { dates: keptDates(row, fields), days: row.bar.days });
  function save(row: TimelineRow, step: Step, said: Said): void {
    if (fields === undefined) {
      return;
    }
    const id = idOf(row);
    const before = book.get(id);
    const offered = book.offerOf(id);
    if (offered !== undefined) {
      undoStore.withdraw(offered);
    }
    const newest = (before?.newest ?? 0) + 1;
    book.put(id, { dates: step.dates, days: step.days, count: (before?.count ?? 0) + 1, newest });
    show(id, step.days);
    void changeDates(row, fields, step.dates).then((refused) => {
      if (refused === null) {
        offerUndo(book, { row, step, said, newest }, () => {
          save(row, stepBack(standing(row) ?? step, step), { move: "set" });
        });
      } else {
        undoStore.offer({ message: refused });
      }
      ended(book, id, show);
    });
  }
  return {
    shown,
    show,
    standing,
    save,
    hold: book.hold,
    release: book.release,
    /** Shows the days of the issue's change on its way, or the server's when none is. */
    settle: (id: string): void => {
      show(id, book.get(id)?.days);
    },
  };
}

/** What the screen saves through. */
export type Saving = ReturnType<typeof useSaving>;
