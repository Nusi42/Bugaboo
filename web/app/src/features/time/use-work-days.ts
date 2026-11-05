import { dataOf } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { addDays, calendarDayOf } from "../../shared/format.ts";
import { timeKeys } from "../../shared/time-keys.ts";
import { useNow } from "../../shared/use-now.ts";
import { workDayNow, type WorkDayNow } from "./work-day-state.ts";

/** How many days back the part looks for a day that was left open. */
const DAYS_BACK = 7;

/** How often the days are read again while the part is on screen, in milliseconds: a day that runs counts until now. */
const REREAD_MS = 60_000;

/** The days from `first` to `last` of the person's own work days (both included). */
export async function readWorkDays(first: string, last: string) {
  return dataOf(
    await api.client.GET("/api/v1/me/work-days", { params: { query: { from: first, to: last } } }),
  );
}

/**
 * The person's work day of today and the days of the week before, as the server shows them, read
 * again when the page gets the focus and every minute. `now` is what a day left
 * open or no day at all comes to.
 */
export function useWorkDayNow(): { readonly now: WorkDayNow | undefined; readonly today: string } {
  const today = calendarDayOf(useNow());
  const first = addDays(today, -DAYS_BACK);
  const days = useQuery({
    queryKey: timeKeys.workDays(first, today),
    queryFn: () => readWorkDays(first, today),
    refetchOnWindowFocus: true,
    refetchInterval: REREAD_MS,
  }).data;
  return { now: days === undefined ? undefined : workDayNow(days.days, today), today };
}
