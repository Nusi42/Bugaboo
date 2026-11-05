import { t } from "@bugaboo/messages";
import type { MenuAction } from "@bugaboo/ui";

import { askTime } from "./time-asks.ts";

/** The name of the menu of the timer: the issue it runs on and how long, once both are known. */
export function timerMenuTitle(issue: string | undefined, clock: string | undefined): string {
  return issue === undefined || clock === undefined
    ? t("time.timer.menu")
    : t("time.timer.menu-of", { issue, clock });
}

/** What can be asked of a running timer, each opening its window or question. */
export function timerActions(first: readonly MenuAction[] = []): readonly MenuAction[] {
  return [
    ...first,
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
