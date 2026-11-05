import { dataOf } from "@bugaboo/api-client";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { fieldOf, messageOf } from "../../shared/errors.ts";
import { refreshHistory, refreshIssues } from "../../shared/query-keys.ts";
import { entryDetails, type EntryValues } from "./entry-values.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

/** What went wrong with a save, in words: for the whole entry, or at one box. */
export interface EntryProblem {
  readonly text: string;
  /** The ID of the own field the refusal is about; none when it is about the entry. */
  readonly field: string | undefined;
}

export interface AddTimeInput {
  readonly issue: string;
  /** The date the time belongs to (`2027-01-14`). */
  readonly date: string;
  readonly minutes: number;
  readonly values: EntryValues;
  readonly defaults: EntryDefaults;
}

/** The save of *Add time*: books the length on the date; the problem when the server refuses it. */
export function useAddTime(onBooked: () => void) {
  const [problem, setProblem] = useState<EntryProblem | undefined>();
  const [isPending, setPending] = useState(false);
  const add = async (input: AddTimeInput): Promise<void> => {
    setPending(true);
    setProblem(undefined);
    try {
      dataOf(
        await api.client.POST("/api/v1/me/time-entries", {
          body: {
            issue: input.issue,
            date: input.date,
            minutes: input.minutes,
            entry: entryDetails(input.values, input.defaults),
          },
        }),
      );
      // What shows the time of the issue is read again: its time spent, its totals, its history.
      refreshIssues();
      refreshHistory();
      onBooked();
    } catch (error) {
      setProblem({ text: messageOf(error), field: fieldOf(error) });
    } finally {
      setPending(false);
    }
  };
  return { add, problem, isPending };
}
