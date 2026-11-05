import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { ActionOverlay, Icon, iconName, partOf, part, type MenuAction } from "@bugaboo/ui";
import { Button } from "react-aria-components";

import { useNowEachSecond } from "../../shared/use-now.ts";
import { askTime } from "./time-asks.ts";
import { clockText, elapsedSeconds } from "./timer-clock.ts";

export interface TimerChipProps {
  readonly running: Schema<"RunningItem">;
  /** How far the clock of the server is from the one of this browser (`useTimer`). */
  readonly serverOffsetMs: number;
  /** The sidebar is narrowed to icons: the chip shows the clock only. */
  readonly narrow?: boolean;
}

/** What the timer can be asked to do, each opening its window or question. */
function timerActions(): readonly MenuAction[] {
  return [
    {
      id: "stop",
      label: t("time.timer.stop"),
      onAction: () => {
        askTime({ kind: "stop" });
      },
    },
    {
      id: "switch",
      label: t("time.timer.switch"),
      onAction: () => {
        askTime({ kind: "switch" });
      },
    },
    {
      id: "change-start",
      label: t("time.timer.change-start"),
      onAction: () => {
        askTime({ kind: "start-time" });
      },
    },
    {
      id: "throw-away",
      label: t("time.timer.throw-away"),
      onAction: () => {
        askTime({ kind: "throw-away" });
      },
    },
  ];
}

/**
 * How long the timer has run, counting each second from the server's start. It is part of the name
 * of the button, which is what the eyes read.
 */
function RunTime({ running, serverOffsetMs }: Pick<TimerChipProps, "running" | "serverOffsetMs">) {
  const now = useNowEachSecond();
  return (
    <span {...partOf(part.timerChipTime)}>
      {clockText(elapsedSeconds(running.started_at, serverOffsetMs, now))}
    </span>
  );
}

/**
 * The running timer (TIME-01, UX §4.9): the issue it runs on and how long, in one button that opens
 * *Stop · Switch issue · Change start time · Throw away*. A timer that runs longer than a work day is
 * tinted as a warning.
 */
export function TimerChip({ running, serverOffsetMs, narrow = false }: TimerChipProps) {
  const issue = running.readable_id ?? undefined;
  const name =
    issue === undefined ? t("time.timer.running-hidden") : t("time.timer.running", { issue });
  return (
    <ActionOverlay
      label={t("time.timer.menu")}
      actions={timerActions()}
      cancelLabel={t("common.cancel")}
      renderTrigger={(onOpen) => (
        <Button
          {...partOf(part.timerChip, {
            variants: [running.runs_long && "long", narrow && "narrow"],
          })}
          {...(narrow ? { "aria-label": name } : {})}
          {...(onOpen === undefined ? {} : { onPress: onOpen })}
        >
          {narrow ? (
            <Icon name={iconName.clock} />
          ) : (
            <>
              <span {...partOf(part.visuallyHidden)}>
                {issue === undefined ? t("time.timer.running-hidden") : t("time.timer.running-on")}
              </span>
              <span {...partOf(part.timerChipLive)} />
              {issue === undefined ? null : <span {...partOf(part.timerChipIssue)}>{issue}</span>}
              <RunTime running={running} serverOffsetMs={serverOffsetMs} />
            </>
          )}
        </Button>
      )}
    />
  );
}
