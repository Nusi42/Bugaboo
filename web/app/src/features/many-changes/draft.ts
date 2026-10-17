import { priorityWords } from "@bugaboo/api-client";

import { wordOf } from "../../shared/present.ts";
import { statusKeyOfChoice } from "../issues/index.ts";
import type { ProjectChoice } from "../fields/index.ts";
import type { RelationChoice } from "../relations/index.ts";
import { manyChangeKind, type ManyChangeKind, type NewManyChangeAction } from "./many-changes.ts";

/** The choice that stands for leaving a field as it is, or for nothing picked yet. */
export const NONE = "";

/** What the person has filled in so far, for whichever of the five things is open. */
export interface Draft {
  readonly status: string;
  readonly priority: string;
  readonly project: ProjectChoice | null;
  readonly label: string;
  /** The key of the picked relation words (`RelationChoice.key`). */
  readonly relation: string;
  /** The other issue of a relation, by its readable ID. */
  readonly other: string;
}

export const emptyDraft: Draft = {
  status: NONE,
  priority: NONE,
  project: null,
  label: NONE,
  relation: NONE,
  other: "",
};

/** The request of a change of status or priority; nothing while both are left as they are. */
function changeOf(draft: Draft): NewManyChangeAction | null {
  const status = draft.status === NONE ? undefined : statusKeyOfChoice(draft.status);
  const priority = wordOf(priorityWords, draft.priority);
  return status === undefined && priority === undefined
    ? null
    : {
        kind: manyChangeKind.change,
        change: {
          ...(status === undefined ? {} : { status }),
          ...(priority === undefined ? {} : { priority }),
        },
      };
}

/** The request of adding a relation; nothing while the words or the other issue are not filled in. */
function relationOf(
  draft: Draft,
  relations: readonly RelationChoice[],
): NewManyChangeAction | null {
  const words = relations.find((each) => each.key === draft.relation);
  return words === undefined || draft.other.trim() === ""
    ? null
    : {
        kind: manyChangeKind.relation,
        relation_kind: words.kind,
        direction: words.direction,
        other: draft.other.trim(),
      };
}

/** What the five things need, as the request says it; nothing while what they need is not filled in. */
export function actionOf(
  kind: ManyChangeKind,
  draft: Draft,
  relations: readonly RelationChoice[],
): NewManyChangeAction | null {
  if (kind === manyChangeKind.change) {
    return changeOf(draft);
  }
  if (kind === manyChangeKind.move) {
    return draft.project === null ? null : { kind, project: draft.project.id };
  }
  if (kind === manyChangeKind.label) {
    return draft.label === NONE ? null : { kind, label: draft.label };
  }
  return kind === manyChangeKind.relation
    ? relationOf(draft, relations)
    : { kind: manyChangeKind.delete };
}
