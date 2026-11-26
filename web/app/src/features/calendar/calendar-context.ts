import { createContext, useContext } from "react";

import type { CalendarEntry } from "./calendar-data.ts";
import type { CalendarModel } from "./calendar-model.ts";

/** What the parts of the calendar share: the model, what opens a window of an issue, and the words that describe its keys. */
export interface CalendarContextValue {
  readonly model: CalendarModel;
  /** *Change due date …* of an issue: opens its window. */
  readonly onChangeDay: (entry: CalendarEntry) => void;
  /** *+5 more* of a day: opens the list of its issues. */
  readonly onOpenDay: (day: string) => void;
  /** The IDs of the words that say how the keys move an issue, and why one cannot be moved. */
  readonly described: { readonly keys: string; readonly locked: string };
}

export const CalendarContext = createContext<CalendarContextValue | undefined>(undefined);

/** The calendar the part is in. */
export function useCalendar(): CalendarContextValue {
  const value = useContext(CalendarContext);
  if (value === undefined) {
    throw new TypeError("A part of the calendar stands outside it.");
  }
  return value;
}
