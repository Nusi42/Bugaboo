import { dataOf } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import type { EntryIssue } from "./entry-issue.ts";

/**
 * The issue a time entry is booked on, as the forms of time need it: its title, project and type.
 * It is the read of the issue page (the same key), so an open issue is not read twice.
 */
export function useEntryIssue(key: string | undefined): EntryIssue | undefined {
  const address = key ?? "";
  const found = useQuery({
    queryKey: queryKeys.issues.one(address),
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: address } } }),
      ),
    enabled: key !== undefined,
  }).data;
  const project = found?.issue.project?.short_name;
  return key === undefined || found === undefined || project === undefined
    ? undefined
    : { key, title: found.issue.title ?? "", project, issueType: found.issue.issue_type?.id };
}
