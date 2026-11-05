import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { StopTimerWindow } from "./StopTimerWindow.tsx";
import { closeTimeAsk } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { throwAwayTimer } from "./timer-requests.ts";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useEntryIssue } from "./use-entry-issue.ts";
import { useTimer } from "./use-timer.ts";

/**
 * A timer whose time cannot be saved (its issue is not seen any more, or time cannot be booked
 * there): the reason in words, and *Throw away* as the way out.
 */
function CannotSave({ why, version }: { readonly why: string; readonly version: number }) {
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          closeTimeAsk();
        }
      }}
      title={t("time.stop.cannot-title")}
      text={why}
      confirmLabel={t("time.timer.throw-away")}
      cancelLabel={t("common.close")}
      confirmLook={buttonLook.danger}
      onConfirm={() => {
        throwAwayTimer(version).catch((error: unknown) => {
          tellTime(messageOf(error), "danger");
        });
      }}
    />
  );
}

/**
 * Finds what *Stop timer* needs (the running timer, its issue and what a new entry on it starts
 * with) and shows the window. A timer on an issue the person cannot see or book on any more says why
 * and offers *Throw away*.
 */
export function StopTimerAsk({ switchingTo }: { readonly switchingTo: string | undefined }) {
  const reading = useTimer().data;
  const running = reading?.timer.running ?? undefined;
  const key = running?.readable_id ?? undefined;
  const issue = useEntryIssue(key);
  const defaults = useEntryDefaults(key ?? "", key !== undefined);
  if (reading === undefined || running === undefined) {
    return null;
  }
  if (key === undefined) {
    return <CannotSave why={t("time.stop.hidden-issue")} version={reading.timer.version} />;
  }
  if (defaults.isError) {
    return <CannotSave why={messageOf(defaults.error)} version={reading.timer.version} />;
  }
  if (issue === undefined || defaults.data === undefined) {
    return null;
  }
  const windowKey = [key, reading.timer.version].join(":");
  return (
    <StopTimerWindow
      key={windowKey}
      reading={reading}
      issue={issue}
      defaults={defaults.data}
      switchingTo={switchingTo}
    />
  );
}
