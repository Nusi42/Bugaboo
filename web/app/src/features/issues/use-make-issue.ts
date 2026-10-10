import { dataOf } from "@bugaboo/api-client";
import { useMutation } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { bodyOf, type NewIssueValues } from "./new-issue-form.ts";

/** Makes an issue in a project; the lists of issues are read again afterwards. */
export function useMakeIssue(project: string | undefined) {
  return useMutation({
    mutationFn: async (values: NewIssueValues) =>
      dataOf(
        await api.client.POST("/api/v1/projects/{project}/issues", {
          params: { path: { project: project ?? "" } },
          body: bodyOf(values),
        }),
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.issues.lists() });
    },
  });
}
