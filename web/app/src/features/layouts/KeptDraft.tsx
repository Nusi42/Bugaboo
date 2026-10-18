import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { DraftNote } from "../../shared/editing/index.ts";
import { formatAgo } from "../../shared/format.ts";
import { useNow } from "../../shared/use-now.ts";
import { type Draft } from "./editor-model.ts";
import { differsFromLayout, restoredDraft } from "./kept-draft.ts";
import { useKeptDraft, useThrowAwayKeptDraft } from "./layouts.ts";

export interface KeptDraftProps {
  readonly editor: Schema<"LayoutEditorBody">;
  readonly layout: string;
  /** Puts the changes of the draft into the editor. */
  readonly onRestore: (draft: Draft) => void;
}

/**
 * What a refused save left behind: the layout changed while the person was editing, so they see it
 * as it is now, and the changes they made are kept as their draft. They put them back on the new
 * layout to look at them and save, or throw them away; nothing is lost without their saying so.
 */
export function KeptDraft({ editor, layout, onRestore }: KeptDraftProps) {
  const found = useKeptDraft(layout);
  const drop = useThrowAwayKeptDraft(layout);
  const now = useNow();
  const [back, setBack] = useState(false);
  const kept = found.data;
  // A draft is offered when it holds changes: after a lost lock it started from this very version.
  if (back || kept === null || kept === undefined || !differsFromLayout(editor, kept.text)) {
    return null;
  }
  const when = formatAgo(kept.saved_at, now);
  const restored = restoredDraft(editor, kept.text);
  return (
    <DraftNote
      text={t("layouts.draft-changed", { when })}
      take={
        restored === undefined
          ? undefined
          : {
              label: t("layouts.draft-restore"),
              onPress: () => {
                onRestore(restored);
                setBack(true);
              },
            }
      }
      drop={{
        label: t("editing.changes-drop"),
        isPending: drop.isPending,
        onPress: () => {
          drop.mutate();
        },
      }}
    />
  );
}
