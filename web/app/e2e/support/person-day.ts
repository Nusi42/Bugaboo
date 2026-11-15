import { dataOf } from "@bugaboo/api-client";

import type { ApiSession } from "./api.ts";
import { momentOfClock, personToday, zoneOfNewDay } from "../../src/shared/clock.ts";
import { addDays } from "../../src/shared/format.ts";
import { regionOf } from "../../src/shared/region.ts";

// The days of a person as the server keeps them (TIME-08), and the one module of days of the tests: a day lies on its own
// date in its zone, and the date and the zone are the person's (their *Region*, else the device's). The tests read them
// the way the app does, from the person's settings, never from the clock of the computer of the test alone: near
// midnight the zone of the computer and the zone of the person show different dates.

export { addDays } from "../../src/shared/format.ts";
export { daysBetween } from "../../src/shared/clock.ts";

/**
 * The day `days` after today (`2027-03-01`), as a person of the tests counts it who keeps the *Region* of the settings,
 * as every person of the tests does: today in the zone of their new days.
 */
export function dayFromToday(days: number): string {
  return addDays(personToday(regionOf(), Date.now()), days);
}

/** A minute: a day starts one minute after the midnight of its date at the earliest. */
const FIRST_MINUTE = 1;

/** Today and yesterday as the person of the session counts them, a day `back` days before today, and the zone their
 * new days lie in. */
export async function daysOf(
  session: ApiSession,
  back = 1,
): Promise<{
  readonly today: string;
  readonly yesterday: string;
  readonly before: string;
  readonly zone: string;
}> {
  const region = regionOf(dataOf(await session.client.GET("/api/v1/me/settings")));
  const today = personToday(region, Date.now());
  return {
    today,
    yesterday: addDays(today, -1),
    before: addDays(today, -back),
    zone: zoneOfNewDay(region.timeZone),
  };
}

/** When the last day of the person before `date` ended, in milliseconds; 0 when none did. */
async function endBefore(session: ApiSession, date: string): Promise<number> {
  const known = dataOf(
    await session.client.GET("/api/v1/me/work-days", {
      params: { query: { from: addDays(date, -1), to: date } },
    }),
  );
  let end = 0;
  for (const day of known.days) {
    if (day.date < date) {
      for (const period of day.periods) {
        end = Math.max(end, Date.parse(period.end ?? period.start));
      }
    }
  }
  return end;
}

/**
 * The moment `seconds` ago, but not before a day of the person's today may start: one minute after its midnight in
 * their zone, and not before their day before ended. A test in the first hour after midnight starts the day as early
 * as its date allows.
 */
export async function earlierToday(session: ApiSession, seconds: number): Promise<string> {
  const { today, zone } = await daysOf(session);
  const now = Date.now();
  const earliest = Math.max(
    Date.parse(momentOfClock(today, FIRST_MINUTE, zone)),
    await endBefore(session, today),
  );
  return new Date(Math.min(now, Math.max(now - seconds * 1000, earliest))).toISOString();
}
