import { t } from "@bugaboo/messages";

import { minutesText } from "../../shared/own-value-text.ts";
import { tellTime } from "./time-notice.ts";
import type { TimerSaved } from "./timer-requests.ts";

/**
 * Tells what a stop or a switch of the timer came to: the time that was saved on the issue, or, for
 * a timer that ran under a minute, that nothing was saved (`issue` is the one the timer ran on).
 */
export function tellSaved(saved: TimerSaved, issue: string): void {
  const entry = saved.entry;
  if (entry === undefined || entry === null) {
    tellTime(t("time.timer.too-short", { issue }), "info", { ids: [issue] });
    return;
  }
  const length = minutesText(entry.minutes);
  const on = entry.readable_id ?? issue;
  tellTime(t("time.timer.saved", { length, issue: on }), "success", {
    ids: [on],
    lengths: [length],
  });
}
