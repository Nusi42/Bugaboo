import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import type { EditedThing } from "./edited-thing.ts";
import { HeldEditNotice } from "./HeldEditNotice.tsx";
import type { EditLock } from "./use-edit-lock.ts";

export interface EditLockNoticesProps {
  readonly lock: EditLock;
  /** What is being edited, as the note about somebody else's lock names it. */
  readonly thing: EditedThing;
  /** The note that the lock was lost and the changes are kept; it comes with *Try again*. */
  readonly lost: string;
}

/** What the person needs to know about the lock on a layout or a workflow: somebody else holds it, or it was lost. */
export function EditLockNotices({ lock, thing, lost }: EditLockNoticesProps) {
  const { state } = lock;
  if (state.phase === "held") {
    return (
      <HeldEditNotice
        held={state.held}
        thing={thing}
        onAskToFinish={lock.askToFinish}
        onEditAnyway={lock.editAnyway}
      >
        <Button onPress={lock.retry}>{t("common.try-again")}</Button>
      </HeldEditNotice>
    );
  }
  if (state.phase === "lost") {
    return (
      <Alert tone={alertTone.warning}>
        <div {...partOf(part.issueNote)}>
          <span>{lost}</span>
          <Button onPress={lock.retry}>{t("common.try-again")}</Button>
        </div>
      </Alert>
    );
  }
  return state.phase === "taking" ? <p {...partOf(part.muted)}>{t("editing.taking")}</p> : null;
}
