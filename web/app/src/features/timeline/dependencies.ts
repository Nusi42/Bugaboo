import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { refreshIssueRelations } from "../../shared/query-keys.ts";
import { refreshTimeline } from "./timeline-data.ts";
import { relationKindLinesOf, useRelationKinds } from "../relation-kinds/index.ts";

/**
 * Dependencies made on the timeline (PLAN-02, ADR-0043, decision 4): a relation of the kind the
 * organization gave *Must finish first* (*blocks* at the start), from the issue that must finish
 * first to the one that waits. It is the same relation the issue page and the relation map show;
 * the server refuses one that would close a loop.
 */

/** The kind a new dependency gets: the first kind that is not retired and has the date effect. */
export function useDependencyKind(): string | undefined {
  const kinds = useRelationKinds();
  return relationKindLinesOf(kinds.data?.pages).find(
    (kind) => !kind.retired && kind.dateEffect === "must-finish-first",
  )?.id;
}

/** Two issues by their readable IDs: the one that must finish first, then the one that waits. */
export interface Pair {
  readonly first: string;
  readonly waiting: string;
}

/** Makes the dependency, with *Undo*; a refusal (a loop, no right) is said in a notice. */
export async function addDependency(kind: string, { first, waiting }: Pair): Promise<boolean> {
  const made = await api.client.POST("/api/v1/issues/{issue}/relations", {
    params: { path: { issue: first } },
    body: { other: waiting, relation_kind: kind, direction: "forward" },
  });
  refreshIssueRelations();
  await refreshTimeline();
  if (made.data === undefined) {
    undoStore.offer({ message: messageOf(made.error) });
    return false;
  }
  const relation = made.data.id;
  undoStore.offer({
    message: t("timeline.dependency-made", { first, waiting }),
    undo: () => {
      void api.client
        .DELETE("/api/v1/relations/{relation}", { params: { path: { relation } } })
        .then(async () => {
          refreshIssueRelations();
          await refreshTimeline();
        });
    },
  });
  return true;
}
