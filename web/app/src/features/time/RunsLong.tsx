import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

/**
 * The words of the warning on the timer itself (UX §4.9) of a timer that runs longer than a work day, for
 * a screen reader: after the clock in the name of the timer's button. The mark is a warning icon.
 */
export function RunsLongWords({ running }: { readonly running: Schema<"RunningItem"> }) {
  return running.runs_long ? (
    <span {...partOf(part.visuallyHidden)}> {t("time.timer.runs-long")}</span>
  ) : null;
}
