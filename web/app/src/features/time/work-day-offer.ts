import { queryClient } from "../../shared/query-client.ts";
import { addDays, calendarDayOf } from "../../shared/format.ts";
import { timeKeys } from "../../shared/time-keys.ts";
import { askTime } from "./time-asks.ts";
import { readWorkDays } from "./use-work-days.ts";
import { workDayNow } from "./work-day-state.ts";

/** How many days back the part looks for a day that was left open (as `use-work-days.ts`). */
const DAYS_BACK = 7;

/** The day on which the person said *Not now*, so the question comes once a day. */
let declinedOn: string | undefined;

/** The person said *Not now* to starting the work day with the timer; the question waits until tomorrow. */
export function declineWorkDayToday(): void {
  declinedOn = calendarDayOf(Date.now());
}

/**
 * After a timer was started on `issue`: asks *Start your work day too?* when no work day of today
 * runs (none has started, or it was ended) and the person has not said *Not now* today. Nothing is asked when the days cannot be
 * read (the timer part says what is wrong when the person tries the work day itself).
 */
export async function offerWorkDay(issue: string): Promise<void> {
  const today = calendarDayOf(Date.now());
  if (declinedOn === today) {
    return;
  }
  try {
    const first = addDays(today, -DAYS_BACK);
    const days = await queryClient.query({
      queryKey: timeKeys.workDays(first, today),
      queryFn: () => readWorkDays(first, today),
      staleTime: 0,
    });
    const { kind } = workDayNow(days.days, today);
    if (kind === "none" || kind === "ended") {
      askTime({ kind: "work-day-too", issue });
    }
  } catch {
    // The days could not be read: the work day is not asked about.
  }
}
