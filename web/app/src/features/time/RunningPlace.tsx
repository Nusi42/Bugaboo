import { t } from "@bugaboo/messages";
import { ActionOverlay, iconName, Length, partOf, part } from "@bugaboo/ui";
import { Button } from "react-aria-components";

import { BarPlace } from "../shell/index.ts";
import type { EntryIssue } from "./entry-issue.ts";
import { RunsLongWords } from "./RunsLong.tsx";
import { timerActions, timerMenuTitle } from "./timer-actions.ts";
import { TimerMenuHeading } from "./TimerMenuHeading.tsx";
import { useRunningClock } from "./use-run-clock.ts";

export interface RunningPlaceProps {
  readonly issue: EntryIssue;
  /** The timer runs on this issue. */
  readonly here: boolean;
  /** Puts the timer on this issue. */
  readonly press: () => void;
}

/** The mark of the place: a warning for a timer that runs longer than a work day, else the stop or the clock. */
function barIcon(here: boolean, runsLong: boolean) {
  if (runsLong) {
    return iconName.warning;
  }
  return here ? iconName.stop : iconName.clock;
}

/** The timer while it runs, here or on another issue: its clock, and the sheet of what to do with it. */
export function RunningPlace({ issue, here, press }: RunningPlaceProps) {
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
          <BarPlace icon={barIcon(here, running.runs_long)}>
            {here || on === undefined ? null : <>{on} </>}
            {clock === undefined ? null : <Length text={clock} />}
          </BarPlace>
          <RunsLongWords running={running} />
        </Button>
      )}
    />
  );
}
