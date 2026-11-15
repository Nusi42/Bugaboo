import { t } from "@bugaboo/messages";
import { part, partOf, SelectField, selectLook } from "@bugaboo/ui";

import { listWord } from "../../shared/list-columns.ts";
import type { ListView } from "../../shared/list-view.ts";
import type { OwnColumns } from "../issues/index.ts";
import { dateWords } from "./group-words.ts";

export interface DatesFromProps {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
  /** The names of the fields the person may read, by their words. */
  readonly names: Readonly<Record<string, string>>;
  readonly own: OwnColumns;
}

/**
 * *Dates from* (UX §4.7, ADR-0043, decision 2): the date fields the bars start at and end at, *Start
 * date* and *Due date* or own date fields, kept with the view. Both are always named together, as one
 * choice that reads "Dates from Start date to Due date"; a screen reader hears each list by what it
 * chooses (*Bars start at*, *Bars end at*).
 */
export function DatesFrom({ view, onView, names, own }: DatesFromProps) {
  const { timeline } = view;
  const words = dateWords(own).map((word) => ({ value: word, label: names[word] ?? word }));
  const [start, end] = [
    timeline.start_field ?? listWord.startDate,
    timeline.end_field ?? listWord.dueDate,
  ];
  // *Start date* and *Due date* are the view's own when it names no fields, as a saved search keeps them.
  const withDates = (first: string, last: string): void => {
    const usual = first === listWord.startDate && last === listWord.dueDate;
    onView({
      ...view,
      timeline: {
        ...timeline,
        start_field: usual ? null : first,
        end_field: usual ? null : last,
      },
    });
  };
  const choice = (label: string, value: string, onChange: (word: string) => void) => (
    <SelectField
      label={label}
      hideLabel
      look={selectLook.inline}
      choices={words}
      value={value}
      onChange={onChange}
    />
  );
  return (
    <div {...partOf(part.field, { variants: [selectLook.inline] })}>
      <span {...partOf(part.fieldLabel)}>{t("timeline.dates-from")}</span>
      {choice(t("timeline.start-field"), start, (word) => {
        withDates(word, end);
      })}
      <span {...partOf(part.fieldLabel)}>{t("timeline.dates-to")}</span>
      {choice(t("timeline.end-field"), end, (word) => {
        withDates(start, word);
      })}
    </div>
  );
}
