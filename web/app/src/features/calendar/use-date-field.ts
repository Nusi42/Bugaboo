import { listWord } from "../../shared/list-columns.ts";
import { dateFieldOfWord, useOwnColumns } from "../issues/index.ts";
import type { CalendarInput } from "./calendar-input.ts";
import { dateWordsOf } from "./date-words.ts";

/** The date field the calendar stands its issues on (*Dates from*), and what it is called; none while an own field is not known yet. */
export function useDateField({ view, language, fields }: CalendarInput) {
  const own = useOwnColumns(language, true);
  const word = view.calendar.dates_field;
  return {
    own,
    field: dateFieldOfWord(word ?? listWord.dueDate, own),
    words: dateWordsOf(word, fields.names),
  };
}
