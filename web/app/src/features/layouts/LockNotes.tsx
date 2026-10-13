import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { formatSince } from "../../shared/format.ts";
import { useNow } from "../../shared/use-now.ts";
import type { LayoutLock } from "./use-layout-lock.ts";

/** What the person needs to know about the lock on the layout: somebody else holds it, or it was lost. */
export function LockNotes({ lock }: { readonly lock: LayoutLock }) {
  const now = useNow();
  const { state } = lock;
  if (state.phase === "held") {
    return (
      <Alert tone={alertTone.info}>
        <div {...partOf(part.issueNote)}>
          <span>
            {t("layouts.is-editing", { name: state.name, since: formatSince(state.since, now) })}
          </span>
          <Button onPress={lock.askToFinish}>
            {t("layouts.ask-finish", { name: state.name })}
          </Button>
          {state.canEditAnyway ? (
            <Button onPress={lock.editAnyway}>{t("layouts.edit-anyway")}</Button>
          ) : null}
          <Button onPress={lock.retry}>{t("layouts.try-again")}</Button>
        </div>
      </Alert>
    );
  }
  if (state.phase === "lost") {
    return (
      <Alert tone={alertTone.warning}>
        <div {...partOf(part.issueNote)}>
          <span>{t("layouts.lost")}</span>
          <Button onPress={lock.retry}>{t("layouts.try-again")}</Button>
        </div>
      </Alert>
    );
  }
  return state.phase === "taking" ? <p {...partOf(part.muted)}>{t("layouts.taking")}</p> : null;
}
