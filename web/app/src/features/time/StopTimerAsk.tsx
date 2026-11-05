import { t } from "@bugaboo/messages";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { CannotSave } from "./CannotSave.tsx";
import { StopTimerWindow } from "./StopTimerWindow.tsx";
import { StopUnderMinute } from "./StopUnderMinute.tsx";
import { wasUnderMinute } from "./timer-clock.ts";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useEntryIssue } from "./use-entry-issue.ts";
import { useTimer } from "./use-timer.ts";

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
  // How long it had run when the question came: whole minutes, as the server saves them. It is counted
  // to the moment the question opened, and only once the timer is loaded: a timer that is not loaded yet
  // is never taken for a short one.
  const [openedAt] = useState(() => Date.now());
  const under = wasUnderMinute(running?.started_at, reading?.serverOffsetMs ?? 0, openedAt);
  if (reading === undefined || running === undefined) {
    return null;
  }
  if (key === undefined) {
    return <CannotSave why={t("time.stop.hidden-issue")} version={reading.timer.version} />;
  }
  // Under a minute nothing is saved: a question, not a window with fields.
  if (under === true && switchingTo === undefined) {
    return <StopUnderMinute issue={key} version={reading.timer.version} />;
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
