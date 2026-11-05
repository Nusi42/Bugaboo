import { t } from "@bugaboo/messages";
import { ActionOverlay, iconName, Length, partOf, part } from "@bugaboo/ui";
import { Button } from "react-aria-components";

import { BarPlace } from "../shell/index.ts";
import type { EntryIssue } from "./entry-issue.ts";
import { timerActions, timerMenuTitle } from "./timer-actions.ts";
import { TimerMenuHeading } from "./TimerMenuHeading.tsx";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useRunningClock } from "./use-run-clock.ts";
import { useTimerOn } from "./use-timer-on.ts";

interface RunningPlaceProps {
  readonly issue: EntryIssue;
  /** The timer runs on this issue. */
  readonly here: boolean;
  /** Puts the timer on this issue. */
  readonly press: () => void;
}

/** The timer while it runs, here or on another issue: its clock, and the sheet of what to do with it. */
function RunningPlace({ issue, here, press }: RunningPlaceProps) {
  const { running, clock } = useRunningClock();
  if (running === undefined) {
    return null;
  }
  const on = running.readable_id ?? undefined;
  const prefix = here ? t("time.timer.stop-on", { issue: issue.key }) : t("time.timer.running-on");
  const first = here
    ? []
    : [{ id: "switch-here", label: t("time.timer.switch-here"), onAction: press }];
  return (
    <ActionOverlay
      label={timerMenuTitle(on ?? issue.key, clock)}
      heading={<TimerMenuHeading issue={on ?? issue.key} clock={clock} />}
      actions={timerActions(first)}
      cancelLabel={t("common.cancel")}
      renderTrigger={(onOpen) => (
        <Button
          {...partOf(part.bottomBarItem)}
          {...(onOpen === undefined ? {} : { onPress: onOpen })}
        >
          <span {...partOf(part.visuallyHidden)}>{prefix}</span>
          <BarPlace icon={here ? iconName.stop : iconName.clock}>
            {here || on === undefined ? null : <>{on} </>}
            {clock === undefined ? null : <Length text={clock} />}
          </BarPlace>
        </Button>
      )}
    />
  );
}

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
