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
 * the issue, or stops it while it runs here. It is there only for a person who may book time on the
 * issue, which the server tells by answering what a new entry starts with.
 */
export function TimerButton({ issue }: TimerButtonProps) {
  const defaults = useEntryDefaults(issue.key).data;
  const { state, press } = useTimerOn(issue.key);
  if (defaults === undefined) {
    return null;
  }
  return state === "here" ? (
    <IconButton icon={iconName.stop} label={t("time.timer.stop-timer")} onPress={press} />
  ) : (
    <IconButton icon={iconName.play} label={t("time.timer.start")} onPress={press} />
  );
}
