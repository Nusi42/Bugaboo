import type { Schema } from "@bugaboo/api-client";

import { forApi } from "../../shared/reference.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

/**
 * What a person fills in for one time entry besides its issue, its date and its length: the note,
 * *Billable* and the values of the own fields of time entries by the key of the field. The form
 * of an entry (`EntryForm`) shows and changes it; the screens that save an entry send it with
 * `entryDetails`. The time-entry form of the timesheet (WP-10) uses the same three.
 */
export interface EntryValues {
  readonly note: string;
  readonly billable: boolean;
  /** The values as the boxes hold them; `null` or none is no value. */
  readonly fields: Readonly<Record<string, unknown>>;
}

/** The values a new entry starts with: no note, not billable, and the defaults the server worked out. */
export function startingValues(defaults: EntryDefaults): EntryValues {
  return { note: "", billable: false, fields: { ...defaults.own_fields } };
}

/** Whether two values of a field are the same value for the API. */
function sameValue(left: unknown, right: unknown): boolean {
  return JSON.stringify(forApi(left) ?? null) === JSON.stringify(forApi(right) ?? null);
}

/**
 * The values of an entry as the API takes them. Only a field the person changed from its default is
 * sent (`null` takes the value away); the server fills in the rest with the same defaults, so the
 * rule that works them out stays in one place.
 */
export function entryDetails(
  values: EntryValues,
  defaults: EntryDefaults,
): Schema<"EntryDetailsBody"> {
  const fields: Record<string, unknown> = {};
  for (const key of new Set([...Object.keys(values.fields), ...Object.keys(defaults.own_fields)])) {
    if (!sameValue(values.fields[key], defaults.own_fields[key])) {
      fields[key] = forApi(values.fields[key]) ?? null;
    }
  }
  return {
    note: values.note,
    ...(defaults.billable_offered ? { billable: values.billable } : {}),
    fields,
  };
}
