import { t } from "@bugaboo/messages";
import { useState } from "react";

import { useWording } from "../../shared/use-wording.ts";
import { relationKindLinesOf, useRelationKinds } from "../relation-kinds/index.ts";
import { useProjectStatusChanges, type ProjectChange } from "../issues/index.ts";
import { relationChoices } from "../relations/index.ts";
import {
  manyChangeKind,
  mayApply,
  targetsOf,
  useApplyManyChanges,
  usePreviewManyChanges,
  type ManyChangeApplied,
  type ManyChangeKind,
} from "./many-changes.ts";
import { actionOf, emptyDraft, type Draft } from "./draft.ts";

/** What is filled in for one of the five things, and the request it makes (nothing while it is not complete). */
function useManyChangeDraft(kind: ManyChangeKind, statusChanges: readonly ProjectChange[]) {
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const kinds = useRelationKinds();
  const relations = relationChoices(relationKindLinesOf(kinds.data?.pages));
  return {
    draft,
    setDraft,
    relations,
    action: actionOf(kind, draft, { relations, statusChanges }),
    plain: kind === manyChangeKind.delete ? t("many-changes.line-delete") : "",
  };
}

/**
 * What the dialog of changing many issues keeps: what is filled in, the preview of it, and the
 * request that applies it on the versions the preview was made from.
 */
export function useManyChangeDialog(
  kind: ManyChangeKind,
  project: string,
  issues: readonly string[],
  onApplied: (applied: ManyChangeApplied) => void,
) {
  const statusChanges = useProjectStatusChanges(project).data ?? [];
  const { draft, setDraft, relations, action, plain } = useManyChangeDraft(kind, statusChanges);
  const preview = usePreviewManyChanges();
  const words = useWording();
  const apply = useApplyManyChanges(onApplied);
  const changes = preview.data === undefined ? [] : targetsOf(preview.data);
  return {
    draft,
    relations,
    statusChanges,
    preview,
    action,
    plain,
    // *Apply* waits for the words of the preview: nobody starts a change they could not read.
    canApply: mayApply(preview.data !== undefined && changes.length > 0, words),
    fill: (next: Draft): void => {
      setDraft(next);
      preview.reset();
    },
    forget: (): void => {
      setDraft(emptyDraft);
      preview.reset();
    },
    ask: (): void => {
      if (action !== null) {
        preview.mutate({ body: { action, issues: [...issues] } });
      }
    },
    send: (): void => {
      if (action !== null && preview.data !== undefined) {
        apply.mutate({ body: { action, issues: changes } });
      }
    },
  };
}
