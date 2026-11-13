import { useState } from "react";

import { undoStore } from "../../shared/optimistic-undo.ts";
import { useChangeBook } from "./change-book.ts";
import { keptDates, type DateFields } from "./date-fields.ts";
import { changeDates, changedText } from "./dates.ts";
import type { BarDays } from "./lines.ts";
import { saveQueue } from "./save-queue.ts";
import { idOf } from "./timeline-data.ts";

/**
 * The changes of dates of this screen (PLAN-03), saved through the queue of `save-queue.ts`, and
 * the days each bar shows while its changes are on their way. Without the two date fields of the
 * view no issue stands anywhere, so nothing is changed.
 */
export function useSaving(fields: DateFields | undefined) {
  const [shown, setShown] = useState<Readonly<Record<string, BarDays>>>({});
  const book = useChangeBook();
  const queue = saveQueue(book, {
    idOf,
    send: (row, dates) =>
      fields === undefined ? Promise.resolve(null) : changeDates(row, fields, dates),
    text: changedText,
    show: (id, days) => {
      setShown(({ [id]: _before, ...others }) =>
        days === undefined ? others : { ...others, [id]: days },
      );
    },
    kept: (row) =>
      fields === undefined || row.bar === null
        ? undefined
        : { dates: keptDates(row, fields), days: row.bar.days },
    offers: undoStore,
  });
  return { shown, ...queue };
}

/** What the screen saves through. */
export type Saving = ReturnType<typeof useSaving>;
