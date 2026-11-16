import { useEffect, useState } from "react";

import {
  askAboutConflict,
  conflictAsker,
  conflictPromptOf,
  type ConflictAsker,
} from "../../shared/editing/index.ts";
import type { Refused } from "../../shared/refused.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { useChangeBook } from "./change-book.ts";
import { keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import { changeDates, changedText, takeVersion } from "./dates.ts";
import type { BarDays } from "./lines.ts";
import { saveQueue, setTo, type Refusal, type SaveQueue } from "./save-queue.ts";
import { idOf, type TimelineRow } from "./timeline-data.ts";

/**
 * A change that somebody else's change came before (UX §4.7, as on the issue page): the bar stays
 * where the person put it while the screen asks *Keep Ben's change*, which shows the dates read
 * again, or *Use mine*, which saves the person's dates again on the version the other change made,
 * as a change of its own with *Undo*.
 */
function askAbout(
  refused: Refused,
  change: { readonly row: TimelineRow; readonly dates: KeptDates },
  screen: { readonly queue: () => SaveQueue<TimelineRow>; readonly asker: ConflictAsker },
): Refusal {
  const { queue } = screen;
  if ("reason" in refused) {
    return refused;
  }
  const { row, dates } = change;
  void conflictPromptOf(refused.conflict, {
    keepTheirs: () => {
      askAboutConflict(null);
      queue().show(idOf(row));
    },
    useMine: () => {
      askAboutConflict(null);
      takeVersion(row, refused.conflict.version ?? 0);
      const now = queue().standing(row);
      const step = now === undefined ? undefined : setTo(now, dates);
      if (step !== undefined) {
        queue().save(row, step, { move: "set" });
      }
    },
  }).then(screen.asker.ask);
  return { asked: true };
}

/**
 * The changes of dates of this screen (PLAN-03), saved through the queue of `save-queue.ts`, and
 * the days each bar shows while its changes are on their way. Without the two date fields of the
 * view no issue stands anywhere, so nothing is changed. A clash asks over the page until it is
 * answered, or until the screen goes.
 */
export function useSaving(fields: DateFields | undefined) {
  const [shown, setShown] = useState<Readonly<Record<string, BarDays>>>({});
  const book = useChangeBook();
  // What the screen asks goes with it; words that are ready after it went are not shown.
  const [asker] = useState(conflictAsker);
  useEffect(() => {
    asker.stay();
    return asker.leave;
  }, [asker]);
  const queue: SaveQueue<TimelineRow> = saveQueue(book, {
    idOf,
    send: async (row, dates) => {
      const refused = fields === undefined ? null : await changeDates(row, fields, dates);
      return refused === null
        ? null
        : askAbout(refused, { row, dates }, { queue: () => queue, asker });
    },
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
