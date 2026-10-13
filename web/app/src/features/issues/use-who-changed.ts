import { dataOf } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { personName } from "../../shared/person.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** How many of the newest changes are read to find the last change of one text. */
const RECENT = 30;

/**
 * Who changed a long text of an issue last, as the reader may see people of the kind *Authors*
 * (M3-F54): the newest change of that part in the history. Nothing while it is read, when the
 * change is not among the newest ones, or when the history does not name the person. Asked only
 * `when` the page says the text changed after the person started.
 */
export function useWhoChanged(issue: string, part: string, when: boolean): string | undefined {
  const found = useQuery({
    queryKey: queryKeys.history.list(issue, `changes:${part}`),
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/issues/{issue}/history", {
          params: { path: { issue }, query: { tab: "changes", limit: RECENT } },
        }),
      ),
    enabled: when,
  });
  for (const item of found.data?.items ?? []) {
    if (item.kind === "change" && item.field === part) {
      return item.by === undefined || item.by === null ? undefined : personName(item.by);
    }
  }
  return undefined;
}
