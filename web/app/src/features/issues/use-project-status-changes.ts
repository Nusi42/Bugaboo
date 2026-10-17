import { dataOf } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import type { ProjectChange } from "./status-choices.ts";

/**
 * The status changes the issues of a project can have, by their words, each once: what a choice of
 * a status change for many issues offers. The browser keeps no list of statuses; each issue is
 * still judged by its own workflow.
 */
export function useProjectStatusChanges(project: string) {
  return useQuery({
    queryKey: queryKeys.issues.statusChanges(project),
    queryFn: async (): Promise<readonly ProjectChange[]> =>
      dataOf(
        await api.client.GET("/api/v1/projects/{project}/statuses", {
          params: { path: { project } },
        }),
      ).changes,
  });
}
