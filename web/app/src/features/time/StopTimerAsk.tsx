import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { closeTimeAsk, askTime } from "./time-asks.ts";
import { StopTimerWindow } from "./StopTimerWindow.tsx";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useEntryIssue } from "./use-entry-issue.ts";
import { useTimer } from "./use-timer.ts";

/**
 * Finds what *Stop timer* needs (the running timer, its issue and what a new entry on it starts
 * with) and shows the window. A person who can no longer book time on the issue is offered only
 * *Throw away*, with the reason.
 */
export function StopTimerAsk({ switchingTo }: { readonly switchingTo: string | undefined }) {
  const reading = useTimer().data;
  const key = reading?.timer.running?.readable_id ?? undefined;
  const issue = useEntryIssue(key);
  const defaults = useEntryDefaults(key ?? "", key !== undefined);
  if (reading === undefined || key === undefined) {
    return null;
  }
  if (defaults.isError) {
    return (
      <Alert tone={alertTone.danger}>
        <span>{messageOf(defaults.error)}</span>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            askTime({ kind: "throw-away" });
          }}
        >
          {t("time.timer.throw-away")}
        </Button>
        <Button look={buttonLook.quiet} onPress={closeTimeAsk}>
          {t("common.close")}
        </Button>
      </Alert>
    );
  }
  if (issue === undefined || defaults.data === undefined) {
    return null;
  }
  return (
    <StopTimerWindow
      reading={reading}
      issue={issue}
      defaults={defaults.data}
      switchingTo={switchingTo}
    />
  );
}
