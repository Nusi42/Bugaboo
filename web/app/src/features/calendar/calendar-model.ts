import type { DateField, OwnColumns } from "../issues/index.ts";
import type { CalendarEntry } from "./calendar-data.ts";
import type { DateWords } from "./date-words.ts";
import type { Mark } from "./marks.ts";
import type { Window } from "./span-days.ts";
import type { DayMoves } from "./use-day-moves.ts";
import type { PickUp } from "./use-pick-up.ts";

/** What the calendar draws and does. */
export interface CalendarModel {
  /** The day of today, in the time zone of the person. */
  readonly today: string;
  /** The days on screen, and *Earlier*, *Later* and *Today* through them. */
  readonly window: Window;
  readonly onStep: (direction: -1 | 1) => void;
  readonly onToday: () => void;
  readonly own: OwnColumns;
  /** The date field the issues stand on; none while an own field of the view is not known yet. */
  readonly field: DateField | undefined;
  readonly words: DateWords;
  readonly moves: DayMoves;
  readonly pick: PickUp;
  readonly entryOf: (id: string) => CalendarEntry | undefined;
  readonly isPending: boolean;
  /** The days hold more issues than the server answers. */
  readonly more: boolean;
  readonly entriesOn: (day: string) => readonly CalendarEntry[];
  readonly marksOn: (day: string) => readonly Mark[];
}
