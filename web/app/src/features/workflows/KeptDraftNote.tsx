import { t } from "@bugaboo/messages";
import { Alert, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import type { Draft } from "./draft-model.ts";

export interface KeptDraftNoteProps {
  /** The draft that a refused publish left; nothing when there is none. */
  readonly left: Draft | undefined;
  /** Puts the changes in place of what is on screen. */
  readonly onRestore: (draft: Draft) => void;
  /** Throws the changes away. */
  readonly onDrop: () => void;
}

/**
 * What a refused publish left behind: the workflow changed while the person was editing, so they
 * see it as it is now, and the changes they made are kept. They put them back on the new workflow
 * to look at them and publish, or throw them away; nothing is lost without their saying so.
 */
export function KeptDraftNote({ left, onRestore, onDrop }: KeptDraftNoteProps) {
  if (left === undefined) {
    return null;
  }
  return (
    <Alert>
      <div {...partOf(part.issueNote)}>
        <span>{t("workflows.draft-changed")}</span>
        <Button
          look={buttonLook.primary}
          onPress={() => {
            onRestore(left);
          }}
        >
          {t("workflows.draft-restore")}
        </Button>
        <Button onPress={onDrop}>{t("workflows.draft-drop")}</Button>
      </div>
    </Alert>
  );
}
