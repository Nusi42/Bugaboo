import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { editedThing } from "../../shared/editing/edited-thing.ts";
import { HeldEditNotice } from "../../shared/editing/HeldEditNotice.tsx";
import type { LayoutLock } from "./use-layout-lock.ts";

/** What the person needs to know about the lock on the layout: somebody else holds it, or it was lost. */
export function EditNotices({ lock }: { readonly lock: LayoutLock }) {
  const { state } = lock;
  if (state.phase === "held") {
    return (
      <HeldEditNotice
        held={state.held}
        thing={editedThing.layout}
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
          <span>{t("layouts.lost")}</span>
          <Button onPress={lock.retry}>{t("common.try-again")}</Button>
        </div>
      </Alert>
    );
  }
  return state.phase === "taking" ? <p {...partOf(part.muted)}>{t("layouts.taking")}</p> : null;
}
