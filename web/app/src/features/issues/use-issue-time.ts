import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/**
 * The time of an issue (its time spent, its worked-out time left and the totals with its
 * sub-issues), read once for the page, and again once a change of an issue has reached the server
 * (`sendChange`): never for a version on screen that the server has not made yet.
 */
export function useIssueTime(issue: string) {
  return useQuery({
    queryKey: queryKeys.issues.time(issue),
    queryFn: async () =>
      (await api.client.GET("/api/v1/issues/{issue}/time", { params: { path: { issue } } })).data,
  });
}
