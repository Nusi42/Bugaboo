import { dataOf, type Schema } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { fieldList, pageSize, usePaged } from "../../shared/paged.ts";
import { known } from "../../shared/present.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import type { IssueDetail } from "./slots.ts";

/** An issue in a list, as the server sends it: the fields can be missing. */
export type ListedIssue = Schema<"IssueItem">;

/** The fields of an issue in a list that the screens need (PERF-02). */
const LIST_FIELDS = fieldList<ListedIssue>([
  "id",
  "key",
  "issue_type",
  "title",
  "status",
  "status_group",
  "priority",
  "assigned_to",
  "updated_at",
]);

/** An issue that has what a line of a list needs. */
export interface IssueLine {
  readonly id: string;
  /** The readable ID (`WEB-142`). */
  readonly key: string;
  readonly title: string;
  readonly status: Schema<"Status">;
  /** The group of the status: it gives the tint of the row and the icon (UX §4.2). */
  readonly group: Schema<"StatusGroup">;
  readonly priority: Schema<"Priority">;
  readonly issueType: Schema<"IssueTypeRef">;
  /** The people it is assigned to, as the reader may see them. */
  readonly assignedTo: readonly Schema<"PersonRef">[];
  /** When it was last changed, if the reader may see it. */
  readonly changedAt: string | undefined;
}

/** The line of an issue, or nothing for an item that came without what it needs. */
export function lineOf(item: ListedIssue): IssueLine | undefined {
  const [id, key, title] = [known(item.id), known(item.key), known(item.title)];
  const [status, group, priority, issueType] = [
    known(item.status),
    known(item.status_group),
    known(item.priority),
    known(item.issue_type),
  ];
  return id === undefined ||
    key === undefined ||
    title === undefined ||
    status === undefined ||
    group === undefined ||
    priority === undefined ||
    issueType === undefined
    ? undefined
    : {
        id,
        key,
        title,
        status,
        group,
        priority,
        issueType,
        assignedTo: item.assigned_to ?? [],
        changedAt: known(item.updated_at),
      };
}

/** The issues of a project the person sees, a page at a time. */
export function useProjectIssues(project: string) {
  return usePaged({
    queryKey: queryKeys.issues.list(project),
    read: (after) =>
      api.client.GET("/api/v1/projects/{project}/issues", {
        params: {
          path: { project },
          query: { fields: LIST_FIELDS, limit: pageSize.usual, ...after },
        },
      }),
  });
}

/** One issue by its readable or internal ID, with what the person may do with it; nothing is read without an ID. */
export function useIssue(address: string | undefined) {
  const issue = address ?? "";
  return useQuery({
    queryKey: queryKeys.issues.one(issue),
    queryFn: async (): Promise<IssueDetail> =>
      dataOf(await api.client.GET("/api/v1/issues/{issue}", { params: { path: { issue } } })),
    enabled: address !== undefined,
  });
}
