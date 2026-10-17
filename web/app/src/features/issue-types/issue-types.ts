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
  /** Whether the person looks after the type: where it was created, they hold the permission. */
  readonly mayManage: boolean;
  /** Where it was created; the whole organization when none. */
  readonly createdIn: Schema<"PlaceRef"> | undefined;
  /** What the project asked about says of the type; only when the list was asked about a project. */
  readonly here: IssueTypeHere | undefined;
}

/** Whether the project a list was asked about offers a type, and whether the person may change that. */
export interface IssueTypeHere {
  readonly offered: boolean;
  /** The line the project has of its own; none while it follows the project above. */
  readonly own: boolean | undefined;
  readonly mayChange: boolean;
}

/** The line of an issue type, or nothing for an item that came without what it needs. */
export function issueTypeLineOf(item: Schema<"IssueTypeItem">): IssueTypeLine | undefined {
  const { id, name, icon, retired, version } = item;
  const here: IssueTypeHere | undefined =
    item.offered_here === undefined
      ? undefined
      : {
          offered: item.offered_here ?? true,
          own: item.own_offer ?? undefined,
          mayChange: item.may_change_offer === true,
        };
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
    : {
        id,
        name,
        icon,
        retired,
        version,
        mayManage: item.may_manage === true,
        createdIn: item.created_in ?? undefined,
        here,
      };
}

/** The issue types in the pages read so far, in the order they are offered in. */
export function issueTypeLinesOf(
  pages: readonly Schema<"Page_IssueTypeItem">[] | undefined,
): IssueTypeLine[] {
  return linesFrom(pages, issueTypeLineOf);
}

/**
 * The issue types, a page at a time (they are few: one page holds them). With a project (a short
 * name or an ID), each type says whether the project offers it.
 */
export function useIssueTypes(project?: string) {
  return usePaged(
    {
      queryKey: queryKeys.issueTypes.list(project),
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
                "created_in",
                "may_manage",
                ...(project === undefined
                  ? []
                  : (["offered_here", "own_offer", "may_change_offer"] as const)),
              ]),
              ...(project === undefined ? {} : { project }),
              limit: pageSize.largest,
              ...after,
            },
          },
        }),
    },
    { enabled: project !== "" },
  );
}
