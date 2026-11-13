import { listWord } from "../../shared/list-columns.ts";
import type { TimelineOptions } from "../../shared/list-view.ts";
import { known } from "../../shared/present.ts";
import type { IssueRequestValues, OwnColumns } from "../issues/index.ts";
import type { TimelineRow } from "./timeline-data.ts";

/**
 * The two date fields of a timeline (ADR-0043, decision 2): *Start date* and *Due date*, or two own
 * date fields the view names. A drag, a key and *Set dates…* change these fields and no others.
 */

/** Where a date of the view is kept on an issue: a built-in field, or an own field by its ID. */
type DateField =
  | { readonly at: "start-date" }
  | { readonly at: "due-date" }
  | { readonly at: "own"; readonly id: string };

/** The two date fields of a view. */
export interface DateFields {
  readonly start: DateField;
  readonly end: DateField;
}

/** The dates of an issue as they are kept: `null` where it has none. */
export interface KeptDates {
  readonly start: string | null;
  readonly end: string | null;
}

/** A date field by its search word; none for a word that is no date field the person may read. */
function fieldOfWord(word: string, own: OwnColumns): DateField | undefined {
  if (word === listWord.startDate) {
    return { at: "start-date" };
  }
  if (word === listWord.dueDate) {
    return { at: "due-date" };
  }
  const column = own[word];
  return column?.kind === "date" ? { at: "own", id: column.id } : undefined;
}

/** The date fields of a view; none while the own fields it names are not known yet. */
export function dateFieldsOf(options: TimelineOptions, own: OwnColumns): DateFields | undefined {
  const [start, end] = [
    fieldOfWord(options.start_field ?? listWord.startDate, own),
    fieldOfWord(options.end_field ?? listWord.dueDate, own),
  ];
  return start === undefined || end === undefined ? undefined : { start, end };
}

/** The date an issue keeps in a field. */
function keptIn(row: TimelineRow, field: DateField): string | null {
  const { issue } = row;
  if (field.at === "own") {
    const value = issue.own_fields?.[field.id];
    return typeof value === "string" ? value : null;
  }
  return known(field.at === "start-date" ? issue.start_date : issue.due_date) ?? null;
}

/** The dates the issue of a row keeps in the fields of the view. */
export function keptDates(row: TimelineRow, fields: DateFields): KeptDates {
  return { start: keptIn(row, fields.start), end: keptIn(row, fields.end) };
}

/** What the change of one field asks the server for. */
function valuesOf(field: DateField, day: string | null): IssueRequestValues {
  if (field.at === "own") {
    return { own: { [field.id]: day } };
  }
  return field.at === "start-date" ? { startDate: day } : { dueDate: day };
}

/** What a change of both dates asks the server for: the own fields of both together. */
export function changeOf(fields: DateFields, dates: KeptDates): IssueRequestValues {
  const [start, end] = [valuesOf(fields.start, dates.start), valuesOf(fields.end, dates.end)];
  const own = { ...start.own, ...end.own };
  return { ...start, ...end, ...(Object.keys(own).length === 0 ? {} : { own }) };
}
