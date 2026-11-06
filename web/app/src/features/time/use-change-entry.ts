import { useState } from "react";

import type { TimeEntry } from "./edits.ts";
import { changeEntry } from "./use-changes.ts";
import { entryDetails, type EntryValues } from "./entry-values.ts";
import { problemOf, type EntryProblem } from "./use-add-time.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

/** What the details of an entry start with: what the entry holds now. */
export function valuesOfEntry(entry: TimeEntry): EntryValues {
  return { note: entry.note, billable: entry.billable === true, fields: { ...entry.own_fields } };
}

/** The values the entry holds now, which a change is compared with: only what differs is sent. */
export function baselineOf(entry: TimeEntry, defaults: EntryDefaults): EntryDefaults {
  return { ...defaults, own_fields: entry.own_fields };
}

export interface ChangeEntryInput {
  readonly entry: TimeEntry;
  readonly minutes: number;
  readonly values: EntryValues;
  readonly defaults: EntryDefaults;
}

/** The save of the details of an entry: its length, note, *Billable* and fields, on its version. */
export function useChangeEntry(onChanged: () => void) {
  const [problem, setProblem] = useState<EntryProblem | undefined>();
  const [isPending, setPending] = useState(false);
  const change = async (input: ChangeEntryInput): Promise<void> => {
    setPending(true);
    setProblem(undefined);
    try {
      await changeEntry({
        entryId: input.entry.id,
        version: input.entry.version,
        minutes: input.minutes,
        entry: entryDetails(input.values, baselineOf(input.entry, input.defaults)),
      });
      onChanged();
    } catch (error) {
      setProblem(problemOf(error));
    } finally {
      setPending(false);
    }
  };
  return { change, problem, isPending };
}
