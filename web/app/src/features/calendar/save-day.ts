import type { Refused } from "../../shared/refused.ts";
import {
  dayValues,
  saveOnNewest,
  takeVersion as takeIssueVersion,
  type DateField,
} from "../issues/index.ts";
import { idOf, type CalendarEntry } from "./calendar-data.ts";

/**
 * A drag, a key and *Change due date …* all end here (PLAN-09): the date field of the view is changed on
 * the issue like any change of it, one after the other, each on the version the one before made
 * (`saveOnNewest`, which the timeline uses too). The server judges every change.
 */

/** The version a change of somebody else made: *Use mine* of a clash saves the day of the person on it. */
export function takeVersion(entry: CalendarEntry, version: number): void {
  takeIssueVersion(idOf(entry), entry.issue.version, version);
}

/** Saves the day in the date field, after every save of the issue that this screen asked for before. */
export function saveDay(
  entry: CalendarEntry,
  field: DateField,
  day: string | null,
): Promise<Refused | null> {
  return saveOnNewest(idOf(entry), entry.issue.version, dayValues(field, day));
}
