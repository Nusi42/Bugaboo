import { listWord } from "../../shared/list-columns.ts";
import { known } from "../../shared/present.ts";
import type { IssueRequestValues } from "./issue-values.ts";
import type { OwnColumns } from "./own-columns.ts";

/**
 * A date of an issue that a screen of dates draws and changes (ADR-0043, decision 2): *Start date*,
 * *Due date* or an own date field. The timeline and the calendar both read and write a date through
 * these three functions, so a day is kept in one place by one rule (ODR).
 */

/** Where a date of the view is kept on an issue: a built-in field, or an own field by its ID. */
export type DateField =
  | { readonly kind: "start-date" }
  | { readonly kind: "due-date" }
  | { readonly kind: "own"; readonly id: string };

/** What of an issue the dates are read from. */
interface WithDates {
  readonly start_date?: string | null | undefined;
  readonly due_date?: string | null | undefined;
  readonly own_fields?: Readonly<Record<string, unknown>> | null | undefined;
}

/** A date field by its search word; none for a word that is no date field the person may read. */
export function dateFieldOfWord(word: string, own: OwnColumns): DateField | undefined {
  if (word === listWord.startDate) {
    return { kind: "start-date" };
  }
  if (word === listWord.dueDate) {
    return { kind: "due-date" };
  }
  const column = own[word];
  return column?.kind === "date" ? { kind: "own", id: column.id } : undefined;
}

/** The day an issue keeps in a field; none while it keeps none. */
export function dayKeptIn(issue: WithDates, field: DateField): string | null {
  if (field.kind === "own") {
    const value = issue.own_fields?.[field.id];
    return typeof value === "string" ? value : null;
  }
  return known(field.kind === "start-date" ? issue.start_date : issue.due_date) ?? null;
}

/** What the change of one date field asks the server for; `null` takes the date away. */
export function dayValues(field: DateField, day: string | null): IssueRequestValues {
  if (field.kind === "own") {
    return { own: { [field.id]: day } };
  }
  return field.kind === "start-date" ? { startDate: day } : { dueDate: day };
}

/** The words of the own fields that hold a day, which a screen of dates may stand on besides *Start date* and *Due date*. */
export function ownDateWords(own: OwnColumns): readonly string[] {
  return Object.keys(own).filter((word) => own[word]?.kind === "date");
}
