import { t } from "@bugaboo/messages";
import { Button } from "@bugaboo/ui";

import type { EntryIssue } from "./entry-issue.ts";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useTimerOn } from "./use-timer-on.ts";

/**
 * *Timer* in the bar of an open issue on a phone (UX §3.3, M3-F178): it starts the timer on the
 * issue, goes on with it when the timer runs on another issue (*Switch issue*), and opens *Stop
 * timer* while it runs here. It is there only for a person who may book time on the issue.
 */
export function TimerBarButton({ issue }: { readonly issue: EntryIssue }) {
  const defaults = useEntryDefaults(issue.key).data;
  const { state, press } = useTimerOn(issue.key);
  if (defaults === undefined) {
    return null;
  }
  return (
    <Button wide onPress={press}>
      {state === "here" ? t("time.timer.stop-timer") : t("time.timer.bar")}
    </Button>
  );
}
