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
 * change is not among the newest ones, or when the history does not name the person. The answer
 * is kept for the `version` the page names (the version the issue has now), so a name that was
 * read for an older change never shows beside a newer one.
 */
export function useWhoChanged(issue: string, part: string, version: number): string | undefined {
  const found = useQuery({
    queryKey: queryKeys.history.changedBy(issue, part, version),
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/issues/{issue}/history", {
          params: { path: { issue }, query: { tab: "changes", limit: RECENT } },
        }),
      ),
  });
  for (const item of found.data?.items ?? []) {
    if (item.kind === "change" && item.change?.field === part) {
      return item.by === undefined || item.by === null ? undefined : personName(item.by);
    }
  }
  return undefined;
}
