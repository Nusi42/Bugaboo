import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { ActionOverlay, AnswerNote, alertTone, Icon, iconName, partOf, part } from "@bugaboo/ui";
import { Button } from "react-aria-components";

import { IdsIn } from "./IdsIn.tsx";
import { askTime } from "./time-asks.ts";
import { timerActions, timerMenuTitle } from "./timer-actions.ts";
import { TimerMenuHeading } from "./TimerMenuHeading.tsx";
import { RunsLongWords } from "./RunsLong.tsx";
import { useRunClock } from "./use-run-clock.ts";

export interface TimerChipProps {
  readonly running: Schema<"RunningItem">;
  /** How far the clock of the server is from the one of this browser (`useTimer`). */
  readonly serverOffsetMs: number;
  /** The sidebar is narrowed to icons: the chip shows the clock only. */
  readonly narrow?: boolean;
}

/** What the button says before the clock, for a screen reader: the issue is in its words, or not known. */
function nameOf(issue: string | undefined, narrow: boolean): string {
  if (issue === undefined) {
    return t("time.timer.running-hidden");
  }
  // With the issue drawn beside it the name needs only the words before it; without, the issue is named too.
  return narrow ? t("time.timer.running", { issue }) : t("time.timer.running-on");
}

/**
 * The running timer (TIME-01, UX §4.9): the issue it runs on and how long, in one button that opens
 * *Stop · Switch issue · Change start time · Throw away*. A timer that runs longer than a work day is
 * tinted as a warning. The button is named by its words, issue and clock, so what the eyes read is in
 * its name; in the narrow sidebar the issue is only in the name.
 */
export function TimerChip({ running, serverOffsetMs, narrow = false }: TimerChipProps) {
  const issue = running.readable_id ?? undefined;
  const { clock } = useRunClock(running, serverOffsetMs);
  return (
    <ActionOverlay
      label={timerMenuTitle(issue, clock)}
      heading={<TimerMenuHeading issue={issue} clock={clock} />}
      actions={timerActions()}
      cancelLabel={t("common.cancel")}
      renderTrigger={(onOpen) => (
        <Button
          {...partOf(part.timerChip, {
            variants: [running.runs_long && "long", narrow && "narrow"],
          })}
          {...(onOpen === undefined ? {} : { onPress: onOpen })}
        >
          <span {...partOf(part.visuallyHidden)}>{nameOf(issue, narrow)}</span>
          {running.runs_long ? (
            <Icon name={iconName.warning} />
          ) : (
            <span {...partOf(part.timerChipLive)} />
          )}
          {issue === undefined || narrow ? null : (
            <span {...partOf(part.timerChipIssue)}>{issue}</span>
          )}
          <span {...partOf(part.timerChipTime)}>{clock}</span>
          <RunsLongWords running={running} />
        </Button>
      )}
    />
  );
}

/**
 * The warning of a timer that runs longer than a work day, on the timer itself (UX §6.1): it says
 * what to do and offers both ways on, *Stop* and *Change start time* for a timer that was forgotten.
 */
export function LongTimer({ running }: { readonly running: Schema<"RunningItem"> }) {
  if (!running.runs_long) {
    return null;
  }
  const issue = running.readable_id ?? undefined;
  return (
    <AnswerNote
      tone={alertTone.warning}
      text={
        issue === undefined ? (
          t("time.timer.runs-long")
        ) : (
          <IdsIn sentence={t("time.timer.runs-long-on", { issue })} ids={[issue]} />
        )
      }
      main={{
        label: t("time.timer.stop"),
        onPress: () => {
          askTime({ kind: "stop" });
        },
      }}
      other={{
        label: t("time.timer.change-start"),
        onPress: () => {
          askTime({ kind: "start-time" });
        },
      }}
    />
  );
}
