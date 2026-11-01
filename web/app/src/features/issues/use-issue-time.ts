import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/**
 * The time of an issue (its time spent, its worked-out time left and the totals with its
 * sub-issues), read once for the page, again when the issue has a new version.
 */
export function useIssueTime(issue: string, version: number) {
  return useQuery({
    queryKey: queryKeys.issues.time(issue, version),
    queryFn: async () =>
      (await api.client.GET("/api/v1/issues/{issue}/time", { params: { path: { issue } } })).data,
  });
}
