import { codeOf, messageOf } from "../../shared/errors.ts";
import { askTime } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { startWorkDay, tapWorkDay } from "./work-day-requests.ts";
import type { WorkDayNow } from "./work-day-state.ts";

/** What a button of the work day does. */
export type WorkDayAction = "start" | "break-start" | "break-end" | "end";

/** The error code that says the last work day was never ended. */
const LEFT_OPEN = "working-hours.day-left-open";

/**
 * Does what a button of the work day says, on the day the server showed; a refusal is told in the
 * timer part, and *Start work day* with an earlier day still open asks when that day ended.
 */
export function useWorkDayActions(now: WorkDayNow): (action: WorkDayAction) => Promise<void> {
  return async (action) => {
    try {
      if (action === "start") {
        await startWorkDay();
      } else if (now.today !== undefined) {
        await tapWorkDay(action, now.today.version);
      }
    } catch (error) {
      if (codeOf(error) === LEFT_OPEN) {
        askTime({ kind: "open-day" });
      } else {
        tellTime(messageOf(error), "danger");
      }
    }
  };
}
