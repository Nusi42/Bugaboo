import { t } from "@bugaboo/messages";
import { ActionOverlay, Icon, iconName, Length, partOf, part, type IconName } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { Button } from "react-aria-components";

import type { EntryIssue } from "./entry-issue.ts";
import { timerActions } from "./timer-actions.ts";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useRunClock } from "./use-run-clock.ts";
import { useTimer } from "./use-timer.ts";
import { useTimerOn } from "./use-timer-on.ts";

/** A start that is none, for the clock of a timer that does not run. */
const notRunning = { started_at: "" };

/** The words and the icon of a place of the bar, drawn as the places of the bar of the app are. */
function Place({ icon, children }: { readonly icon: IconName; readonly children: ReactNode }) {
  return (
    <>
      <span {...partOf(part.bottomBarLabel)}>{children}</span>
      <span {...partOf(part.bottomBarIcon)}>
        <Icon name={icon} />
      </span>
    </>
  );
}

interface RunningPlaceProps {
  readonly issue: EntryIssue;
  /** The timer runs on this issue. */
  readonly here: boolean;
  /** Puts the timer on this issue. */
  readonly press: () => void;
}

/** The timer while it runs, here or on another issue: its clock, and the sheet of what to do with it. */
function RunningPlace({ issue, here, press }: RunningPlaceProps) {
  const reading = useTimer().data;
  const running = reading?.timer.running ?? undefined;
  const { clock } = useRunClock(running ?? notRunning, reading?.serverOffsetMs ?? 0);
  if (running === undefined) {
    return null;
  }
  const on = running.readable_id ?? undefined;
  const prefix = here ? t("time.timer.stop-on", { issue: issue.key }) : t("time.timer.running-on");
  const sheet =
    clock === undefined
      ? t("time.timer.menu")
      : t("time.timer.menu-of", { issue: on ?? issue.key, clock });
  const first = here
    ? []
    : [{ id: "switch-here", label: t("time.timer.switch-here"), onAction: press }];
  return (
    <ActionOverlay
      label={sheet}
      actions={timerActions(first)}
      cancelLabel={t("common.cancel")}
      renderTrigger={(onOpen) => (
        <Button
          {...partOf(part.bottomBarItem)}
          {...(onOpen === undefined ? {} : { onPress: onOpen })}
        >
          <span {...partOf(part.visuallyHidden)}>{prefix}</span>
          <Place icon={here ? iconName.stop : iconName.clock}>
            {here || on === undefined ? null : <>{on} </>}
            {clock === undefined ? null : <Length text={clock} />}
          </Place>
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
        <Place icon={iconName.play}>{t("time.timer.bar")}</Place>
      </Button>
    );
  }
  const here = state === "here";
  return <RunningPlace issue={issue} here={here} press={press} />;
}
