import type { Schema } from "@bugaboo/api-client";
import { dataOf } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A status as the server sends it: its key, its name and its status group. */
export type StatusOption = Schema<"StatusRef">;

/**
 * The statuses the issues of a project can have, each once: what a choice of a status for many
 * issues offers. The browser keeps no list of statuses; each issue is still judged by its own workflow.
 */
export function useProjectStatuses(project: string) {
  return useQuery({
    queryKey: queryKeys.issues.statuses(project),
    queryFn: async (): Promise<readonly StatusOption[]> =>
      dataOf(
        await api.client.GET("/api/v1/projects/{project}/statuses", {
          params: { path: { project } },
        }),
      ).statuses,
  });
}
