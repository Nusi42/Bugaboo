import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { ActionOverlay, partOf, part } from "@bugaboo/ui";
import { Button } from "react-aria-components";

import { timerActions } from "./timer-actions.ts";
import { useRunClock } from "./use-run-clock.ts";

export interface TimerChipProps {
  readonly running: Schema<"RunningItem">;
  /** How far the clock of the server is from the one of this browser (`useTimer`). */
  readonly serverOffsetMs: number;
  /** The sidebar is narrowed to icons: the chip shows the clock only. */
  readonly narrow?: boolean;
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
  const prefix = issue === undefined ? t("time.timer.running-hidden") : t("time.timer.running-on");
  const sheet =
    issue === undefined || clock === undefined
      ? t("time.timer.menu")
      : t("time.timer.menu-of", { issue, clock });
  return (
    <ActionOverlay
      label={sheet}
      actions={timerActions()}
      cancelLabel={t("common.cancel")}
      renderTrigger={(onOpen) => (
        <Button
          {...partOf(part.timerChip, {
            variants: [running.runs_long && "long", narrow && "narrow"],
          })}
          {...(onOpen === undefined ? {} : { onPress: onOpen })}
        >
          <span {...partOf(part.visuallyHidden)}>
            {narrow ? `${prefix} ${issue ?? ""}` : prefix}
          </span>
          <span {...partOf(part.timerChipLive)} />
          {issue === undefined || narrow ? null : (
            <span {...partOf(part.timerChipIssue)}>{issue}</span>
          )}
          <span {...partOf(part.timerChipTime)}>{clock}</span>
        </Button>
      )}
    />
  );
}
