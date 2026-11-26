import type { CalendarEntry } from "./calendar-data.ts";
import { useCalendar } from "./calendar-context.ts";
import type { Mark } from "./marks.ts";

/**
 * A line of a day: the place of an issue carried with the keys (the last), a mark, or an issue.
 * They all count toward the lines a day has, so a day never grows.
 */
export type Line =
  | { readonly carried: CalendarEntry }
  | { readonly mark: Mark }
  | { readonly entry: CalendarEntry };

/** The lines of a day, marks first, then issues in the order of the search. */
export function useLinesOf(day: string): readonly Line[] {
  const { model } = useCalendar();
  const { picked } = model.pick;
  const carried =
    picked !== undefined && picked.day === day && model.moves.dayOf(picked.entry) !== day
      ? [{ carried: picked.entry }]
      : [];
  return [
    ...model.marksOn(day).map((mark) => ({ mark })),
    ...model.entriesOn(day).map((entry) => ({ entry })),
    ...carried,
  ];
}
