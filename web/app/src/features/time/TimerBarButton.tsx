import { t } from "@bugaboo/messages";
import { iconName, partOf, part } from "@bugaboo/ui";
import { Button } from "react-aria-components";

import { BarPlace } from "../shell/index.ts";
import type { EntryIssue } from "./entry-issue.ts";
import { RunningPlace } from "./RunningPlace.tsx";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useTimerOn } from "./use-timer-on.ts";

/**
 * *Timer* in the bar of an open issue on a phone (UX §3.3, M3-F178, M6-F81): *Timer* with a play
 * mark while no timer runs, which starts it here; while it runs here the clock with a stop mark,
 * and on another issue that issue and its clock. A press while it runs opens *Stop · Switch issue ·
 * Change start time · Throw away* (and *Switch to this issue* for a timer on another issue). It is
 * there only for a person who may book time on the issue.
 */
export function TimerBarButton({ issue }: { readonly issue: EntryIssue }) {
  const defaults = useEntryDefaults(issue.key).data;
  const { state, press } = useTimerOn(issue.key);
  if (defaults === undefined) {
    return null;
  }
  if (state === "idle") {
    return (
      <Button {...partOf(part.bottomBarItem)} onPress={press}>
        <BarPlace icon={iconName.play}>{t("time.timer.bar")}</BarPlace>
      </Button>
    );
  }
  const here = state === "here";
  return <RunningPlace issue={issue} here={here} press={press} />;
}
