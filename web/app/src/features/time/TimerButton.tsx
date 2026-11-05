import { t } from "@bugaboo/messages";
import { IconButton, iconName } from "@bugaboo/ui";

import type { EntryIssue } from "./entry-issue.ts";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useTimerOn } from "./use-timer-on.ts";

export interface TimerButtonProps {
  readonly issue: EntryIssue;
}

/**
 * The play button beside *Time spent* on the issue page (TIME-01, UX §3.4): it starts the timer on
 * the issue (while it runs here the row *Your timer* has *Stop*). It is there only for a person who may book time on the
 * issue, which the server tells by answering what a new entry starts with.
 */
export function TimerButton({ issue }: TimerButtonProps) {
  const defaults = useEntryDefaults(issue.key).data;
  const { state, press } = useTimerOn(issue.key);
  // While the timer runs here the row *Your timer* has *Stop*, so this row has no second way to stop it.
  if (defaults === undefined || state === "here") {
    return null;
  }
  return <IconButton icon={iconName.play} label={t("time.timer.start")} onPress={press} />;
}
