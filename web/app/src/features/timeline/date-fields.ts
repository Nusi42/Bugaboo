import { listWord } from "../../shared/list-columns.ts";
import type { TimelineOptions } from "../../shared/list-view.ts";
import {
  dateFieldOfWord,
  dayKeptIn,
  dayValues,
  type DateField,
  type IssueRequestValues,
  type OwnColumns,
} from "../issues/index.ts";
import type { TimelineRow } from "./timeline-data.ts";

/**
 * The two date fields of a timeline (ADR-0043, decision 2): *Start date* and *Due date*, or two own
 * date fields the view names. A drag, a key and *Set dates…* change these fields and no others.
 */

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

/** The date fields of a view; none while the own fields it names are not known yet. */
export function dateFieldsOf(options: TimelineOptions, own: OwnColumns): DateFields | undefined {
  const [start, end] = [
    dateFieldOfWord(options.start_field ?? listWord.startDate, own),
    dateFieldOfWord(options.end_field ?? listWord.dueDate, own),
  ];
  return start === undefined || end === undefined ? undefined : { start, end };
}

/** The dates the issue of a row keeps in the fields of the view. */
export function keptDates(row: TimelineRow, fields: DateFields): KeptDates {
  return { start: dayKeptIn(row.issue, fields.start), end: dayKeptIn(row.issue, fields.end) };
}

/** What a change of both dates asks the server for: the own fields of both together. */
export function changeOf(fields: DateFields, dates: KeptDates): IssueRequestValues {
  const [start, end] = [dayValues(fields.start, dates.start), dayValues(fields.end, dates.end)];
  const own = { ...start.own, ...end.own };
  return { ...start, ...end, ...(Object.keys(own).length === 0 ? {} : { own }) };
}
