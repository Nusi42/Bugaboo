import { t } from "@bugaboo/messages";

import { messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import { relationKindLinesOf, useRelationKinds } from "../relation-kinds/index.ts";
import { createRelation, deleteRelation } from "../relations/index.ts";
import { refreshTimeline, type TimelineRow } from "./timeline-data.ts";

/**
 * Dependencies made on the timeline (PLAN-02, ADR-0043, decision 4): a relation of the kind the
 * organization gave *Must finish first* (*blocks* at the start), from the issue that must finish
 * first to the one that waits. It is made and taken back as every relation is (`relations/`); the
 * server decides who may make it and refuses one that would close a loop, and its reason is said.
 */

/** The kind a new dependency gets: the first kind that is not retired and has the date effect. */
function useDependencyKind(): string | undefined {
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

/** Takes a dependency back (*Undo*); a refusal is said. */
async function takeBack(relation: string): Promise<void> {
  try {
    await deleteRelation(relation);
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
  await refreshTimeline();
}

/** Makes the dependency, with *Undo*; a refusal (a loop, no right) says the server's reason. */
async function addDependency(kind: string, { first, waiting }: Pair): Promise<void> {
  try {
    const made = await createRelation(first, {
      other: waiting,
      relation_kind: kind,
      direction: "forward",
    });
    undoStore.offer({
      message: t("timeline.dependency-made", { first, waiting }),
      undo: () => {
        void takeBack(made.id);
      },
    });
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
  await refreshTimeline();
}

/** The two ways to make a dependency: by the issues of two rows (the dot), or by readable IDs. */
export function useLinks(rowOf: (issue: string) => TimelineRow | undefined) {
  const kind = useDependencyKind();
  const onPair = (pair: Pair): void => {
    if (kind === undefined) {
      undoStore.offer({ message: t("timeline.no-dependency-kind") });
    } else {
      void addDependency(kind, pair);
    }
  };
  return {
    onPair,
    onLink: (first: string, waiting: string): void => {
      const [from, to] = [
        known(rowOf(first)?.issue.readable_id),
        known(rowOf(waiting)?.issue.readable_id),
      ];
      if (from !== undefined && to !== undefined) {
        onPair({ first: from, waiting: to });
      }
    },
  };
}
