import { dataOf, type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { fieldList, pageSize, usePaged } from "../../shared/paged.ts";
import { queryKeys, refreshIssueRelations } from "../../shared/query-keys.ts";
import type { RelationKindLine } from "../relation-kinds/index.ts";

/** What a block of the issue page needs of the page: the readable ID of the issue. */
export interface BlockProps {
  readonly readableId: string;
}

/** How an issue stands among the others: the issue it sits under and the progress of its parts. */
export function useIssueRelations(issue: string) {
  return api.query.useQuery("get", "/api/v1/issues/{issue}/relations", {
    params: { path: { issue } },
  });
}

/** The sub-issues of an issue the person may see, a page at a time. */
export function useSubIssues(issue: string) {
  return usePaged({
    queryKey: queryKeys.issueRelations.subIssues(issue),
    read: (after) =>
      api.client.GET("/api/v1/issues/{issue}/sub-issues", {
        params: {
          path: { issue },
          query: {
            fields: fieldList<Schema<"IssueRef">>(["id", "key", "issue_type", "title", "status"]),
            limit: pageSize.short,
            ...after,
          },
        },
      }),
  });
}

/** The related issues of an issue whose other end the person may see, a page at a time. */
export function useRelatedIssues(issue: string) {
  return usePaged({
    queryKey: queryKeys.issueRelations.related(issue),
    read: (after) =>
      api.client.GET("/api/v1/issues/{issue}/related-issues", {
        params: {
          path: { issue },
          query: {
            fields: fieldList<Schema<"RelatedIssueItem">>([
              "id",
              "kind",
              "direction",
              "name",
              "issue",
            ]),
            limit: pageSize.short,
            ...after,
          },
        },
      }),
  });
}

/**
 * Puts the issue `child` under `parent`, or takes it out (`null`). A change is made on the version
 * that was looked at, so the child is read first.
 */
export async function setParentOf(child: string, parent: string | null): Promise<void> {
  const issue = dataOf(
    await api.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: child } } }),
  );
  const version = issue.issue.version;
  if (version === undefined || version === null) {
    return;
  }
  dataOf(
    await api.client.PUT("/api/v1/issues/{issue}/parent", {
      params: { path: { issue: child } },
      body: { version, parent },
    }),
  );
  refreshIssueRelations();
}

/** What a person picks when they add a relation: one of the words of a kind, and which end it is. */
export interface RelationChoice {
  readonly key: string;
  readonly kind: string;
  readonly direction: Schema<"Direction">;
  readonly name: string;
}

/**
 * The words to pick from when adding a relation: for each kind that is not retired, what the
 * issue reads (*blocks*) and, when it is another word, what the other one reads (*is blocked by*).
 * A kind that reads alike from both ends is offered once.
 */
export function relationChoices(kinds: readonly RelationKindLine[]): RelationChoice[] {
  return kinds
    .filter((kind) => !kind.retired)
    .flatMap((kind) => {
      const forward: RelationChoice = {
        key: `${kind.id}:forward`,
        kind: kind.id,
        direction: "forward",
        name: kind.forward,
      };
      return kind.backward.toLowerCase() === kind.forward.toLowerCase()
        ? [forward]
        : [
            forward,
            {
              key: `${kind.id}:backward`,
              kind: kind.id,
              direction: "backward",
              name: kind.backward,
            },
          ];
    });
}
