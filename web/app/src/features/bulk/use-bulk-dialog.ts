import { t } from "@bugaboo/messages";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useLabels } from "../labels/index.ts";
import { relationKindLinesOf, useRelationKinds } from "../relation-kinds/index.ts";
import { relationChoices } from "../relations/index.ts";
import {
  bulkKind,
  targetsOf,
  useApplyBulk,
  usePreviewBulk,
  type BulkAction,
  type BulkKind,
  type BulkPreview,
  type BulkResult,
} from "./bulk.ts";
import { actionOf, emptyDraft, type Draft } from "./draft.ts";

/** What came of applying a change: what the server answered, or why it did not work. */
export interface BulkApplied {
  readonly action: BulkAction;
  readonly preview: BulkPreview;
  readonly result: BulkResult | null;
  readonly failed: string | null;
}

/** The line under an issue that changes when there is no more to say than the thing itself. */
function plainLine(kind: BulkKind, draft: Draft, labelName: string | undefined): string {
  if (kind === bulkKind.move && draft.project !== null) {
    return t("bulk.line-move", { project: draft.project.name });
  }
  if (kind === bulkKind.label && labelName !== undefined) {
    return t("bulk.line-label", { label: labelName });
  }
  return kind === bulkKind.delete ? t("bulk.line-delete") : "";
}

/** What is filled in for one of the five things, and the request it makes (nothing while it is not complete). */
function useBulkDraft(kind: BulkKind) {
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const labels = useLabels();
  const kinds = useRelationKinds();
  const relations = relationChoices(relationKindLinesOf(kinds.data?.pages));
  const labelName = labels.data?.labels.find((each) => each.id === draft.label)?.name;
  return {
    draft,
    setDraft,
    relations,
    action: actionOf(kind, draft, relations),
    plain: plainLine(kind, draft, labelName),
  };
}

/**
 * What the dialog of changing many issues keeps: what is filled in, the preview of it, and the
 * request that applies it on the versions the preview was made from.
 */
export function useBulkDialog(
  kind: BulkKind,
  issues: readonly string[],
  onApplied: (applied: BulkApplied) => void,
) {
  const { draft, setDraft, relations, action, plain } = useBulkDraft(kind);
  const preview = usePreviewBulk();
  const apply = useApplyBulk();
  const changes = preview.data === undefined ? [] : targetsOf(preview.data);
  return {
    draft,
    relations,
    preview,
    action,
    plain,
    canApply: preview.data !== undefined && changes.length > 0,
    fill: (next: Draft): void => {
      setDraft(next);
      preview.reset();
    },
    forget: (): void => {
      setDraft(emptyDraft);
      preview.reset();
      apply.reset();
    },
    ask: (): void => {
      if (action !== null) {
        preview.mutate({ body: { action, issues: [...issues] } });
      }
    },
    send: (): void => {
      const shown = preview.data;
      if (action === null || shown === undefined) {
        return;
      }
      apply.mutate(
        { body: { action, issues: changes } },
        {
          onSuccess: (result: BulkResult) => {
            onApplied({ action, preview: shown, result, failed: null });
          },
          onError: (error: unknown) => {
            onApplied({ action, preview: shown, result: null, failed: messageOf(error) });
          },
        },
      );
    },
  };
}
