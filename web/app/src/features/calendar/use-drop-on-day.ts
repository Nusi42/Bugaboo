import type { RefObject } from "react";

import { dragKinds } from "../../shared/drag-kinds.ts";
import { useDropOf } from "../../shared/drop-of.ts";
import type { CalendarEntry } from "./calendar-data.ts";

/** What a day needs of the calendar to take an issue dropped on it. */
interface Dropping {
  readonly entryOf: (id: string) => CalendarEntry | undefined;
  readonly moves: { readonly move: (entry: CalendarEntry, day: string) => void };
}

/** Makes the element of a day a place where an issue of the calendar can be dropped: it moves to that day. */
export function useDropOnDay(
  day: string,
  ref: Readonly<RefObject<HTMLElement | null>>,
  model: Dropping,
) {
  return useDropOf(dragKinds.calendarIssue.kind, ref, true, (id) => {
    const entry = model.entryOf(id);
    if (entry !== undefined) {
      model.moves.move(entry, day);
    }
  });
}
