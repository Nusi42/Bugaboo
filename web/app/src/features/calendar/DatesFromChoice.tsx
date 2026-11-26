import { t } from "@bugaboo/messages";
import { SelectField, selectLook } from "@bugaboo/ui";

import { listWord } from "../../shared/list-columns.ts";
import type { ListView } from "../../shared/list-view.ts";
import { ownDateWords, type OwnColumns } from "../issues/index.ts";

export interface DatesFromChoiceProps {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
  /** The names of the fields the person may read, by their words. */
  readonly names: Readonly<Record<string, string>>;
  readonly own: OwnColumns;
}

/**
 * *Dates from* (UX §4.16, M10-Q5): the date field the issues stand on, *Due date* at the start, or *Start
 * date*, or an own date field, kept with the view like *Dates from* of the timeline.
 */
export function DatesFromChoice({ view, onView, names, own }: DatesFromChoiceProps) {
  const { calendar } = view;
  const words = [listWord.dueDate, listWord.startDate, ...ownDateWords(own)];
  return (
    <SelectField
      label={t("calendar.dates-from")}
      look={selectLook.inline}
      choices={words.map((word) => ({ value: word, label: names[word] ?? word }))}
      value={calendar.dates_field ?? listWord.dueDate}
      onChange={(word) => {
        // *Due date* is the view's own when it names no field, as a saved search keeps it.
        onView({
          ...view,
          calendar: { ...calendar, dates_field: word === listWord.dueDate ? null : word },
        });
      }}
    />
  );
}
