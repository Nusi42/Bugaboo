import { dataOf } from "@bugaboo/api-client";
import { useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { queryKeys, refreshRoutes } from "../../shared/query-keys.ts";
import { keepRulesOptions, periodText } from "./keep-rules.ts";

/** How long a file of people's data is kept, in words, from the keep rules; none while they are read. */
export function useExportsKept(): string | undefined {
  const rules = useQuery(keepRulesOptions());
  const kept = rules.data?.rules.find((rule) => rule.kind === "data-exports")?.period;
  return kept === undefined ? undefined : periodText(kept);
}

/** Every file of people's data, newest first, a page at a time. */
export function useDataExports() {
  return usePaged({
    queryKey: queryKeys.dataExports.list(),
    read: (after) =>
      api.client.GET("/api/v1/admin/data-exports", {
        params: { query: { limit: pageSize.usual, ...after } },
      }),
  });
}

/** *Download someone's data*: the file of a person's data, always with a reason; the list is read again after it. */
export function useAskSomeonesData() {
  return useMutation({
    mutationFn: async ({ person, reason }: { readonly person: string; readonly reason: string }) =>
      dataOf(
        await api.client.POST("/api/v1/people/{person}/data-export", {
          params: { path: { person } },
          body: { reason },
        }),
      ),
    onSuccess: () => {
      refreshRoutes(["/api/v1/admin/data-exports"]);
    },
  });
}
