import { readStored, storageKey, writeStored } from "@bugaboo/ui";

import { queryClient } from "../../shared/query-client.ts";
import { calendarDayOf } from "../../shared/format.ts";
import { askTime } from "./time-asks.ts";
import { workDaysQuery } from "./use-work-days.ts";
import { workDayNow } from "./work-day-state.ts";

/** The storage of this tab: *Not now* is kept while the tab is open, also through a reload, and never beyond it. */
const thisTab = (): Storage => sessionStorage;

/** The person said *Not now* to starting the work day with the timer; the question waits until tomorrow. */
export function declineWorkDayToday(): void {
  writeStored(storageKey.workDayDeclined, calendarDayOf(Date.now()), thisTab);
}

/**
 * After a timer was started on `issue`: asks *Start your work day too?* when no work day of today
 * runs (none has started, or it was ended) and the person has not said *Not now* today. Nothing is asked when the days cannot be
 * read (the timer part says what is wrong when the person tries the work day itself).
 */
export async function offerWorkDay(issue: string): Promise<void> {
  const today = calendarDayOf(Date.now());
  if (readStored(storageKey.workDayDeclined, thisTab) === today) {
    return;
  }
  try {
    const days = await queryClient.query({ ...workDaysQuery(today), staleTime: 0 });
    const { kind } = workDayNow(days.days, today);
    if (kind === "none" || kind === "ended") {
      askTime({ kind: "work-day-too", issue });
    }
  } catch {
    // The days could not be read: the work day is not asked about.
  }
}
