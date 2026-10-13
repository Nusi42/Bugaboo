import { t } from "@bugaboo/messages";
import { useState } from "react";

import { relationKindLinesOf, useRelationKinds } from "../relation-kinds/index.ts";
import { relationChoices } from "../relations/index.ts";
import {
  bulkKind,
  targetsOf,
  useApplyBulk,
  usePreviewBulk,
  type BulkApplied,
  type BulkKind,
} from "./bulk.ts";
import { actionOf, emptyDraft, type Draft } from "./draft.ts";

/** What is filled in for one of the five things, and the request it makes (nothing while it is not complete). */
function useBulkDraft(kind: BulkKind) {
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const kinds = useRelationKinds();
  const relations = relationChoices(relationKindLinesOf(kinds.data?.pages));
  return {
    draft,
    setDraft,
    relations,
    action: actionOf(kind, draft, relations),
    plain: kind === bulkKind.delete ? t("bulk.line-delete") : "",
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
  const apply = useApplyBulk(onApplied);
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
