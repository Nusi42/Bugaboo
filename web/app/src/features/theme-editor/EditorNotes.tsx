import { t } from "@bugaboo/messages";
import { SaveStatus, type SaveState } from "@bugaboo/ui";

import { DraftNote } from "../../shared/editing/index.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import type { KeptDraft } from "./use-kept-draft.ts";

/** The state the note shows: changes that are not saved yet. */
const NOT_SAVED: SaveState = "not-saved";

/** *Not saved yet*, a quiet note next to the title while the editor has changes that are not saved. */
export function UnsavedNote() {
  return <SaveStatus state={NOT_SAVED} labels={saveLabels()} />;
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
