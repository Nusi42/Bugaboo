import { dataOf, type Schema } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { timeKeys } from "../../shared/time-keys.ts";

/** What a new time entry on an issue starts with, as the server works it out. */
export type EntryDefaults = Schema<"EntryDefaultsItem">;

/**
 * What a new time entry on an issue starts with (ADR-0035, decision 1): the server works out the
 * values of the issue, its project and the projects above, and says whether *Billable* is offered.
 * It answers only a person who may book time on the issue, so no data also means no time can be
 * booked there: the buttons that book time wait for it.
 */
export function useEntryDefaults(issue: string) {
  return useQuery({
    queryKey: timeKeys.defaults(issue),
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/issues/{issue}/time-entry-defaults", {
          params: { path: { issue } },
        }),
      ),
  });
}
