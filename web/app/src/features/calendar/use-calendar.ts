import { idOf, useCalendarRead } from "./calendar-data.ts";
import type { CalendarInput } from "./calendar-input.ts";
import type { CalendarModel } from "./calendar-model.ts";
import { entriesByDay } from "./entries-by-day.ts";
import { useDateField } from "./use-date-field.ts";
import { useDayMoves } from "./use-day-moves.ts";
import { useMarks } from "./use-marks.ts";
import { usePickUp } from "./use-pick-up.ts";
import { useShownDays } from "./use-shown-days.ts";

export type { CalendarInput } from "./calendar-input.ts";
export type { CalendarModel } from "./calendar-model.ts";

/**
 * Everything the calendar draws (UX §4.16), from what the server answers: the issues of the days on
 * screen, each on its day, the marks of goals and releases, and what a person does to an issue. No
 * date rule is worked out here.
 */
export function useCalendarModel(input: CalendarInput): CalendarModel {
  const days = useShownDays(input);
  const date = useDateField(input);
  const read = useCalendarRead({
    text: input.text,
    within: input.within,
    options: input.view.calendar,
    first: days.window.first,
    last: days.window.last,
  });
  const moves = useDayMoves(date);
  const pick = usePickUp(moves);
  const all = read.data?.entries ?? [];
  const entries = entriesByDay(all, moves.dayOf);
  const marks = useMarks(input.within, read.data?.marks ?? [], days.window);
  return {
    ...days,
    ...date,
    moves,
    pick,
    entryOf: (id) => all.find((entry) => idOf(entry) === id),
    isPending: read.isPending,
    more: read.data?.more ?? false,
    entriesOn: (day) => entries.get(day) ?? [],
    marksOn: (day) => marks.get(day) ?? [],
  };
}
