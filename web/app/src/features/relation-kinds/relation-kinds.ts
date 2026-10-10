import { type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A kind of relation that has what a line of the list and its editor need. */
export interface RelationKindLine {
  readonly id: string;
  /** What the issue the relation starts from reads (*blocks*). */
  readonly forward: string;
  /** What the other issue reads (*is blocked by*). */
  readonly backward: string;
  readonly retired: boolean;
  /** The version the editor looked at; a change is made on it. */
  readonly version: number;
}

/** The line of a kind of relation, or nothing for an item that came without what it needs. */
export function relationKindLineOf(item: Schema<"RelationKindItem">): RelationKindLine | undefined {
  const { id, forward, backward, retired, version } = item;
  return id === undefined ||
    id === null ||
    forward === undefined ||
    forward === null ||
    backward === undefined ||
    backward === null ||
    retired === undefined ||
    retired === null ||
    version === undefined ||
    version === null
    ? undefined
    : { id, forward, backward, retired, version };
}

/** The kinds of relation in the pages read so far, in the order they are offered in. */
export function relationKindLinesOf(
  pages: readonly Schema<"Page_RelationKindItem">[] | undefined,
): RelationKindLine[] {
  return linesFrom(pages, relationKindLineOf);
}

/** The kinds of relation, a page at a time (they are few: one page holds them). */
export function useRelationKinds() {
  return usePaged({
    queryKey: queryKeys.relationKinds.list(),
    read: (after) =>
      api.client.GET("/api/v1/relation-kinds", {
        params: {
          query: {
            fields: fieldList<Schema<"RelationKindItem">>([
              "id",
              "forward",
              "backward",
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
