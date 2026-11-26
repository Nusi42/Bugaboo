import { part, partOf } from "@bugaboo/ui";

import { known } from "../../shared/present.ts";
import type { CalendarEntry } from "./calendar-data.ts";

/**
 * Where an issue carried with the keys would stand: a dashed place on the day it is over, with its ID, so
 * the day shows where it goes while the issue stays where it is.
 */
export function Carried({ entry }: { readonly entry: CalendarEntry }) {
  return (
    <li {...partOf(part.calendarItem, { states: ["ghost"] })} aria-hidden>
      {known(entry.issue.readable_id) ?? ""}
    </li>
  );
}
