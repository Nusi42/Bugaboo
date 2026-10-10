import { type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** An issue type that has what a line of the list and its editor need. */
export interface IssueTypeLine {
  readonly id: string;
  readonly name: string;
  readonly icon: Schema<"IssueIcon">;
  readonly retired: boolean;
  /** The version the editor looked at; a change is made on it. */
  readonly version: number;
}

/** The line of an issue type, or nothing for an item that came without what it needs. */
export function issueTypeLineOf(item: Schema<"IssueTypeItem">): IssueTypeLine | undefined {
  const { id, name, icon, retired, version } = item;
  return id === undefined ||
    id === null ||
    name === undefined ||
    name === null ||
    icon === undefined ||
    icon === null ||
    retired === undefined ||
    retired === null ||
    version === undefined ||
    version === null
    ? undefined
    : { id, name, icon, retired, version };
}

/** The issue types in the pages read so far, in the order they are offered in. */
export function issueTypeLinesOf(
  pages: readonly Schema<"Page_IssueTypeItem">[] | undefined,
): IssueTypeLine[] {
  return linesFrom(pages, issueTypeLineOf);
}

/** The issue types, a page at a time (they are few: one page holds them). */
export function useIssueTypes() {
  return usePaged({
    queryKey: queryKeys.issueTypes.list(),
    read: (after) =>
      api.client.GET("/api/v1/issue-types", {
        params: {
          query: {
            fields: fieldList<Schema<"IssueTypeItem">>([
              "id",
              "name",
              "icon",
              "retired",
              "version",
            ]),
            limit: pageSize.largest,
            ...after,
          },
        },
      }),
  });
}
