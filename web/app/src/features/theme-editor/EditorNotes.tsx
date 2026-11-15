import { t } from "@bugaboo/messages";
import { SaveStatus, type SaveState, keepsRoom } from "@bugaboo/ui";

import { DraftNote } from "../../shared/editing/index.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import type { KeptDraft } from "./use-kept-draft.ts";

/**
 * *Not saved yet*, a quiet note beside *Save theme* while the editor has changes that are not saved,
 * and *Saved* there once a save went through, until the next change (R7: where it happened). It is
 * there, silent, before the first change too, so the bar keeps its shape and a screen reader hears
 * the first change (UX §6.4).
 */
export function UnsavedNote({
  dirty,
  saved,
}: {
  readonly dirty: boolean;
  /** A save went through since the editor opened. */
  readonly saved: boolean;
}) {
  const state: SaveState = dirty ? "not-saved" : saved ? "saved" : "idle";
  return <SaveStatus state={state} labels={saveLabels()} variant={keepsRoom} />;
}

/** The note that the editor started from changes the person left, with the way back to the saved theme. */
export function KeptDraftNote({ kept }: { readonly kept: KeptDraft }) {
  return kept.kept ? (
    <DraftNote
      text={t("theme-editor.draft-kept")}
      drop={{ label: t("editing.draft-drop"), onPress: kept.drop }}
    />
  ) : null;
}
