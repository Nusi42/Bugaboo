import { t } from "@bugaboo/messages";

/** The title of the window that starts the timer, or switches it to another issue while it runs. */
export function switchIssueTitle(timerRuns: boolean): string {
  return timerRuns ? t("time.switch.title") : t("time.timer.start-title");
}

/** The title of *Change start time*. */
export function changeStartTitle(): string {
  return t("time.start-time.title");
}
